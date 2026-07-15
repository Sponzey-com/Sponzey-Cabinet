use cabinet_domain::connector::{
    ConnectorCredentialReference, ConnectorDefinition, ConnectorId, ConnectorInstallation,
    ConnectorInstallationId, ConnectorInstallationState, ConnectorKind, ConnectorScopeSet,
};
use cabinet_ports::connector::{
    ConnectorDefinitionRegistryPort, ConnectorInstallationRepositoryPort, ConnectorPortError,
};
use cabinet_usecases::connector::{
    ConnectorUsecaseError, DisableConnectorInput, DisableConnectorUsecase, GetConnectorStatusInput,
    GetConnectorStatusUsecase, InstallConnectorInput, InstallConnectorUsecase,
    ListConnectorDefinitionsUsecase, StartConnectorSyncInput, StartConnectorSyncUsecase,
};

#[derive(Default)]
struct FakeDefinitionRegistry {
    definitions: Vec<ConnectorDefinition>,
    fail: bool,
}

impl ConnectorDefinitionRegistryPort for FakeDefinitionRegistry {
    fn list_definitions(&self) -> Result<Vec<ConnectorDefinition>, ConnectorPortError> {
        if self.fail {
            return Err(ConnectorPortError::StoreUnavailable);
        }
        Ok(self.definitions.clone())
    }

    fn find_definition(
        &self,
        id: &ConnectorId,
    ) -> Result<Option<ConnectorDefinition>, ConnectorPortError> {
        if self.fail {
            return Err(ConnectorPortError::StoreUnavailable);
        }
        Ok(self
            .definitions
            .iter()
            .find(|definition| definition.id() == id)
            .cloned())
    }
}

#[derive(Default)]
struct FakeInstallationRepository {
    installations: Vec<ConnectorInstallation>,
    fail: bool,
}

impl ConnectorInstallationRepositoryPort for FakeInstallationRepository {
    fn save_installation(
        &mut self,
        installation: ConnectorInstallation,
    ) -> Result<(), ConnectorPortError> {
        if self.fail {
            return Err(ConnectorPortError::StoreUnavailable);
        }
        self.installations
            .retain(|existing| existing.id() != installation.id());
        self.installations.push(installation);
        Ok(())
    }

    fn find_installation(
        &self,
        id: &ConnectorInstallationId,
    ) -> Result<Option<ConnectorInstallation>, ConnectorPortError> {
        if self.fail {
            return Err(ConnectorPortError::StoreUnavailable);
        }
        Ok(self
            .installations
            .iter()
            .find(|installation| installation.id() == id)
            .cloned())
    }

    fn list_installations(
        &self,
        workspace_id_hash: &str,
    ) -> Result<Vec<ConnectorInstallation>, ConnectorPortError> {
        if self.fail {
            return Err(ConnectorPortError::StoreUnavailable);
        }
        Ok(self
            .installations
            .iter()
            .filter(|installation| installation.workspace_id_hash() == workspace_id_hash)
            .cloned()
            .collect())
    }
}

#[test]
fn list_connector_definitions_returns_available_definitions() {
    let registry = FakeDefinitionRegistry {
        definitions: vec![definition("connector-slack", ConnectorKind::Slack)],
        fail: false,
    };

    let output = ListConnectorDefinitionsUsecase::new()
        .execute(&registry)
        .expect("list definitions");

    assert_eq!(output.definitions().len(), 1);
    assert_eq!(output.definitions()[0].id().as_str(), "connector-slack");
}

#[test]
fn list_connector_definitions_maps_registry_failure() {
    let registry = FakeDefinitionRegistry {
        definitions: Vec::new(),
        fail: true,
    };

    let error = ListConnectorDefinitionsUsecase::new()
        .execute(&registry)
        .expect_err("registry failure");

    assert_eq!(error, ConnectorUsecaseError::RegistryUnavailable);
    assert_eq!(error.code(), "connector_usecase.registry_unavailable");
}

