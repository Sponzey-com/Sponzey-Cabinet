import { spawn } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const Phase010FirstRunWorkspaceState = Object.freeze({
  Pending: "Pending",
  ReadingPrerequisites: "ReadingPrerequisites",
  RunningFirstRunTests: "RunningFirstRunTests",
  RunningHealthTests: "RunningHealthTests",
  RunningRepairTests: "RunningRepairTests",
  WritingResult: "WritingResult",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase010FirstRunWorkspaceEvent = Object.freeze({
  Start: "Start",
  PrerequisitesRead: "PrerequisitesRead",
  FirstRunTestsPassed: "FirstRunTestsPassed",
  HealthTestsPassed: "HealthTestsPassed",
  RepairTestsPassed: "RepairTestsPassed",
  ResultWritten: "ResultWritten",
  Fail: "Fail",
});

export const Phase010FirstRunWorkspaceErrorCode = Object.freeze({
  PackagedLaunchMarkerMissing: "PHASE010_PACKAGED_LAUNCH_MARKER_MISSING",
  FirstRunTestsFailed: "PHASE010_FIRST_RUN_TESTS_FAILED",
  HealthTestsFailed: "PHASE010_WORKSPACE_HEALTH_TESTS_FAILED",
  BootstrapTestsFailed: "PHASE010_NATIVE_BOOTSTRAP_TESTS_FAILED",
  RepairTestsFailed: "PHASE010_WORKSPACE_REPAIR_TESTS_FAILED",
  IoFailed: "PHASE010_FIRST_RUN_WORKSPACE_IO_FAILED",
  InvalidTransition: "PHASE010_FIRST_RUN_WORKSPACE_INVALID_TRANSITION",
});

export function transitionPhase010FirstRunWorkspaceState(currentState, event, detail = {}) {
  if (
    currentState === Phase010FirstRunWorkspaceState.Pending &&
    event === Phase010FirstRunWorkspaceEvent.Start
  ) {
    return { state: Phase010FirstRunWorkspaceState.ReadingPrerequisites };
  }
  if (
    currentState === Phase010FirstRunWorkspaceState.ReadingPrerequisites &&
    event === Phase010FirstRunWorkspaceEvent.PrerequisitesRead
  ) {
    return { state: Phase010FirstRunWorkspaceState.RunningFirstRunTests };
  }
  if (
    currentState === Phase010FirstRunWorkspaceState.RunningFirstRunTests &&
    event === Phase010FirstRunWorkspaceEvent.FirstRunTestsPassed
  ) {
    return { state: Phase010FirstRunWorkspaceState.RunningHealthTests };
  }
  if (
    currentState === Phase010FirstRunWorkspaceState.RunningHealthTests &&
    event === Phase010FirstRunWorkspaceEvent.HealthTestsPassed
  ) {
    return { state: Phase010FirstRunWorkspaceState.RunningRepairTests };
  }
  if (
    currentState === Phase010FirstRunWorkspaceState.RunningRepairTests &&
    event === Phase010FirstRunWorkspaceEvent.RepairTestsPassed
  ) {
    return { state: Phase010FirstRunWorkspaceState.WritingResult };
  }
  if (
    currentState === Phase010FirstRunWorkspaceState.WritingResult &&
    event === Phase010FirstRunWorkspaceEvent.ResultWritten
  ) {
    return { state: Phase010FirstRunWorkspaceState.Passed };
  }
  if (
    [
      Phase010FirstRunWorkspaceState.ReadingPrerequisites,
      Phase010FirstRunWorkspaceState.RunningFirstRunTests,
      Phase010FirstRunWorkspaceState.RunningHealthTests,
      Phase010FirstRunWorkspaceState.RunningRepairTests,
      Phase010FirstRunWorkspaceState.WritingResult,
    ].includes(currentState) &&
    event === Phase010FirstRunWorkspaceEvent.Fail
  ) {
    return {
      state: Phase010FirstRunWorkspaceState.Failed,
      errorCode: detail.errorCode ?? Phase010FirstRunWorkspaceErrorCode.IoFailed,
      findingId: detail.findingId,
      failedStepId: detail.failedStepId,
    };
  }
  return {
    state: Phase010FirstRunWorkspaceState.Failed,
    errorCode: Phase010FirstRunWorkspaceErrorCode.InvalidTransition,
  };
}

export function buildPhase010FirstRunWorkspaceCommandPlan() {
  return [
    step("firstRunCore", ["cargo", "test", "-p", "cabinet-core", "--test", "first_run_tests"]),
    step("firstRunInitializer", [
      "cargo",
      "test",
      "-p",
      "cabinet-core",
      "--test",
      "first_run_initializer_tests",
    ]),
    step("firstRunStore", [
      "cargo",
      "test",
      "-p",
      "cabinet-adapters",
      "--test",
      "local_first_run_store_tests",
    ]),
    step("setupHealth", [
      "cargo",
      "test",
      "-p",
      "cabinet-adapters",
      "--test",
      "local_setup_health_checker_tests",
    ]),
    step("nativeBootstrap", [
      "cargo",
      "test",
      "-p",
      "cabinet-platform",
      "--test",
      "local_desktop_bootstrap_state_tests",
    ]),
    step("startupRepair", [
      "cargo",
      "test",
      "-p",
      "cabinet-platform",
      "--test",
      "startup_repair_smoke",
    ]),
  ];
}

export function evaluatePhase010FirstRunWorkspaceGate({ packagedLaunchText, commandResults }) {
  let state = transitionPhase010FirstRunWorkspaceState(
    Phase010FirstRunWorkspaceState.Pending,
    Phase010FirstRunWorkspaceEvent.Start,
  );

  if (!packagedLaunchText.includes("phase010_packaged_launch_gate=passed")) {
    state = transitionPhase010FirstRunWorkspaceState(
      state.state,
      Phase010FirstRunWorkspaceEvent.Fail,
      {
        errorCode: Phase010FirstRunWorkspaceErrorCode.PackagedLaunchMarkerMissing,
        findingId: ".tasks/phase010-packaged-launch-gate-result.md",
      },
    );
    return failedResult(state, commandResults);
  }

  state = transitionPhase010FirstRunWorkspaceState(
    state.state,
    Phase010FirstRunWorkspaceEvent.PrerequisitesRead,
  );

  for (const stepId of ["firstRunCore", "firstRunInitializer", "firstRunStore"]) {
    const result = commandResults[stepId];
    if (!result?.passed) {
      state = transitionPhase010FirstRunWorkspaceState(
        state.state,
        Phase010FirstRunWorkspaceEvent.Fail,
        {
          errorCode: Phase010FirstRunWorkspaceErrorCode.FirstRunTestsFailed,
          findingId: stepId,
          failedStepId: stepId,
        },
      );
      return failedResult(state, commandResults);
    }
  }
  state = transitionPhase010FirstRunWorkspaceState(
    state.state,
    Phase010FirstRunWorkspaceEvent.FirstRunTestsPassed,
  );

  const health = commandResults.setupHealth;
  if (!health?.passed) {
    state = transitionPhase010FirstRunWorkspaceState(
      state.state,
      Phase010FirstRunWorkspaceEvent.Fail,
      {
        errorCode: Phase010FirstRunWorkspaceErrorCode.HealthTestsFailed,
        findingId: "setupHealth",
        failedStepId: "setupHealth",
      },
    );
    return failedResult(state, commandResults);
  }
  state = transitionPhase010FirstRunWorkspaceState(
    state.state,
    Phase010FirstRunWorkspaceEvent.HealthTestsPassed,
  );

  for (const stepId of ["nativeBootstrap", "startupRepair"]) {
    const result = commandResults[stepId];
    if (!result?.passed) {
      const errorCode =
        stepId === "nativeBootstrap"
          ? Phase010FirstRunWorkspaceErrorCode.BootstrapTestsFailed
          : Phase010FirstRunWorkspaceErrorCode.RepairTestsFailed;
      state = transitionPhase010FirstRunWorkspaceState(
        state.state,
        Phase010FirstRunWorkspaceEvent.Fail,
        {
          errorCode,
          findingId: stepId,
          failedStepId: stepId,
        },
      );
      return failedResult(state, commandResults);
    }
  }

  state = transitionPhase010FirstRunWorkspaceState(
    state.state,
    Phase010FirstRunWorkspaceEvent.RepairTestsPassed,
  );
  state = transitionPhase010FirstRunWorkspaceState(
    state.state,
    Phase010FirstRunWorkspaceEvent.ResultWritten,
  );

  return {
    passed: true,
    state: state.state,
    commandCount: Object.keys(commandResults).length,
    commandResults,
  };
}

export async function runPhase010FirstRunWorkspaceGate({
  root = process.cwd(),
  writeArtifact = true,
  runner = runCommandStep,
  steps = buildPhase010FirstRunWorkspaceCommandPlan(),
} = {}) {
  const packagedLaunchPath = ".tasks/phase010-packaged-launch-gate-result.md";
  let packagedLaunchText;
  try {
    packagedLaunchText = await readFile(join(root, packagedLaunchPath), "utf8");
  } catch (error) {
    return {
      passed: false,
      state: Phase010FirstRunWorkspaceState.Failed,
      errorCode: Phase010FirstRunWorkspaceErrorCode.IoFailed,
      findingId: error.path ?? packagedLaunchPath,
      commandResults: {},
    };
  }

  if (!packagedLaunchText.includes("phase010_packaged_launch_gate=passed")) {
    return evaluatePhase010FirstRunWorkspaceGate({
      packagedLaunchText,
      commandResults: {},
    });
  }

  const commandResults = {};
  for (const gateStep of steps) {
    const started = Date.now();
    const execution = await runner(gateStep, { root });
    commandResults[gateStep.id] = {
      command: gateStep.command.join(" "),
      passed: execution.exitCode === 0 && !execution.signal,
      exitCode: execution.exitCode,
      signal: execution.signal ?? null,
      durationMs: execution.durationMs ?? Date.now() - started,
    };
    const partial = evaluatePhase010FirstRunWorkspaceGate({
      packagedLaunchText,
      commandResults,
    });
    if (!partial.passed && commandResults[gateStep.id].passed === false) {
      if (writeArtifact) {
        await writeArtifactFile(root, partial);
      }
      return partial;
    }
  }

  const result = evaluatePhase010FirstRunWorkspaceGate({
    packagedLaunchText,
    commandResults,
  });

  if (writeArtifact) {
    await writeArtifactFile(root, result);
  }

  return result;
}

export function renderPhase010FirstRunWorkspaceArtifact(result) {
  const passed = result.passed === true;
  const summaryLines = passed
    ? ["first_run_idempotent=true", "setup_health_status=healthy", "repair_evidence=verified"]
    : [
        "first_run_idempotent=not_verified",
        `setup_health_status=${result.failedStepId === "setupHealth" ? "failed" : "not_verified"}`,
        `repair_evidence=${["nativeBootstrap", "startupRepair"].includes(result.failedStepId) ? "failed" : "not_verified"}`,
      ];
  const lines = [
    "# Phase 010 First-Run Workspace Gate Result",
    "",
    passed
      ? "phase010_first_run_workspace_gate=passed"
      : "phase010_first_run_workspace_gate=failed",
    `validation_state=${result.state}`,
    ...summaryLines,
    "",
    "- phase: `Phase 010.2`",
    "- gate: `First-Run Workspace, App Data, Migration, and Health Gate`",
    `- status: \`${passed ? "passed" : "failed"}\``,
    "- prerequisites:",
    "  - `.tasks/phase010-packaged-launch-gate-result.md` with `phase010_packaged_launch_gate=passed`",
    "- validation commands:",
    ...buildPhase010FirstRunWorkspaceCommandPlan().map(
      (gateStep) => `  - \`${gateStep.command.join(" ")}\``,
    ),
    "- changed layers: `release-tooling`, `task-tooling`; product runtime code is verified but not changed by this gate.",
    "- p95 300ms path impact: none. This gate validates startup, health, and repair behavior.",
    "- Product Log candidates: `workspace.first_run.started`, `workspace.first_run.completed`, `workspace.first_run.failed`, `workspace.health.failed`, `workspace.repair.completed`.",
    "- Field Debug candidates: setup role, setup status, stable error code, retryable, masked workspace id.",
    "- setup health evidence: clean profile healthy, missing required directory detected, file-instead-of-directory detected, missing first-run marker detected.",
    "- first-run evidence: clean profile creates metadata, version-store, asset, search-index, and workspace directories; second run is idempotent.",
    "- repair evidence: startup repair smoke rebuilds corrupted index without losing current workspace data.",
    "- sensitive-data exclusion: this artifact records command ids, counts, states, and stable error codes only. It excludes raw command stdout, document body, asset content, AI prompt, AI answer, authentication material, and personal absolute paths.",
    "",
    "## Command Results",
    "",
    "| command | status | exit code | duration ms |",
    "| --- | --- | ---: | ---: |",
    ...Object.entries(result.commandResults ?? {}).map(
      ([id, entry]) =>
        `| \`${id}\` | ${entry.passed ? "passed" : "failed"} | ${entry.exitCode ?? "null"} | ${entry.durationMs ?? 0} |`,
    ),
  ];

  if (!passed) {
    lines.push("");
    lines.push(`- error_code: \`${result.errorCode}\``);
    lines.push(`- finding_id: \`${result.findingId ?? "unknown"}\``);
    if (result.failedStepId) {
      lines.push(`- failed_step: \`${result.failedStepId}\``);
    }
  }

  lines.push("");
  return lines.join("\n");
}

async function writeArtifactFile(root, result) {
  await mkdir(join(root, ".tasks"), { recursive: true });
  await writeFile(
    join(root, ".tasks", "phase010-first-run-workspace-gate-result.md"),
    renderPhase010FirstRunWorkspaceArtifact(result),
  );
}

function failedResult(state, commandResults) {
  return {
    passed: false,
    state: state.state,
    errorCode: state.errorCode,
    findingId: state.findingId,
    failedStepId: state.failedStepId,
    commandResults,
  };
}

function step(id, command) {
  return { id, command };
}

function runCommandStep(gateStep, { root }) {
  return new Promise((resolve, reject) => {
    const started = Date.now();
    const [command, ...args] = gateStep.command;
    const child = spawn(command, args, { cwd: root, stdio: "inherit" });
    child.on("error", reject);
    child.on("exit", (exitCode, signal) =>
      resolve({ exitCode, signal, durationMs: Date.now() - started }),
    );
  });
}

async function main() {
  const result = await runPhase010FirstRunWorkspaceGate({
    root: process.cwd(),
    writeArtifact: true,
  });
  if (!result.passed) {
    console.error(`${result.errorCode} ${result.findingId ?? ""}`.trim());
    process.exit(1);
  }
  console.log("phase010_first_run_workspace_gate=passed");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    console.error(error);
    process.exit(1);
  });
}
