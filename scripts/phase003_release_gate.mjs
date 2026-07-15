import { spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const Phase003ReleaseGateState = Object.freeze({
  Pending: "Pending",
  Running: "Running",
  StepPassed: "StepPassed",
  StepFailed: "StepFailed",
  Reported: "Reported",
  Failed: "Failed",
});

export const Phase003ReleaseGateEvent = Object.freeze({
  Start: "Start",
  StepStart: "StepStart",
  StepExit: "StepExit",
  WriteReport: "WriteReport",
  Fail: "Fail",
});

export const Phase003ReleaseGateErrorCode = Object.freeze({
  InvalidTransition: "PHASE003_RELEASE_GATE_INVALID_TRANSITION",
});

class Phase003ReleaseGateError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "Phase003ReleaseGateError";
    this.code = code;
  }
}

export function transitionPhase003ReleaseGateState(state, event, detail = {}) {
  if (state === Phase003ReleaseGateState.Pending && event === Phase003ReleaseGateEvent.Start) {
    return { state: Phase003ReleaseGateState.Running };
  }
  if (
    [Phase003ReleaseGateState.Running, Phase003ReleaseGateState.StepPassed].includes(state) &&
    event === Phase003ReleaseGateEvent.StepStart
  ) {
    return { state: Phase003ReleaseGateState.Running, currentStepId: detail.stepId };
  }
  if (state === Phase003ReleaseGateState.Running && event === Phase003ReleaseGateEvent.StepExit) {
    if (detail.status === "passed") {
      return { state: Phase003ReleaseGateState.StepPassed, currentStepId: detail.stepId };
    }
    return {
      state: Phase003ReleaseGateState.StepFailed,
      failedStepId: detail.stepId,
      failureCategory: detail.failureCategory ?? "command_failed",
    };
  }
  if (
    [Phase003ReleaseGateState.StepPassed, Phase003ReleaseGateState.StepFailed].includes(state) &&
    event === Phase003ReleaseGateEvent.WriteReport
  ) {
    return { state: Phase003ReleaseGateState.Reported };
  }
  if (event === Phase003ReleaseGateEvent.Fail) {
    return {
      state: Phase003ReleaseGateState.Failed,
      failedStepId: detail.stepId,
      failureCategory: detail.failureCategory ?? "phase003_release_gate_failed",
    };
  }
  throw new Phase003ReleaseGateError(
    Phase003ReleaseGateErrorCode.InvalidTransition,
    `invalid phase003 release gate transition: ${state}:${event}`,
  );
}

export function buildPhase003ReleaseGateCommandPlan() {
  return [
    step("runtime_persistence_gate_tests", ["npm", "run", "run:phase003-gate-tests"]),
    step("runtime_persistence_gate", ["npm", "run", "run:phase003-gate"]),
    step("product_smoke_gate_tests", [
      "npm",
      "run",
      "run:phase003-product-smoke-gate-tests",
    ]),
    step("product_smoke_gate", ["npm", "run", "run:phase003-product-smoke-gate"]),
    step("packaging_gate_tests", ["npm", "run", "run:phase003-packaging-gate-tests"]),
    step("packaging_gate", ["npm", "run", "run:phase003-packaging-gate"]),
    step("security_log_scanner_tests", ["npm", "run", "run:security-log-scanner-tests"]),
    step("security_log_scan", ["npm", "run", "run:security-log-scanner"]),
    step("runbook_validator_tests", ["npm", "run", "run:runbook-validator-tests"]),
    step("runbook_validation", ["npm", "run", "run:runbook-validator"]),
    step("hardening_coverage_audit_tests", [
      "npm",
      "run",
      "run:phase003-hardening-coverage-audit-tests",
    ]),
    step("hardening_coverage_audit", [
      "npm",
      "run",
      "run:phase003-hardening-coverage-audit",
    ]),
  ];
}

export async function runPhase003ReleaseGateWithRunner(
  runner,
  {
    startedAt = new Date(),
    completedAtProvider = () => new Date(),
    steps = buildPhase003ReleaseGateCommandPlan(),
  } = {},
) {
  let state = transitionPhase003ReleaseGateState(
    Phase003ReleaseGateState.Pending,
    Phase003ReleaseGateEvent.Start,
  );
  const stepResults = [];
  let failedStep = null;
  let failureCategory = "none";

  for (const gateStep of steps) {
    state = transitionPhase003ReleaseGateState(
      state.state,
      Phase003ReleaseGateEvent.StepStart,
      { stepId: gateStep.id },
    );
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
    state = transitionPhase003ReleaseGateState(
      state.state,
      Phase003ReleaseGateEvent.StepExit,
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
    phase: "Phase 003",
    gate: "Final Release Gate",
    status: failedStep ? "failed" : "passed",
    state: state.state,
    releaseConclusion: failedStep
      ? "production hardening incomplete"
      : "production hardening complete",
    startedAt: startedAt.toISOString(),
    completedAt: completedAtProvider().toISOString(),
    steps: stepResults,
    failedStep,
    failureCategory,
  };
}

export function renderPhase003ReleaseGateMarkdown(result) {
  const releaseMarker =
    result.status === "passed" ? "phase003_release_gate=passed" : "phase003_release_gate=failed";
  const lines = [
    "# Phase 003 Final Release Gate Result",
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
    "- production hardening complete",
    "- security_log_scan",
    "- runbook_validation",
    "- product_smoke_gate",
    "- packaging_gate",
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
    "- This gate proves Phase 003 runtime persistence, product smoke, packaging, security scan, runbook validation, and hardening coverage are release-reviewed together.",
    "- SaaS multi-tenant, realtime collaboration engine, AI generation, plugin marketplace, signed installers, notarization, and container publishing remain outside Phase 003 final release scope.",
  ];
  return `${lines.join("\n")}\n`;
}

async function runCli() {
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const resultPath = path.join(repoRoot, ".tasks/phase003/final-release-gate-result.md");
  const result = await runPhase003ReleaseGateWithRunner((gateStep) =>
    spawnCommand(gateStep.command[0], gateStep.command.slice(1), repoRoot),
  );
  await mkdir(path.dirname(resultPath), { recursive: true });
  await writeFile(resultPath, renderPhase003ReleaseGateMarkdown(result), "utf8");

  if (result.status === "passed") {
    console.log("phase003_release_gate=passed");
    console.log(`release_conclusion=${result.releaseConclusion}`);
    console.log(`command_count=${result.steps.length}`);
    console.log(`result_path=${resultPath}`);
    return;
  }

  console.error("phase003_release_gate=failed");
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
