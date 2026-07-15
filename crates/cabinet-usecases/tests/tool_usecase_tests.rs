use cabinet_domain::tool::{
    ToolExecutionRequest, ToolExecutionState, ToolId, ToolOperation, ToolScope,
};
use cabinet_usecases::tool::AuthorizeToolExecutionUsecase;

#[test]
fn authorize_tool_execution_allows_request_with_required_scope() {
    let request = request(
        ToolOperation::SearchRetrieval,
        vec![ToolScope::Read, ToolScope::Search],
    );

    let output = AuthorizeToolExecutionUsecase::new()
        .execute(request)
        .expect("authorization");

    assert!(output.allowed());
    assert_eq!(output.result().state(), ToolExecutionState::Completed);
    assert_eq!(output.result().error_code(), None);
    assert_eq!(output.required_scope(), ToolScope::Search);
}

#[test]
fn authorize_tool_execution_denies_request_missing_required_scope() {
    let request = request(ToolOperation::QueryGraph, vec![ToolScope::Read]);

    let output = AuthorizeToolExecutionUsecase::new()
        .execute(request)
        .expect("authorization");

    assert!(!output.allowed());
    assert_eq!(output.result().state(), ToolExecutionState::Denied);
    assert_eq!(output.result().error_code(), Some("tool.scope_denied"));
    assert_eq!(output.required_scope(), ToolScope::Query);
}

#[test]
fn authorize_tool_execution_limits_write_operation_to_draft_suggestion_scope() {
    let denied = AuthorizeToolExecutionUsecase::new()
        .execute(request(
            ToolOperation::CreateDraftSuggestion,
            vec![ToolScope::Read, ToolScope::Search],
        ))
        .expect("authorization");

    assert!(!denied.allowed());
    assert_eq!(denied.required_scope(), ToolScope::WriteSuggestion);

    let allowed = AuthorizeToolExecutionUsecase::new()
        .execute(request(
            ToolOperation::CreateDraftSuggestion,
            vec![ToolScope::WriteSuggestion],
        ))
        .expect("authorization");

    assert!(allowed.allowed());
    assert_eq!(allowed.result().state(), ToolExecutionState::Completed);
}

#[test]
fn authorize_tool_execution_result_does_not_expose_token_or_credential() {
    let output = AuthorizeToolExecutionUsecase::new()
        .execute(request(
            ToolOperation::ReadCurrentDocument,
            vec![ToolScope::Read],
        ))
        .expect("authorization");

    assert!(!format!("{:?}", output.result()).contains("token"));
    assert!(!format!("{:?}", output.result()).contains("credential"));
    assert!(!format!("{:?}", output.result()).contains("provider_api_key_fixture"));
}

fn request(operation: ToolOperation, scopes: Vec<ToolScope>) -> ToolExecutionRequest {
    ToolExecutionRequest::new(
        ToolId::new("tool.test").expect("tool id"),
        "workspace-1",
        "actor-1",
        operation,
        scopes,
    )
    .expect("request")
}
