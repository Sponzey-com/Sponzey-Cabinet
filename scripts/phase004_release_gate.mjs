import { spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const Phase004ReleaseGateState = Object.freeze({
  Pending: "Pending",
  Running: "Running",
  StepPassed: "StepPassed",
  StepFailed: "StepFailed",
  Reported: "Reported",
  Failed: "Failed",
});

export const Phase004ReleaseGateEvent = Object.freeze({
  Start: "Start",
  StepStart: "StepStart",
  StepExit: "StepExit",
  WriteReport: "WriteReport",
  Fail: "Fail",
});

export const Phase004ReleaseGateErrorCode = Object.freeze({
  InvalidTransition: "PHASE004_RELEASE_GATE_INVALID_TRANSITION",
});

class Phase004ReleaseGateError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "Phase004ReleaseGateError";
    this.code = code;
  }
}

export function transitionPhase004ReleaseGateState(state, event, detail = {}) {
  if (state === Phase004ReleaseGateState.Pending && event === Phase004ReleaseGateEvent.Start) {
    return { state: Phase004ReleaseGateState.Running };
  }
  if (
    [Phase004ReleaseGateState.Running, Phase004ReleaseGateState.StepPassed].includes(state) &&
    event === Phase004ReleaseGateEvent.StepStart
  ) {
    return { state: Phase004ReleaseGateState.Running, currentStepId: detail.stepId };
  }
  if (state === Phase004ReleaseGateState.Running && event === Phase004ReleaseGateEvent.StepExit) {
    if (detail.status === "passed") {
      return { state: Phase004ReleaseGateState.StepPassed, currentStepId: detail.stepId };
    }
    return {
      state: Phase004ReleaseGateState.StepFailed,
      failedStepId: detail.stepId,
      failureCategory: detail.failureCategory ?? "command_failed",
    };
  }
  if (
    [Phase004ReleaseGateState.StepPassed, Phase004ReleaseGateState.StepFailed].includes(state) &&
    event === Phase004ReleaseGateEvent.WriteReport
  ) {
    return { state: Phase004ReleaseGateState.Reported };
  }
  if (event === Phase004ReleaseGateEvent.Fail) {
    return {
      state: Phase004ReleaseGateState.Failed,
      failedStepId: detail.stepId,
      failureCategory: detail.failureCategory ?? "phase004_release_gate_failed",
    };
  }
  throw new Phase004ReleaseGateError(
    Phase004ReleaseGateErrorCode.InvalidTransition,
    `invalid phase004 release gate transition: ${state}:${event}`,
  );
}

export function buildPhase004ReleaseGateCommandPlan() {
  return [
    step("phase004_plan_validator_tests", ["npm", "run", "run:phase004-plan-validator-tests"]),
    step("phase004_plan_validator", ["npm", "run", "run:phase004-plan-validator"]),
    step("phase004_product_smoke_gate_tests", [
      "npm",
      "run",
      "run:phase004-product-smoke-gate-tests",
    ]),
    step("phase004_product_smoke_gate", ["npm", "run", "run:phase004-product-smoke-gate"]),
    step("runbook_validator_tests", ["npm", "run", "run:runbook-validator-tests"]),
    step("runbook_validation", ["npm", "run", "run:runbook-validator"]),
    step("security_log_scanner_tests", ["npm", "run", "run:security-log-scanner-tests"]),
    step("security_log_scan", ["npm", "run", "run:security-log-scanner"]),
  ];
}

