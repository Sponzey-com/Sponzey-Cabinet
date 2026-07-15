use cabinet_domain::tool::{
    ToolError, ToolExecutionEvent, ToolExecutionRequest, ToolExecutionState, ToolId, ToolOperation,
    ToolScope, transition_tool_execution,
};

#[test]
fn tool_execution_request_requires_explicit_scope() {
    let request = ToolExecutionRequest::new(
        ToolId::new("tool.search").expect("tool id"),
        "workspace-1",
        "actor-1",
        ToolOperation::SearchRetrieval,
        vec![ToolScope::Search],
    )
    .expect("request");

    assert_eq!(request.tool_id().as_str(), "tool.search");
    assert_eq!(request.workspace_id(), "workspace-1");
    assert_eq!(request.actor_id(), "actor-1");
    assert_eq!(request.operation(), ToolOperation::SearchRetrieval);
    assert_eq!(request.granted_scopes(), &[ToolScope::Search]);
    assert_eq!(
        ToolExecutionRequest::new(
            ToolId::new("tool.search").expect("tool id"),
            "workspace-1",
            "actor-1",
            ToolOperation::SearchRetrieval,
            vec![],
        ),
        Err(ToolError::MissingScope),
    );
}

#[test]
fn tool_operation_maps_to_required_scope_without_direct_write_scope() {
    assert_eq!(
        ToolOperation::ReadCurrentDocument.required_scope(),
        ToolScope::Read
    );
    assert_eq!(
        ToolOperation::SearchRetrieval.required_scope(),
        ToolScope::Search
    );
    assert_eq!(ToolOperation::QueryGraph.required_scope(), ToolScope::Query);
    assert_eq!(ToolOperation::ReadCanvas.required_scope(), ToolScope::Read);
    assert_eq!(
        ToolOperation::CreateAiAnswerJob.required_scope(),
        ToolScope::AiQuestion
    );
    assert_eq!(
        ToolOperation::CreateDraftSuggestion.required_scope(),
        ToolScope::WriteSuggestion,
    );
}

#[test]
fn tool_execution_state_machine_supports_completed_denied_and_rate_limited_paths() {
    let checking = transition_tool_execution(
        ToolExecutionState::Received,
        ToolExecutionEvent::StartScopeCheck,
    )
    .expect("checking");
    let executing =
        transition_tool_execution(checking, ToolExecutionEvent::Allow).expect("executing");
    let completed =
        transition_tool_execution(executing, ToolExecutionEvent::Complete).expect("completed");

    assert_eq!(checking, ToolExecutionState::ScopeChecking);
    assert_eq!(executing, ToolExecutionState::Executing);
    assert_eq!(completed, ToolExecutionState::Completed);
    assert_eq!(
        transition_tool_execution(ToolExecutionState::ScopeChecking, ToolExecutionEvent::Deny)
            .expect("denied"),
        ToolExecutionState::Denied,
    );
    assert_eq!(
        transition_tool_execution(ToolExecutionState::Executing, ToolExecutionEvent::RateLimit)
            .expect("rate limited"),
        ToolExecutionState::RateLimited,
    );
}

#[test]
fn tool_execution_state_machine_rejects_invalid_transitions() {
    assert_eq!(
        transition_tool_execution(ToolExecutionState::Received, ToolExecutionEvent::Complete),
        Err(ToolError::InvalidTransition),
    );
    assert_eq!(
        transition_tool_execution(ToolExecutionState::Completed, ToolExecutionEvent::Allow),
        Err(ToolError::InvalidTransition),
    );
}
