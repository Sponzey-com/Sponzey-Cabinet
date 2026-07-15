use cabinet_server::auth::{AuthHeaderMapper, AuthenticatedActorContext};
use cabinet_usecases::auth::{AuthenticatedActor, ValidateSessionOutput};

#[test]
fn auth_header_mapper_extracts_bearer_token_without_leaking_header_in_error() {
    let input = AuthHeaderMapper::new()
        .authorization_header_to_input(Some("Bearer token-1"))
        .expect("valid bearer token");

    assert_eq!(input.token().expose_secret(), "token-1");

    let error = AuthHeaderMapper::new()
        .authorization_header_to_input(Some("Basic raw-secret-token"))
        .expect_err("malformed auth header must fail");

    assert_eq!(error.code_str(), "SERVER_AUTH_MALFORMED_AUTHORIZATION");
    assert!(!format!("{error:?}").contains("raw-secret-token"));
}

#[test]
fn auth_middleware_maps_validate_output_to_actor_context_without_rbac_decision() {
    let output = ValidateSessionOutput::active(AuthenticatedActor::new("user-1"));

    let actor = AuthenticatedActorContext::from_validate_output(output);

    assert_eq!(actor.user_id(), "user-1");
    assert!(!actor.has_permission("document.read"));
}
