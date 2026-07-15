use cabinet_domain::document::DocumentId;
use cabinet_domain::projection_work::{
    ProjectionChangeKind, ProjectionKind, ProjectionWorkIdentity, ProjectionWorkState,
};
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_version::{
    CurrentDocumentVersionPointerError, CurrentDocumentVersionPointerPort,
};
use cabinet_ports::projection_work::{ProjectionWorkRepository, ProjectionWorkRepositoryError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetCurrentProjectionFreshnessInput {
    workspace_id: String,
    document_id: String,
}

impl GetCurrentProjectionFreshnessInput {
    pub fn new(workspace_id: &str, document_id: &str) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionFreshnessState {
    Ready,
    Stale,
    Repairing,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectionKindFreshness {
    kind: ProjectionKind,
    state: ProjectionFreshnessState,
}

impl ProjectionKindFreshness {
    pub const fn kind(&self) -> ProjectionKind {
        self.kind
    }

    pub const fn state(&self) -> ProjectionFreshnessState {
        self.state
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetCurrentProjectionFreshnessOutput {
    current_version_id: VersionId,
    aggregate_state: ProjectionFreshnessState,
    projections: Vec<ProjectionKindFreshness>,
}

impl GetCurrentProjectionFreshnessOutput {
    pub fn current_version_id(&self) -> &VersionId {
        &self.current_version_id
    }

    pub const fn aggregate_state(&self) -> ProjectionFreshnessState {
        self.aggregate_state
    }

    pub fn projections(&self) -> &[ProjectionKindFreshness] {
        &self.projections
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GetCurrentProjectionFreshnessError {
    InvalidInput,
    CurrentVersionNotFound,
    PointerUnavailable,
    RepositoryUnavailable,
    CorruptedState,
}

impl GetCurrentProjectionFreshnessError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "projection_freshness.invalid_input",
            Self::CurrentVersionNotFound => "projection_freshness.current_version_not_found",
            Self::PointerUnavailable => "projection_freshness.pointer_unavailable",
            Self::RepositoryUnavailable => "projection_freshness.repository_unavailable",
            Self::CorruptedState => "projection_freshness.corrupted_state",
        }
    }

    pub const fn retryable(self) -> bool {
        matches!(self, Self::PointerUnavailable | Self::RepositoryUnavailable)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GetCurrentProjectionFreshnessUsecase;

impl GetCurrentProjectionFreshnessUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: GetCurrentProjectionFreshnessInput,
        pointer: &impl CurrentDocumentVersionPointerPort,
        repository: &impl ProjectionWorkRepository,
    ) -> Result<GetCurrentProjectionFreshnessOutput, GetCurrentProjectionFreshnessError> {
        let workspace_id = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| GetCurrentProjectionFreshnessError::InvalidInput)?;
        let document_id = DocumentId::new(&input.document_id)
            .map_err(|_| GetCurrentProjectionFreshnessError::InvalidInput)?;
        let current_version_id = pointer
            .load_current_version(&workspace_id, &document_id)
            .map_err(map_pointer_error)?
            .ok_or(GetCurrentProjectionFreshnessError::CurrentVersionNotFound)?;
        let mut projections = Vec::with_capacity(3);
        for kind in [
            ProjectionKind::Search,
            ProjectionKind::Links,
            ProjectionKind::Graph,
        ] {
            let mut state = None;
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
                    workspace_id.clone(),
                    document_id.clone(),
                    current_version_id.clone(),
                    kind,
                    change_kind,
                );
                if let Some(work) = repository.get(&identity).map_err(map_repository_error)? {
                    let work_state = map_state(work.state());
                    state = Some(
                        state.map_or(work_state, |current| highest_priority(current, work_state)),
                    );
                }
            }
            let state = state.unwrap_or(ProjectionFreshnessState::Stale);
            projections.push(ProjectionKindFreshness { kind, state });
        }
        let aggregate_state = projections
            .iter()
            .fold(ProjectionFreshnessState::Ready, |aggregate, item| {
                highest_priority(aggregate, item.state)
            });
        Ok(GetCurrentProjectionFreshnessOutput {
            current_version_id,
            aggregate_state,
            projections,
        })
    }
}

const fn map_state(state: ProjectionWorkState) -> ProjectionFreshnessState {
    match state {
        ProjectionWorkState::Ready => ProjectionFreshnessState::Ready,
        ProjectionWorkState::Pending | ProjectionWorkState::RetryScheduled => {
            ProjectionFreshnessState::Stale
        }
        ProjectionWorkState::Indexing => ProjectionFreshnessState::Repairing,
        ProjectionWorkState::Failed => ProjectionFreshnessState::Failed,
    }
}

const fn highest_priority(
    left: ProjectionFreshnessState,
    right: ProjectionFreshnessState,
) -> ProjectionFreshnessState {
    if priority(right) > priority(left) {
        right
    } else {
        left
    }
}

const fn priority(state: ProjectionFreshnessState) -> u8 {
    match state {
        ProjectionFreshnessState::Ready => 0,
        ProjectionFreshnessState::Stale => 1,
        ProjectionFreshnessState::Repairing => 2,
        ProjectionFreshnessState::Failed => 3,
    }
}

const fn map_pointer_error(
    error: CurrentDocumentVersionPointerError,
) -> GetCurrentProjectionFreshnessError {
    match error {
        CurrentDocumentVersionPointerError::StorageUnavailable
        | CurrentDocumentVersionPointerError::Conflict => {
            GetCurrentProjectionFreshnessError::PointerUnavailable
        }
        CurrentDocumentVersionPointerError::CorruptedPointer => {
            GetCurrentProjectionFreshnessError::CorruptedState
        }
    }
}

const fn map_repository_error(
    error: ProjectionWorkRepositoryError,
) -> GetCurrentProjectionFreshnessError {
    match error {
        ProjectionWorkRepositoryError::StorageUnavailable
        | ProjectionWorkRepositoryError::Conflict => {
            GetCurrentProjectionFreshnessError::RepositoryUnavailable
        }
        ProjectionWorkRepositoryError::InvalidLimit
        | ProjectionWorkRepositoryError::NotFound
        | ProjectionWorkRepositoryError::CorruptedRecord
        | ProjectionWorkRepositoryError::UnsupportedSchema => {
            GetCurrentProjectionFreshnessError::CorruptedState
        }
    }
}
