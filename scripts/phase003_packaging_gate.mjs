import { spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const PackagingGateState = Object.freeze({
  Pending: "Pending",
  Running: "Running",
  StepPassed: "StepPassed",
  StepFailed: "StepFailed",
  Reported: "Reported",
  Failed: "Failed",
});

export const PackagingGateEvent = Object.freeze({
  Start: "Start",
  StepStart: "StepStart",
  StepExit: "StepExit",
  WriteReport: "WriteReport",
  Fail: "Fail",
});

export const PackagingGateErrorCode = Object.freeze({
  InvalidTransition: "PHASE003_PACKAGING_GATE_INVALID_TRANSITION",
});

class PackagingGateError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "PackagingGateError";
    this.code = code;
  }
}

export function transitionPackagingGateState(state, event, detail = {}) {
  if (state === PackagingGateState.Pending && event === PackagingGateEvent.Start) {
    return { state: PackagingGateState.Running };
  }
  if (
    [PackagingGateState.Running, PackagingGateState.StepPassed].includes(state) &&
    event === PackagingGateEvent.StepStart
  ) {
    return { state: PackagingGateState.Running, currentStepId: detail.stepId };
  }
  if (state === PackagingGateState.Running && event === PackagingGateEvent.StepExit) {
    if (detail.status === "passed") {
      return { state: PackagingGateState.StepPassed, currentStepId: detail.stepId };
    }
    return {
      state: PackagingGateState.StepFailed,
      failedStepId: detail.stepId,
      failureCategory: detail.failureCategory ?? "command_failed",
    };
  }
  if (
    [PackagingGateState.StepPassed, PackagingGateState.StepFailed].includes(state) &&
    event === PackagingGateEvent.WriteReport
  ) {
    return { state: PackagingGateState.Reported };
  }
  if (event === PackagingGateEvent.Fail) {
    return {
      state: PackagingGateState.Failed,
      failedStepId: detail.stepId,
      failureCategory: detail.failureCategory ?? "packaging_gate_failed",
    };
  }
  throw new PackagingGateError(
    PackagingGateErrorCode.InvalidTransition,
    `invalid packaging gate transition: ${state}:${event}`,
  );
}

export function buildPackagingGateCommandPlan() {
  return [
    step("packaging_coverage_audit_tests", [
      "npm",
      "run",
      "run:phase003-packaging-coverage-audit-tests",
    ]),
    step("packaging_coverage_audit", [
      "npm",
      "run",
      "run:phase003-packaging-coverage-audit",
    ]),
    step("self_host_server_package_smoke_tests", [
      "npm",
      "run",
      "run:self-host-server-package-smoke-tests",
    ]),
    step("self_host_server_package_smoke", [
      "npm",
      "run",
      "run:self-host-server-package-smoke",
    ]),
    step("self_host_upgrade_smoke_tests", [
      "npm",
      "run",
      "run:self-host-upgrade-smoke-tests",
    ]),
    step("self_host_upgrade_smoke", ["npm", "run", "run:self-host-upgrade-smoke"]),
    step("browser_smoke", ["npm", "run", "run:browser-smoke"]),
    step("desktop_package_smoke", ["npm", "run", "run:desktop-package-smoke"]),
    step("desktop_packaged_app_smoke", [
      "npm",
      "run",
      "run:desktop-packaged-app-smoke",
    ]),
    step("desktop_dist_browser_smoke", [
      "npm",
      "run",
      "run:desktop-dist-browser-smoke",
    ]),
  ];
}

export async function runPackagingGateWithRunner(
  runner,
  {
    startedAt = new Date(),
    completedAtProvider = () => new Date(),
    steps = buildPackagingGateCommandPlan(),
  } = {},
) {
  let state = transitionPackagingGateState(
    PackagingGateState.Pending,
    PackagingGateEvent.Start,
  );
  const stepResults = [];
  let failedStep = null;
  let failureCategory = "none";

  for (const gateStep of steps) {
    state = transitionPackagingGateState(state.state, PackagingGateEvent.StepStart, {
      stepId: gateStep.id,
    });
    const started = Date.now();
    const execution = await runner(gateStep);
    const status =
      execution.exitCode === 0 && !execution.signal ? "passed" : "failed";
    const stepResult = {
      ...gateStep,
      status,
      exitCode: execution.exitCode,
      signal: execution.signal,
      durationMs: execution.durationMs ?? Date.now() - started,
    };
    stepResults.push(stepResult);
    state = transitionPackagingGateState(state.state, PackagingGateEvent.StepExit, {
      stepId: gateStep.id,
      status,
      failureCategory: execution.signal ? "command_signal" : "command_exit_nonzero",
    });

    if (status === "passed") {
      continue;
    }

    failedStep = gateStep;
    failureCategory = execution.signal ? "command_signal" : "command_exit_nonzero";
    break;
  }

  return {
    phase: "Phase 003.4",
    gate: "Packaging Gate",
    status: failedStep ? "failed" : "passed",
    state: state.state,
    startedAt: startedAt.toISOString(),
    completedAt: completedAtProvider().toISOString(),
    steps: stepResults,
    failedStep,
    failureCategory,
  };
}

export function renderPackagingGateMarkdown(result) {
  const lines = [
    "# Phase 003 Packaging Gate Result",
    "",
    `- phase: \`${result.phase}\``,
    `- gate: \`${result.gate}\``,
    `- status: \`${result.status}\``,
    `- state: \`${result.state}\``,
    `- failed step: \`${result.failedStep?.id ?? "none"}\``,
    `- failure category: \`${result.failureCategory}\``,
    `- started at: \`${result.startedAt}\``,
    `- completed at: \`${result.completedAt}\``,
    "",
    "## Steps",
    "",
    "| Step | Status | Exit Code | Signal | Duration ms |",
    "| --- | --- | ---: | --- | ---: |",
    ...result.steps.map(
      (entry) =>
        `| \`${entry.id}\` | ${entry.status} | ${entry.exitCode ?? "null"} | ${entry.signal ?? "none"} | ${entry.durationMs} |`,
    ),
    "",
    "## Review Notes",
    "",
    "- This gate covers Phase 003.4 packaging coverage, self-host server package smoke, upgrade migration smoke, Web browser smoke, desktop package smoke, packaged app smoke, and desktop dist browser smoke.",
    "- Signed installers, container images, notarization, security scanner, runbook validator, and final release gate remain separate Phase 003 hardening work.",
  ];
  return `${lines.join("\n")}\n`;
}

async function runCli() {
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const resultPath = path.join(repoRoot, ".tasks/phase003/packaging-gate-result.md");
  const result = await runPackagingGateWithRunner((gateStep) =>
    spawnCommand(gateStep.command[0], gateStep.command.slice(1), repoRoot),
  );
  await mkdir(path.dirname(resultPath), { recursive: true });
  await writeFile(resultPath, renderPackagingGateMarkdown(result), "utf8");

  if (result.status === "passed") {
    console.log("phase003_packaging_gate=passed");
    console.log(`command_count=${result.steps.length}`);
    console.log(`result_path=${resultPath}`);
    return;
  }

  console.error("phase003_packaging_gate=failed");
  console.error(`failed_step=${result.failedStep?.id ?? "unknown"}`);
  console.error(`failure_category=${result.failureCategory}`);
  process.exitCode = 1;
}

function spawnCommand(command, args, cwd) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd, stdio: "inherit" });
    child.on("error", reject);
    child.on("exit", (exitCode, signal) => resolve({ exitCode, signal }));
  });
}

function step(id, command) {
  return { id, command };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await runCli();
}
