#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolId(String);

impl ToolId {
    pub fn new(value: &str) -> Result<Self, ToolError> {
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
            return Err(ToolError::InvalidToolId);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolScope {
    Read,
    Search,
    Query,
    AiQuestion,
    WriteSuggestion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolOperation {
    ReadCurrentDocument,
    SearchRetrieval,
    QueryGraph,
    ReadCanvas,
    CreateAiAnswerJob,
    CreateDraftSuggestion,
}

impl ToolOperation {
    pub const fn required_scope(self) -> ToolScope {
        match self {
            Self::ReadCurrentDocument | Self::ReadCanvas => ToolScope::Read,
            Self::SearchRetrieval => ToolScope::Search,
            Self::QueryGraph => ToolScope::Query,
            Self::CreateAiAnswerJob => ToolScope::AiQuestion,
            Self::CreateDraftSuggestion => ToolScope::WriteSuggestion,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolExecutionRequest {
    tool_id: ToolId,
    workspace_id: String,
    actor_id: String,
    operation: ToolOperation,
    granted_scopes: Vec<ToolScope>,
}

impl ToolExecutionRequest {
    pub fn new(
        tool_id: ToolId,
        workspace_id: &str,
        actor_id: &str,
        operation: ToolOperation,
        granted_scopes: Vec<ToolScope>,
    ) -> Result<Self, ToolError> {
        if granted_scopes.is_empty() {
            return Err(ToolError::MissingScope);
        }
        Ok(Self {
            tool_id,
            workspace_id: normalize_context_value(workspace_id)?,
            actor_id: normalize_context_value(actor_id)?,
            operation,
            granted_scopes,
        })
    }

    pub fn tool_id(&self) -> &ToolId {
        &self.tool_id
    }

    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    pub fn actor_id(&self) -> &str {
        &self.actor_id
    }

    pub const fn operation(&self) -> ToolOperation {
        self.operation
    }

    pub fn granted_scopes(&self) -> &[ToolScope] {
        &self.granted_scopes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolExecutionResult {
    state: ToolExecutionState,
    error_code: Option<&'static str>,
}

impl ToolExecutionResult {
    pub const fn completed() -> Self {
        Self {
            state: ToolExecutionState::Completed,
            error_code: None,
        }
    }

    pub const fn denied(error_code: &'static str) -> Self {
        Self {
            state: ToolExecutionState::Denied,
            error_code: Some(error_code),
        }
    }

    pub const fn state(self) -> ToolExecutionState {
        self.state
    }

    pub const fn error_code(self) -> Option<&'static str> {
        self.error_code
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolExecutionState {
    Received,
    ScopeChecking,
    Executing,
    Completed,
    Denied,
    Failed,
    RateLimited,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolExecutionEvent {
    StartScopeCheck,
    Allow,
    Complete,
    Deny,
    Fail,
    RateLimit,
}

pub fn transition_tool_execution(
    state: ToolExecutionState,
    event: ToolExecutionEvent,
) -> Result<ToolExecutionState, ToolError> {
    use ToolExecutionEvent as Event;
    use ToolExecutionState as State;

    match (state, event) {
        (State::Received, Event::StartScopeCheck) => Ok(State::ScopeChecking),
        (State::ScopeChecking, Event::Allow) => Ok(State::Executing),
        (State::Executing, Event::Complete) => Ok(State::Completed),
        (State::ScopeChecking, Event::Deny) => Ok(State::Denied),
        (State::Executing, Event::Fail) => Ok(State::Failed),
        (State::Executing, Event::RateLimit) => Ok(State::RateLimited),
        _ => Err(ToolError::InvalidTransition),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolError {
    InvalidToolId,
    InvalidContext,
    MissingScope,
    InvalidTransition,
}

impl ToolError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidToolId => "tool.invalid_tool_id",
            Self::InvalidContext => "tool.invalid_context",
            Self::MissingScope => "tool.missing_scope",
            Self::InvalidTransition => "tool.invalid_transition",
        }
    }
}

fn normalize_context_value(value: &str) -> Result<String, ToolError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
        return Err(ToolError::InvalidContext);
    }
    Ok(trimmed.to_string())
}
