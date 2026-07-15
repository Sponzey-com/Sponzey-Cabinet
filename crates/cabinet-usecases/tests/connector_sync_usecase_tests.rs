use cabinet_domain::connector::{
    ConnectorActivity, ConnectorActivityKind, ConnectorCredentialReference, ConnectorError,
    ConnectorExternalObjectReference, ConnectorId, ConnectorInstallation, ConnectorInstallationId,
    ConnectorInstallationState, ConnectorScopeSet,
};
use cabinet_ports::connector::{
    ConnectorActivityStorePort, ConnectorGatewayPort, ConnectorGatewayPortError,
    ConnectorGatewaySyncResult, ConnectorInstallationRepositoryPort, ConnectorPortError,
};
use cabinet_usecases::connector::{
    ConnectorUsecaseError, RunConnectorSyncInput, RunConnectorSyncUsecase,
};

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

struct FakeGateway {
    result: Result<ConnectorGatewaySyncResult, ConnectorGatewayPortError>,
}

impl ConnectorGatewayPort for FakeGateway {
    fn sync(
        &self,
        _installation: &ConnectorInstallation,
    ) -> Result<ConnectorGatewaySyncResult, ConnectorGatewayPortError> {
        self.result.clone()
    }
}

#[derive(Default)]
struct FakeActivityStore {
    activities: Vec<ConnectorActivity>,
}

impl ConnectorActivityStorePort for FakeActivityStore {
    fn record_activity(&mut self, activity: ConnectorActivity) -> Result<(), ConnectorPortError> {
        self.activities.push(activity);
        Ok(())
    }

    fn list_activities(
        &self,
        workspace_id_hash: &str,
    ) -> Result<Vec<ConnectorActivity>, ConnectorPortError> {
        Ok(self
            .activities
            .iter()
            .filter(|activity| activity.workspace_id_hash() == workspace_id_hash)
            .cloned()
            .collect())
    }
}

#[test]
fn run_connector_sync_marks_synced_and_records_completed_activity() {
    let mut repository = seeded_repository(ConnectorInstallationState::SyncQueued);
    let gateway = FakeGateway {
        result: Ok(ConnectorGatewaySyncResult::new(vec![
            ConnectorExternalObjectReference::new("external-object:jira-1")
                .expect("external object"),
        ])),
    };
    let mut activities = FakeActivityStore::default();

    let output = RunConnectorSyncUsecase::new()
        .execute(input(), &mut repository, &gateway, &mut activities)
        .expect("run sync");

    assert_eq!(
        output.installation().state(),
        ConnectorInstallationState::Synced,
    );
    assert_eq!(output.object_count(), 1);
    assert_eq!(activities.activities.len(), 1);
    assert_eq!(
        activities.activities[0].kind(),
        ConnectorActivityKind::SyncCompleted
    );
    assert_eq!(activities.activities[0].object_count(), 1);
}

#[test]
fn run_connector_sync_records_retry_activity_when_gateway_fails() {
    let mut repository = seeded_repository(ConnectorInstallationState::SyncQueued);
    let gateway = FakeGateway {
        result: Err(ConnectorGatewayPortError::GatewayUnavailable),
    };
    let mut activities = FakeActivityStore::default();

    let output = RunConnectorSyncUsecase::new()
        .execute(input(), &mut repository, &gateway, &mut activities)
        .expect("retry scheduled");

    assert_eq!(
        output.installation().state(),
        ConnectorInstallationState::RetryScheduled,
    );
    assert_eq!(output.object_count(), 0);
    assert_eq!(activities.activities.len(), 1);
    assert_eq!(
        activities.activities[0].kind(),
        ConnectorActivityKind::SyncFailed
    );
    assert_eq!(
        activities.activities[0].error_code(),
        Some("connector_gateway.gateway_unavailable"),
    );
}

#[test]
fn run_connector_sync_rejects_invalid_state_transition() {
    let mut repository = seeded_repository(ConnectorInstallationState::Disabled);
    let gateway = FakeGateway {
        result: Ok(ConnectorGatewaySyncResult::new(vec![])),
    };
    let mut activities = FakeActivityStore::default();

    let error = RunConnectorSyncUsecase::new()
        .execute(input(), &mut repository, &gateway, &mut activities)
        .expect_err("invalid transition");

    assert_eq!(error, ConnectorUsecaseError::InvalidTransition);
    assert_eq!(error.code(), "connector_usecase.invalid_transition");
}

#[test]
fn connector_external_object_reference_rejects_raw_payload_fixture() {
    assert_eq!(
        ConnectorExternalObjectReference::new("external-object:connector_payload"),
        Err(ConnectorError::InvalidExternalObjectReference),
    );
}

fn input() -> RunConnectorSyncInput {
    RunConnectorSyncInput::new(
        ConnectorInstallationId::new("installation-1").expect("installation id"),
    )
}

fn seeded_repository(state: ConnectorInstallationState) -> FakeInstallationRepository {
    let mut repository = FakeInstallationRepository::default();
    repository
        .save_installation(installation(state))
        .expect("seed installation");
    repository
}

fn installation(state: ConnectorInstallationState) -> ConnectorInstallation {
    ConnectorInstallation::new(
        ConnectorInstallationId::new("installation-1").expect("installation id"),
        "workspace-hash-1",
        ConnectorId::new("connector-jira").expect("connector id"),
        ConnectorCredentialReference::new("connector-credential:installation-1")
            .expect("credential"),
        ConnectorScopeSet::read_only(),
        state,
    )
    .expect("installation")
}
