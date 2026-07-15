use cabinet_domain::tool::{ToolExecutionRequest, ToolId, ToolOperation, ToolScope};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalToolKind {
    Mcp,
    PublicApi,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalToolRequest {
    kind: ExternalToolKind,
    tool_id: String,
    workspace_id: String,
    actor_id: String,
    operation_key: String,
    scope_keys: Vec<String>,
    auth_header_present: bool,
}

impl ExternalToolRequest {
    pub fn new(
        kind: ExternalToolKind,
        tool_id: &str,
        workspace_id: &str,
        actor_id: &str,
        operation_key: &str,
        scope_keys: Vec<&str>,
        auth_header_present: bool,
    ) -> Self {
        Self {
            kind,
            tool_id: tool_id.to_string(),
            workspace_id: workspace_id.to_string(),
            actor_id: actor_id.to_string(),
            operation_key: operation_key.to_string(),
            scope_keys: scope_keys.into_iter().map(str::to_string).collect(),
            auth_header_present,
        }
    }

    pub const fn kind(&self) -> ExternalToolKind {
        self.kind
    }

    pub const fn auth_header_present(&self) -> bool {
        self.auth_header_present
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolRequestMapper;

impl ToolRequestMapper {
    pub const fn new() -> Self {
        Self
    }

    pub fn map_request(
        &self,
        request: ExternalToolRequest,
    ) -> Result<ToolExecutionRequest, ToolMapperError> {
        let operation = parse_operation(&request.operation_key)?;
        let scopes = request
            .scope_keys
            .iter()
            .map(|scope| parse_scope(scope))
            .collect::<Result<Vec<_>, _>>()?;
        ToolExecutionRequest::new(
            ToolId::new(&request.tool_id).map_err(ToolMapperError::from_tool_error)?,
            &request.workspace_id,
            &request.actor_id,
            operation,
            scopes,
        )
        .map_err(ToolMapperError::from_tool_error)
    }
}

impl Default for ToolRequestMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolMapperError {
    UnknownOperation,
    UnknownScope,
    InvalidRequest,
}

impl ToolMapperError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::UnknownOperation => "tool_mapper.unknown_operation",
            Self::UnknownScope => "tool_mapper.unknown_scope",
            Self::InvalidRequest => "tool_mapper.invalid_request",
        }
    }

    const fn from_tool_error(_error: cabinet_domain::tool::ToolError) -> Self {
        Self::InvalidRequest
    }
}

fn parse_operation(value: &str) -> Result<ToolOperation, ToolMapperError> {
    match value {
        "read_current_document" => Ok(ToolOperation::ReadCurrentDocument),
        "search_retrieval" => Ok(ToolOperation::SearchRetrieval),
        "query_graph" => Ok(ToolOperation::QueryGraph),
        "read_canvas" => Ok(ToolOperation::ReadCanvas),
        "create_ai_answer_job" => Ok(ToolOperation::CreateAiAnswerJob),
        "create_draft_suggestion" => Ok(ToolOperation::CreateDraftSuggestion),
        _ => Err(ToolMapperError::UnknownOperation),
    }
}

fn parse_scope(value: &str) -> Result<ToolScope, ToolMapperError> {
    match value {
        "read" => Ok(ToolScope::Read),
        "search" => Ok(ToolScope::Search),
        "query" => Ok(ToolScope::Query),
        "ai_question" => Ok(ToolScope::AiQuestion),
        "write_suggestion" => Ok(ToolScope::WriteSuggestion),
        _ => Err(ToolMapperError::UnknownScope),
    }
}
