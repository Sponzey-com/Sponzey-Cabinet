use cabinet_adapters::tool_mapper::{
    ExternalToolKind, ExternalToolRequest, ToolMapperError, ToolRequestMapper,
};
use cabinet_domain::tool::{ToolOperation, ToolScope};

#[test]
fn tool_mapper_maps_mcp_like_search_request_to_internal_request() {
    let external = ExternalToolRequest::new(
        ExternalToolKind::Mcp,
        "tool.search",
        "workspace-1",
        "actor-1",
        "search_retrieval",
        vec!["read", "search"],
        true,
    );

    let request = ToolRequestMapper::new()
        .map_request(external)
        .expect("mapped request");

    assert_eq!(request.tool_id().as_str(), "tool.search");
    assert_eq!(request.workspace_id(), "workspace-1");
    assert_eq!(request.actor_id(), "actor-1");
    assert_eq!(request.operation(), ToolOperation::SearchRetrieval);
    assert_eq!(
        request.granted_scopes(),
        &[ToolScope::Read, ToolScope::Search]
    );
}

#[test]
fn tool_mapper_maps_api_like_write_suggestion_request_without_direct_write() {
    let external = ExternalToolRequest::new(
        ExternalToolKind::PublicApi,
        "tool.suggest",
        "workspace-1",
        "actor-1",
        "create_draft_suggestion",
        vec!["write_suggestion"],
        true,
    );

    let request = ToolRequestMapper::new()
        .map_request(external)
        .expect("mapped request");

    assert_eq!(request.operation(), ToolOperation::CreateDraftSuggestion);
    assert_eq!(request.granted_scopes(), &[ToolScope::WriteSuggestion]);
}

#[test]
fn tool_mapper_rejects_unknown_operation_and_scope_with_stable_errors() {
    let unknown_operation = ToolRequestMapper::new()
        .map_request(ExternalToolRequest::new(
            ExternalToolKind::Mcp,
            "tool.unknown",
            "workspace-1",
            "actor-1",
            "delete_document",
            vec!["read"],
            true,
        ))
        .expect_err("unknown operation");

    assert_eq!(unknown_operation, ToolMapperError::UnknownOperation);
    assert_eq!(unknown_operation.code(), "tool_mapper.unknown_operation");

    let unknown_scope = ToolRequestMapper::new()
        .map_request(ExternalToolRequest::new(
            ExternalToolKind::PublicApi,
            "tool.search",
            "workspace-1",
            "actor-1",
            "search_retrieval",
            vec!["admin_root"],
            true,
        ))
        .expect_err("unknown scope");

    assert_eq!(unknown_scope, ToolMapperError::UnknownScope);
    assert_eq!(unknown_scope.code(), "tool_mapper.unknown_scope");
}

#[test]
fn tool_mapper_output_does_not_expose_token_or_credential_fixture() {
    let external = ExternalToolRequest::new(
        ExternalToolKind::Mcp,
        "tool.read",
        "workspace-1",
        "actor-1",
        "read_current_document",
        vec!["read"],
        true,
    );

    let request = ToolRequestMapper::new()
        .map_request(external)
        .expect("mapped request");
    let debug = format!("{request:?}");

    assert!(!debug.contains("provider_api_key_fixture"));
    assert!(!debug.contains("connector_access_token_fixture"));
    assert!(!debug.contains("token"));
    assert!(!debug.contains("credential"));
}
