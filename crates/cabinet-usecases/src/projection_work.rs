use cabinet_domain::document::DocumentId;
use cabinet_domain::projection_work::{
    ProjectionChangeKind, ProjectionKind, ProjectionWork, ProjectionWorkIdentity,
};
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::projection_work::{
    ProjectionEnqueueOutcome, ProjectionWorkRepository, ProjectionWorkRepositoryError,
};

use crate::document::DocumentChangeEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnqueueProjectionWorkUsecase;

impl Default for EnqueueProjectionWorkUsecase {
    fn default() -> Self {
        Self::new()
    }
}

impl EnqueueProjectionWorkUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        event: DocumentChangeEvent,
        repository: &mut impl ProjectionWorkRepository,
    ) -> Result<EnqueueProjectionWorkOutput, EnqueueProjectionWorkError> {
        let (workspace_id, document_id, version_id, change_kind) = versioned_identity(event)?;
        let workspace_id = WorkspaceId::new(&workspace_id)
            .map_err(|_| EnqueueProjectionWorkError::InvalidIdentity)?;
        let document_id = DocumentId::new(&document_id)
            .map_err(|_| EnqueueProjectionWorkError::InvalidIdentity)?;
        let version_id =
            VersionId::new(&version_id).map_err(|_| EnqueueProjectionWorkError::InvalidIdentity)?;

        let mut enqueued_count = 0;
        let mut duplicate_count = 0;
        for kind in [
            ProjectionKind::Search,
            ProjectionKind::Links,
            ProjectionKind::Graph,
        ] {
            let identity = ProjectionWorkIdentity::for_change(
                workspace_id.clone(),
                document_id.clone(),
                version_id.clone(),
                kind,
                change_kind,
            );
            match repository
                .enqueue(ProjectionWork::pending(identity))
                .map_err(map_repository_error)?
            {
                ProjectionEnqueueOutcome::Enqueued => enqueued_count += 1,
                ProjectionEnqueueOutcome::AlreadyExists => duplicate_count += 1,
            }
        }
        Ok(EnqueueProjectionWorkOutput {
            enqueued_count,
            duplicate_count,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnqueueProjectionWorkOutput {
    enqueued_count: usize,
    duplicate_count: usize,
}

impl EnqueueProjectionWorkOutput {
    pub const fn enqueued_count(self) -> usize {
        self.enqueued_count
    }

    pub const fn duplicate_count(self) -> usize {
        self.duplicate_count
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnqueueProjectionWorkError {
    UnsupportedEvent,
    InvalidIdentity,
    RepositoryUnavailable,
    RepositoryCorrupted,
    RepositoryConflict,
}

impl EnqueueProjectionWorkError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::UnsupportedEvent => "projection_enqueue.unsupported_event",
            Self::InvalidIdentity => "projection_enqueue.invalid_identity",
            Self::RepositoryUnavailable => "projection_enqueue.repository_unavailable",
            Self::RepositoryCorrupted => "projection_enqueue.repository_corrupted",
            Self::RepositoryConflict => "projection_enqueue.repository_conflict",
        }
    }
}

fn versioned_identity(
    event: DocumentChangeEvent,
) -> Result<(String, String, String, ProjectionChangeKind), EnqueueProjectionWorkError> {
    match event {
        DocumentChangeEvent::DocumentCreated {
            workspace_id,
            document_id,
            version_id,
            ..
        } => Ok((
            workspace_id,
            document_id,
            version_id,
            ProjectionChangeKind::Created,
        )),
        DocumentChangeEvent::DocumentUpdated {
            workspace_id,
            document_id,
            version_id,
            ..
        } => Ok((
            workspace_id,
            document_id,
            version_id,
            ProjectionChangeKind::Updated,
        )),
        DocumentChangeEvent::DocumentRestored {
            workspace_id,
            document_id,
            restored_version_id,
            ..
        } => Ok((
            workspace_id,
            document_id,
            restored_version_id,
            ProjectionChangeKind::Restored,
        )),
        DocumentChangeEvent::DocumentRenamed {
            workspace_id,
            document_id,
            version_id,
            ..
        } => Ok((
            workspace_id,
            document_id,
            version_id,
            ProjectionChangeKind::Renamed,
        )),
        DocumentChangeEvent::DocumentDeleted {
            workspace_id,
            document_id,
            version_id,
        } => Ok((
            workspace_id,
            document_id,
            version_id,
            ProjectionChangeKind::Deleted,
        )),
        DocumentChangeEvent::DocumentAssetAttached {
            workspace_id,
            document_id,
            version_id,
            ..
        } => Ok((
            workspace_id,
            document_id,
            version_id,
            ProjectionChangeKind::AssetAttached,
        )),
    }
}

fn map_repository_error(error: ProjectionWorkRepositoryError) -> EnqueueProjectionWorkError {
    match error {
        ProjectionWorkRepositoryError::StorageUnavailable => {
            EnqueueProjectionWorkError::RepositoryUnavailable
        }
        ProjectionWorkRepositoryError::CorruptedRecord
        | ProjectionWorkRepositoryError::UnsupportedSchema => {
            EnqueueProjectionWorkError::RepositoryCorrupted
        }
        ProjectionWorkRepositoryError::InvalidLimit
        | ProjectionWorkRepositoryError::NotFound
        | ProjectionWorkRepositoryError::Conflict => EnqueueProjectionWorkError::RepositoryConflict,
    }
}
