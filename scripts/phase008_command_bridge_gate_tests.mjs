import assert from "node:assert/strict";
import test from "node:test";

import {
  CommandBridgeGateErrorCode,
  CommandBridgeGateEvent,
  CommandBridgeGateState,
  evaluateCommandBridgeGate,
  renderCommandBridgeGateArtifact,
  renderCommandBridgeGateResult,
  transitionCommandBridgeGateState,
} from "./phase008_command_bridge_gate.mjs";

test("command bridge gate rejects missing native bootstrap marker", () => {
  const result = evaluateCommandBridgeGate({
    planValidationText: "phase008_plan_validation=passed",
    nativeBootstrapText: "phase008_native_bootstrap_gate=failed",
    commandResults: passingCommandResults(),
    routeSourceText: safeRouteSource(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, CommandBridgeGateErrorCode.NativeBootstrapMissing);
  assert.equal(result.findingId, ".tasks/phase008-native-bootstrap-gate-result.md");
});

test("command bridge gate rejects failed rust boundary command without output dump", () => {
  const result = evaluateCommandBridgeGate({
    planValidationText: "phase008_plan_validation=passed",
    nativeBootstrapText: "phase008_native_bootstrap_gate=passed",
    commandResults: {
      ...passingCommandResults(),
      rustBoundary: { command: "cargo test rust", passed: false, exitCode: 101 },
    },
    routeSourceText: safeRouteSource(),
  });
  const artifact = renderCommandBridgeGateArtifact(result);

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, CommandBridgeGateErrorCode.RustBoundaryTestsFailed);
  assert.equal(result.findingId, "rustBoundary");
  assert.doesNotMatch(artifact, /raw_document_body_fixture/);
  assert.doesNotMatch(artifact, /provider_api_key_fixture/);
});

test("command bridge gate rejects forbidden direct storage or env access in route", () => {
  const result = evaluateCommandBridgeGate({
    planValidationText: "phase008_plan_validation=passed",
    nativeBootstrapText: "phase008_native_bootstrap_gate=passed",
    commandResults: passingCommandResults(),
    routeSourceText: "pub fn route_local_desktop_command() { std::env::var(\"SECRET\"); }",
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, CommandBridgeGateErrorCode.ForbiddenBoundaryAccess);
  assert.equal(result.findingId, "std::env::var");
});

test("command bridge gate passes complete evidence and renders safe marker", () => {
  const result = evaluateCommandBridgeGate({
    planValidationText: "phase008_plan_validation=passed",
    nativeBootstrapText: "phase008_native_bootstrap_gate=passed",
    commandResults: passingCommandResults(),
    routeSourceText: safeRouteSource(),
  });
  const rendered = renderCommandBridgeGateResult(result);
  const artifact = renderCommandBridgeGateArtifact(result);

  assert.equal(result.passed, true);
  assert.equal(result.state, CommandBridgeGateState.Passed);
  assert.match(rendered, /phase008_command_bridge_gate=passed/);
  assert.match(artifact, /allowed_command_count=7/);
  assert.doesNotMatch(artifact, /raw_document_body_fixture/);
  assert.doesNotMatch(artifact, /provider_api_key_fixture/);
  assert.doesNotMatch(artifact, /personal_absolute_path_fixture/);
});

test("command bridge gate state machine exposes terminal states", () => {
  const checkingPrerequisites = transitionCommandBridgeGateState(
    CommandBridgeGateState.NotStarted,
    CommandBridgeGateEvent.Start,
  );
  const runningContractTests = transitionCommandBridgeGateState(
    checkingPrerequisites.state,
    CommandBridgeGateEvent.PrerequisitesChecked,
  );
  const runningRustBoundaryTests = transitionCommandBridgeGateState(
    runningContractTests.state,
    CommandBridgeGateEvent.ContractTestsPassed,
  );
  const scanningBoundary = transitionCommandBridgeGateState(
    runningRustBoundaryTests.state,
    CommandBridgeGateEvent.RustBoundaryTestsPassed,
  );
  const writingResult = transitionCommandBridgeGateState(
    scanningBoundary.state,
    CommandBridgeGateEvent.BoundaryScanPassed,
  );
  const passed = transitionCommandBridgeGateState(
    writingResult.state,
    CommandBridgeGateEvent.ResultWritten,
  );
  const invalid = transitionCommandBridgeGateState(
    CommandBridgeGateState.NotStarted,
    CommandBridgeGateEvent.RustBoundaryTestsPassed,
  );

  assert.equal(checkingPrerequisites.state, CommandBridgeGateState.CheckingPrerequisites);
  assert.equal(runningContractTests.state, CommandBridgeGateState.RunningContractTests);
  assert.equal(runningRustBoundaryTests.state, CommandBridgeGateState.RunningRustBoundaryTests);
  assert.equal(scanningBoundary.state, CommandBridgeGateState.ScanningBoundary);
  assert.equal(writingResult.state, CommandBridgeGateState.WritingResult);
  assert.equal(passed.state, CommandBridgeGateState.Passed);
  assert.equal(invalid.errorCode, CommandBridgeGateErrorCode.InvalidTransition);
});

function passingCommandResults() {
  return {
    contract: {
      command: "npm run run:phase008-command-bridge-contract-tests",
      passed: true,
      exitCode: 0,
    },
    rustBoundary: {
      command: "cargo test -p cabinet-desktop-shell local_desktop_command",
      passed: true,
      exitCode: 0,
    },
  };
}

function safeRouteSource() {
  return [
    "pub fn route_local_desktop_command(command_name: String) -> DesktopLocalCommandRouteResponse {",
    "let accepted = LOCAL_DESKTOP_COMMAND_NAMES.contains(&command_name.as_str());",
    "DesktopLocalCommandRouteResponse { boundary: cabinet_platform::layer_name().to_string(), command_name, accepted, error_code: None }",
    "}",
  ].join("\n");
}
