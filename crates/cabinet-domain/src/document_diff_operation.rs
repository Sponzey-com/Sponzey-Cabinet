const MAX_OPERATION_ID_LENGTH: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DocumentDiffOperationId(String);

impl DocumentDiffOperationId {
    pub fn new(value: &str) -> Result<Self, DocumentDiffOperationError> {
        let value = value.trim();
        if value.is_empty()
            || value.len() > MAX_OPERATION_ID_LENGTH
            || value.chars().any(char::is_control)
        {
            return Err(DocumentDiffOperationError::InvalidOperationId);
        }
        Ok(Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentDiffOperationState {
    Accepted,
    Running,
    Completed,
    Cancelled,
    Expired,
    Failed,
}

impl DocumentDiffOperationState {
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Cancelled | Self::Expired | Self::Failed
        )
    }

    pub const fn product_log_event(self) -> &'static str {
        match self {
            Self::Accepted => "document.diff.background.accepted",
            Self::Running => "document.diff.background.running",
            Self::Completed => "document.diff.background.completed",
            Self::Cancelled => "document.diff.background.cancelled",
            Self::Expired => "document.diff.background.expired",
            Self::Failed => "document.diff.background.failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentDiffOperationEvent {
    Start,
    Complete,
    Cancel,
    Expire,
    Fail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentDiffOperationSideEffect {
    RunDiff,
    RequestCancellation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentDiffOperationError {
    InvalidOperationId,
    InvalidTransition {
        state: DocumentDiffOperationState,
        event: DocumentDiffOperationEvent,
    },
    TerminalState {
        state: DocumentDiffOperationState,
    },
}

impl DocumentDiffOperationError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidOperationId => "document_diff_operation.invalid_operation_id",
            Self::InvalidTransition { .. } => "document_diff_operation.invalid_transition",
            Self::TerminalState { .. } => "document_diff_operation.terminal_state",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentDiffOperation {
    operation_id: DocumentDiffOperationId,
    state: DocumentDiffOperationState,
}

impl DocumentDiffOperation {
    pub const fn accepted(operation_id: DocumentDiffOperationId) -> Self {
        Self {
            operation_id,
            state: DocumentDiffOperationState::Accepted,
        }
    }

    pub const fn restore(
        operation_id: DocumentDiffOperationId,
        state: DocumentDiffOperationState,
    ) -> Self {
        Self {
            operation_id,
            state,
        }
    }

    pub const fn operation_id(&self) -> &DocumentDiffOperationId {
        &self.operation_id
    }

    pub const fn state(&self) -> DocumentDiffOperationState {
        self.state
    }

    pub fn transition(
        &self,
        event: DocumentDiffOperationEvent,
    ) -> Result<DocumentDiffOperationTransition, DocumentDiffOperationError> {
        if self.state.is_terminal() {
            return Err(DocumentDiffOperationError::TerminalState { state: self.state });
        }

        let (next_state, side_effect) = match (self.state, event) {
            (DocumentDiffOperationState::Accepted, DocumentDiffOperationEvent::Start) => (
                DocumentDiffOperationState::Running,
                Some(DocumentDiffOperationSideEffect::RunDiff),
            ),
            (DocumentDiffOperationState::Accepted, DocumentDiffOperationEvent::Cancel) => {
                (DocumentDiffOperationState::Cancelled, None)
            }
            (DocumentDiffOperationState::Accepted, DocumentDiffOperationEvent::Expire) => {
                (DocumentDiffOperationState::Expired, None)
            }
            (DocumentDiffOperationState::Accepted, DocumentDiffOperationEvent::Fail) => {
                (DocumentDiffOperationState::Failed, None)
            }
            (DocumentDiffOperationState::Running, DocumentDiffOperationEvent::Complete) => {
                (DocumentDiffOperationState::Completed, None)
            }
            (DocumentDiffOperationState::Running, DocumentDiffOperationEvent::Cancel) => (
                DocumentDiffOperationState::Cancelled,
                Some(DocumentDiffOperationSideEffect::RequestCancellation),
            ),
            (DocumentDiffOperationState::Running, DocumentDiffOperationEvent::Expire) => {
                (DocumentDiffOperationState::Expired, None)
            }
            (DocumentDiffOperationState::Running, DocumentDiffOperationEvent::Fail) => {
                (DocumentDiffOperationState::Failed, None)
            }
            _ => {
                return Err(DocumentDiffOperationError::InvalidTransition {
                    state: self.state,
                    event,
                });
            }
        };

        Ok(DocumentDiffOperationTransition {
            operation: Self {
                operation_id: self.operation_id.clone(),
                state: next_state,
            },
            side_effect,
            product_log_event: next_state.product_log_event(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentDiffOperationTransition {
    operation: DocumentDiffOperation,
    side_effect: Option<DocumentDiffOperationSideEffect>,
    product_log_event: &'static str,
}

impl DocumentDiffOperationTransition {
    pub const fn operation(&self) -> &DocumentDiffOperation {
        &self.operation
    }

    pub fn into_operation(self) -> DocumentDiffOperation {
        self.operation
    }

    pub const fn side_effect(&self) -> Option<DocumentDiffOperationSideEffect> {
        self.side_effect
    }

    pub const fn product_log_event(&self) -> &'static str {
        self.product_log_event
    }
}
