import assert from "node:assert/strict";
import test from "node:test";

import {
  Phase009UxReleaseGateErrorCode,
  Phase009UxReleaseGateEvent,
  Phase009UxReleaseGateState,
  evaluatePhase009UxReleaseGate,
  renderPhase009UxReleaseGateArtifact,
  transitionPhase009UxReleaseGateState,
} from "./phase009_ux_release_gate.mjs";

test("phase009 UX release gate rejects missing lower marker", () => {
  const sources = completeSources();
  sources[".tasks/phase009-command-runtime-gate-result.md"] =
    "phase009_command_runtime_gate=failed";

  const result = evaluatePhase009UxReleaseGate({
    sources,
    commandResults: passingCommandResults(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase009UxReleaseGateErrorCode.MissingMarker);
  assert.equal(result.findingId, "phase009_command_runtime_gate");
});

test("phase009 UX release gate rejects missing performance budget", () => {
  const sources = completeSources();
  sources[".tasks/release/performance-budget-phase009.md"] = "phase009_performance_budget=failed";

  const result = evaluatePhase009UxReleaseGate({
    sources,
    commandResults: passingCommandResults(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase009UxReleaseGateErrorCode.PerformanceBudgetFailed);
  assert.equal(result.findingId, "phase009_performance_budget");
});

test("phase009 UX release gate rejects failed security scan command", () => {
  const result = evaluatePhase009UxReleaseGate({
    sources: completeSources(),
    commandResults: {
      ...passingCommandResults(),
      security: { command: "security", passed: false, exitCode: 1 },
    },
  });
  const artifact = renderPhase009UxReleaseGateArtifact(result);

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase009UxReleaseGateErrorCode.SecurityLogScanFailed);
  assert.equal(result.findingId, "security");
  assert.doesNotMatch(artifact, /raw markdown body should not leak/);
  assert.doesNotMatch(artifact, /provider_api_key_fixture/);
});

test("phase009 UX release gate rejects failed product smoke command", () => {
  const result = evaluatePhase009UxReleaseGate({
    sources: completeSources(),
    commandResults: {
      ...passingCommandResults(),
      productSmoke: { command: "product smoke", passed: false, exitCode: 1 },
    },
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase009UxReleaseGateErrorCode.ProductSmokeFailed);
  assert.equal(result.findingId, "productSmoke");
});

test("phase009 UX release gate passes complete evidence and renders safe marker", () => {
  const result = evaluatePhase009UxReleaseGate({
    sources: completeSources(),
    commandResults: passingCommandResults(),
  });
  const artifact = renderPhase009UxReleaseGateArtifact(result);

  assert.equal(result.passed, true);
  assert.equal(result.state, Phase009UxReleaseGateState.Passed);
  assert.equal(result.evidenceCount, 13);
  assert.match(artifact, /phase009_ux_release_gate=passed/);
  assert.match(artifact, /personal local desktop installable knowledge management app only/);
  assert.match(artifact, /p95 300ms paths: documented and validated by release evidence/);
  assert.doesNotMatch(artifact, /asset binary content should not leak/);
  assert.doesNotMatch(artifact, /\/Users\/example\/private/);
});

test("phase009 UX release gate state machine reaches terminal states", () => {
  const reading = transitionPhase009UxReleaseGateState(
    Phase009UxReleaseGateState.Pending,
    Phase009UxReleaseGateEvent.Start,
  );
  const artifacts = transitionPhase009UxReleaseGateState(
    reading.state,
    Phase009UxReleaseGateEvent.MarkersRead,
  );
  const security = transitionPhase009UxReleaseGateState(
    artifacts.state,
    Phase009UxReleaseGateEvent.ArtifactsValidated,
  );
  const runbook = transitionPhase009UxReleaseGateState(
    security.state,
    Phase009UxReleaseGateEvent.SecurityScanPassed,
  );
  const smoke = transitionPhase009UxReleaseGateState(
    runbook.state,
    Phase009UxReleaseGateEvent.RunbookValidationPassed,
  );
  const writing = transitionPhase009UxReleaseGateState(
    smoke.state,
    Phase009UxReleaseGateEvent.ProductSmokePassed,
  );
  const passed = transitionPhase009UxReleaseGateState(
    writing.state,
    Phase009UxReleaseGateEvent.ResultWritten,
  );
  const invalid = transitionPhase009UxReleaseGateState(
    Phase009UxReleaseGateState.Pending,
    Phase009UxReleaseGateEvent.ResultWritten,
  );

  assert.equal(reading.state, Phase009UxReleaseGateState.ReadingMarkers);
  assert.equal(artifacts.state, Phase009UxReleaseGateState.ValidatingArtifacts);
  assert.equal(security.state, Phase009UxReleaseGateState.RunningSecurityScan);
  assert.equal(runbook.state, Phase009UxReleaseGateState.RunningRunbookValidation);
  assert.equal(smoke.state, Phase009UxReleaseGateState.RunningProductSmoke);
  assert.equal(writing.state, Phase009UxReleaseGateState.WritingResult);
  assert.equal(passed.state, Phase009UxReleaseGateState.Passed);
  assert.equal(invalid.errorCode, Phase009UxReleaseGateErrorCode.InvalidTransition);
});

function passingCommandResults() {
  return {
    security: { command: "security scan", passed: true, exitCode: 0 },
    runbook: { command: "runbook validation", passed: true, exitCode: 0 },
    productSmoke: { command: "desktop product smoke", passed: true, exitCode: 0 },
  };
}

function completeSources() {
  return {
    ".tasks/phase009-current-implementation-inventory.md": [
      "phase009_current_inventory=passed",
      "product_scope: `personal_local_desktop`",
      "run_desktop_shell.sh is internal shell smoke and not the product UI launcher",
      "Phase 009 follow-up tasks must not require server/SaaS/multi-user paths",
    ].join("\n"),
    ".tasks/phase009-plan-validation-result.md": [
      "phase009_plan_validation=passed",
      "scope lock: personal local desktop only",
      "completion evidence: marker artifacts only",
    ].join("\n"),
    ".tasks/phase009-desktop-launch-gate-result.md": [
      "phase009_desktop_launch_gate=passed",
      "product app command: `npm run run:desktop-app`",
      "DESKTOP_BLANK_SCREEN_DETECTED",
    ].join("\n"),
    ".tasks/phase009-command-runtime-gate-result.md": [
      "phase009_command_runtime_gate=passed",
      "command_count=17",
      "LocalDesktopCommandState",
    ].join("\n"),
    ".tasks/phase009-document-authoring-gate-result.md": [
      "phase009_document_authoring_gate=passed",
      "DocumentEditorState",
      "p95 budget impact",
    ].join("\n"),
    ".tasks/phase009-discovery-assets-gate-result.md": [
      "phase009_discovery_assets_gate=passed",
      "search result, backlink group, unresolved link group, graph node/edge, and asset metadata panel",
      "p95 budget impact",
    ].join("\n"),
    ".tasks/phase009-recovery-backup-ux-gate-result.md": [
      "phase009_recovery_backup_ux_gate=passed",
      "backup summary, import preview, restore confirmation, and recovery action panels",
      "runbook follow-up",
    ].join("\n"),
    ".tasks/release/performance-budget-phase009.md": [
      "phase009_performance_budget=passed",
      "current_document_read",
      "history_list",
      "search",
      "p95 target",
      "300ms",
    ].join("\n"),
    ".tasks/release/product-log-event-matrix-phase009.md": [
      "phase009_product_log_matrix=passed",
      "Product Log",
      "Field Debug Log",
      "Development Log",
      "document.save.completed",
      "backup.created",
    ].join("\n"),
    ".tasks/release/security-log-policy-manifest-phase009.json": [
      "phase009_security_log_manifest=passed",
      "__CABINET_DENY_PROVIDER_KEY__",
      "__CABINET_DENY_TOKEN__",
      "__CABINET_DENY_CREDENTIAL__",
      "__CABINET_DENY_DOCUMENT_BODY__",
      "__CABINET_DENY_ASSET_CONTENT__",
      "__CABINET_DENY_LOCAL_PATH__",
    ].join("\n"),
    ".tasks/release/local-desktop-runbook-phase009.md": [
      "phase009_runbook=passed",
      "Product Launch",
      "Blank Screen Recovery",
      "run_desktop_shell.sh is an internal shell smoke path",
      "Field Debug requires explicit scope and expiry",
    ].join("\n"),
    ".tasks/release/runbook-validation-manifest-phase009.json": "phase009_local_desktop",
    "package.json": [
      "run:phase009-ux-release-gate-tests",
      "run:phase009-ux-release-gate",
      "run:phase009-release-evidence",
    ].join("\n"),
  };
}
