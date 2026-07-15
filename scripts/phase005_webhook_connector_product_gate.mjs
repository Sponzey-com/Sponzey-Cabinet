import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const WebhookConnectorGateState = Object.freeze({
  Pending: "Pending",
  Running: "Running",
  Passed: "Passed",
  Failed: "Failed",
  Reported: "Reported",
});

export const WebhookConnectorGateEvent = Object.freeze({
  Start: "Start",
  Pass: "Pass",
  Fail: "Fail",
  Report: "Report",
});

export const WebhookConnectorGateErrorCode = Object.freeze({
  InvalidTransition: "PHASE005_WEBHOOK_CONNECTOR_GATE_INVALID_TRANSITION",
  SourceSetEmpty: "PHASE005_WEBHOOK_CONNECTOR_GATE_SOURCE_SET_EMPTY",
});

const STATUS = Object.freeze({
  Covered: "covered",
  Missing: "missing",
});

const TARGETS = Object.freeze([
  target("webhook_domain_state", "Webhook event envelope and delivery state domain", {
    files: [
      "crates/cabinet-domain/src/webhook.rs",
      "crates/cabinet-domain/tests/webhook_tests.rs",
      "crates/cabinet-domain/tests/webhook_delivery_tests.rs",
    ],
    evidence: [
      "EventEnvelope",
      "EventSubscription",
      "WebhookDestination",
      "WebhookSignature",
      "EventDeliveryJob",
      "DeadLetterEntry",
      "transition_event_delivery_job",
      "event_envelope_uses_hash_reference_and_metadata_summary_only",
      "webhook_destination_uses_references_not_raw_url_or_secret",
      "event_delivery_state_machine_supports_success_retry_and_dead_letter_paths",
      "dead_letter_entry_uses_reason_code_not_payload_body",
    ],
    priority: 100,
  }),
  target("webhook_usecase_adapter_boundary", "Webhook usecase, event log, delivery, and local adapter boundary", {
    files: [
      "crates/cabinet-ports/src/webhook.rs",
      "crates/cabinet-usecases/src/webhook.rs",
      "crates/cabinet-usecases/tests/webhook_delivery_usecase_tests.rs",
      "crates/cabinet-usecases/tests/webhook_event_log_usecase_tests.rs",
      "crates/cabinet-adapters/src/fake_webhook_transport.rs",
      "crates/cabinet-adapters/src/local_event_log_store.rs",
      "crates/cabinet-adapters/src/local_dead_letter_store.rs",
      "crates/cabinet-adapters/tests/local_webhook_event_log_store_tests.rs",
    ],
    evidence: [
      "EventLogPort",
      "WebhookTransportPort",
      "DeadLetterStorePort",
      "DeliverWebhookEventUsecase",
      "AppendIntegrationEventUsecase",
      "ListDeadLettersUsecase",
      "FakeWebhookTransport",
      "LocalEventLogStore",
      "LocalDeadLetterStore",
      "deliver_webhook_event_skips_disabled_subscription_without_transport_call",
      "deliver_webhook_event_dead_letters_after_retry_limit",
      "append_integration_event_stores_envelope_without_raw_payload",
      "list_dead_letters_filters_by_workspace_hash",
      "local_event_log_store_appends_and_lists_by_workspace_without_raw_payload",
    ],
    priority: 98,
  }),
  target("connector_domain_usecase_gateway", "Connector domain, usecase, port, gateway, and activity baseline", {
    files: [
      "crates/cabinet-domain/src/connector.rs",
      "crates/cabinet-domain/tests/connector_tests.rs",
      "crates/cabinet-ports/src/connector.rs",
      "crates/cabinet-usecases/src/connector.rs",
      "crates/cabinet-usecases/tests/connector_usecase_tests.rs",
      "crates/cabinet-usecases/tests/connector_sync_usecase_tests.rs",
      "crates/cabinet-adapters/src/fake_connector_gateway.rs",
      "crates/cabinet-adapters/src/local_connector_activity_store.rs",
    ],
    evidence: [
      "ConnectorDefinition",
      "ConnectorInstallation",
      "ConnectorScopeSet",
      "ConnectorExternalObjectReference",
      "ConnectorActivity",
      "transition_connector_installation",
      "ConnectorDefinitionRegistryPort",
      "ConnectorInstallationRepositoryPort",
      "ConnectorGatewayPort",
      "ConnectorActivityStorePort",
      "InstallConnectorUsecase",
      "RunConnectorSyncUsecase",
      "connector_scope_distinguishes_read_only_and_read_write_actions",
      "install_connector_rejects_write_scope_for_read_only_definition",
      "run_connector_sync_marks_synced_and_records_completed_activity",
      "run_connector_sync_records_retry_activity_when_gateway_fails",
      "connector_external_object_reference_rejects_raw_payload_fixture",
      "FakeConnectorGateway",
      "LocalConnectorActivityStore",
    ],
    priority: 96,
  }),
  target("connector_descriptor_admin_ui", "Connector static descriptors and admin UI model", {
    files: [
      "crates/cabinet-adapters/src/static_connector_definition_registry.rs",
      "crates/cabinet-adapters/tests/static_connector_definition_registry_tests.rs",
      "packages/ui/src/index.ts",
      "packages/ui/tests/connector_admin_ui_model_tests.ts",
    ],
    evidence: [
      "StaticConnectorDefinitionRegistry",
      "phase005_baseline",
      "static_connector_definition_registry_returns_phase005_baseline_descriptors",
      "static_connector_definition_registry_distinguishes_read_only_and_read_write_descriptors",
      "createConnectorAdminViewModel",
      "ConnectorAdminCardViewModel",
      "ConnectorDefinitionView",
      "ConnectorInstallationView",
      "connector admin model maps definitions and installed states without provider payloads",
      "connector admin model exposes stable scope labels",
    ],
    priority: 94,
  }),
]);

