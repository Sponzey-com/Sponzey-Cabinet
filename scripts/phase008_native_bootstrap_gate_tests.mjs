import assert from "node:assert/strict";
import test from "node:test";

import {
  evaluateNativeBootstrapGate,
  NativeBootstrapGateErrorCode,
  NativeBootstrapGateEvent,
  NativeBootstrapGateState,
  renderNativeBootstrapGateArtifact,
  renderNativeBootstrapGateResult,
  transitionNativeBootstrapGateState,
} from "./phase008_native_bootstrap_gate.mjs";

test("native bootstrap gate rejects missing phase008 plan validation marker", () => {
  const result = evaluateNativeBootstrapGate({
    planValidationText: "phase008_plan_validation=failed",
    commandResults: passingCommandResults(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, NativeBootstrapGateErrorCode.PlanValidationMissing);
  assert.equal(result.findingId, ".tasks/phase008-plan-validation-result.md");
});

test("native bootstrap gate rejects failed first-run command without dumping output", () => {
  const result = evaluateNativeBootstrapGate({
    planValidationText: "phase008_plan_validation=passed",
    commandResults: {
      ...passingCommandResults(),
      firstRunStore: { command: "cargo test first-run", passed: false, exitCode: 101 },
    },
  });
  const artifact = renderNativeBootstrapGateArtifact(result);

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, NativeBootstrapGateErrorCode.FirstRunTestsFailed);
  assert.equal(result.findingId, "firstRunStore");
  assert.doesNotMatch(artifact, /raw_document_body_fixture/);
  assert.doesNotMatch(artifact, /provider_api_key_fixture/);
  assert.doesNotMatch(artifact, /\/tmp\/sponzey-cabinet/);
});

test("native bootstrap gate passes complete command evidence and renders safe marker", () => {
  const result = evaluateNativeBootstrapGate({
    planValidationText: "phase008_plan_validation=passed",
    commandResults: passingCommandResults(),
  });
  const rendered = renderNativeBootstrapGateResult(result);
  const artifact = renderNativeBootstrapGateArtifact(result);

  assert.equal(result.passed, true);
  assert.equal(result.state, NativeBootstrapGateState.Passed);
  assert.match(rendered, /phase008_native_bootstrap_gate=passed/);
  assert.match(artifact, /Product Log candidates/);
  assert.match(artifact, /sensitive-data exclusion/);
  assert.doesNotMatch(artifact, /raw_document_body_fixture/);
  assert.doesNotMatch(artifact, /provider_api_key_fixture/);
  assert.doesNotMatch(artifact, /personal_absolute_path_fixture/);
});

test("native bootstrap gate state machine exposes failure and terminal states", () => {
  const checkingPrerequisites = transitionNativeBootstrapGateState(
    NativeBootstrapGateState.NotStarted,
    NativeBootstrapGateEvent.Start,
  );
  const runningContractTests = transitionNativeBootstrapGateState(
    checkingPrerequisites.state,
    NativeBootstrapGateEvent.PrerequisitesChecked,
  );
  const runningFirstRunTests = transitionNativeBootstrapGateState(
    runningContractTests.state,
    NativeBootstrapGateEvent.ContractTestsPassed,
  );
  const writingResult = transitionNativeBootstrapGateState(
    runningFirstRunTests.state,
    NativeBootstrapGateEvent.FirstRunTestsPassed,
  );
  const passed = transitionNativeBootstrapGateState(
    writingResult.state,
    NativeBootstrapGateEvent.ResultWritten,
  );
  const failed = transitionNativeBootstrapGateState(
    runningContractTests.state,
    NativeBootstrapGateEvent.Fail,
    {
      errorCode: NativeBootstrapGateErrorCode.ContractTestsFailed,
      findingId: "contract",
    },
  );
  const invalid = transitionNativeBootstrapGateState(
    NativeBootstrapGateState.NotStarted,
    NativeBootstrapGateEvent.FirstRunTestsPassed,
  );

  assert.equal(checkingPrerequisites.state, NativeBootstrapGateState.CheckingPrerequisites);
  assert.equal(runningContractTests.state, NativeBootstrapGateState.RunningContractTests);
  assert.equal(runningFirstRunTests.state, NativeBootstrapGateState.RunningFirstRunTests);
  assert.equal(writingResult.state, NativeBootstrapGateState.WritingResult);
  assert.equal(passed.state, NativeBootstrapGateState.Passed);
  assert.equal(failed.state, NativeBootstrapGateState.Failed);
  assert.equal(failed.findingId, "contract");
  assert.equal(invalid.errorCode, NativeBootstrapGateErrorCode.InvalidTransition);
});

function passingCommandResults() {
  return {
    contract: {
      command: "npm run run:phase008-native-bootstrap-contract-tests",
      passed: true,
      exitCode: 0,
    },
    firstRunStore: {
      command: "cargo test -p cabinet-adapters --test local_first_run_store_tests",
      passed: true,
      exitCode: 0,
    },
    setupHealth: {
      command: "cargo test -p cabinet-adapters --test local_setup_health_checker_tests",
      passed: true,
      exitCode: 0,
    },
    platform: {
      command: "cargo test -p cabinet-platform",
      passed: true,
      exitCode: 0,
    },
  };
}
