use cabinet_ports::server_health::{
    HealthComponentState, HealthComponentSummary, ServerHealthProbe,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetServerHealthInput {
    include_components: bool,
}

impl GetServerHealthInput {
    pub const fn new() -> Self {
        Self {
            include_components: true,
        }
    }

    pub const fn without_components() -> Self {
        Self {
            include_components: false,
        }
    }
}

impl Default for GetServerHealthInput {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetServerHealthOutput {
    state: ServerHealthState,
    components: Vec<HealthComponentSummary>,
}

impl GetServerHealthOutput {
    pub const fn state(&self) -> ServerHealthState {
        self.state
    }

    pub fn components(&self) -> &[HealthComponentSummary] {
        &self.components
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerHealthState {
    Healthy,
    Degraded,
    Unavailable,
}

impl ServerHealthState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Healthy => "Healthy",
            Self::Degraded => "Degraded",
            Self::Unavailable => "Unavailable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerHealthProductEvent {
    ServerHealthDegraded {
        state: ServerHealthState,
        component_count: usize,
    },
}

impl ServerHealthProductEvent {
    pub const fn event_name(&self) -> &'static str {
        match self {
            Self::ServerHealthDegraded { .. } => "server.health.degraded",
        }
    }
}

pub trait ServerHealthProductLogger {
    fn write_product(&mut self, event: ServerHealthProductEvent);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetServerHealthUsecase;

impl GetServerHealthUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: GetServerHealthInput,
        probe: &impl ServerHealthProbe,
        product_logger: &mut impl ServerHealthProductLogger,
    ) -> GetServerHealthOutput {
        let components = probe.check_components();
        let state = calculate_state(&components);

        if state != ServerHealthState::Healthy {
            product_logger.write_product(ServerHealthProductEvent::ServerHealthDegraded {
                state,
                component_count: components.len(),
            });
        }

        GetServerHealthOutput {
            state,
            components: if input.include_components {
                components
            } else {
                Vec::new()
            },
        }
    }
}

impl Default for GetServerHealthUsecase {
    fn default() -> Self {
        Self::new()
    }
}

fn calculate_state(components: &[HealthComponentSummary]) -> ServerHealthState {
    if components
        .iter()
        .any(|component| component.state() == HealthComponentState::Unavailable)
    {
        return ServerHealthState::Unavailable;
    }
    if components
        .iter()
        .any(|component| component.state() == HealthComponentState::Degraded)
    {
        return ServerHealthState::Degraded;
    }
    ServerHealthState::Healthy
}
