use cabinet_domain::connector::{
    ConnectorCredentialReference, ConnectorDefinition, ConnectorId, ConnectorInstallation,
    ConnectorInstallationId, ConnectorInstallationState, ConnectorKind, ConnectorScopeSet,
};
use cabinet_ports::connector::{
    ConnectorDefinitionRegistryPort, ConnectorInstallationRepositoryPort, ConnectorPortError,
};

#[derive(Default)]
struct FakeDefinitionRegistry {
    definitions: Vec<ConnectorDefinition>,
}

impl ConnectorDefinitionRegistryPort for FakeDefinitionRegistry {
    fn list_definitions(&self) -> Result<Vec<ConnectorDefinition>, ConnectorPortError> {
        Ok(self.definitions.clone())
    }

    fn find_definition(
        &self,
        id: &ConnectorId,
    ) -> Result<Option<ConnectorDefinition>, ConnectorPortError> {
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
}

impl ConnectorInstallationRepositoryPort for FakeInstallationRepository {
    fn save_installation(
        &mut self,
        installation: ConnectorInstallation,
    ) -> Result<(), ConnectorPortError> {
        self.installations
            .retain(|existing| existing.id() != installation.id());
        self.installations.push(installation);
        Ok(())
    }

    fn find_installation(
        &self,
        id: &ConnectorInstallationId,
    ) -> Result<Option<ConnectorInstallation>, ConnectorPortError> {
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
        Ok(self
            .installations
            .iter()
            .filter(|installation| installation.workspace_id_hash() == workspace_id_hash)
            .cloned()
            .collect())
    }
}

#[test]
fn connector_ports_support_definition_registry_and_installation_repository_contracts() {
    let registry = FakeDefinitionRegistry {
        definitions: vec![definition()],
    };
    let mut repository = FakeInstallationRepository::default();

    assert_eq!(registry.list_definitions().expect("list").len(), 1);
    assert!(
        registry
            .find_definition(&ConnectorId::new("connector-jira").expect("connector id"))
            .expect("find")
            .is_some()
    );

    repository
        .save_installation(installation())
        .expect("save installation");
    assert!(
        repository
            .find_installation(
                &ConnectorInstallationId::new("installation-1").expect("installation id"),
            )
            .expect("find installation")
            .is_some()
    );
    assert_eq!(
        repository
            .list_installations("workspace-hash-1")
            .expect("list installations")
            .len(),
        1,
    );
}

#[test]
fn connector_port_error_has_stable_code() {
    assert_eq!(
        ConnectorPortError::StoreUnavailable.code(),
        "connector_port.store_unavailable",
    );
}

fn definition() -> ConnectorDefinition {
    ConnectorDefinition::new(
        ConnectorId::new("connector-jira").expect("connector id"),
        ConnectorKind::Jira,
        "Jira",
        ConnectorScopeSet::read_write(),
    )
    .expect("definition")
}

fn installation() -> ConnectorInstallation {
    ConnectorInstallation::new(
        ConnectorInstallationId::new("installation-1").expect("installation id"),
        "workspace-hash-1",
        ConnectorId::new("connector-jira").expect("connector id"),
        ConnectorCredentialReference::new("connector-credential:installation-1")
            .expect("credential"),
        ConnectorScopeSet::read_only(),
        ConnectorInstallationState::Installed,
    )
    .expect("installation")
}