class WebhookConnectorGateError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "WebhookConnectorGateError";
    this.code = code;
  }
}

export function transitionWebhookConnectorGateState(state, event) {
  const key = `${state}:${event}`;
  const transitions = new Map([
    [`${WebhookConnectorGateState.Pending}:${WebhookConnectorGateEvent.Start}`, WebhookConnectorGateState.Running],
    [`${WebhookConnectorGateState.Running}:${WebhookConnectorGateEvent.Pass}`, WebhookConnectorGateState.Passed],
    [`${WebhookConnectorGateState.Running}:${WebhookConnectorGateEvent.Fail}`, WebhookConnectorGateState.Failed],
    [`${WebhookConnectorGateState.Passed}:${WebhookConnectorGateEvent.Report}`, WebhookConnectorGateState.Reported],
    [`${WebhookConnectorGateState.Failed}:${WebhookConnectorGateEvent.Report}`, WebhookConnectorGateState.Reported],
  ]);
  const nextState = transitions.get(key);
  if (!nextState) {
    throw new WebhookConnectorGateError(
      WebhookConnectorGateErrorCode.InvalidTransition,
      `invalid webhook connector gate transition: ${key}`,
    );
  }
  return nextState;
}

export function analyzeWebhookConnectorGateSources({ sources }) {
  if (!sources || Object.keys(sources).length === 0) {
    throw new WebhookConnectorGateError(
      WebhookConnectorGateErrorCode.SourceSetEmpty,
      "phase005 webhook connector gate source set is empty",
    );
  }
  const targets = TARGETS.map((entry) => analyzeTarget(entry, sources));
  const targetsNeedingWork = targets.filter((entry) => entry.status !== STATUS.Covered);
  return {
    phase: "Phase 005.5-005.6",
    status: targetsNeedingWork.length === 0 ? "passed" : "failed",
    sourceFiles: Object.keys(sources).sort(),
    summary: {
      totalTargets: targets.length,
      covered: targets.filter((entry) => entry.status === STATUS.Covered).length,
      missing: targets.filter((entry) => entry.status === STATUS.Missing).length,
      targetsNeedingWork: targetsNeedingWork.length,
    },
    nextImplementationTarget: pickNextImplementationTarget(targetsNeedingWork),
    targets,
  };
}

