use cabinet_domain::connector::{
    ConnectorActivity, ConnectorDefinition, ConnectorExternalObjectReference, ConnectorId,
    ConnectorInstallation, ConnectorInstallationId,
};

pub trait ConnectorDefinitionRegistryPort {
    fn list_definitions(&self) -> Result<Vec<ConnectorDefinition>, ConnectorPortError>;

    fn find_definition(
        &self,
        id: &ConnectorId,
    ) -> Result<Option<ConnectorDefinition>, ConnectorPortError>;
}

pub trait ConnectorInstallationRepositoryPort {
    fn save_installation(
        &mut self,
        installation: ConnectorInstallation,
    ) -> Result<(), ConnectorPortError>;

    fn find_installation(
        &self,
        id: &ConnectorInstallationId,
    ) -> Result<Option<ConnectorInstallation>, ConnectorPortError>;

    fn list_installations(
        &self,
        workspace_id_hash: &str,
    ) -> Result<Vec<ConnectorInstallation>, ConnectorPortError>;
}

pub trait ConnectorGatewayPort {
    fn sync(
        &self,
        installation: &ConnectorInstallation,
    ) -> Result<ConnectorGatewaySyncResult, ConnectorGatewayPortError>;
}

pub trait ConnectorActivityStorePort {
    fn record_activity(&mut self, activity: ConnectorActivity) -> Result<(), ConnectorPortError>;

    fn list_activities(
        &self,
        workspace_id_hash: &str,
    ) -> Result<Vec<ConnectorActivity>, ConnectorPortError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorGatewaySyncResult {
    external_objects: Vec<ConnectorExternalObjectReference>,
}

impl ConnectorGatewaySyncResult {
    pub fn new(external_objects: Vec<ConnectorExternalObjectReference>) -> Self {
        Self { external_objects }
    }

    pub fn external_objects(&self) -> &[ConnectorExternalObjectReference] {
        &self.external_objects
    }

    pub fn object_count(&self) -> u32 {
        self.external_objects.len() as u32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectorGatewayPortError {
    GatewayUnavailable,
}

impl ConnectorGatewayPortError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::GatewayUnavailable => "connector_gateway.gateway_unavailable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectorPortError {
    StoreUnavailable,
}

impl ConnectorPortError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::StoreUnavailable => "connector_port.store_unavailable",
        }
    }
}
