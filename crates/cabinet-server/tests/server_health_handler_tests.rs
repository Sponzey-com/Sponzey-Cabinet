use cabinet_core::server_config::ServerConfigInput;
use cabinet_server::adapter::{HttpMethod, ServerRequest, handle_request};
use cabinet_server::composition::build_server_composition;
use cabinet_server::health::{NoopServerHealthProductLogger, build_local_dev_health_target};

#[test]
fn health_handler_returns_json_state_and_component_summary() {
    let config = ServerConfigInput::local_dev_defaults()
        .validate()
        .expect("valid server config");
    let composition = build_server_composition(config.clone());
    let target = build_local_dev_health_target(config, NoopServerHealthProductLogger::default());

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(HttpMethod::Get, "/api/health", None),
    )
    .expect("health route should return response");

    assert_eq!(response.status_code(), 200);
    assert!(response.body().contains("\"state\":\"Healthy\""));
    assert!(response.body().contains("\"name\":\"server-config\""));
    assert!(response.body().contains("\"name\":\"metadata-store\""));
    assert!(!response.body().contains("SPONZEY_CABINET"));
}
