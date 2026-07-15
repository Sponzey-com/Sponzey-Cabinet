use cabinet_adapters::static_connector_definition_registry::StaticConnectorDefinitionRegistry;
use cabinet_domain::connector::{ConnectorAction, ConnectorId, ConnectorKind};
use cabinet_ports::connector::ConnectorDefinitionRegistryPort;

#[test]
fn static_connector_definition_registry_returns_phase005_baseline_descriptors() {
    let registry = StaticConnectorDefinitionRegistry::phase005_baseline();

    let definitions = registry.list_definitions().expect("definitions");

    assert_eq!(definitions.len(), 3);
    assert!(
        definitions
            .iter()
            .any(|definition| definition.kind() == ConnectorKind::Slack)
    );
    assert!(
        definitions
            .iter()
            .any(|definition| definition.kind() == ConnectorKind::Teams)
    );
    assert!(
        definitions
            .iter()
            .any(|definition| definition.kind() == ConnectorKind::Jira)
    );
}

#[test]
fn static_connector_definition_registry_distinguishes_read_only_and_read_write_descriptors() {
    let registry = StaticConnectorDefinitionRegistry::phase005_baseline();
    let teams = registry
        .find_definition(&ConnectorId::new("connector-teams").expect("teams id"))
        .expect("find teams")
        .expect("teams descriptor");
    let jira = registry
        .find_definition(&ConnectorId::new("connector-jira").expect("jira id"))
        .expect("find jira")
        .expect("jira descriptor");

    assert!(teams.scopes().allows(ConnectorAction::Read));
    assert!(!teams.scopes().allows(ConnectorAction::Write));
    assert!(jira.scopes().allows(ConnectorAction::Read));
    assert!(jira.scopes().allows(ConnectorAction::Write));
}

#[test]
fn static_connector_definition_registry_does_not_expose_sensitive_fixtures() {
    let registry = StaticConnectorDefinitionRegistry::phase005_baseline();
    let debug = format!("{registry:?}");

    assert!(!debug.contains("connector_access_token_fixture"));
    assert!(!debug.contains("connector_refresh_token_fixture"));
    assert!(!debug.contains("connector_client_secret_fixture"));
    assert!(!debug.contains("connector_payload"));
}
