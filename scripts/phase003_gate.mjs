import { spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const Phase003GateState = Object.freeze({
  Pending: "Pending",
  Running: "Running",
  StepPassed: "StepPassed",
  StepFailed: "StepFailed",
  Reported: "Reported",
  Failed: "Failed",
});

export const Phase003GateEvent = Object.freeze({
  Start: "Start",
  StepStart: "StepStart",
  StepExit: "StepExit",
  WriteReport: "WriteReport",
  Fail: "Fail",
});

export const Phase003GateErrorCode = Object.freeze({
  InvalidTransition: "PHASE003_GATE_INVALID_TRANSITION",
});

class Phase003GateError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "Phase003GateError";
    this.code = code;
  }
}

export function transitionPhase003GateState(state, event, detail = {}) {
  if (state === Phase003GateState.Pending && event === Phase003GateEvent.Start) {
    return { state: Phase003GateState.Running };
  }
  if (
    [Phase003GateState.Running, Phase003GateState.StepPassed].includes(state) &&
    event === Phase003GateEvent.StepStart
  ) {
    return { state: Phase003GateState.Running, currentStepId: detail.stepId };
  }
  if (state === Phase003GateState.Running && event === Phase003GateEvent.StepExit) {
    if (detail.status === "passed") {
      return { state: Phase003GateState.StepPassed, currentStepId: detail.stepId };
    }
    return {
      state: Phase003GateState.StepFailed,
      failedStepId: detail.stepId,
      failureCategory: detail.failureCategory ?? "command_failed",
    };
  }
  if (
    [Phase003GateState.StepPassed, Phase003GateState.StepFailed].includes(state) &&
    event === Phase003GateEvent.WriteReport
  ) {
    return { state: Phase003GateState.Reported };
  }
  if (event === Phase003GateEvent.Fail) {
    return {
      state: Phase003GateState.Failed,
      failedStepId: detail.stepId,
      failureCategory: detail.failureCategory ?? "phase003_gate_failed",
    };
  }
  throw new Phase003GateError(
    Phase003GateErrorCode.InvalidTransition,
    `invalid Phase 003 gate transition: ${state}:${event}`,
  );
}

export function planPhase003GateCommands() {
  return [
    step("runtime_wiring_audit_tests", [
      "npm",
      "run",
      "run:phase003-runtime-wiring-audit-tests",
    ]),
    step("runtime_wiring_audit", [
      "npm",
      "run",
      "run:phase003-runtime-wiring-audit",
    ]),
    step("persistence_gap_audit_tests", [
      "npm",
      "run",
      "run:phase003-persistence-gap-audit-tests",
    ]),
    step("persistence_gap_audit", [
      "npm",
      "run",
      "run:phase003-persistence-gap-audit",
    ]),
    step("durable_dependency_manifest_audit_tests", [
      "npm",
      "run",
      "run:phase003-durable-dependency-manifest-audit-tests",
    ]),
    step("durable_dependency_manifest_audit", [
      "npm",
      "run",
      "run:phase003-durable-dependency-manifest-audit",
    ]),
    step("recovery_coverage_audit_tests", [
      "npm",
      "run",
      "run:phase003-recovery-coverage-audit-tests",
    ]),
    step("recovery_coverage_audit", [
      "npm",
      "run",
      "run:phase003-recovery-coverage-audit",
    ]),
  ];
}

export async function runPhase003Gate({
  commandRunner,
  startedAt = new Date(),
  completedAtProvider = () => new Date(),
  root = process.cwd(),
  steps = planPhase003GateCommands(),
}) {
  let state = transitionPhase003GateState(
    Phase003GateState.Pending,
    Phase003GateEvent.Start,
  );
  const commandResults = [];
  let failedStepId = null;
  let failureCategory = "none";

  for (const gateStep of steps) {
    state = transitionPhase003GateState(state.state, Phase003GateEvent.StepStart, {
      stepId: gateStep.id,
    });
    const started = Date.now();
    const execution = await commandRunner(
      gateStep.command[0],
      gateStep.command.slice(1),
      root,
    );
    const status =
      execution.exitCode === 0 && !execution.signal ? "passed" : "failed";
    const commandResult = {
      id: gateStep.id,
      command: gateStep.command,
      status,
      exitCode: execution.exitCode,
      signal: execution.signal,
      durationMs: Date.now() - started,
    };
    commandResults.push(commandResult);
    state = transitionPhase003GateState(state.state, Phase003GateEvent.StepExit, {
      stepId: gateStep.id,
      status,
      failureCategory: execution.signal ? "command_signal" : "command_exit_nonzero",
    });

    if (status === "passed") {
      continue;
    }

    failedStepId = gateStep.id;
    failureCategory = execution.signal ? "command_signal" : "command_exit_nonzero";
    break;
  }

  return {
    phase: "Phase 003",
    gate: "Runtime Persistence Gate",
    status: failedStepId ? "failed" : "passed",
    state: state.state,
    startedAt: startedAt.toISOString(),
    completedAt: completedAtProvider().toISOString(),
    commandResults,
    failedStepId,
    failureCategory,
  };
}

export function renderPhase003GateMarkdown(result) {
  const lines = [
    "# Phase 003 Runtime Persistence Gate Result",
    "",
    `- phase: \`${result.phase}\``,
    `- gate: \`${result.gate}\``,
    `- status: \`${result.status}\``,
    `- state: \`${result.state}\``,
    `- failed step: \`${result.failedStepId ?? "none"}\``,
    `- failure category: \`${result.failureCategory}\``,
    `- started at: \`${result.startedAt}\``,
    `- completed at: \`${result.completedAt}\``,
    "",
    "## Steps",
    "",
    "| Step | Status | Exit Code | Signal | Duration ms |",
    "| --- | --- | ---: | --- | ---: |",
    ...result.commandResults.map(
      (entry) =>
        `| \`${entry.id}\` | ${entry.status} | ${entry.exitCode ?? "null"} | ${entry.signal ?? "none"} | ${entry.durationMs} |`,
    ),
    "",
    "## Review Notes",
    "",
    "- This gate covers Phase 003 runtime wiring, persistence gap, durable dependency manifest, and recovery coverage audits.",
    "- Product smoke, packaging, install once, security scanner, and runbook gates remain separate Phase 003 hardening work.",
  ];
  return `${lines.join("\n")}\n`;
}

async function runCli() {
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const resultPath = path.join(repoRoot, ".tasks/phase003/phase003-gate-result.md");
  const result = await runPhase003Gate({
    root: repoRoot,
    commandRunner: spawnCommand,
    startedAt: new Date(),
  });
  await mkdir(path.dirname(resultPath), { recursive: true });
  await writeFile(resultPath, renderPhase003GateMarkdown(result));

  if (result.status === "passed") {
    console.log("phase003_gate=passed");
    console.log(`command_count=${result.commandResults.length}`);
    console.log(`result_path=${resultPath}`);
    return;
  }

  console.error("phase003_gate=failed");
  console.error(`failed_step_id=${result.failedStepId}`);
  console.error(`failure_category=${result.failureCategory}`);
  console.error(`result_path=${resultPath}`);
  process.exit(1);
}

function spawnCommand(command, args, cwd) {
  return new Promise((resolve) => {
    const child = spawn(command, args, { cwd, stdio: "inherit" });
    child.on("exit", (exitCode, signal) => {
      resolve({ exitCode, signal });
    });
  });
}

function step(id, command) {
  return Object.freeze({ id, command: Object.freeze(command) });
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await runCli();
}
