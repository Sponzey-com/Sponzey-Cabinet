use std::cell::RefCell;

use cabinet_core::server_config::{ObjectStorageBackend, ServerConfig};
use cabinet_ports::server_health::{
    HealthComponentState, HealthComponentSummary, ServerHealthProbe,
};
use cabinet_usecases::server_health::{
    GetServerHealthInput, GetServerHealthOutput, GetServerHealthUsecase, ServerHealthProductEvent,
    ServerHealthProductLogger, ServerHealthState,
};

use crate::adapter::{ServerUsecaseTarget, UsecaseInputDto, UsecaseOutputDto};
use crate::errors::{ServerBoundaryError, ServerErrorCode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalDevServerHealthProbe {
    components: Vec<HealthComponentSummary>,
}

impl LocalDevServerHealthProbe {
    pub fn from_config(config: &ServerConfig) -> Self {
        let object_storage_state = match config.object_storage_backend() {
            ObjectStorageBackend::LocalDisk => {
                HealthComponentSummary::new("object-storage", HealthComponentState::Healthy, None)
            }
            ObjectStorageBackend::S3Compatible => HealthComponentSummary::new(
                "object-storage",
                HealthComponentState::Degraded,
                Some("OBJECT_STORAGE_PROBE_NOT_CONNECTED"),
            ),
        }
        .expect("static component name is valid");

        Self {
            components: vec![
                HealthComponentSummary::new("server-config", HealthComponentState::Healthy, None)
                    .expect("static component name is valid"),
                HealthComponentSummary::new("metadata-store", HealthComponentState::Healthy, None)
                    .expect("static component name is valid"),
                object_storage_state,
            ],
        }
    }
}

impl ServerHealthProbe for LocalDevServerHealthProbe {
    fn check_components(&self) -> Vec<HealthComponentSummary> {
        self.components.clone()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct NoopServerHealthProductLogger {
    events: Vec<ServerHealthProductEvent>,
}

impl NoopServerHealthProductLogger {
    pub fn events(&self) -> &[ServerHealthProductEvent] {
        &self.events
    }
}

impl ServerHealthProductLogger for NoopServerHealthProductLogger {
    fn write_product(&mut self, event: ServerHealthProductEvent) {
        self.events.push(event);
    }
}

pub struct HealthRouteTarget<P, L> {
    probe: P,
    product_logger: RefCell<L>,
}

impl<P, L> HealthRouteTarget<P, L> {
    pub const fn new(probe: P, product_logger: L) -> Self {
        Self {
            probe,
            product_logger: RefCell::new(product_logger),
        }
    }
}

impl<P, L> ServerUsecaseTarget for HealthRouteTarget<P, L>
where
    P: ServerHealthProbe,
    L: ServerHealthProductLogger,
{
    fn handle(&self, input: UsecaseInputDto) -> Result<UsecaseOutputDto, ServerBoundaryError> {
        if input.route_id() != "health.check" {
            return Err(ServerBoundaryError::new(
                ServerErrorCode::TargetFailed,
                "unsupported route target",
            ));
        }

        let output = GetServerHealthUsecase::new().execute(
            GetServerHealthInput::new(),
            &self.probe,
            &mut *self.product_logger.borrow_mut(),
        );
        let status_code = status_code_for_health(output.state());
        let body = render_health_json(&output);
        Ok(UsecaseOutputDto::new(status_code, &body))
    }
}

pub fn build_local_dev_health_target(
    config: ServerConfig,
    product_logger: NoopServerHealthProductLogger,
) -> HealthRouteTarget<LocalDevServerHealthProbe, NoopServerHealthProductLogger> {
    HealthRouteTarget::new(
        LocalDevServerHealthProbe::from_config(&config),
        product_logger,
    )
}

fn status_code_for_health(state: ServerHealthState) -> u16 {
    match state {
        ServerHealthState::Healthy | ServerHealthState::Degraded => 200,
        ServerHealthState::Unavailable => 503,
    }
}

fn render_health_json(output: &GetServerHealthOutput) -> String {
    let components = output
        .components()
        .iter()
        .map(render_component_json)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"state\":\"{}\",\"components\":[{}]}}",
        output.state().as_str(),
        components
    )
}

fn render_component_json(component: &HealthComponentSummary) -> String {
    let mut fields = vec![
        format!("\"name\":\"{}\"", escape_json(component.name())),
        format!("\"state\":\"{}\"", component.state().as_str()),
    ];
    if let Some(error_code) = component.error_code() {
        fields.push(format!("\"error_code\":\"{}\"", escape_json(error_code)));
    }
    format!("{{{}}}", fields.join(","))
}

fn escape_json(value: &str) -> String {
    value
        .chars()
        .flat_map(|ch| match ch {
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect::<Vec<_>>(),
            other => vec![other],
        })
        .collect()
}
