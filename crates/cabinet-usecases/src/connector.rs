use cabinet_domain::connector::{
    ConnectorActivity, ConnectorActivityId, ConnectorActivityKind, ConnectorCredentialReference,
    ConnectorDefinition, ConnectorError, ConnectorId, ConnectorInstallation,
    ConnectorInstallationEvent, ConnectorInstallationId, ConnectorInstallationState,
    ConnectorScopeSet, transition_connector_installation,
};
use cabinet_ports::connector::{
    ConnectorActivityStorePort, ConnectorDefinitionRegistryPort, ConnectorGatewayPort,
    ConnectorInstallationRepositoryPort, ConnectorPortError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListConnectorDefinitionsOutput {
    definitions: Vec<ConnectorDefinition>,
}

impl ListConnectorDefinitionsOutput {
    fn new(definitions: Vec<ConnectorDefinition>) -> Self {
        Self { definitions }
    }

    pub fn definitions(&self) -> &[ConnectorDefinition] {
        &self.definitions
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallConnectorInput {
    installation_id: ConnectorInstallationId,
    workspace_id_hash: String,
    connector_id: ConnectorId,
    credential_reference: ConnectorCredentialReference,
    requested_scopes: ConnectorScopeSet,
}

impl InstallConnectorInput {
    pub fn new(
        installation_id: ConnectorInstallationId,
        workspace_id_hash: &str,
        connector_id: ConnectorId,
        credential_reference: ConnectorCredentialReference,
        requested_scopes: ConnectorScopeSet,
    ) -> Self {
        Self {
            installation_id,
            workspace_id_hash: workspace_id_hash.to_string(),
            connector_id,
            credential_reference,
            requested_scopes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisableConnectorInput {
    installation_id: ConnectorInstallationId,
}

impl DisableConnectorInput {
    pub fn new(installation_id: ConnectorInstallationId) -> Self {
        Self { installation_id }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetConnectorStatusInput {
    installation_id: ConnectorInstallationId,
}

impl GetConnectorStatusInput {
    pub fn new(installation_id: ConnectorInstallationId) -> Self {
        Self { installation_id }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartConnectorSyncInput {
    installation_id: ConnectorInstallationId,
}

impl StartConnectorSyncInput {
    pub fn new(installation_id: ConnectorInstallationId) -> Self {
        Self { installation_id }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunConnectorSyncInput {
    installation_id: ConnectorInstallationId,
}

impl RunConnectorSyncInput {
    pub fn new(installation_id: ConnectorInstallationId) -> Self {
        Self { installation_id }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorInstallationOutput {
    installation: ConnectorInstallation,
}

impl ConnectorInstallationOutput {
    fn new(installation: ConnectorInstallation) -> Self {
        Self { installation }
    }

    pub fn installation(&self) -> &ConnectorInstallation {
        &self.installation
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunConnectorSyncOutput {
    installation: ConnectorInstallation,
    object_count: u32,
}

impl RunConnectorSyncOutput {
    fn new(installation: ConnectorInstallation, object_count: u32) -> Self {
        Self {
            installation,
            object_count,
        }
    }

    pub fn installation(&self) -> &ConnectorInstallation {
        &self.installation
    }

    pub const fn object_count(&self) -> u32 {
        self.object_count
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListConnectorDefinitionsUsecase;

impl ListConnectorDefinitionsUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        registry: &impl ConnectorDefinitionRegistryPort,
    ) -> Result<ListConnectorDefinitionsOutput, ConnectorUsecaseError> {
        let definitions = registry
            .list_definitions()
            .map_err(ConnectorUsecaseError::from_registry_error)?;
        Ok(ListConnectorDefinitionsOutput::new(definitions))
    }
}

impl Default for ListConnectorDefinitionsUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstallConnectorUsecase;

impl InstallConnectorUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: InstallConnectorInput,
        registry: &impl ConnectorDefinitionRegistryPort,
        repository: &mut impl ConnectorInstallationRepositoryPort,
    ) -> Result<ConnectorInstallationOutput, ConnectorUsecaseError> {
        let definition = registry
            .find_definition(&input.connector_id)
            .map_err(ConnectorUsecaseError::from_registry_error)?
            .ok_or(ConnectorUsecaseError::DefinitionNotFound)?;

        if !definition.scopes().contains_all(&input.requested_scopes) {
            return Err(ConnectorUsecaseError::ScopeDenied);
        }

        let authorization_requested = transition_connector_installation(
            ConnectorInstallationState::NotInstalled,
            ConnectorInstallationEvent::RequestAuthorization,
        )
        .map_err(ConnectorUsecaseError::from_domain_error)?;
        let installed = transition_connector_installation(
            authorization_requested,
            ConnectorInstallationEvent::Authorize,
        )
        .map_err(ConnectorUsecaseError::from_domain_error)?;
        let installation = ConnectorInstallation::new(
            input.installation_id,
            &input.workspace_id_hash,
            input.connector_id,
            input.credential_reference,
            input.requested_scopes,
            installed,
        )
        .map_err(ConnectorUsecaseError::from_domain_error)?;
        repository
            .save_installation(installation.clone())
            .map_err(ConnectorUsecaseError::from_repository_error)?;
        Ok(ConnectorInstallationOutput::new(installation))
    }
}

impl Default for InstallConnectorUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisableConnectorUsecase;

impl DisableConnectorUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: DisableConnectorInput,
        repository: &mut impl ConnectorInstallationRepositoryPort,
    ) -> Result<ConnectorInstallationOutput, ConnectorUsecaseError> {
        let installation = find_installation(repository, &input.installation_id)?;
        let disabled_state = transition_connector_installation(
            installation.state(),
            ConnectorInstallationEvent::Disable,
        )
        .map_err(ConnectorUsecaseError::from_domain_error)?;
        let disabled = installation.with_state(disabled_state);
        repository
            .save_installation(disabled.clone())
            .map_err(ConnectorUsecaseError::from_repository_error)?;
        Ok(ConnectorInstallationOutput::new(disabled))
    }
}

impl Default for DisableConnectorUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetConnectorStatusUsecase;

impl GetConnectorStatusUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: GetConnectorStatusInput,
        repository: &impl ConnectorInstallationRepositoryPort,
    ) -> Result<ConnectorInstallationOutput, ConnectorUsecaseError> {
        let installation = find_installation(repository, &input.installation_id)?;
        Ok(ConnectorInstallationOutput::new(installation))
    }
}

impl Default for GetConnectorStatusUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartConnectorSyncUsecase;

impl StartConnectorSyncUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: StartConnectorSyncInput,
        repository: &mut impl ConnectorInstallationRepositoryPort,
    ) -> Result<ConnectorInstallationOutput, ConnectorUsecaseError> {
        let installation = find_installation(repository, &input.installation_id)?;
        let sync_queued = transition_connector_installation(
            installation.state(),
            ConnectorInstallationEvent::QueueSync,
        )
        .map_err(ConnectorUsecaseError::from_domain_error)?;
        let queued = installation.with_state(sync_queued);
        repository
            .save_installation(queued.clone())
            .map_err(ConnectorUsecaseError::from_repository_error)?;
        Ok(ConnectorInstallationOutput::new(queued))
    }
}

impl Default for StartConnectorSyncUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunConnectorSyncUsecase;

impl RunConnectorSyncUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: RunConnectorSyncInput,
        repository: &mut impl ConnectorInstallationRepositoryPort,
        gateway: &impl ConnectorGatewayPort,
        activity_store: &mut impl ConnectorActivityStorePort,
    ) -> Result<RunConnectorSyncOutput, ConnectorUsecaseError> {
        let installation = find_installation(repository, &input.installation_id)?;
        let syncing = transition_connector_installation(
            installation.state(),
            ConnectorInstallationEvent::StartSync,
        )
        .map_err(ConnectorUsecaseError::from_domain_error)?;
        let syncing_installation = installation.with_state(syncing);

        match gateway.sync(&syncing_installation) {
            Ok(result) => {
                let synced = transition_connector_installation(
                    syncing,
                    ConnectorInstallationEvent::CompleteSync,
                )
                .map_err(ConnectorUsecaseError::from_domain_error)?;
                let synced_installation = syncing_installation.with_state(synced);
                repository
                    .save_installation(synced_installation.clone())
                    .map_err(ConnectorUsecaseError::from_repository_error)?;
                let object_count = result.object_count();
                activity_store
                    .record_activity(build_sync_activity(
                        &synced_installation,
                        ConnectorActivityKind::SyncCompleted,
                        object_count,
                        None,
                    )?)
                    .map_err(ConnectorUsecaseError::from_activity_store_error)?;
                Ok(RunConnectorSyncOutput::new(
                    synced_installation,
                    object_count,
                ))
            }
            Err(error) => {
                let retry = transition_connector_installation(
                    syncing,
                    ConnectorInstallationEvent::ScheduleRetry,
                )
                .map_err(ConnectorUsecaseError::from_domain_error)?;
                let retry_installation = syncing_installation.with_state(retry);
                repository
                    .save_installation(retry_installation.clone())
                    .map_err(ConnectorUsecaseError::from_repository_error)?;
                activity_store
                    .record_activity(build_sync_activity(
                        &retry_installation,
                        ConnectorActivityKind::SyncFailed,
                        0,
                        Some(error.code()),
                    )?)
                    .map_err(ConnectorUsecaseError::from_activity_store_error)?;
                Ok(RunConnectorSyncOutput::new(retry_installation, 0))
            }
        }
    }
}

impl Default for RunConnectorSyncUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectorUsecaseError {
    InvalidInput,
    RegistryUnavailable,
    RepositoryUnavailable,
    DefinitionNotFound,
    InstallationNotFound,
    ScopeDenied,
    InvalidTransition,
    GatewayUnavailable,
    ActivityStoreUnavailable,
}

impl ConnectorUsecaseError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "connector_usecase.invalid_input",
            Self::RegistryUnavailable => "connector_usecase.registry_unavailable",
            Self::RepositoryUnavailable => "connector_usecase.repository_unavailable",
            Self::DefinitionNotFound => "connector_usecase.definition_not_found",
            Self::InstallationNotFound => "connector_usecase.installation_not_found",
            Self::ScopeDenied => "connector_usecase.scope_denied",
            Self::InvalidTransition => "connector_usecase.invalid_transition",
            Self::GatewayUnavailable => "connector_usecase.gateway_unavailable",
            Self::ActivityStoreUnavailable => "connector_usecase.activity_store_unavailable",
        }
    }

    fn from_registry_error(error: ConnectorPortError) -> Self {
        match error {
            ConnectorPortError::StoreUnavailable => Self::RegistryUnavailable,
        }
    }

    fn from_repository_error(error: ConnectorPortError) -> Self {
        match error {
            ConnectorPortError::StoreUnavailable => Self::RepositoryUnavailable,
        }
    }

    fn from_domain_error(error: ConnectorError) -> Self {
        match error {
            ConnectorError::ScopeDenied => Self::ScopeDenied,
            ConnectorError::InvalidTransition => Self::InvalidTransition,
            ConnectorError::InvalidId
            | ConnectorError::InvalidName
            | ConnectorError::EmptyScope
            | ConnectorError::InvalidCredentialReference
            | ConnectorError::InvalidExternalObjectReference
            | ConnectorError::InvalidActivity => Self::InvalidInput,
        }
    }

    fn from_activity_store_error(error: ConnectorPortError) -> Self {
        match error {
            ConnectorPortError::StoreUnavailable => Self::ActivityStoreUnavailable,
        }
    }
}

fn find_installation(
    repository: &impl ConnectorInstallationRepositoryPort,
    installation_id: &ConnectorInstallationId,
) -> Result<ConnectorInstallation, ConnectorUsecaseError> {
    repository
        .find_installation(installation_id)
        .map_err(ConnectorUsecaseError::from_repository_error)?
        .ok_or(ConnectorUsecaseError::InstallationNotFound)
}

fn build_sync_activity(
    installation: &ConnectorInstallation,
    kind: ConnectorActivityKind,
    object_count: u32,
    error_code: Option<&str>,
) -> Result<ConnectorActivity, ConnectorUsecaseError> {
    let suffix = match kind {
        ConnectorActivityKind::SyncCompleted => "sync-completed",
        ConnectorActivityKind::SyncFailed => "sync-failed",
    };
    ConnectorActivity::new(
        ConnectorActivityId::new(&format!(
            "activity:{}:{}",
            installation.id().as_str(),
            suffix
        ))
        .map_err(ConnectorUsecaseError::from_domain_error)?,
        installation.id().clone(),
        installation.connector_id().clone(),
        installation.workspace_id_hash(),
        kind,
        object_count,
        error_code,
    )
    .map_err(ConnectorUsecaseError::from_domain_error)
}