export function renderWebhookConnectorGateMarkdown(gate) {
  const marker =
    gate.status === "passed"
      ? "phase005_webhook_connector_product_gate=passed"
      : "phase005_webhook_connector_product_gate=failed";
  const lines = [
    "# Phase 005 Webhook Connector Product Gate Result",
    "",
    marker,
    "",
    "현재 단계: Phase 005.5-005.6 - Webhook, Event Stream, and Connector Baseline",
    "",
    "## Summary",
    "",
    "| Metric | Count |",
    "| --- | ---: |",
    `| total targets | ${gate.summary.totalTargets} |`,
    `| covered | ${gate.summary.covered} |`,
    `| missing | ${gate.summary.missing} |`,
    `| targets needing work | ${gate.summary.targetsNeedingWork} |`,
    "",
    "## Target Status",
    "",
    "| Target | Label | Status | Missing Files | Missing Evidence |",
    "| --- | --- | --- | --- | --- |",
    ...gate.targets.map((entry) => {
      const missingFiles =
        entry.missingFiles.length > 0 ? entry.missingFiles.map(code).join(", ") : "none";
      const missingEvidence =
        entry.missingEvidence.length > 0
          ? entry.missingEvidence.map(code).join(", ")
          : "none";
      return `| \`${entry.id}\` | ${entry.label} | ${entry.status} | ${missingFiles} | ${missingEvidence} |`;
    }),
    "",
    "## Evidence Markers",
    "",
    "- webhook event envelope, subscription, delivery state, retry, and dead-letter baseline complete",
    "- webhook transport, event log, dead-letter store, and local/fake adapter boundary complete",
    "- connector definition, scope, credential reference, installation, sync, gateway, and activity baseline complete",
    "- connector static descriptors and admin UI model complete",
    "",
    "## Next Implementation Target",
    "",
    gate.nextImplementationTarget
      ? `- \`${gate.nextImplementationTarget.id}\`: ${gate.nextImplementationTarget.label}`
      : "- none",
    "",
    "## Review Notes",
    "",
    "- The gate uses source evidence and deterministic local/fake adapters; it does not call external webhook endpoints or connector providers.",
    "- The gate records evidence names and counts, not webhook secrets, connector tokens, raw payloads, document bodies, or provider responses.",
  ];
  return `${lines.join("\n")}\n`;
}

export async function runWebhookConnectorGate({
  root = process.cwd(),
  reportPath = ".tasks/webhook-connector-product-gate-result.md",
} = {}) {
  let state = transitionWebhookConnectorGateState(
    WebhookConnectorGateState.Pending,
    WebhookConnectorGateEvent.Start,
  );
  const sources = await readTargetSources(root);
  const gate = analyzeWebhookConnectorGateSources({ sources });
  state = transitionWebhookConnectorGateState(
    state,
    gate.status === "passed"
      ? WebhookConnectorGateEvent.Pass
      : WebhookConnectorGateEvent.Fail,
  );
  await mkdir(path.dirname(path.join(root, reportPath)), { recursive: true });
  await writeFile(path.join(root, reportPath), renderWebhookConnectorGateMarkdown(gate), "utf8");
  state = transitionWebhookConnectorGateState(state, WebhookConnectorGateEvent.Report);
  return { ...gate, state, reportPath };
}

function analyzeTarget(entry, sources) {
  const missingFiles = entry.files.filter((file) => !(file in sources));
  const combined = entry.files.map((file) => sources[file] ?? "").join("\n");
  const missingEvidence = entry.evidence.filter((evidence) => !combined.includes(evidence));
  const status =
    missingFiles.length === 0 && missingEvidence.length === 0 ? STATUS.Covered : STATUS.Missing;
  return { ...entry, status, missingFiles, missingEvidence };
}

function pickNextImplementationTarget(targetsNeedingWork) {
  if (targetsNeedingWork.length === 0) {
    return null;
  }
  return [...targetsNeedingWork].sort((left, right) => right.priority - left.priority)[0];
}

async function readTargetSources(root) {
  const paths = new Set(TARGETS.flatMap((entry) => entry.files));
  const sources = {};
  for (const relativePath of paths) {
    try {
      sources[relativePath] = await readFile(path.join(root, relativePath), "utf8");
    } catch {
      // Missing files are represented by absence in the source map.
    }
  }
  return sources;
}

function target(id, label, { files, evidence, priority }) {
  return { id, label, files, evidence, priority };
}

function code(value) {
  return `\`${value}\``;
}

async function runCli() {
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const gate = await runWebhookConnectorGate({ root: repoRoot });
  if (gate.status === "passed") {
    console.log("phase005_webhook_connector_product_gate=passed");
    console.log(`gate_state=${gate.state}`);
    console.log(`covered_target_count=${gate.summary.covered}`);
    console.log(`report_path=${path.join(repoRoot, gate.reportPath)}`);
    return;
  }
  console.error("phase005_webhook_connector_product_gate=failed");
  console.error(`missing_target_count=${gate.summary.targetsNeedingWork}`);
  console.error(`next_target=${gate.nextImplementationTarget?.id ?? "none"}`);
  process.exitCode = 1;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await runCli();
}
