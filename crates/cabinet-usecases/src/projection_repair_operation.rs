use cabinet_domain::document::DocumentId;
use cabinet_domain::projection_repair::{
    ProjectionRepairEvent, ProjectionRepairOperation, ProjectionRepairOperationId,
    ProjectionRepairState, ProjectionRepairTransitionError,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::projection_repair::{
    ProjectionRepairCreateOutcome, ProjectionRepairRepository, ProjectionRepairRepositoryError,
};

pub trait ProjectionRepairOperationIdGenerator {
    fn next_id(&mut self) -> Result<String, ()>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartProjectionRepairInput {
    workspace_id: String,
    document_id: String,
}
impl StartProjectionRepairInput {
    pub fn new(workspace_id: &str, document_id: &str) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            document_id: document_id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionRepairOperationOutput {
    operation: ProjectionRepairOperation,
    product_log_event: Option<&'static str>,
}
impl ProjectionRepairOperationOutput {
    pub fn operation(&self) -> &ProjectionRepairOperation {
        &self.operation
    }
    pub const fn product_log_event(&self) -> Option<&'static str> {
        self.product_log_event
    }
}

pub struct StartProjectionRepairUsecase;
impl StartProjectionRepairUsecase {
    pub const fn new() -> Self {
        Self
    }
    pub fn execute<G: ProjectionRepairOperationIdGenerator, R: ProjectionRepairRepository>(
        &self,
        input: StartProjectionRepairInput,
        ids: &mut G,
        repository: &mut R,
    ) -> Result<ProjectionRepairOperationOutput, ProjectionRepairUsecaseError> {
        let workspace = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| ProjectionRepairUsecaseError::InvalidInput)?;
        let document = DocumentId::new(&input.document_id)
            .map_err(|_| ProjectionRepairUsecaseError::InvalidInput)?;
        let raw_id = ids
            .next_id()
            .map_err(|_| ProjectionRepairUsecaseError::OperationIdUnavailable)?;
        let id = ProjectionRepairOperationId::new(&raw_id)
            .map_err(|_| ProjectionRepairUsecaseError::OperationIdUnavailable)?;
        let operation = ProjectionRepairOperation::queued(id, workspace, document);
        match repository
            .create(operation.clone())
            .map_err(map_repository_error)?
        {
            ProjectionRepairCreateOutcome::Created => Ok(ProjectionRepairOperationOutput {
                operation,
                product_log_event: Some("projection.reindex.requested"),
            }),
            ProjectionRepairCreateOutcome::AlreadyExists => {
                Err(ProjectionRepairUsecaseError::AlreadyExists)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetProjectionRepairStatusInput {
    workspace_id: String,
    operation_id: String,
}
impl GetProjectionRepairStatusInput {
    pub fn new(workspace_id: &str, operation_id: &str) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            operation_id: operation_id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionRepairStatusOutput {
    state: ProjectionRepairState,
    attempt: u32,
    completed_units: u8,
    total_units: u8,
}
impl ProjectionRepairStatusOutput {
    pub const fn state(&self) -> ProjectionRepairState {
        self.state
    }
    pub const fn attempt(&self) -> u32 {
        self.attempt
    }
    pub const fn completed_units(&self) -> u8 {
        self.completed_units
    }
    pub const fn total_units(&self) -> u8 {
        self.total_units
    }
}

pub struct GetProjectionRepairStatusUsecase;
impl GetProjectionRepairStatusUsecase {
    pub const fn new() -> Self {
        Self
    }
    pub fn execute<R: ProjectionRepairRepository>(
        &self,
        input: GetProjectionRepairStatusInput,
        repository: &R,
    ) -> Result<ProjectionRepairStatusOutput, ProjectionRepairUsecaseError> {
        let operation = scoped_operation(&input.workspace_id, &input.operation_id, repository)?;
        let progress = operation.progress();
        Ok(ProjectionRepairStatusOutput {
            state: operation.state(),
            attempt: operation.attempt(),
            completed_units: progress.completed_units(),
            total_units: progress.total_units(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancelProjectionRepairInput {
    workspace_id: String,
    operation_id: String,
}
impl CancelProjectionRepairInput {
    pub fn new(workspace_id: &str, operation_id: &str) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            operation_id: operation_id.into(),
        }
    }
}
pub struct CancelProjectionRepairUsecase;
impl CancelProjectionRepairUsecase {
    pub const fn new() -> Self {
        Self
    }
    pub fn execute<R: ProjectionRepairRepository>(
        &self,
        input: CancelProjectionRepairInput,
        repository: &mut R,
    ) -> Result<ProjectionRepairOperationOutput, ProjectionRepairUsecaseError> {
        transition_scoped(
            &input.workspace_id,
            &input.operation_id,
            ProjectionRepairEvent::CancelRequested,
            repository,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryProjectionRepairInput {
    workspace_id: String,
    operation_id: String,
}
impl RetryProjectionRepairInput {
    pub fn new(workspace_id: &str, operation_id: &str) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            operation_id: operation_id.into(),
        }
    }
}
pub struct RetryProjectionRepairUsecase;
impl RetryProjectionRepairUsecase {
    pub const fn new() -> Self {
        Self
    }
    pub fn execute<R: ProjectionRepairRepository>(
        &self,
        input: RetryProjectionRepairInput,
        repository: &mut R,
    ) -> Result<ProjectionRepairOperationOutput, ProjectionRepairUsecaseError> {
        transition_scoped(
            &input.workspace_id,
            &input.operation_id,
            ProjectionRepairEvent::Retry,
            repository,
        )
    }
}

fn transition_scoped<R: ProjectionRepairRepository>(
    workspace: &str,
    id: &str,
    event: ProjectionRepairEvent,
    repository: &mut R,
) -> Result<ProjectionRepairOperationOutput, ProjectionRepairUsecaseError> {
    let current = scoped_operation(workspace, id, repository)?;
    let expected = current.state();
    let transition = current.transition(event).map_err(map_transition_error)?;
    let product_log_event = transition.product_log_event();
    let operation = transition.into_operation();
    repository
        .replace(operation.clone(), expected)
        .map_err(map_repository_error)?;
    Ok(ProjectionRepairOperationOutput {
        operation,
        product_log_event,
    })
}

fn scoped_operation<R: ProjectionRepairRepository>(
    workspace: &str,
    id: &str,
    repository: &R,
) -> Result<ProjectionRepairOperation, ProjectionRepairUsecaseError> {
    let workspace =
        WorkspaceId::new(workspace).map_err(|_| ProjectionRepairUsecaseError::InvalidInput)?;
    let id = ProjectionRepairOperationId::new(id)
        .map_err(|_| ProjectionRepairUsecaseError::InvalidInput)?;
    let operation = repository
        .get(&id)
        .map_err(map_repository_error)?
        .ok_or(ProjectionRepairUsecaseError::NotFound)?;
    if operation.workspace_id() != &workspace {
        return Err(ProjectionRepairUsecaseError::NotFound);
    }
    Ok(operation)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionRepairUsecaseError {
    InvalidInput,
    OperationIdUnavailable,
    AlreadyExists,
    NotFound,
    InvalidTransition,
    CancellationTooLate,
    Conflict,
    RepositoryUnavailable,
    CorruptedState,
}
impl ProjectionRepairUsecaseError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "projection_repair.invalid_input",
            Self::OperationIdUnavailable => "projection_repair.operation_id_unavailable",
            Self::AlreadyExists => "projection_repair.already_exists",
            Self::NotFound => "projection_repair.not_found",
            Self::InvalidTransition => "projection_repair.invalid_transition",
            Self::CancellationTooLate => "projection_repair.cancellation_too_late",
            Self::Conflict => "projection_repair.conflict",
            Self::RepositoryUnavailable => "projection_repair.repository_unavailable",
            Self::CorruptedState => "projection_repair.corrupted_state",
        }
    }
    pub const fn retryable(self) -> bool {
        matches!(
            self,
            Self::OperationIdUnavailable | Self::Conflict | Self::RepositoryUnavailable
        )
    }
}

fn map_transition_error(error: ProjectionRepairTransitionError) -> ProjectionRepairUsecaseError {
    match error {
        ProjectionRepairTransitionError::CancellationTooLate => {
            ProjectionRepairUsecaseError::CancellationTooLate
        }
        ProjectionRepairTransitionError::InvalidOperationId
        | ProjectionRepairTransitionError::InvalidSnapshot => {
            ProjectionRepairUsecaseError::CorruptedState
        }
        ProjectionRepairTransitionError::InvalidTransition
        | ProjectionRepairTransitionError::TerminalState => {
            ProjectionRepairUsecaseError::InvalidTransition
        }
    }
}
fn map_repository_error(error: ProjectionRepairRepositoryError) -> ProjectionRepairUsecaseError {
    match error {
        ProjectionRepairRepositoryError::NotFound => ProjectionRepairUsecaseError::NotFound,
        ProjectionRepairRepositoryError::Conflict => ProjectionRepairUsecaseError::Conflict,
        ProjectionRepairRepositoryError::StorageUnavailable => {
            ProjectionRepairUsecaseError::RepositoryUnavailable
        }
        ProjectionRepairRepositoryError::CorruptedRecord
        | ProjectionRepairRepositoryError::UnsupportedSchema => {
            ProjectionRepairUsecaseError::CorruptedState
        }
        ProjectionRepairRepositoryError::InvalidLimit => ProjectionRepairUsecaseError::InvalidInput,
    }
}
