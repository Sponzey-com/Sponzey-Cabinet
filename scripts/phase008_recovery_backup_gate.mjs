import { readFile, writeFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { pathToFileURL } from "node:url";

export const RecoveryBackupGateState = Object.freeze({
  NotStarted: "NotStarted",
  CheckingPrerequisites: "CheckingPrerequisites",
  RunningRecoveryTests: "RunningRecoveryTests",
  RunningBackupTests: "RunningBackupTests",
  RunningUiTests: "RunningUiTests",
  WritingResult: "WritingResult",
  Passed: "Passed",
  Failed: "Failed",
});

export const RecoveryBackupGateEvent = Object.freeze({
  Start: "Start",
  PrerequisitesChecked: "PrerequisitesChecked",
  RecoveryTestsPassed: "RecoveryTestsPassed",
  BackupTestsPassed: "BackupTestsPassed",
  UiTestsPassed: "UiTestsPassed",
  ResultWritten: "ResultWritten",
  Fail: "Fail",
});

export const RecoveryBackupGateErrorCode = Object.freeze({
  AssetLifecycleGateMissing: "PHASE008_RECOVERY_BACKUP_ASSET_LIFECYCLE_GATE_MISSING",
  RecoveryTestsFailed: "PHASE008_RECOVERY_BACKUP_RECOVERY_TESTS_FAILED",
  BackupTestsFailed: "PHASE008_RECOVERY_BACKUP_BACKUP_TESTS_FAILED",
  UiTestsFailed: "PHASE008_RECOVERY_BACKUP_UI_TESTS_FAILED",
  IoFailed: "PHASE008_RECOVERY_BACKUP_IO_FAILED",
  InvalidTransition: "PHASE008_RECOVERY_BACKUP_INVALID_TRANSITION",
});

const commandOrder = [
  {
    key: "recovery",
    errorCode: RecoveryBackupGateErrorCode.RecoveryTestsFailed,
    command: "npm run run:phase008-recovery-backup-recovery-tests",
    spawn: ["npm", ["run", "run:phase008-recovery-backup-recovery-tests"]],
  },
  {
    key: "backup",
    errorCode: RecoveryBackupGateErrorCode.BackupTestsFailed,
    command: "npm run run:phase008-recovery-backup-package-tests",
    spawn: ["npm", ["run", "run:phase008-recovery-backup-package-tests"]],
  },
  {
    key: "ui",
    errorCode: RecoveryBackupGateErrorCode.UiTestsFailed,
    command: "npm run run:phase008-recovery-backup-ui-tests",
    spawn: ["npm", ["run", "run:phase008-recovery-backup-ui-tests"]],
  },
];

export function transitionRecoveryBackupGateState(currentState, event, detail = {}) {
  if (currentState === RecoveryBackupGateState.NotStarted && event === RecoveryBackupGateEvent.Start) {
    return { state: RecoveryBackupGateState.CheckingPrerequisites };
  }
  if (
    currentState === RecoveryBackupGateState.CheckingPrerequisites &&
    event === RecoveryBackupGateEvent.PrerequisitesChecked
  ) {
    return { state: RecoveryBackupGateState.RunningRecoveryTests };
  }
  if (
    currentState === RecoveryBackupGateState.RunningRecoveryTests &&
    event === RecoveryBackupGateEvent.RecoveryTestsPassed
  ) {
    return { state: RecoveryBackupGateState.RunningBackupTests };
  }
  if (
    currentState === RecoveryBackupGateState.RunningBackupTests &&
    event === RecoveryBackupGateEvent.BackupTestsPassed
  ) {
    return { state: RecoveryBackupGateState.RunningUiTests };
  }
  if (
    currentState === RecoveryBackupGateState.RunningUiTests &&
    event === RecoveryBackupGateEvent.UiTestsPassed
  ) {
    return { state: RecoveryBackupGateState.WritingResult };
  }
  if (
    currentState === RecoveryBackupGateState.WritingResult &&
    event === RecoveryBackupGateEvent.ResultWritten
  ) {
    return { state: RecoveryBackupGateState.Passed };
  }
  if (
    [
      RecoveryBackupGateState.CheckingPrerequisites,
      RecoveryBackupGateState.RunningRecoveryTests,
      RecoveryBackupGateState.RunningBackupTests,
      RecoveryBackupGateState.RunningUiTests,
      RecoveryBackupGateState.WritingResult,
    ].includes(currentState) &&
    event === RecoveryBackupGateEvent.Fail
  ) {
    return {
      state: RecoveryBackupGateState.Failed,
      errorCode: detail.errorCode ?? RecoveryBackupGateErrorCode.IoFailed,
      findingId: detail.findingId,
      failedCommandExitCode: detail.failedCommandExitCode,
    };
  }
  return { state: RecoveryBackupGateState.Failed, errorCode: RecoveryBackupGateErrorCode.InvalidTransition };
}

export function evaluateRecoveryBackupGate({ assetLifecycleText, commandResults }) {
  let state = transitionRecoveryBackupGateState(
    RecoveryBackupGateState.NotStarted,
    RecoveryBackupGateEvent.Start,
  );
  if (!assetLifecycleText.includes("phase008_asset_lifecycle_gate=passed")) {
    state = transitionRecoveryBackupGateState(state.state, RecoveryBackupGateEvent.Fail, {
      errorCode: RecoveryBackupGateErrorCode.AssetLifecycleGateMissing,
      findingId: ".tasks/phase008-asset-lifecycle-gate-result.md",
    });
    return failedResult(state, commandResults);
  }

  state = transitionRecoveryBackupGateState(state.state, RecoveryBackupGateEvent.PrerequisitesChecked);

  const recoveryCommand = commandOrder.find((candidate) => candidate.key === "recovery");
  const recoveryResult = commandResults[recoveryCommand.key];
  if (!recoveryResult?.passed) {
    state = transitionRecoveryBackupGateState(state.state, RecoveryBackupGateEvent.Fail, {
      errorCode: recoveryCommand.errorCode,
      findingId: recoveryCommand.key,
      failedCommandExitCode: recoveryResult?.exitCode,
    });
    return failedResult(state, commandResults);
  }
  state = transitionRecoveryBackupGateState(state.state, RecoveryBackupGateEvent.RecoveryTestsPassed);

  const backupCommand = commandOrder.find((candidate) => candidate.key === "backup");
  const backupResult = commandResults[backupCommand.key];
  if (!backupResult?.passed) {
    state = transitionRecoveryBackupGateState(state.state, RecoveryBackupGateEvent.Fail, {
      errorCode: backupCommand.errorCode,
      findingId: backupCommand.key,
      failedCommandExitCode: backupResult?.exitCode,
    });
    return failedResult(state, commandResults);
  }
  state = transitionRecoveryBackupGateState(state.state, RecoveryBackupGateEvent.BackupTestsPassed);

  const uiCommand = commandOrder.find((candidate) => candidate.key === "ui");
  const uiResult = commandResults[uiCommand.key];
  if (!uiResult?.passed) {
    state = transitionRecoveryBackupGateState(state.state, RecoveryBackupGateEvent.Fail, {
      errorCode: uiCommand.errorCode,
      findingId: uiCommand.key,
      failedCommandExitCode: uiResult?.exitCode,
    });
    return failedResult(state, commandResults);
  }
  state = transitionRecoveryBackupGateState(state.state, RecoveryBackupGateEvent.UiTestsPassed);
  state = transitionRecoveryBackupGateState(state.state, RecoveryBackupGateEvent.ResultWritten);

  return { passed: true, state: state.state, commandCount: commandOrder.length };
}

export function renderRecoveryBackupGateArtifact(result) {
  const status = result.passed ? "passed" : "failed";
  return [
    "# Phase 008 Recovery Backup Gate Result",
    "",
    result.passed
      ? [
          "phase008_recovery_backup_gate=passed",
          `validation_state=${result.state}`,
          `validation_command_count=${result.commandCount}`,
          "restore_confirmation_guard=true",
          "backup_package_sensitive_data_excluded=true",
        ].join("\n")
      : [
          "phase008_recovery_backup_gate=failed",
          `validation_state=${result.state}`,
          `error_code=${result.errorCode}`,
          result.findingId ? `finding_id=${result.findingId}` : undefined,
        ].filter(Boolean).join("\n"),
    "",
    "- phase: `Phase 008.6`",
    "- gate: `Workspace Recovery, Backup, Import, Export, Restore, and Upgrade Safety`",
    `- status: \`${status}\``,
    "- prerequisites:",
    "  - `.tasks/phase008-asset-lifecycle-gate-result.md` with `phase008_asset_lifecycle_gate=passed`",
    "- validation commands:",
    ...commandOrder.map(({ command }) => `  - \`${command}\``),
    "- evidence: setup health, migration status, startup repair, data preservation, backup job lifecycle, backup package store, import preview staging, export plan, restore preview/apply guard, and desktop recovery UI safety.",
    "- sensitive-data exclusion: this artifact records markers, command ids, counts, states, and stable error codes only. It does not record raw document body, asset content, local absolute path, provider key, token, credential, local machine secret, Field Debug activation state, or package internal file contents.",
    "- follow-up limitation: Phase 008.7 native product smoke and final release gate remain incomplete.",
    "",
  ].join("\n");
}

function failedResult(state, commandResults) {
  return {
    passed: false,
    state: state.state,
    errorCode: state.errorCode,
    findingId: state.findingId,
    failedCommandExitCode: state.failedCommandExitCode,
    commandResults,
  };
}

function runCommand({ key, command, spawn }) {
  const [program, args] = spawn;
  const completed = spawnSync(program, args, {
    cwd: process.cwd(),
    encoding: "utf8",
    stdio: "inherit",
  });
  return { key, command, passed: completed.status === 0, exitCode: completed.status ?? 1 };
}

async function runRecoveryBackupGateCli() {
  let result;
  try {
    const assetLifecycleText = await readFile(".tasks/phase008-asset-lifecycle-gate-result.md", "utf8");
    const commandResults = assetLifecycleText.includes("phase008_asset_lifecycle_gate=passed")
      ? Object.fromEntries(commandOrder.map((commandSpec) => [commandSpec.key, runCommand(commandSpec)]))
      : {};
    result = evaluateRecoveryBackupGate({ assetLifecycleText, commandResults });
  } catch {
    const state = {
      state: RecoveryBackupGateState.Failed,
      errorCode: RecoveryBackupGateErrorCode.IoFailed,
      findingId: ".tasks/phase008-asset-lifecycle-gate-result.md",
    };
    result = failedResult(state, {});
  }
  await writeFile(
    ".tasks/phase008-recovery-backup-gate-result.md",
    renderRecoveryBackupGateArtifact(result),
  );
  if (result.passed) {
    console.log("phase008_recovery_backup_gate=passed");
    return;
  }
  console.error("phase008_recovery_backup_gate=failed");
  process.exit(1);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runRecoveryBackupGateCli();
}
