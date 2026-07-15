use cabinet_ports::server_health::{
    HealthComponentState, HealthComponentSummary, ServerHealthProbe,
};
use cabinet_usecases::server_health::{
    GetServerHealthInput, GetServerHealthUsecase, ServerHealthProductEvent,
    ServerHealthProductLogger, ServerHealthState,
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

#[derive(Default)]
struct FakeProductLogger {
    events: Vec<ServerHealthProductEvent>,
}

impl ServerHealthProductLogger for FakeProductLogger {
    fn write_product(&mut self, event: ServerHealthProductEvent) {
        self.events.push(event);
    }
}

#[test]
fn health_is_healthy_when_all_components_are_healthy() {
    let probe = FakeHealthProbe {
        components: vec![
            component("server-config", HealthComponentState::Healthy, None),
            component("metadata-store", HealthComponentState::Healthy, None),
        ],
    };
    let mut logger = FakeProductLogger::default();

    let output =
        GetServerHealthUsecase::new().execute(GetServerHealthInput::new(), &probe, &mut logger);

    assert_eq!(output.state(), ServerHealthState::Healthy);
    assert_eq!(output.components().len(), 2);
    assert!(logger.events.is_empty());
}

#[test]
fn health_is_degraded_when_any_component_is_degraded() {
    let probe = FakeHealthProbe {
        components: vec![
            component("server-config", HealthComponentState::Healthy, None),
            component(
                "metadata-store",
                HealthComponentState::Degraded,
                Some("METADATA_PROBE_NOT_CONNECTED"),
            ),
        ],
    };
    let mut logger = FakeProductLogger::default();

    let output =
        GetServerHealthUsecase::new().execute(GetServerHealthInput::new(), &probe, &mut logger);

    assert_eq!(output.state(), ServerHealthState::Degraded);
    assert_eq!(
        logger.events,
        vec![ServerHealthProductEvent::ServerHealthDegraded {
            state: ServerHealthState::Degraded,
            component_count: 2,
        }]
    );
}

#[test]
fn health_is_unavailable_when_any_component_is_unavailable() {
    let probe = FakeHealthProbe {
        components: vec![
            component("server-config", HealthComponentState::Healthy, None),
            component(
                "metadata-store",
                HealthComponentState::Unavailable,
                Some("METADATA_UNAVAILABLE"),
            ),
        ],
    };
    let mut logger = FakeProductLogger::default();

    let output =
        GetServerHealthUsecase::new().execute(GetServerHealthInput::new(), &probe, &mut logger);

    assert_eq!(output.state(), ServerHealthState::Unavailable);
    assert_eq!(
        logger.events,
        vec![ServerHealthProductEvent::ServerHealthDegraded {
            state: ServerHealthState::Unavailable,
            component_count: 2,
        }]
    );
}

#[test]
fn health_product_log_excludes_component_details_and_raw_payloads() {
    let event = ServerHealthProductEvent::ServerHealthDegraded {
        state: ServerHealthState::Degraded,
        component_count: 1,
    };

    let rendered = format!("{event:?}");

    assert!(rendered.contains("Degraded"));
    assert!(!rendered.contains("metadata-store"));
    assert!(!rendered.contains("secret"));
    assert!(!rendered.contains("token"));
}

fn component(
    name: &str,
    state: HealthComponentState,
    error_code: Option<&str>,
) -> HealthComponentSummary {
    HealthComponentSummary::new(name, state, error_code).expect("valid component")
}
