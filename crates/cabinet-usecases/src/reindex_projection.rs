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
pub struct ReindexCurrentProjectionInput {
    workspace_id: String,
    document_id: String,
}
impl ReindexCurrentProjectionInput {
    pub fn new(workspace_id: &str, document_id: &str) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReindexCurrentProjectionOutput {
    enqueued: usize,
    reset: usize,
    already_active: usize,
}
impl ReindexCurrentProjectionOutput {
    pub const fn enqueued_count(self) -> usize {
        self.enqueued
    }
    pub const fn reset_count(self) -> usize {
        self.reset
    }
    pub const fn already_active_count(self) -> usize {
        self.already_active
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReindexCurrentProjectionError {
    InvalidInput,
    CurrentVersionNotFound,
    PointerUnavailable,
    RepositoryUnavailable,
    RepositoryConflict,
    CorruptedState,
    InvalidTransition,
}
impl ReindexCurrentProjectionError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "projection_reindex.invalid_input",
            Self::CurrentVersionNotFound => "projection_reindex.current_version_not_found",
            Self::PointerUnavailable => "projection_reindex.pointer_unavailable",
            Self::RepositoryUnavailable => "projection_reindex.repository_unavailable",
            Self::RepositoryConflict => "projection_reindex.repository_conflict",
            Self::CorruptedState => "projection_reindex.corrupted_state",
            Self::InvalidTransition => "projection_reindex.invalid_transition",
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
pub struct ReindexCurrentProjectionUsecase;
impl ReindexCurrentProjectionUsecase {
    pub const fn new() -> Self {
        Self
    }
    pub fn execute(
        &self,
        input: ReindexCurrentProjectionInput,
        pointer: &impl CurrentDocumentVersionPointerPort,
        repository: &mut impl ProjectionWorkRepository,
    ) -> Result<ReindexCurrentProjectionOutput, ReindexCurrentProjectionError> {
        let workspace = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| ReindexCurrentProjectionError::InvalidInput)?;
        let document = DocumentId::new(&input.document_id)
            .map_err(|_| ReindexCurrentProjectionError::InvalidInput)?;
        let version = pointer
            .load_current_version(&workspace, &document)
            .map_err(map_pointer)?
            .ok_or(ReindexCurrentProjectionError::CurrentVersionNotFound)?;
        let mut output = ReindexCurrentProjectionOutput {
            enqueued: 0,
            reset: 0,
            already_active: 0,
        };
        for kind in [
            ProjectionKind::Search,
            ProjectionKind::Links,
            ProjectionKind::Graph,
        ] {
            let mut terminal = None;
            let mut active = false;
            for change_kind in [
                ProjectionChangeKind::Created,
                ProjectionChangeKind::Updated,
                ProjectionChangeKind::Restored,
                ProjectionChangeKind::Renamed,
                ProjectionChangeKind::Deleted,
                ProjectionChangeKind::AssetAttached,
                ProjectionChangeKind::AssetDetached,
            ] {
                let identity = ProjectionWorkIdentity::for_change(
                    workspace.clone(),
                    document.clone(),
                    version.clone(),
                    kind,
                    change_kind,
                );
                if let Some(work) = repository.get(&identity).map_err(map_repository)? {
                    if work.state().is_resumable() {
                        active = true;
                        break;
                    }
                    terminal = Some(work);
                }
            }
            if active {
                output.already_active += 1;
            } else if let Some(work) = terminal {
                let expected = work.state();
                let reset = work
                    .transition(ProjectionWorkEvent::ReindexRequested)
                    .map_err(|_| ReindexCurrentProjectionError::InvalidTransition)?;
                repository
                    .replace(reset, expected)
                    .map_err(map_repository)?;
                output.reset += 1;
            } else {
                let identity = ProjectionWorkIdentity::new(
                    workspace.clone(),
                    document.clone(),
                    version.clone(),
                    kind,
                );
                repository
                    .enqueue(ProjectionWork::pending(identity))
                    .map_err(map_repository)?;
                output.enqueued += 1;
            }
        }
        Ok(output)
    }
}

const fn map_pointer(error: CurrentDocumentVersionPointerError) -> ReindexCurrentProjectionError {
    match error {
        CurrentDocumentVersionPointerError::StorageUnavailable
        | CurrentDocumentVersionPointerError::Conflict => {
            ReindexCurrentProjectionError::PointerUnavailable
        }
        CurrentDocumentVersionPointerError::CorruptedPointer => {
            ReindexCurrentProjectionError::CorruptedState
        }
    }
}
const fn map_repository(error: ProjectionWorkRepositoryError) -> ReindexCurrentProjectionError {
    match error {
        ProjectionWorkRepositoryError::StorageUnavailable => {
            ReindexCurrentProjectionError::RepositoryUnavailable
        }
        ProjectionWorkRepositoryError::Conflict => {
            ReindexCurrentProjectionError::RepositoryConflict
        }
        ProjectionWorkRepositoryError::InvalidLimit
        | ProjectionWorkRepositoryError::NotFound
        | ProjectionWorkRepositoryError::CorruptedRecord
        | ProjectionWorkRepositoryError::UnsupportedSchema => {
            ReindexCurrentProjectionError::CorruptedState
        }
    }
}