#[test]
fn install_connector_rejects_write_scope_for_read_only_definition() {
    let registry = FakeDefinitionRegistry {
        definitions: vec![
            ConnectorDefinition::new(
                ConnectorId::new("connector-slack").expect("connector id"),
                ConnectorKind::Slack,
                "Slack",
                ConnectorScopeSet::read_only(),
            )
            .expect("definition"),
        ],
        fail: false,
    };
    let mut repository = FakeInstallationRepository::default();

    let error = InstallConnectorUsecase::new()
        .execute(
            InstallConnectorInput::new(
                ConnectorInstallationId::new("installation-1").expect("installation id"),
                "workspace-hash-1",
                ConnectorId::new("connector-slack").expect("connector id"),
                credential(),
                ConnectorScopeSet::read_write(),
            ),
            &registry,
            &mut repository,
        )
        .expect_err("scope denied");

    assert_eq!(error, ConnectorUsecaseError::ScopeDenied);
    assert_eq!(error.code(), "connector_usecase.scope_denied");
}

#[test]
fn install_get_status_and_disable_connector() {
    let registry = FakeDefinitionRegistry {
        definitions: vec![definition("connector-jira", ConnectorKind::Jira)],
        fail: false,
    };
    let mut repository = FakeInstallationRepository::default();

    let install_output = InstallConnectorUsecase::new()
        .execute(
            InstallConnectorInput::new(
                ConnectorInstallationId::new("installation-1").expect("installation id"),
                "workspace-hash-1",
                ConnectorId::new("connector-jira").expect("connector id"),
                credential(),
                ConnectorScopeSet::read_only(),
            ),
            &registry,
            &mut repository,
        )
        .expect("install connector");

    assert_eq!(
        install_output.installation().state(),
        ConnectorInstallationState::Installed,
    );

    let status_output = GetConnectorStatusUsecase::new()
        .execute(
            GetConnectorStatusInput::new(
                ConnectorInstallationId::new("installation-1").expect("installation id"),
            ),
            &repository,
        )
        .expect("status");

    assert_eq!(
        status_output.installation().state(),
        ConnectorInstallationState::Installed,
    );

    let disabled_output = DisableConnectorUsecase::new()
        .execute(
            DisableConnectorInput::new(
                ConnectorInstallationId::new("installation-1").expect("installation id"),
            ),
            &mut repository,
        )
        .expect("disable");

    assert_eq!(
        disabled_output.installation().state(),
        ConnectorInstallationState::Disabled,
    );
}

#[test]
fn start_connector_sync_moves_installed_connector_to_sync_queued() {
    let mut repository = FakeInstallationRepository::default();
    repository
        .save_installation(installation(ConnectorInstallationState::Installed))
        .expect("seed installation");

    let output = StartConnectorSyncUsecase::new()
        .execute(
            StartConnectorSyncInput::new(
                ConnectorInstallationId::new("installation-1").expect("installation id"),
            ),
            &mut repository,
        )
        .expect("start sync");

    assert_eq!(
        output.installation().state(),
        ConnectorInstallationState::SyncQueued,
    );
}

#[test]
fn start_connector_sync_returns_missing_installation_error() {
    let mut repository = FakeInstallationRepository::default();

    let error = StartConnectorSyncUsecase::new()
        .execute(
            StartConnectorSyncInput::new(
                ConnectorInstallationId::new("missing-installation").expect("installation id"),
            ),
            &mut repository,
        )
        .expect_err("missing installation");

    assert_eq!(error, ConnectorUsecaseError::InstallationNotFound);
    assert_eq!(error.code(), "connector_usecase.installation_not_found");
}

fn definition(id: &str, kind: ConnectorKind) -> ConnectorDefinition {
    ConnectorDefinition::new(
        ConnectorId::new(id).expect("connector id"),
        kind,
        id,
        ConnectorScopeSet::read_write(),
    )
    .expect("definition")
}

fn installation(state: ConnectorInstallationState) -> ConnectorInstallation {
    ConnectorInstallation::new(
        ConnectorInstallationId::new("installation-1").expect("installation id"),
        "workspace-hash-1",
        ConnectorId::new("connector-jira").expect("connector id"),
        credential(),
        ConnectorScopeSet::read_only(),
        state,
    )
    .expect("installation")
}

fn credential() -> ConnectorCredentialReference {
    ConnectorCredentialReference::new("connector-credential:installation-1")
        .expect("credential reference")
}
