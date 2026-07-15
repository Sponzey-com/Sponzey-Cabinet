use std::cell::Cell;

use cabinet_domain::connector::{ConnectorExternalObjectReference, ConnectorInstallation};
use cabinet_ports::connector::{
    ConnectorGatewayPort, ConnectorGatewayPortError, ConnectorGatewaySyncResult,
};

#[derive(Debug)]
pub struct FakeConnectorGateway {
    result: Result<ConnectorGatewaySyncResult, ConnectorGatewayPortError>,
    call_count: Cell<usize>,
}

impl FakeConnectorGateway {
    pub fn succeeding(external_objects: Vec<ConnectorExternalObjectReference>) -> Self {
        Self {
            result: Ok(ConnectorGatewaySyncResult::new(external_objects)),
            call_count: Cell::new(0),
        }
    }

    pub const fn failing(error: ConnectorGatewayPortError) -> Self {
        Self {
            result: Err(error),
            call_count: Cell::new(0),
        }
    }

    pub fn call_count(&self) -> usize {
        self.call_count.get()
    }
}

impl ConnectorGatewayPort for FakeConnectorGateway {
    fn sync(
        &self,
        _installation: &ConnectorInstallation,
    ) -> Result<ConnectorGatewaySyncResult, ConnectorGatewayPortError> {
        self.call_count.set(self.call_count.get() + 1);
        self.result.clone()
    }
}
