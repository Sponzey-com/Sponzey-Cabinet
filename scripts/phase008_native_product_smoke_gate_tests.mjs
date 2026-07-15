import assert from "node:assert/strict";
import test from "node:test";

import {
  NativeProductSmokeGateErrorCode,
  NativeProductSmokeGateEvent,
  NativeProductSmokeGateState,
  evaluateNativeProductSmokeGate,
  renderNativeProductSmokeGateArtifact,
  transitionNativeProductSmokeGateState,
} from "./phase008_native_product_smoke_gate.mjs";

test("native product smoke gate rejects missing lower marker", () => {
  const markerTexts = passingMarkerTexts();
  markerTexts[".tasks/phase008-document-runtime-gate-result.md"] = "phase008_document_runtime_gate=failed";

  const result = evaluateNativeProductSmokeGate({
    markerTexts,
    commandResults: passingCommandResults(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, NativeProductSmokeGateErrorCode.LowerGateMissing);
  assert.equal(result.findingId, ".tasks/phase008-document-runtime-gate-result.md");
});

test("native product smoke gate rejects failed browser smoke command safely", () => {
  const result = evaluateNativeProductSmokeGate({
    markerTexts: passingMarkerTexts(),
    commandResults: {
      ...passingCommandResults(),
      browser: { command: "browser", passed: false, exitCode: 1 },
    },
  });
  const artifact = renderNativeProductSmokeGateArtifact(result);

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, NativeProductSmokeGateErrorCode.BrowserSmokeFailed);
  assert.doesNotMatch(artifact, /raw markdown body should not leak/);
  assert.doesNotMatch(artifact, /asset binary content should not leak/);
  assert.doesNotMatch(artifact, /provider_api_key_fixture/);
  assert.doesNotMatch(artifact, /\/Users\/example\/private/);
});

test("native product smoke gate passes complete evidence and renders safe marker", () => {
  const result = evaluateNativeProductSmokeGate({
    markerTexts: passingMarkerTexts(),
    commandResults: passingCommandResults(),
  });
  const artifact = renderNativeProductSmokeGateArtifact(result);

  assert.equal(result.passed, true);
  assert.equal(result.state, NativeProductSmokeGateState.Passed);
  assert.match(artifact, /phase008_native_product_smoke_gate=passed/);
  assert.doesNotMatch(artifact, /raw_document_body_fixture/);
  assert.doesNotMatch(artifact, /personal_absolute_path_fixture/);
});

test("native product smoke gate state machine reaches terminal states", () => {
  const readingEvidence = transitionNativeProductSmokeGateState(
    NativeProductSmokeGateState.NotStarted,
    NativeProductSmokeGateEvent.Start,
  );
  const runningNative = transitionNativeProductSmokeGateState(
    readingEvidence.state,
    NativeProductSmokeGateEvent.EvidenceReady,
  );
  const runningUi = transitionNativeProductSmokeGateState(
    runningNative.state,
    NativeProductSmokeGateEvent.NativeRuntimeSmokePassed,
  );
  const runningBrowser = transitionNativeProductSmokeGateState(
    runningUi.state,
    NativeProductSmokeGateEvent.DesktopUiSmokePassed,
  );
  const writingResult = transitionNativeProductSmokeGateState(
    runningBrowser.state,
    NativeProductSmokeGateEvent.BrowserSmokePassed,
  );
  const passed = transitionNativeProductSmokeGateState(
    writingResult.state,
    NativeProductSmokeGateEvent.ResultWritten,
  );
  const invalid = transitionNativeProductSmokeGateState(
    NativeProductSmokeGateState.NotStarted,
    NativeProductSmokeGateEvent.BrowserSmokePassed,
  );

  assert.equal(readingEvidence.state, NativeProductSmokeGateState.ReadingEvidence);
  assert.equal(runningNative.state, NativeProductSmokeGateState.RunningNativeRuntimeSmoke);
  assert.equal(runningUi.state, NativeProductSmokeGateState.RunningDesktopUiSmoke);
  assert.equal(runningBrowser.state, NativeProductSmokeGateState.RunningBrowserSmoke);
  assert.equal(writingResult.state, NativeProductSmokeGateState.WritingResult);
  assert.equal(passed.state, NativeProductSmokeGateState.Passed);
  assert.equal(invalid.errorCode, NativeProductSmokeGateErrorCode.InvalidTransition);
});

function passingMarkerTexts() {
  return {
    ".tasks/phase008-plan-validation-result.md": "phase008_plan_validation=passed",
    ".tasks/phase008-native-bootstrap-gate-result.md": "phase008_native_bootstrap_gate=passed",
    ".tasks/phase008-command-bridge-gate-result.md": "phase008_command_bridge_gate=passed",
    ".tasks/phase008-document-runtime-gate-result.md": "phase008_document_runtime_gate=passed",
    ".tasks/phase008-projection-index-gate-result.md": "phase008_projection_index_gate=passed",
    ".tasks/phase008-asset-lifecycle-gate-result.md": "phase008_asset_lifecycle_gate=passed",
    ".tasks/phase008-recovery-backup-gate-result.md": "phase008_recovery_backup_gate=passed",
  };
}

function passingCommandResults() {
  return {
    runtime: { command: "runtime smoke", passed: true, exitCode: 0 },
    ui: { command: "ui smoke", passed: true, exitCode: 0 },
    browser: { command: "browser smoke", passed: true, exitCode: 0 },
  };
}
