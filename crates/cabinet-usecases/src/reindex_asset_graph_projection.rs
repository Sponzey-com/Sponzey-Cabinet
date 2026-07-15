use cabinet_domain::document::DocumentId;
use cabinet_domain::projection_work::{
    ProjectionChangeKind, ProjectionKind, ProjectionWork, ProjectionWorkEvent,
    ProjectionWorkIdentity,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_version::{
    CurrentDocumentVersionPointerError, CurrentDocumentVersionPointerPort,
};
use cabinet_ports::projection_work::{ProjectionWorkRepository, ProjectionWorkRepositoryError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReindexAssetGraphProjectionInput {
    workspace_id: String,
    document_id: String,
    change_kind: ProjectionChangeKind,
}

impl ReindexAssetGraphProjectionInput {
    pub fn new(
        workspace_id: &str,
        document_id: &str,
        change_kind: ProjectionChangeKind,
    ) -> Result<Self, ReindexAssetGraphProjectionError> {
        if !matches!(
            change_kind,
            ProjectionChangeKind::AssetAttached | ProjectionChangeKind::AssetDetached
        ) {
            return Err(ReindexAssetGraphProjectionError::InvalidInput);
        }
        Ok(Self {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            change_kind,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReindexAssetGraphProjectionOutput {
    enqueued: usize,
    reset: usize,
    already_active: usize,
    already_ready: usize,
}

impl ReindexAssetGraphProjectionOutput {
    pub const fn enqueued_count(self) -> usize {
        self.enqueued
    }
    pub const fn reset_count(self) -> usize {
        self.reset
    }
    pub const fn already_active_count(self) -> usize {
        self.already_active
    }
    pub const fn already_ready_count(self) -> usize {
        self.already_ready
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReindexAssetGraphProjectionError {
    InvalidInput,
    CurrentVersionNotFound,
    PointerUnavailable,
    RepositoryUnavailable,
    RepositoryConflict,
    CorruptedState,
    InvalidTransition,
}

impl ReindexAssetGraphProjectionError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "asset_graph_reindex.invalid_input",
            Self::CurrentVersionNotFound => "asset_graph_reindex.current_version_not_found",
            Self::PointerUnavailable => "asset_graph_reindex.pointer_unavailable",
            Self::RepositoryUnavailable => "asset_graph_reindex.repository_unavailable",
            Self::RepositoryConflict => "asset_graph_reindex.repository_conflict",
            Self::CorruptedState => "asset_graph_reindex.corrupted_state",
            Self::InvalidTransition => "asset_graph_reindex.invalid_transition",
        }
    }

    pub const fn retryable(self) -> bool {
        matches!(
            self,
            Self::PointerUnavailable | Self::RepositoryUnavailable | Self::RepositoryConflict
        )
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ReindexAssetGraphProjectionUsecase;

impl ReindexAssetGraphProjectionUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: ReindexAssetGraphProjectionInput,
        pointer: &impl CurrentDocumentVersionPointerPort,
        repository: &mut impl ProjectionWorkRepository,
    ) -> Result<ReindexAssetGraphProjectionOutput, ReindexAssetGraphProjectionError> {
        let workspace = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| ReindexAssetGraphProjectionError::InvalidInput)?;
        let document = DocumentId::new(&input.document_id)
            .map_err(|_| ReindexAssetGraphProjectionError::InvalidInput)?;
        let version = pointer
            .load_current_version(&workspace, &document)
            .map_err(map_pointer)?
            .ok_or(ReindexAssetGraphProjectionError::CurrentVersionNotFound)?;
        let requested = ProjectionWorkIdentity::for_change(
            workspace.clone(),
            document.clone(),
            version.clone(),
            ProjectionKind::Graph,
            input.change_kind,
        );

        if let Some(work) = repository.get(&requested).map_err(map_repository)? {
            if work.state().is_resumable() {
                return Ok(output(0, 0, 1));
            }
            let expected = work.state();
            let reset = work
                .transition(ProjectionWorkEvent::ReindexRequested)
                .map_err(|_| ReindexAssetGraphProjectionError::InvalidTransition)?;
            repository
                .replace(reset, expected)
                .map_err(map_repository)?;
            return Ok(output(0, 1, 0));
        }

        for change in all_change_kinds() {
            let candidate = ProjectionWorkIdentity::for_change(
                workspace.clone(),
                document.clone(),
                version.clone(),
                ProjectionKind::Graph,
                change,
            );
            if repository
                .get(&candidate)
                .map_err(map_repository)?
                .is_some_and(|work| work.state().is_resumable())
            {
                return Ok(output(0, 0, 1));
            }
        }

        repository
            .enqueue(ProjectionWork::pending(requested))
            .map_err(map_repository)?;
        Ok(output(1, 0, 0))
    }

    pub fn ensure(
        &self,
        input: ReindexAssetGraphProjectionInput,
        pointer: &impl CurrentDocumentVersionPointerPort,
        repository: &mut impl ProjectionWorkRepository,
    ) -> Result<ReindexAssetGraphProjectionOutput, ReindexAssetGraphProjectionError> {
        let workspace = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| ReindexAssetGraphProjectionError::InvalidInput)?;
        let document = DocumentId::new(&input.document_id)
            .map_err(|_| ReindexAssetGraphProjectionError::InvalidInput)?;
        let version = pointer
            .load_current_version(&workspace, &document)
            .map_err(map_pointer)?
            .ok_or(ReindexAssetGraphProjectionError::CurrentVersionNotFound)?;
        let requested = ProjectionWorkIdentity::for_change(
            workspace.clone(),
            document.clone(),
            version.clone(),
            ProjectionKind::Graph,
            input.change_kind,
        );
        if let Some(work) = repository.get(&requested).map_err(map_repository)? {
            if work.state().is_resumable() {
                return Ok(output(0, 0, 1));
            }
            if work.state() == cabinet_domain::projection_work::ProjectionWorkState::Ready {
                return Ok(ReindexAssetGraphProjectionOutput {
                    enqueued: 0,
                    reset: 0,
                    already_active: 0,
                    already_ready: 1,
                });
            }
            let expected = work.state();
            let reset = work
                .transition(ProjectionWorkEvent::ReindexRequested)
                .map_err(|_| ReindexAssetGraphProjectionError::InvalidTransition)?;
            repository
                .replace(reset, expected)
                .map_err(map_repository)?;
            return Ok(output(0, 1, 0));
        }
        for change in all_change_kinds() {
            let candidate = ProjectionWorkIdentity::for_change(
                workspace.clone(),
                document.clone(),
                version.clone(),
                ProjectionKind::Graph,
                change,
            );
            if repository
                .get(&candidate)
                .map_err(map_repository)?
                .is_some_and(|work| work.state().is_resumable())
            {
                return Ok(output(0, 0, 1));
            }
        }
        repository
            .enqueue(ProjectionWork::pending(requested))
            .map_err(map_repository)?;
        Ok(output(1, 0, 0))
    }
}

const fn output(
    enqueued: usize,
    reset: usize,
    already_active: usize,
) -> ReindexAssetGraphProjectionOutput {
    ReindexAssetGraphProjectionOutput {
        enqueued,
        reset,
        already_active,
        already_ready: 0,
    }
}

const fn all_change_kinds() -> [ProjectionChangeKind; 7] {
    [
        ProjectionChangeKind::Created,
        ProjectionChangeKind::Updated,
        ProjectionChangeKind::Restored,
        ProjectionChangeKind::Renamed,
        ProjectionChangeKind::Deleted,
        ProjectionChangeKind::AssetAttached,
        ProjectionChangeKind::AssetDetached,
    ]
}

const fn map_pointer(
    error: CurrentDocumentVersionPointerError,
) -> ReindexAssetGraphProjectionError {
    match error {
        CurrentDocumentVersionPointerError::StorageUnavailable
        | CurrentDocumentVersionPointerError::Conflict => {
            ReindexAssetGraphProjectionError::PointerUnavailable
        }
        CurrentDocumentVersionPointerError::CorruptedPointer => {
            ReindexAssetGraphProjectionError::CorruptedState
        }
    }
}

const fn map_repository(error: ProjectionWorkRepositoryError) -> ReindexAssetGraphProjectionError {
    match error {
        ProjectionWorkRepositoryError::StorageUnavailable => {
            ReindexAssetGraphProjectionError::RepositoryUnavailable
        }
        ProjectionWorkRepositoryError::Conflict => {
            ReindexAssetGraphProjectionError::RepositoryConflict
        }
        ProjectionWorkRepositoryError::InvalidLimit
        | ProjectionWorkRepositoryError::NotFound
        | ProjectionWorkRepositoryError::CorruptedRecord
        | ProjectionWorkRepositoryError::UnsupportedSchema => {
            ReindexAssetGraphProjectionError::CorruptedState
        }
    }
}
