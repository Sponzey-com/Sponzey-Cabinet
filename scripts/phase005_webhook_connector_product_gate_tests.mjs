import assert from "node:assert/strict";
import test from "node:test";

import {
  WebhookConnectorGateErrorCode,
  WebhookConnectorGateEvent,
  WebhookConnectorGateState,
  analyzeWebhookConnectorGateSources,
  renderWebhookConnectorGateMarkdown,
  transitionWebhookConnectorGateState,
} from "./phase005_webhook_connector_product_gate.mjs";

const completeSources = {
  "crates/cabinet-domain/src/webhook.rs":
    "EventEnvelope EventSubscription WebhookDestination WebhookSignature EventDeliveryJob DeadLetterEntry transition_event_delivery_job",
  "crates/cabinet-domain/tests/webhook_tests.rs":
    "event_envelope_uses_hash_reference_and_metadata_summary_only webhook_destination_uses_references_not_raw_url_or_secret event_subscription_requires_event_types_and_can_be_disabled",
  "crates/cabinet-domain/tests/webhook_delivery_tests.rs":
    "webhook_signature_requires_signature_reference_not_raw_secret event_delivery_state_machine_supports_success_retry_and_dead_letter_paths dead_letter_entry_uses_reason_code_not_payload_body",
  "crates/cabinet-ports/src/webhook.rs":
    "EventLogPort EventSubscriptionRepositoryPort WebhookTransportPort DeadLetterStorePort WebhookEventLogPortError",
  "crates/cabinet-usecases/src/webhook.rs":
    "CreateEventSubscriptionUsecase DeliverWebhookEventUsecase AppendIntegrationEventUsecase ListDeadLettersUsecase WebhookEventUsecaseError",
  "crates/cabinet-usecases/tests/webhook_delivery_usecase_tests.rs":
    "deliver_webhook_event_skips_disabled_subscription_without_transport_call deliver_webhook_event_dead_letters_after_retry_limit",
  "crates/cabinet-usecases/tests/webhook_event_log_usecase_tests.rs":
    "append_integration_event_stores_envelope_without_raw_payload list_dead_letters_filters_by_workspace_hash",
  "crates/cabinet-adapters/src/fake_webhook_transport.rs":
    "FakeWebhookTransport WebhookTransportPort",
  "crates/cabinet-adapters/src/local_event_log_store.rs":
    "LocalEventLogStore EventLogPort",
  "crates/cabinet-adapters/src/local_dead_letter_store.rs":
    "LocalDeadLetterStore DeadLetterStorePort",
  "crates/cabinet-adapters/tests/local_webhook_event_log_store_tests.rs":
    "local_event_log_store_appends_and_lists_by_workspace_without_raw_payload local_dead_letter_store_lists_entries_by_workspace",
  "crates/cabinet-domain/src/connector.rs":
    "ConnectorDefinition ConnectorInstallation ConnectorScopeSet ConnectorCredentialReference ConnectorExternalObjectReference ConnectorActivity transition_connector_installation",
  "crates/cabinet-domain/tests/connector_tests.rs":
    "connector_definition_requires_non_empty_scope_set connector_scope_distinguishes_read_only_and_read_write_actions connector_credential_reference_rejects_tokens_secrets_and_payload_fixtures connector_installation_state_machine_supports_authorize_sync_retry_and_disable",
  "crates/cabinet-ports/src/connector.rs":
    "ConnectorDefinitionRegistryPort ConnectorInstallationRepositoryPort ConnectorGatewayPort ConnectorActivityStorePort ConnectorGatewaySyncResult",
  "crates/cabinet-usecases/src/connector.rs":
    "InstallConnectorUsecase StartConnectorSyncUsecase RunConnectorSyncUsecase ConnectorUsecaseError",
  "crates/cabinet-usecases/tests/connector_usecase_tests.rs":
    "install_connector_rejects_write_scope_for_read_only_definition start_connector_sync_moves_installed_connector_to_sync_queued",
  "crates/cabinet-usecases/tests/connector_sync_usecase_tests.rs":
    "run_connector_sync_marks_synced_and_records_completed_activity run_connector_sync_records_retry_activity_when_gateway_fails connector_external_object_reference_rejects_raw_payload_fixture",
  "crates/cabinet-adapters/src/fake_connector_gateway.rs":
    "FakeConnectorGateway ConnectorGatewayPort",
  "crates/cabinet-adapters/src/local_connector_activity_store.rs":
    "LocalConnectorActivityStore ConnectorActivityStorePort",
  "crates/cabinet-adapters/src/static_connector_definition_registry.rs":
    "StaticConnectorDefinitionRegistry phase005_baseline",
  "crates/cabinet-adapters/tests/static_connector_definition_registry_tests.rs":
    "static_connector_definition_registry_returns_phase005_baseline_descriptors static_connector_definition_registry_distinguishes_read_only_and_read_write_descriptors",
  "packages/ui/src/index.ts":
    "createConnectorAdminViewModel ConnectorAdminCardViewModel ConnectorDefinitionView ConnectorInstallationView",
  "packages/ui/tests/connector_admin_ui_model_tests.ts":
    "connector admin model maps definitions and installed states without provider payloads connector admin model exposes stable scope labels",
};

