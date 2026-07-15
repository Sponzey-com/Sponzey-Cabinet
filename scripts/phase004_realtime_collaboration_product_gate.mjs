import { spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const RealtimeCollaborationProductGateState = Object.freeze({
  Pending: "Pending",
  Running: "Running",
  StepPassed: "StepPassed",
  StepFailed: "StepFailed",
  Reported: "Reported",
});

export const RealtimeCollaborationProductGateEvent = Object.freeze({
  Start: "Start",
  StepStart: "StepStart",
  StepExit: "StepExit",
  WriteReport: "WriteReport",
});

export const RealtimeCollaborationProductGateErrorCode = Object.freeze({
  InvalidTransition: "PHASE004_REALTIME_COLLABORATION_PRODUCT_GATE_INVALID_TRANSITION",
});

class RealtimeCollaborationProductGateError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "RealtimeCollaborationProductGateError";
    this.code = code;
  }
}

export function transitionRealtimeCollaborationProductGateState(state, event, detail = {}) {
  if (state === RealtimeCollaborationProductGateState.Pending && event === RealtimeCollaborationProductGateEvent.Start) {
    return RealtimeCollaborationProductGateState.Running;
  }
  if (
    [RealtimeCollaborationProductGateState.Running, RealtimeCollaborationProductGateState.StepPassed].includes(state) &&
    event === RealtimeCollaborationProductGateEvent.StepStart
  ) {
    return RealtimeCollaborationProductGateState.Running;
  }
  if (state === RealtimeCollaborationProductGateState.Running && event === RealtimeCollaborationProductGateEvent.StepExit) {
    return detail.status === "passed"
      ? RealtimeCollaborationProductGateState.StepPassed
      : RealtimeCollaborationProductGateState.StepFailed;
  }
  if (
    [RealtimeCollaborationProductGateState.StepPassed, RealtimeCollaborationProductGateState.StepFailed].includes(state) &&
    event === RealtimeCollaborationProductGateEvent.WriteReport
  ) {
    return RealtimeCollaborationProductGateState.Reported;
  }
  throw new RealtimeCollaborationProductGateError(
    RealtimeCollaborationProductGateErrorCode.InvalidTransition,
    `invalid realtime collaboration product gate transition: ${state}:${event}`,
  );
}

export function buildRealtimeCollaborationProductGateCommandPlan() {
  return [
    step("collaboration_coverage_audit_tests", [
      "npm",
      "run",
      "run:phase004-collaboration-coverage-audit-tests",
    ]),
    step("collaboration_coverage_audit", [
      "npm",
      "run",
      "run:phase004-collaboration-coverage-audit",
    ]),
    step("realtime_collaboration_smoke_tests", [
      "npm",
      "run",
      "run:phase004-realtime-collaboration-smoke-tests",
    ]),
    step("realtime_collaboration_smoke", [
      "npm",
      "run",
      "run:phase004-realtime-collaboration-smoke",
    ]),
    step("security_log_scanner_tests", ["npm", "run", "run:security-log-scanner-tests"]),
    step("security_log_scan", ["npm", "run", "run:security-log-scanner"]),
  ];
}

export async function runRealtimeCollaborationProductGateWithRunner(
  runner,
  {
    startedAt = new Date(),
    completedAtProvider = () => new Date(),
    steps = buildRealtimeCollaborationProductGateCommandPlan(),
  } = {},
) {
  let state = transitionRealtimeCollaborationProductGateState(
    RealtimeCollaborationProductGateState.Pending,
    RealtimeCollaborationProductGateEvent.Start,
  );
  const results = [];
  let failedStep = null;
  let failureCategory = "none";

  for (const gateStep of steps) {
    state = transitionRealtimeCollaborationProductGateState(
      state,
      RealtimeCollaborationProductGateEvent.StepStart,
      { stepId: gateStep.id },
    );
    const started = Date.now();
    const execution = await runner(gateStep);
    const status = execution.exitCode === 0 && !execution.signal ? "passed" : "failed";
    const result = {
      ...gateStep,
      status,
      exitCode: execution.exitCode,
      signal: execution.signal,
      durationMs: execution.durationMs ?? Date.now() - started,
    };
    results.push(result);
    state = transitionRealtimeCollaborationProductGateState(
      state,
      RealtimeCollaborationProductGateEvent.StepExit,
      { stepId: gateStep.id, status },
    );
    if (status === "passed") {
      continue;
    }
    failedStep = gateStep;
    failureCategory = execution.signal ? "command_signal" : "command_exit_nonzero";
    break;
  }

  return {
    phase: "Phase 004.4",
    gate: "Realtime Collaboration Product Gate",
    status: failedStep ? "failed" : "passed",
    state,
    startedAt: startedAt.toISOString(),
    completedAt: completedAtProvider().toISOString(),
    steps: results,
    failedStep,
    failureCategory,
  };
}

export function renderRealtimeCollaborationProductGateMarkdown(result) {
  const marker =
    result.status === "passed"
      ? "phase004_realtime_collaboration_product_gate=passed"
      : "phase004_realtime_collaboration_product_gate=failed";
  const lines = [
    "# Phase 004 Realtime Collaboration Product Gate Result",
    "",
    marker,
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
    "- This gate covers Phase 004.4 collaboration coverage audit, realtime collaboration smoke, and security log scan.",
    "- It calls lower-level commands and does not duplicate collaboration domain, runtime, client, editor, or scanner logic.",
    "- Actual network listener smoke, Web/desktop collaborative editing UI smoke, Canvas, mobile collaboration, and final Phase 004 release gates remain separate work.",
  ];
  return `${lines.join("\n")}\n`;
}

async function runCli() {
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const resultPath = path.join(repoRoot, ".tasks/realtime-collaboration-product-gate-result.md");
  const result = await runRealtimeCollaborationProductGateWithRunner(
    (gateStep) => spawnCommand(gateStep.command[0], gateStep.command.slice(1), repoRoot),
    { startedAt: new Date() },
  );
  await mkdir(path.dirname(resultPath), { recursive: true });
  await writeFile(resultPath, renderRealtimeCollaborationProductGateMarkdown(result));

  if (result.status === "passed") {
    console.log("phase004_realtime_collaboration_product_gate=passed");
    console.log(`step_count=${result.steps.length}`);
    console.log(`result_path=${resultPath}`);
    return;
  }

  console.error("phase004_realtime_collaboration_product_gate=failed");
  console.error(`failed_step=${result.failedStep?.id ?? "none"}`);
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
