use cabinet_domain::tool::{
    ToolError, ToolExecutionEvent, ToolExecutionRequest, ToolExecutionResult, ToolExecutionState,
    ToolScope, transition_tool_execution,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthorizeToolExecutionUsecase;

impl AuthorizeToolExecutionUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        request: ToolExecutionRequest,
    ) -> Result<ToolAuthorizationOutput, ToolAuthorizationError> {
        let checking = transition_tool_execution(
            ToolExecutionState::Received,
            ToolExecutionEvent::StartScopeCheck,
        )
        .map_err(ToolAuthorizationError::from_domain_error)?;
        let required_scope = request.operation().required_scope();
        if !request.granted_scopes().contains(&required_scope) {
            let denied = transition_tool_execution(checking, ToolExecutionEvent::Deny)
                .map_err(ToolAuthorizationError::from_domain_error)?;
            return Ok(ToolAuthorizationOutput::new(
                false,
                required_scope,
                ToolExecutionResult::denied("tool.scope_denied"),
                denied,
            ));
        }

        let executing = transition_tool_execution(checking, ToolExecutionEvent::Allow)
            .map_err(ToolAuthorizationError::from_domain_error)?;
        let completed = transition_tool_execution(executing, ToolExecutionEvent::Complete)
            .map_err(ToolAuthorizationError::from_domain_error)?;
        Ok(ToolAuthorizationOutput::new(
            true,
            required_scope,
            ToolExecutionResult::completed(),
            completed,
        ))
    }
}

impl Default for AuthorizeToolExecutionUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolAuthorizationOutput {
    allowed: bool,
    required_scope: ToolScope,
    result: ToolExecutionResult,
    terminal_state: ToolExecutionState,
}

impl ToolAuthorizationOutput {
    const fn new(
        allowed: bool,
        required_scope: ToolScope,
        result: ToolExecutionResult,
        terminal_state: ToolExecutionState,
    ) -> Self {
        Self {
            allowed,
            required_scope,
            result,
            terminal_state,
        }
    }

    pub const fn allowed(self) -> bool {
        self.allowed
    }

    pub const fn required_scope(self) -> ToolScope {
        self.required_scope
    }

    pub const fn result(self) -> ToolExecutionResult {
        self.result
    }

    pub const fn terminal_state(self) -> ToolExecutionState {
        self.terminal_state
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolAuthorizationError {
    InvalidInput,
    InvalidTransition,
}

impl ToolAuthorizationError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "tool_authorization.invalid_input",
            Self::InvalidTransition => "tool_authorization.invalid_transition",
        }
    }

    fn from_domain_error(error: ToolError) -> Self {
        match error {
            ToolError::InvalidToolId | ToolError::InvalidContext | ToolError::MissingScope => {
                Self::InvalidInput
            }
            ToolError::InvalidTransition => Self::InvalidTransition,
        }
    }
}