test("webhook/connector gate marks complete fixture as passed", () => {
  const gate = analyzeWebhookConnectorGateSources({ sources: completeSources });

  assert.equal(gate.status, "passed");
  assert.equal(gate.summary.covered, 4);
  assert.equal(gate.summary.targetsNeedingWork, 0);
});

test("webhook/connector gate reports missing connector UI evidence", () => {
  const {
    "packages/ui/src/index.ts": _ui,
    "packages/ui/tests/connector_admin_ui_model_tests.ts": _tests,
    ...sources
  } = completeSources;

  const gate = analyzeWebhookConnectorGateSources({ sources });

  assert.equal(gate.status, "failed");
  assert.equal(gate.nextImplementationTarget.id, "connector_descriptor_admin_ui");
});

test("webhook/connector gate state machine rejects invalid transitions", () => {
  const running = transitionWebhookConnectorGateState(
    WebhookConnectorGateState.Pending,
    WebhookConnectorGateEvent.Start,
  );
  const passed = transitionWebhookConnectorGateState(
    running,
    WebhookConnectorGateEvent.Pass,
  );
  const reported = transitionWebhookConnectorGateState(
    passed,
    WebhookConnectorGateEvent.Report,
  );

  assert.equal(running, WebhookConnectorGateState.Running);
  assert.equal(passed, WebhookConnectorGateState.Passed);
  assert.equal(reported, WebhookConnectorGateState.Reported);
  assert.throws(
    () =>
      transitionWebhookConnectorGateState(
        WebhookConnectorGateState.Pending,
        WebhookConnectorGateEvent.Report,
      ),
    (error) => error.code === WebhookConnectorGateErrorCode.InvalidTransition,
  );
});

test("webhook/connector gate markdown excludes sensitive raw fixtures", () => {
  const gate = analyzeWebhookConnectorGateSources({ sources: completeSources });
  const markdown = renderWebhookConnectorGateMarkdown(gate);

  assert.match(markdown, /# Phase 005 Webhook Connector Product Gate Result/);
  assert.match(markdown, /phase005_webhook_connector_product_gate=passed/);
  assert.doesNotMatch(markdown, /webhook_secret_fixture/);
  assert.doesNotMatch(markdown, /webhook_payload_body_fixture/);
  assert.doesNotMatch(markdown, /connector_access_token_fixture/);
  assert.doesNotMatch(markdown, /connector_refresh_token_fixture/);
  assert.doesNotMatch(markdown, /connector_client_secret_fixture/);
  assert.doesNotMatch(markdown, /connector_payload/);
});
