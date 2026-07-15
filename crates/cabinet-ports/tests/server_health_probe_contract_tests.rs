use cabinet_ports::server_health::{
    HealthComponentState, HealthComponentSummary, ServerHealthProbe,
};

#[derive(Clone)]
struct FakeHealthProbe {
    components: Vec<HealthComponentSummary>,
}

impl ServerHealthProbe for FakeHealthProbe {
    fn check_components(&self) -> Vec<HealthComponentSummary> {
        self.components.clone()
    }
}

#[test]
fn health_probe_contract_returns_component_summaries_without_external_types() {
    let probe = FakeHealthProbe {
        components: vec![
            HealthComponentSummary::new("metadata-store", HealthComponentState::Healthy, None)
                .expect("valid component"),
            HealthComponentSummary::new(
                "object-storage",
                HealthComponentState::Degraded,
                Some("OBJECT_STORAGE_NOT_CONNECTED"),
            )
            .expect("valid degraded component"),
        ],
    };

    let components = probe.check_components();

    assert_eq!(components.len(), 2);
    assert_eq!(components[0].name(), "metadata-store");
    assert_eq!(components[0].state(), HealthComponentState::Healthy);
    assert_eq!(components[1].state(), HealthComponentState::Degraded);
    assert_eq!(
        components[1].error_code(),
        Some("OBJECT_STORAGE_NOT_CONNECTED")
    );
}

#[test]
fn health_component_rejects_empty_name() {
    let error = HealthComponentSummary::new(" ", HealthComponentState::Healthy, None)
        .expect_err("empty component name must fail");

    assert_eq!(error, "component name is required");
}
