use cabinet_domain::projection_work::{
    ProjectionChangeKind, ProjectionKind, ProjectionWork, ProjectionWorkEvent,
    ProjectionWorkIdentity, ProjectionWorkState,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_projection_catalog::{
    CurrentDocumentProjectionCatalog, CurrentDocumentProjectionCatalogError,
};
use cabinet_ports::projection_work::{
    ProjectionEnqueueOutcome, ProjectionWorkRepository, ProjectionWorkRepositoryError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebuildRestoreProjectionsInput {
    workspace_id: String,
}
impl RebuildRestoreProjectionsInput {
    pub fn new(workspace_id: &str) -> Self {
        Self {
            workspace_id: workspace_id.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RebuildRestoreProjectionsOutput {
    document_count: usize,
    enqueued_count: usize,
    duplicate_count: usize,
    reset_count: usize,
}
impl RebuildRestoreProjectionsOutput {
    pub const fn document_count(self) -> usize {
        self.document_count
    }
    pub const fn enqueued_count(self) -> usize {
        self.enqueued_count
    }
    pub const fn duplicate_count(self) -> usize {
        self.duplicate_count
    }

    pub const fn reset_count(self) -> usize {
        self.reset_count
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebuildRestoreProjectionsError {
    InvalidInput,
    CatalogUnavailable,
    CatalogCorrupted,
    CapacityExceeded,
    RepositoryUnavailable,
    RepositoryCorrupted,
}
impl RebuildRestoreProjectionsError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "RESTORE_PROJECTION_INVALID_INPUT",
            Self::CatalogUnavailable => "RESTORE_PROJECTION_CATALOG_UNAVAILABLE",
            Self::CatalogCorrupted => "RESTORE_PROJECTION_CATALOG_CORRUPTED",
            Self::CapacityExceeded => "RESTORE_PROJECTION_CAPACITY_EXCEEDED",
            Self::RepositoryUnavailable => "RESTORE_PROJECTION_REPOSITORY_UNAVAILABLE",
            Self::RepositoryCorrupted => "RESTORE_PROJECTION_REPOSITORY_CORRUPTED",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RebuildRestoreProjectionsUsecase {
    document_limit: usize,
}
impl RebuildRestoreProjectionsUsecase {
    pub const fn new(document_limit: usize) -> Self {
        Self { document_limit }
    }

    pub fn execute(
        &self,
        input: RebuildRestoreProjectionsInput,
        catalog: &impl CurrentDocumentProjectionCatalog,
        repository: &mut impl ProjectionWorkRepository,
    ) -> Result<RebuildRestoreProjectionsOutput, RebuildRestoreProjectionsError> {
        if self.document_limit == 0 {
            return Err(RebuildRestoreProjectionsError::InvalidInput);
        }
        let workspace = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| RebuildRestoreProjectionsError::InvalidInput)?;
        let identities = catalog
            .list_current_projection_identities(&workspace, self.document_limit)
            .map_err(map_catalog_error)?;
        let mut enqueued_count = 0;
        let mut duplicate_count = 0;
        let mut reset_count = 0;
        for current in &identities {
            for kind in [
                ProjectionKind::Search,
                ProjectionKind::Links,
                ProjectionKind::Graph,
            ] {
                for change_kind in [
                    ProjectionChangeKind::Created,
                    ProjectionChangeKind::Updated,
                    ProjectionChangeKind::Restored,
                    ProjectionChangeKind::Renamed,
                    ProjectionChangeKind::Deleted,
                    ProjectionChangeKind::AssetAttached,
                    ProjectionChangeKind::AssetDetached,
                ] {
                    let existing_identity = ProjectionWorkIdentity::for_change(
                        workspace.clone(),
                        current.document_id().clone(),
                        current.version_id().clone(),
                        kind,
                        change_kind,
                    );
                    let Some(existing) = repository
                        .get(&existing_identity)
                        .map_err(map_repository_error)?
                    else {
                        continue;
                    };
                    if existing.state() != ProjectionWorkState::Failed {
                        continue;
                    }
                    let reset = existing
                        .transition(ProjectionWorkEvent::ReindexRequested)
                        .map_err(|_| RebuildRestoreProjectionsError::RepositoryCorrupted)?;
                    repository
                        .replace(reset, ProjectionWorkState::Failed)
                        .map_err(map_repository_error)?;
                    reset_count += 1;
                }
                let identity = ProjectionWorkIdentity::for_change(
                    workspace.clone(),
                    current.document_id().clone(),
                    current.version_id().clone(),
                    kind,
                    ProjectionChangeKind::Restored,
                );
                match repository
                    .enqueue(ProjectionWork::pending(identity))
                    .map_err(map_repository_error)?
                {
                    ProjectionEnqueueOutcome::Enqueued => enqueued_count += 1,
                    ProjectionEnqueueOutcome::AlreadyExists => duplicate_count += 1,
                }
            }
        }
        Ok(RebuildRestoreProjectionsOutput {
            document_count: identities.len(),
            enqueued_count,
            duplicate_count,
            reset_count,
        })
    }
}

fn map_catalog_error(
    error: CurrentDocumentProjectionCatalogError,
) -> RebuildRestoreProjectionsError {
    match error {
        CurrentDocumentProjectionCatalogError::InvalidLimit => {
            RebuildRestoreProjectionsError::InvalidInput
        }
        CurrentDocumentProjectionCatalogError::LimitExceeded => {
            RebuildRestoreProjectionsError::CapacityExceeded
        }
        CurrentDocumentProjectionCatalogError::StorageUnavailable => {
            RebuildRestoreProjectionsError::CatalogUnavailable
        }
        CurrentDocumentProjectionCatalogError::CorruptedRecord => {
            RebuildRestoreProjectionsError::CatalogCorrupted
        }
    }
}
fn map_repository_error(error: ProjectionWorkRepositoryError) -> RebuildRestoreProjectionsError {
    match error {
        ProjectionWorkRepositoryError::CorruptedRecord
        | ProjectionWorkRepositoryError::UnsupportedSchema => {
            RebuildRestoreProjectionsError::RepositoryCorrupted
        }
        _ => RebuildRestoreProjectionsError::RepositoryUnavailable,
    }
}
