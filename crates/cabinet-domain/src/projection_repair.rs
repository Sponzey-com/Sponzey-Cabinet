use crate::document::DocumentId;
use crate::workspace::WorkspaceId;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProjectionRepairOperationId(String);

impl ProjectionRepairOperationId {
    pub fn new(value: &str) -> Result<Self, ProjectionRepairTransitionError> {
        let value = value.trim();
        if value.is_empty() || value.chars().any(char::is_control) {
            return Err(ProjectionRepairTransitionError::InvalidOperationId);
        }
        Ok(Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionRepairState {
    Queued,
    Running,
    Publishing,
    CancelPending,
    Succeeded,
    FailedRetryable,
    FailedFatal,
    Cancelled,
}

impl ProjectionRepairState {
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::FailedFatal | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionRepairEvent {
    Start,
    PublishStarted,
    Succeeded,
    FailedRetryable,
    FailedFatal,
    CancelRequested,
    Cancelled,
    Retry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionRepairSideEffect {
    RunProjectionRepair,
    RequestCancellation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectionRepairProgress {
    completed_units: u8,
    total_units: u8,
}

impl ProjectionRepairProgress {
    pub const fn new(
        completed_units: u8,
        total_units: u8,
    ) -> Result<Self, ProjectionRepairTransitionError> {
        if total_units == 0 || completed_units > total_units {
            return Err(ProjectionRepairTransitionError::InvalidSnapshot);
        }
        Ok(Self {
            completed_units,
            total_units,
        })
    }

    pub const fn completed_units(self) -> u8 {
        self.completed_units
    }

    pub const fn total_units(self) -> u8 {
        self.total_units
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionRepairTransitionError {
    InvalidOperationId,
    InvalidSnapshot,
    InvalidTransition,
    TerminalState,
    CancellationTooLate,
}

impl ProjectionRepairTransitionError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidOperationId => "projection_repair.invalid_operation_id",
            Self::InvalidSnapshot => "projection_repair.invalid_snapshot",
            Self::InvalidTransition => "projection_repair.invalid_transition",
            Self::TerminalState => "projection_repair.terminal_state",
            Self::CancellationTooLate => "projection_repair.cancellation_too_late",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionRepairOperation {
    operation_id: ProjectionRepairOperationId,
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    state: ProjectionRepairState,
    attempt: u32,
    progress: ProjectionRepairProgress,
}

impl ProjectionRepairOperation {
    pub const fn queued(
        operation_id: ProjectionRepairOperationId,
        workspace_id: WorkspaceId,
        document_id: DocumentId,
    ) -> Self {
        Self {
            operation_id,
            workspace_id,
            document_id,
            state: ProjectionRepairState::Queued,
            attempt: 0,
            progress: ProjectionRepairProgress {
                completed_units: 0,
                total_units: 3,
            },
        }
    }

    pub fn restore(
        operation_id: ProjectionRepairOperationId,
        workspace_id: WorkspaceId,
        document_id: DocumentId,
        state: ProjectionRepairState,
        attempt: u32,
        progress: ProjectionRepairProgress,
    ) -> Result<Self, ProjectionRepairTransitionError> {
        if (matches!(
            state,
            ProjectionRepairState::Running
                | ProjectionRepairState::Publishing
                | ProjectionRepairState::CancelPending
                | ProjectionRepairState::Succeeded
                | ProjectionRepairState::FailedRetryable
                | ProjectionRepairState::FailedFatal
        ) && attempt == 0)
            || (state == ProjectionRepairState::Publishing && progress.completed_units != 2)
            || (state == ProjectionRepairState::Succeeded
                && progress.completed_units != progress.total_units)
        {
            return Err(ProjectionRepairTransitionError::InvalidSnapshot);
        }
        Ok(Self {
            operation_id,
            workspace_id,
            document_id,
            state,
            attempt,
            progress,
        })
    }

    pub fn operation_id(&self) -> &ProjectionRepairOperationId {
        &self.operation_id
    }

    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    pub fn document_id(&self) -> &DocumentId {
        &self.document_id
    }

    pub const fn state(&self) -> ProjectionRepairState {
        self.state
    }

    pub const fn attempt(&self) -> u32 {
        self.attempt
    }

    pub const fn progress(&self) -> ProjectionRepairProgress {
        self.progress
    }

    pub fn transition(
        &self,
        event: ProjectionRepairEvent,
    ) -> Result<ProjectionRepairTransition, ProjectionRepairTransitionError> {
        if self.state.is_terminal() {
            return Err(ProjectionRepairTransitionError::TerminalState);
        }
        if self.state == ProjectionRepairState::Publishing
            && event == ProjectionRepairEvent::CancelRequested
        {
            return Err(ProjectionRepairTransitionError::CancellationTooLate);
        }

        let (state, attempt, progress, side_effect, product_log_event) = match (self.state, event) {
            (ProjectionRepairState::Queued, ProjectionRepairEvent::Start) => (
                ProjectionRepairState::Running,
                self.attempt + 1,
                progress(0),
                Some(ProjectionRepairSideEffect::RunProjectionRepair),
                Some("projection.reindex.started"),
            ),
            (ProjectionRepairState::Queued, ProjectionRepairEvent::CancelRequested) => (
                ProjectionRepairState::Cancelled,
                self.attempt,
                self.progress,
                None,
                Some("projection.reindex.cancelled"),
            ),
            (ProjectionRepairState::Running, ProjectionRepairEvent::PublishStarted) => (
                ProjectionRepairState::Publishing,
                self.attempt,
                progress(2),
                None,
                None,
            ),
            (ProjectionRepairState::Running, ProjectionRepairEvent::CancelRequested) => (
                ProjectionRepairState::CancelPending,
                self.attempt,
                self.progress,
                Some(ProjectionRepairSideEffect::RequestCancellation),
                None,
            ),
            (ProjectionRepairState::CancelPending, ProjectionRepairEvent::Cancelled) => (
                ProjectionRepairState::Cancelled,
                self.attempt,
                self.progress,
                None,
                Some("projection.reindex.cancelled"),
            ),
            (
                ProjectionRepairState::Running | ProjectionRepairState::Publishing,
                ProjectionRepairEvent::Succeeded,
            ) => (
                ProjectionRepairState::Succeeded,
                self.attempt,
                progress(3),
                None,
                Some("projection.reindex.completed"),
            ),
            (
                ProjectionRepairState::Running
                | ProjectionRepairState::Publishing
                | ProjectionRepairState::CancelPending,
                ProjectionRepairEvent::FailedRetryable,
            ) => (
                ProjectionRepairState::FailedRetryable,
                self.attempt,
                self.progress,
                None,
                Some("projection.reindex.failed"),
            ),
            (
                ProjectionRepairState::Running
                | ProjectionRepairState::Publishing
                | ProjectionRepairState::CancelPending,
                ProjectionRepairEvent::FailedFatal,
            ) => (
                ProjectionRepairState::FailedFatal,
                self.attempt,
                self.progress,
                None,
                Some("projection.reindex.failed"),
            ),
            (ProjectionRepairState::FailedRetryable, ProjectionRepairEvent::Retry) => (
                ProjectionRepairState::Queued,
                self.attempt,
                progress(0),
                None,
                Some("projection.reindex.retry_requested"),
            ),
            _ => return Err(ProjectionRepairTransitionError::InvalidTransition),
        };

        Ok(ProjectionRepairTransition {
            operation: Self {
                operation_id: self.operation_id.clone(),
                workspace_id: self.workspace_id.clone(),
                document_id: self.document_id.clone(),
                state,
                attempt,
                progress,
            },
            side_effect,
            product_log_event,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionRepairTransition {
    operation: ProjectionRepairOperation,
    side_effect: Option<ProjectionRepairSideEffect>,
    product_log_event: Option<&'static str>,
}

impl ProjectionRepairTransition {
    pub const fn operation(&self) -> &ProjectionRepairOperation {
        &self.operation
    }

    pub fn into_operation(self) -> ProjectionRepairOperation {
        self.operation
    }

    pub const fn side_effect(&self) -> Option<ProjectionRepairSideEffect> {
        self.side_effect
    }

    pub const fn product_log_event(&self) -> Option<&'static str> {
        self.product_log_event
    }
}

const fn progress(completed_units: u8) -> ProjectionRepairProgress {
    ProjectionRepairProgress {
        completed_units,
        total_units: 3,
    }
}
