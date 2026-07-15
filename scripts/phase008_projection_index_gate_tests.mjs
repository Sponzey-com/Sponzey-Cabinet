import assert from "node:assert/strict";
import test from "node:test";

import {
  ProjectionIndexGateErrorCode,
  ProjectionIndexGateEvent,
  ProjectionIndexGateState,
  evaluateProjectionIndexGate,
  renderPerformanceBudgetArtifact,
  renderProjectionIndexGateArtifact,
  transitionProjectionIndexGateState,
} from "./phase008_projection_index_gate.mjs";

test("projection index gate rejects missing document runtime marker", () => {
  const result = evaluateProjectionIndexGate({
    documentRuntimeText: "phase008_document_runtime_gate=failed",
    commandResults: passingCommandResults(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, ProjectionIndexGateErrorCode.DocumentRuntimeMissing);
});

test("projection index gate rejects failed performance command safely", () => {
  const result = evaluateProjectionIndexGate({
    documentRuntimeText: "phase008_document_runtime_gate=passed",
    commandResults: {
      ...passingCommandResults(),
      performance: { command: "performance", passed: false, exitCode: 101 },
    },
  });
  const artifact = renderProjectionIndexGateArtifact(result);

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, ProjectionIndexGateErrorCode.PerformanceTestsFailed);
  assert.doesNotMatch(artifact, /raw_document_body_fixture/);
  assert.doesNotMatch(artifact, /provider_api_key_fixture/);
});

test("projection index gate renders safe gate and performance artifacts", () => {
  const result = evaluateProjectionIndexGate({
    documentRuntimeText: "phase008_document_runtime_gate=passed",
    commandResults: passingCommandResults(),
  });
  const gateArtifact = renderProjectionIndexGateArtifact(result);
  const performanceArtifact = renderPerformanceBudgetArtifact(result);

  assert.equal(result.passed, true);
  assert.equal(result.state, ProjectionIndexGateState.Passed);
  assert.match(gateArtifact, /phase008_projection_index_gate=passed/);
  assert.match(performanceArtifact, /phase008_performance_budget=passed/);
  assert.match(performanceArtifact, /p95_target_ms=300/);
  assert.doesNotMatch(gateArtifact, /personal_absolute_path_fixture/);
  assert.doesNotMatch(performanceArtifact, /raw_document_body_fixture/);
});

test("projection index gate state machine reaches terminal states", () => {
  const checkingPrerequisites = transitionProjectionIndexGateState(
    ProjectionIndexGateState.NotStarted,
    ProjectionIndexGateEvent.Start,
  );
  const runningProjectionTests = transitionProjectionIndexGateState(
    checkingPrerequisites.state,
    ProjectionIndexGateEvent.PrerequisitesChecked,
  );
  const runningPerformanceTests = transitionProjectionIndexGateState(
    runningProjectionTests.state,
    ProjectionIndexGateEvent.ProjectionTestsPassed,
  );
  const runningDesktopTests = transitionProjectionIndexGateState(
    runningPerformanceTests.state,
    ProjectionIndexGateEvent.PerformanceTestsPassed,
  );
  const writingResult = transitionProjectionIndexGateState(
    runningDesktopTests.state,
    ProjectionIndexGateEvent.DesktopTestsPassed,
  );
  const passed = transitionProjectionIndexGateState(
    writingResult.state,
    ProjectionIndexGateEvent.ResultWritten,
  );
  const invalid = transitionProjectionIndexGateState(
    ProjectionIndexGateState.NotStarted,
    ProjectionIndexGateEvent.PerformanceTestsPassed,
  );

  assert.equal(checkingPrerequisites.state, ProjectionIndexGateState.CheckingPrerequisites);
  assert.equal(runningProjectionTests.state, ProjectionIndexGateState.RunningProjectionTests);
  assert.equal(runningPerformanceTests.state, ProjectionIndexGateState.RunningPerformanceTests);
  assert.equal(runningDesktopTests.state, ProjectionIndexGateState.RunningDesktopTests);
  assert.equal(writingResult.state, ProjectionIndexGateState.WritingResult);
  assert.equal(passed.state, ProjectionIndexGateState.Passed);
  assert.equal(invalid.errorCode, ProjectionIndexGateErrorCode.InvalidTransition);
});

function passingCommandResults() {
  return {
    projection: { command: "projection tests", passed: true, exitCode: 0 },
    performance: { command: "performance tests", passed: true, exitCode: 0 },
    desktop: { command: "desktop discovery", passed: true, exitCode: 0 },
  };
}
