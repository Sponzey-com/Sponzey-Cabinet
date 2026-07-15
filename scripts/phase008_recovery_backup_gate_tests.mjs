import assert from "node:assert/strict";
import test from "node:test";

import {
  RecoveryBackupGateErrorCode,
  RecoveryBackupGateEvent,
  RecoveryBackupGateState,
  evaluateRecoveryBackupGate,
  renderRecoveryBackupGateArtifact,
  transitionRecoveryBackupGateState,
} from "./phase008_recovery_backup_gate.mjs";

test("recovery backup gate rejects missing asset lifecycle marker", () => {
  const result = evaluateRecoveryBackupGate({
    assetLifecycleText: "phase008_asset_lifecycle_gate=failed",
    commandResults: passingCommandResults(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, RecoveryBackupGateErrorCode.AssetLifecycleGateMissing);
});

test("recovery backup gate rejects failed backup command safely", () => {
  const result = evaluateRecoveryBackupGate({
    assetLifecycleText: "phase008_asset_lifecycle_gate=passed",
    commandResults: {
      ...passingCommandResults(),
      backup: { command: "backup", passed: false, exitCode: 101 },
    },
  });
  const artifact = renderRecoveryBackupGateArtifact(result);

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, RecoveryBackupGateErrorCode.BackupTestsFailed);
  assert.doesNotMatch(artifact, /raw markdown body should not leak/);
  assert.doesNotMatch(artifact, /asset binary content should not leak/);
  assert.doesNotMatch(artifact, /phase008-provider-api-key-should-not-log/);
  assert.doesNotMatch(artifact, /local-machine-secret-fixture/);
});

test("recovery backup gate passes complete evidence and renders safe marker", () => {
  const result = evaluateRecoveryBackupGate({
    assetLifecycleText: "phase008_asset_lifecycle_gate=passed",
    commandResults: passingCommandResults(),
  });
  const artifact = renderRecoveryBackupGateArtifact(result);

  assert.equal(result.passed, true);
  assert.equal(result.state, RecoveryBackupGateState.Passed);
  assert.match(artifact, /phase008_recovery_backup_gate=passed/);
  assert.doesNotMatch(artifact, /\/Users\/example\/private/);
  assert.doesNotMatch(artifact, /raw_document_body_fixture/);
  assert.doesNotMatch(artifact, /personal_absolute_path_fixture/);
});

test("recovery backup gate state machine reaches terminal states", () => {
  const checkingPrerequisites = transitionRecoveryBackupGateState(
    RecoveryBackupGateState.NotStarted,
    RecoveryBackupGateEvent.Start,
  );
  const runningRecoveryTests = transitionRecoveryBackupGateState(
    checkingPrerequisites.state,
    RecoveryBackupGateEvent.PrerequisitesChecked,
  );
  const runningBackupTests = transitionRecoveryBackupGateState(
    runningRecoveryTests.state,
    RecoveryBackupGateEvent.RecoveryTestsPassed,
  );
  const runningUiTests = transitionRecoveryBackupGateState(
    runningBackupTests.state,
    RecoveryBackupGateEvent.BackupTestsPassed,
  );
  const writingResult = transitionRecoveryBackupGateState(
    runningUiTests.state,
    RecoveryBackupGateEvent.UiTestsPassed,
  );
  const passed = transitionRecoveryBackupGateState(
    writingResult.state,
    RecoveryBackupGateEvent.ResultWritten,
  );
  const invalid = transitionRecoveryBackupGateState(
    RecoveryBackupGateState.NotStarted,
    RecoveryBackupGateEvent.UiTestsPassed,
  );

  assert.equal(checkingPrerequisites.state, RecoveryBackupGateState.CheckingPrerequisites);
  assert.equal(runningRecoveryTests.state, RecoveryBackupGateState.RunningRecoveryTests);
  assert.equal(runningBackupTests.state, RecoveryBackupGateState.RunningBackupTests);
  assert.equal(runningUiTests.state, RecoveryBackupGateState.RunningUiTests);
  assert.equal(writingResult.state, RecoveryBackupGateState.WritingResult);
  assert.equal(passed.state, RecoveryBackupGateState.Passed);
  assert.equal(invalid.errorCode, RecoveryBackupGateErrorCode.InvalidTransition);
});

function passingCommandResults() {
  return {
    recovery: { command: "recovery tests", passed: true, exitCode: 0 },
    backup: { command: "backup tests", passed: true, exitCode: 0 },
    ui: { command: "ui tests", passed: true, exitCode: 0 },
  };
}