export async function runPhase004ReleaseGateWithRunner(
  runner,
  {
    startedAt = new Date(),
    completedAtProvider = () => new Date(),
    steps = buildPhase004ReleaseGateCommandPlan(),
  } = {},
) {
  let state = transitionPhase004ReleaseGateState(
    Phase004ReleaseGateState.Pending,
    Phase004ReleaseGateEvent.Start,
  );
  const stepResults = [];
  let failedStep = null;
  let failureCategory = "none";

  for (const gateStep of steps) {
    state = transitionPhase004ReleaseGateState(
      state.state,
      Phase004ReleaseGateEvent.StepStart,
      { stepId: gateStep.id },
    );
    const started = Date.now();
    const execution = await runner(gateStep);
    const status = execution.exitCode === 0 && !execution.signal ? "passed" : "failed";
    const stepResult = {
      ...gateStep,
      status,
      exitCode: execution.exitCode,
      signal: execution.signal,
      durationMs: execution.durationMs ?? Date.now() - started,
    };
    stepResults.push(stepResult);
    state = transitionPhase004ReleaseGateState(
      state.state,
      Phase004ReleaseGateEvent.StepExit,
      {
        stepId: gateStep.id,
        status,
        failureCategory: execution.signal ? "command_signal" : "command_exit_nonzero",
      },
    );
    if (status === "passed") {
      continue;
    }
    failedStep = gateStep;
    failureCategory = execution.signal ? "command_signal" : "command_exit_nonzero";
    break;
  }

  return {
    phase: "Phase 004",
    gate: "Final Release Gate",
    status: failedStep ? "failed" : "passed",
    state: state.state,
    releaseConclusion: failedStep
      ? "knowledge graph and realtime collaboration UX expansion incomplete"
      : "knowledge graph and realtime collaboration UX expansion complete",
    startedAt: startedAt.toISOString(),
    completedAt: completedAtProvider().toISOString(),
    steps: stepResults,
    failedStep,
    failureCategory,
  };
}

export function renderPhase004ReleaseGateMarkdown(result) {
  const releaseMarker =
    result.status === "passed" ? "phase004_release_gate=passed" : "phase004_release_gate=failed";
  const lines = [
    "# Phase 004 Final Release Gate Result",
    "",
    releaseMarker,
    "",
    `- phase: \`${result.phase}\``,
    `- gate: \`${result.gate}\``,
    `- status: \`${result.status}\``,
    `- state: \`${result.state}\``,
    `- release conclusion: \`${result.releaseConclusion}\``,
    `- failed step: \`${result.failedStep?.id ?? "none"}\``,
    `- failure category: \`${result.failureCategory}\``,
    `- started at: \`${result.startedAt}\``,
    `- completed at: \`${result.completedAt}\``,
    "",
    "## Evidence Markers",
    "",
    "- knowledge graph and realtime collaboration UX expansion complete",
    "- phase004_product_smoke_gate",
    "- runbook_validation",
    "- security_log_scan",
    "- phase004_plan_validator",
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
    "- This gate proves Phase 004 planning, product smoke, runbook validation, and security scanning are release-reviewed together.",
    "- AI answer generation, semantic search, MCP server, plugin marketplace, CRM custom object, and SaaS multi-tenant operation remain outside Phase 004 final release scope.",
  ];
  return `${lines.join("\n")}\n`;
}

async function runCli() {
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const resultPath = path.join(repoRoot, ".tasks/phase004-final-release-gate-result.md");
  const result = await runPhase004ReleaseGateWithRunner((gateStep) =>
    spawnCommand(gateStep.command[0], gateStep.command.slice(1), repoRoot),
  );
  await mkdir(path.dirname(resultPath), { recursive: true });
  await writeFile(resultPath, renderPhase004ReleaseGateMarkdown(result), "utf8");

  if (result.status === "passed") {
    console.log("phase004_release_gate=passed");
    console.log(`release_conclusion=${result.releaseConclusion}`);
    console.log(`command_count=${result.steps.length}`);
    console.log(`result_path=${resultPath}`);
    return;
  }

  console.error("phase004_release_gate=failed");
  console.error(`failed_step=${result.failedStep?.id ?? "unknown"}`);
  console.error(`failure_category=${result.failureCategory}`);
  console.error(`result_path=${resultPath}`);
  process.exitCode = 1;
}

function spawnCommand(command, args, cwd) {
  return new Promise((resolve, reject) => {
    const started = Date.now();
    const child = spawn(command, args, { cwd, stdio: "inherit" });
    child.on("error", reject);
    child.on("exit", (exitCode, signal) =>
      resolve({ exitCode, signal, durationMs: Date.now() - started }),
    );
  });
}

function step(id, command) {
  return { id, command };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await runCli();
}
