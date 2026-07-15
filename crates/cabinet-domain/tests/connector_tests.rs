use cabinet_domain::connector::{
    ConnectorAction, ConnectorCredentialReference, ConnectorDefinition, ConnectorError,
    ConnectorId, ConnectorInstallationEvent, ConnectorInstallationState, ConnectorKind,
    ConnectorScope, ConnectorScopeSet, transition_connector_installation,
};

#[test]
fn connector_definition_requires_non_empty_scope_set() {
    assert_eq!(
        ConnectorScopeSet::new(vec![]),
        Err(ConnectorError::EmptyScope),
    );
}

#[test]
fn connector_scope_distinguishes_read_only_and_read_write_actions() {
    let read_only = ConnectorScopeSet::read_only();
    let read_write = ConnectorScopeSet::read_write();

    assert!(read_only.allows(ConnectorAction::Read));
    assert!(!read_only.allows(ConnectorAction::Write));
    assert_eq!(
        read_only.require_action(ConnectorAction::Write),
        Err(ConnectorError::ScopeDenied),
    );
    assert!(read_write.allows(ConnectorAction::Read));
    assert!(read_write.allows(ConnectorAction::Write));
}

#[test]
fn connector_definition_keeps_provider_shape_out_of_core_domain() {
    let definition = ConnectorDefinition::new(
        ConnectorId::new("connector-jira").expect("connector id"),
        ConnectorKind::Jira,
        "Jira",
        ConnectorScopeSet::new(vec![ConnectorScope::Read, ConnectorScope::Write])
            .expect("scope set"),
    )
    .expect("definition");

    assert_eq!(definition.id().as_str(), "connector-jira");
    assert_eq!(definition.kind(), ConnectorKind::Jira);
    assert_eq!(definition.display_name(), "Jira");
    assert!(definition.scopes().allows(ConnectorAction::Write));
}

#[test]
fn connector_credential_reference_rejects_tokens_secrets_and_payload_fixtures() {
    let reference = ConnectorCredentialReference::new("connector-credential:slack-installation-1")
        .expect("credential reference");

    assert_eq!(
        reference.as_str(),
        "connector-credential:slack-installation-1"
    );
    assert_eq!(
        ConnectorCredentialReference::new("connector_access_token_fixture"),
        Err(ConnectorError::InvalidCredentialReference),
    );
    assert_eq!(
        ConnectorCredentialReference::new("connector_refresh_token_fixture"),
        Err(ConnectorError::InvalidCredentialReference),
    );
    assert_eq!(
        ConnectorCredentialReference::new("connector_client_secret_fixture"),
        Err(ConnectorError::InvalidCredentialReference),
    );
    assert_eq!(
        ConnectorCredentialReference::new("connector_payload"),
        Err(ConnectorError::InvalidCredentialReference),
    );
}

#[test]
fn connector_installation_state_machine_supports_authorize_sync_retry_and_disable() {
    let requested = transition_connector_installation(
        ConnectorInstallationState::NotInstalled,
        ConnectorInstallationEvent::RequestAuthorization,
    )
    .expect("authorization requested");
    let installed =
        transition_connector_installation(requested, ConnectorInstallationEvent::Authorize)
            .expect("installed");
    let queued =
        transition_connector_installation(installed, ConnectorInstallationEvent::QueueSync)
            .expect("queued");
    let syncing = transition_connector_installation(queued, ConnectorInstallationEvent::StartSync)
        .expect("syncing");
    let retry =
        transition_connector_installation(syncing, ConnectorInstallationEvent::ScheduleRetry)
            .expect("retry");
    let retrying = transition_connector_installation(retry, ConnectorInstallationEvent::Retry)
        .expect("retrying");
    let synced =
        transition_connector_installation(retrying, ConnectorInstallationEvent::CompleteSync)
            .expect("synced");
    let disabled = transition_connector_installation(synced, ConnectorInstallationEvent::Disable)
        .expect("disabled");

    assert_eq!(
        requested,
        ConnectorInstallationState::AuthorizationRequested
    );
    assert_eq!(installed, ConnectorInstallationState::Installed);
    assert_eq!(queued, ConnectorInstallationState::SyncQueued);
    assert_eq!(syncing, ConnectorInstallationState::Syncing);
    assert_eq!(retry, ConnectorInstallationState::RetryScheduled);
    assert_eq!(synced, ConnectorInstallationState::Synced);
    assert_eq!(disabled, ConnectorInstallationState::Disabled);
}

#[test]
fn connector_installation_state_machine_rejects_invalid_transitions() {
    assert_eq!(
        transition_connector_installation(
            ConnectorInstallationState::NotInstalled,
            ConnectorInstallationEvent::StartSync,
        ),
        Err(ConnectorError::InvalidTransition),
    );
    assert_eq!(
        transition_connector_installation(
            ConnectorInstallationState::Disabled,
            ConnectorInstallationEvent::QueueSync,
        ),
        Err(ConnectorError::InvalidTransition),
    );
    assert_eq!(
        ConnectorError::InvalidTransition.code(),
        "connector.invalid_transition",
    );
}
