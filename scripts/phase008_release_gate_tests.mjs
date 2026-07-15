import assert from "node:assert/strict";
import test from "node:test";

import {
  Phase008ReleaseGateErrorCode,
  Phase008ReleaseGateEvent,
  Phase008ReleaseGateState,
  evaluatePhase008ReleaseGate,
  renderPhase008ReleaseGateArtifact,
  transitionPhase008ReleaseGateState,
} from "./phase008_release_gate.mjs";

test("phase008 release gate rejects missing lower marker", () => {
  const sources = completeSources();
  sources[".tasks/phase008-native-product-smoke-gate-result.md"] =
    "phase008_native_product_smoke_gate=failed";

  const result = evaluatePhase008ReleaseGate({
    sources,
    commandResults: passingCommandResults(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase008ReleaseGateErrorCode.RequiredEvidenceMissing);
  assert.equal(result.findingId, "phase008_native_product_smoke_gate");
});

test("phase008 release gate rejects missing security denied fixture", () => {
  const sources = completeSources();
  sources[".tasks/release/security-log-policy-manifest.json"] = JSON.stringify({
    deniedFixtures: [{ id: "phase008_raw_document_body_fixture" }],
    scanTargets: [{ id: "phase008_native_product_smoke_gate_result" }],
  });

  const result = evaluatePhase008ReleaseGate({
    sources,
    commandResults: passingCommandResults(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase008ReleaseGateErrorCode.RequiredEvidenceMissing);
  assert.equal(result.findingId, "phase008_security_manifest");
});

test("phase008 release gate rejects failed runbook validation command", () => {
  const result = evaluatePhase008ReleaseGate({
    sources: completeSources(),
    commandResults: {
      ...passingCommandResults(),
      runbook: { command: "runbook", passed: false, exitCode: 1 },
    },
  });
  const artifact = renderPhase008ReleaseGateArtifact(result);

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase008ReleaseGateErrorCode.RunbookValidationFailed);
  assert.doesNotMatch(artifact, /raw markdown body should not leak/);
  assert.doesNotMatch(artifact, /provider_api_key_fixture/);
});

test("phase008 release gate rejects missing performance budget", () => {
  const sources = completeSources();
  sources[".tasks/release/performance-budget-phase008.md"] = "phase008_performance_budget=failed";

  const result = evaluatePhase008ReleaseGate({
    sources,
    commandResults: passingCommandResults(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase008ReleaseGateErrorCode.PerformanceBudgetMissing);
  assert.equal(result.findingId, "phase008_performance_budget");
});

test("phase008 release gate passes complete evidence and renders safe marker", () => {
  const result = evaluatePhase008ReleaseGate({
    sources: completeSources(),
    commandResults: passingCommandResults(),
  });
  const artifact = renderPhase008ReleaseGateArtifact(result);

  assert.equal(result.passed, true);
  assert.equal(result.state, Phase008ReleaseGateState.Passed);
  assert.match(artifact, /phase008_release_gate=passed/);
  assert.doesNotMatch(artifact, /asset binary content should not leak/);
  assert.doesNotMatch(artifact, /\/Users\/example\/private/);
});

test("phase008 release gate state machine reaches terminal states", () => {
  const reading = transitionPhase008ReleaseGateState(
    Phase008ReleaseGateState.Pending,
    Phase008ReleaseGateEvent.Start,
  );
  const security = transitionPhase008ReleaseGateState(
    reading.state,
    Phase008ReleaseGateEvent.EvidenceRead,
  );
  const runbook = transitionPhase008ReleaseGateState(
    security.state,
    Phase008ReleaseGateEvent.SecurityScanPassed,
  );
  const performance = transitionPhase008ReleaseGateState(
    runbook.state,
    Phase008ReleaseGateEvent.RunbookValidationPassed,
  );
  const writing = transitionPhase008ReleaseGateState(
    performance.state,
    Phase008ReleaseGateEvent.PerformanceValidated,
  );
  const passed = transitionPhase008ReleaseGateState(
    writing.state,
    Phase008ReleaseGateEvent.ResultWritten,
  );
  const invalid = transitionPhase008ReleaseGateState(
    Phase008ReleaseGateState.Pending,
    Phase008ReleaseGateEvent.ResultWritten,
  );

  assert.equal(reading.state, Phase008ReleaseGateState.ReadingEvidence);
  assert.equal(security.state, Phase008ReleaseGateState.RunningSecurityScan);
  assert.equal(runbook.state, Phase008ReleaseGateState.RunningRunbookValidation);
  assert.equal(performance.state, Phase008ReleaseGateState.ValidatingPerformance);
  assert.equal(writing.state, Phase008ReleaseGateState.WritingResult);
  assert.equal(passed.state, Phase008ReleaseGateState.Passed);
  assert.equal(invalid.errorCode, Phase008ReleaseGateErrorCode.InvalidTransition);
});

function passingCommandResults() {
  return {
    security: { command: "security scan", passed: true, exitCode: 0 },
    runbook: { command: "runbook validation", passed: true, exitCode: 0 },
  };
}

function completeSources() {
  return {
    ".tasks/phase008-plan-validation-result.md": "phase008_plan_validation=passed",
    ".tasks/phase008-native-bootstrap-gate-result.md": "phase008_native_bootstrap_gate=passed",
    ".tasks/phase008-command-bridge-gate-result.md": "phase008_command_bridge_gate=passed",
    ".tasks/phase008-document-runtime-gate-result.md": "phase008_document_runtime_gate=passed",
    ".tasks/phase008-projection-index-gate-result.md": "phase008_projection_index_gate=passed",
    ".tasks/phase008-asset-lifecycle-gate-result.md": "phase008_asset_lifecycle_gate=passed",
    ".tasks/phase008-recovery-backup-gate-result.md": "phase008_recovery_backup_gate=passed",
    ".tasks/phase008-native-product-smoke-gate-result.md": "phase008_native_product_smoke_gate=passed",
    ".tasks/release/performance-budget-phase008.md": "phase008_performance_budget=passed p95 300ms",
    ".tasks/release/security-log-policy-manifest.json": [
      "phase008_raw_document_body_fixture",
      "phase008_asset_binary_content_fixture",
      "phase008_ai_prompt_fixture",
      "phase008_ai_answer_fixture",
      "phase008_provider_key_fixture",
      "phase008_token_fixture",
      "phase008_credential_fixture",
      "phase008_secret_fixture",
      "phase008_raw_local_absolute_path_fixture",
      "phase008_native_product_smoke_gate_result",
      "phase008_product_log_event_matrix",
      "phase008_local_desktop_runbook",
    ].join("\n"),
    ".tasks/release/product-log-event-matrix.md": [
      "Product Log",
      "Field Debug Log",
      "Development Log",
      "app.start.completed",
      "workspace.ready",
      "document.save.completed",
      "backup.created",
      "Do not record document body, asset content, AI prompt, AI answer, provider key, token, credential, secret, raw local absolute path",
    ].join("\n"),
    ".tasks/release/local-desktop-runbook.md": [
      "Phase 008 Local Desktop Runbook",
      "clean install",
      "startup repair",
      "backup",
      "restore",
      "import preview",
      "p95 300ms",
    ].join("\n"),
    ".tasks/release/runbook-validation-manifest.json": "phase008_local_desktop_release",
    "package.json": "run:phase008-release-gate run:phase008-release-gate-tests",
  };
}
