use cabinet_adapters::fake_connector_gateway::FakeConnectorGateway;
use cabinet_adapters::local_connector_activity_store::LocalConnectorActivityStore;
use cabinet_domain::connector::{
    ConnectorActivity, ConnectorActivityId, ConnectorActivityKind, ConnectorCredentialReference,
    ConnectorExternalObjectReference, ConnectorId, ConnectorInstallation, ConnectorInstallationId,
    ConnectorInstallationState, ConnectorScopeSet,
};
use cabinet_ports::connector::{
    ConnectorActivityStorePort, ConnectorGatewayPort, ConnectorGatewayPortError,
};

#[test]
fn fake_connector_gateway_returns_configured_success_or_failure() {
    let success = FakeConnectorGateway::succeeding(vec![
        ConnectorExternalObjectReference::new("external-object:jira-1").expect("object"),
    ]);

    let result = success.sync(&installation()).expect("sync");

    assert_eq!(result.object_count(), 1);
    assert_eq!(success.call_count(), 1);

    let failure = FakeConnectorGateway::failing(ConnectorGatewayPortError::GatewayUnavailable);
    assert_eq!(
        failure.sync(&installation()),
        Err(ConnectorGatewayPortError::GatewayUnavailable),
    );
    assert_eq!(failure.call_count(), 1);
}

#[test]
fn local_connector_activity_store_records_and_lists_by_workspace() {
    let mut store = LocalConnectorActivityStore::default();
    store
        .record_activity(activity("activity-1", "workspace-hash-1"))
        .expect("record first");
    store
        .record_activity(activity("activity-2", "workspace-hash-2"))
        .expect("record second");

    let activities = store
        .list_activities("workspace-hash-1")
        .expect("list activities");

    assert_eq!(activities.len(), 1);
    assert_eq!(activities[0].id().as_str(), "activity-1");
    assert_eq!(activities[0].kind(), ConnectorActivityKind::SyncCompleted);
}

fn installation() -> ConnectorInstallation {
    ConnectorInstallation::new(
        ConnectorInstallationId::new("installation-1").expect("installation id"),
        "workspace-hash-1",
        ConnectorId::new("connector-jira").expect("connector id"),
        ConnectorCredentialReference::new("connector-credential:installation-1")
            .expect("credential"),
        ConnectorScopeSet::read_only(),
        ConnectorInstallationState::SyncQueued,
    )
    .expect("installation")
}

fn activity(id: &str, workspace_id_hash: &str) -> ConnectorActivity {
    ConnectorActivity::new(
        ConnectorActivityId::new(id).expect("activity id"),
        ConnectorInstallationId::new("installation-1").expect("installation id"),
        ConnectorId::new("connector-jira").expect("connector id"),
        workspace_id_hash,
        ConnectorActivityKind::SyncCompleted,
        1,
        None,
    )
    .expect("activity")
}
