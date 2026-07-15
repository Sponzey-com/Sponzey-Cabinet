import assert from "node:assert/strict";
import test from "node:test";

import {
  DocumentRuntimeGateErrorCode,
  DocumentRuntimeGateEvent,
  DocumentRuntimeGateState,
  evaluateDocumentRuntimeGate,
  renderDocumentRuntimeGateArtifact,
  renderDocumentRuntimeGateResult,
  transitionDocumentRuntimeGateState,
} from "./phase008_document_runtime_gate.mjs";

test("document runtime gate rejects missing command bridge marker", () => {
  const result = evaluateDocumentRuntimeGate({
    commandBridgeText: "phase008_command_bridge_gate=failed",
    commandResults: passingCommandResults(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, DocumentRuntimeGateErrorCode.CommandBridgeMissing);
});

test("document runtime gate rejects failed adapter command without output dump", () => {
  const result = evaluateDocumentRuntimeGate({
    commandBridgeText: "phase008_command_bridge_gate=passed",
    commandResults: {
      ...passingCommandResults(),
      adapter: { command: "adapter tests", passed: false, exitCode: 101 },
    },
  });
  const artifact = renderDocumentRuntimeGateArtifact(result);

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, DocumentRuntimeGateErrorCode.AdapterTestsFailed);
  assert.equal(result.findingId, "adapter");
  assert.doesNotMatch(artifact, /raw_document_body_fixture/);
  assert.doesNotMatch(artifact, /provider_api_key_fixture/);
});

test("document runtime gate passes complete evidence and renders safe marker", () => {
  const result = evaluateDocumentRuntimeGate({
    commandBridgeText: "phase008_command_bridge_gate=passed",
    commandResults: passingCommandResults(),
  });
  const rendered = renderDocumentRuntimeGateResult(result);
  const artifact = renderDocumentRuntimeGateArtifact(result);

  assert.equal(result.passed, true);
  assert.equal(result.state, DocumentRuntimeGateState.Passed);
  assert.match(rendered, /phase008_document_runtime_gate=passed/);
  assert.match(artifact, /current_history_separated=true/);
  assert.doesNotMatch(artifact, /raw_document_body_fixture/);
  assert.doesNotMatch(artifact, /personal_absolute_path_fixture/);
});

test("document runtime gate state machine exposes terminal states", () => {
  const checkingPrerequisites = transitionDocumentRuntimeGateState(
    DocumentRuntimeGateState.NotStarted,
    DocumentRuntimeGateEvent.Start,
  );
  const runningUsecaseTests = transitionDocumentRuntimeGateState(
    checkingPrerequisites.state,
    DocumentRuntimeGateEvent.PrerequisitesChecked,
  );
  const runningAdapterTests = transitionDocumentRuntimeGateState(
    runningUsecaseTests.state,
    DocumentRuntimeGateEvent.UsecaseTestsPassed,
  );
  const runningDesktopTests = transitionDocumentRuntimeGateState(
    runningAdapterTests.state,
    DocumentRuntimeGateEvent.AdapterTestsPassed,
  );
  const writingResult = transitionDocumentRuntimeGateState(
    runningDesktopTests.state,
    DocumentRuntimeGateEvent.DesktopTestsPassed,
  );
  const passed = transitionDocumentRuntimeGateState(
    writingResult.state,
    DocumentRuntimeGateEvent.ResultWritten,
  );
  const invalid = transitionDocumentRuntimeGateState(
    DocumentRuntimeGateState.NotStarted,
    DocumentRuntimeGateEvent.DesktopTestsPassed,
  );

  assert.equal(checkingPrerequisites.state, DocumentRuntimeGateState.CheckingPrerequisites);
  assert.equal(runningUsecaseTests.state, DocumentRuntimeGateState.RunningUsecaseTests);
  assert.equal(runningAdapterTests.state, DocumentRuntimeGateState.RunningAdapterTests);
  assert.equal(runningDesktopTests.state, DocumentRuntimeGateState.RunningDesktopTests);
  assert.equal(writingResult.state, DocumentRuntimeGateState.WritingResult);
  assert.equal(passed.state, DocumentRuntimeGateState.Passed);
  assert.equal(invalid.errorCode, DocumentRuntimeGateErrorCode.InvalidTransition);
});

function passingCommandResults() {
  return {
    usecase: { command: "usecase tests", passed: true, exitCode: 0 },
    adapter: { command: "adapter tests", passed: true, exitCode: 0 },
    desktop: { command: "desktop tests", passed: true, exitCode: 0 },
  };
}
