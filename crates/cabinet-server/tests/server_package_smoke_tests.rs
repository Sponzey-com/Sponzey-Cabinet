use cabinet_core::server_config::ServerConfigInput;
use cabinet_server::package_smoke::{
    SelfHostServerPackageSmokeInput, run_self_host_server_package_smoke,
};

#[test]
fn self_host_server_package_smoke_verifies_default_composition_and_health() {
    let config = ServerConfigInput::local_dev_defaults()
        .validate()
        .expect("local dev defaults should be valid");

    let report = run_self_host_server_package_smoke(SelfHostServerPackageSmokeInput::new(config))
        .expect("server package smoke should pass");

    assert!(report.passed());
    assert!(report.route_count() > 0);
    assert_eq!(report.health_status_code(), 200);
    assert_eq!(report.framework(), "axum-tokio");
    assert!(report.default_profile_without_external_services());
    assert!(report.sensitive_output_absent());
}
