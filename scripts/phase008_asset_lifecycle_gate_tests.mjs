import assert from "node:assert/strict";
import test from "node:test";

import {
  AssetLifecycleGateErrorCode,
  AssetLifecycleGateEvent,
  AssetLifecycleGateState,
  evaluateAssetLifecycleGate,
  renderAssetLifecycleGateArtifact,
  transitionAssetLifecycleGateState,
} from "./phase008_asset_lifecycle_gate.mjs";

test("asset lifecycle gate rejects missing projection marker", () => {
  const result = evaluateAssetLifecycleGate({
    projectionText: "phase008_projection_index_gate=failed",
    commandResults: passingCommandResults(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, AssetLifecycleGateErrorCode.ProjectionGateMissing);
});

test("asset lifecycle gate rejects failed adapter command safely", () => {
  const result = evaluateAssetLifecycleGate({
    projectionText: "phase008_projection_index_gate=passed",
    commandResults: {
      ...passingCommandResults(),
      adapter: { command: "adapter", passed: false, exitCode: 101 },
    },
  });
  const artifact = renderAssetLifecycleGateArtifact(result);

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, AssetLifecycleGateErrorCode.AdapterTestsFailed);
  assert.doesNotMatch(artifact, /asset binary content should not leak/);
  assert.doesNotMatch(artifact, /provider_api_key_fixture/);
});

test("asset lifecycle gate passes complete evidence and renders safe marker", () => {
  const result = evaluateAssetLifecycleGate({
    projectionText: "phase008_projection_index_gate=passed",
    commandResults: passingCommandResults(),
  });
  const artifact = renderAssetLifecycleGateArtifact(result);

  assert.equal(result.passed, true);
  assert.equal(result.state, AssetLifecycleGateState.Passed);
  assert.match(artifact, /phase008_asset_lifecycle_gate=passed/);
  assert.doesNotMatch(artifact, /raw_document_body_fixture/);
  assert.doesNotMatch(artifact, /personal_absolute_path_fixture/);
});

test("asset lifecycle gate state machine reaches terminal states", () => {
  const checkingPrerequisites = transitionAssetLifecycleGateState(
    AssetLifecycleGateState.NotStarted,
    AssetLifecycleGateEvent.Start,
  );
  const runningAssetTests = transitionAssetLifecycleGateState(
    checkingPrerequisites.state,
    AssetLifecycleGateEvent.PrerequisitesChecked,
  );
  const runningUiTests = transitionAssetLifecycleGateState(
    runningAssetTests.state,
    AssetLifecycleGateEvent.AssetTestsPassed,
  );
  const writingResult = transitionAssetLifecycleGateState(
    runningUiTests.state,
    AssetLifecycleGateEvent.UiTestsPassed,
  );
  const passed = transitionAssetLifecycleGateState(
    writingResult.state,
    AssetLifecycleGateEvent.ResultWritten,
  );
  const invalid = transitionAssetLifecycleGateState(
    AssetLifecycleGateState.NotStarted,
    AssetLifecycleGateEvent.UiTestsPassed,
  );

  assert.equal(checkingPrerequisites.state, AssetLifecycleGateState.CheckingPrerequisites);
  assert.equal(runningAssetTests.state, AssetLifecycleGateState.RunningAssetTests);
  assert.equal(runningUiTests.state, AssetLifecycleGateState.RunningUiTests);
  assert.equal(writingResult.state, AssetLifecycleGateState.WritingResult);
  assert.equal(passed.state, AssetLifecycleGateState.Passed);
  assert.equal(invalid.errorCode, AssetLifecycleGateErrorCode.InvalidTransition);
});

function passingCommandResults() {
  return {
    asset: { command: "asset tests", passed: true, exitCode: 0 },
    adapter: { command: "adapter tests", passed: true, exitCode: 0 },
    ui: { command: "ui tests", passed: true, exitCode: 0 },
  };
}
