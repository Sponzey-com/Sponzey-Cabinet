import { spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const GraphProductGateState = Object.freeze({
  Pending: "Pending",
  Running: "Running",
  StepPassed: "StepPassed",
  StepFailed: "StepFailed",
  Reported: "Reported",
});

export const GraphProductGateEvent = Object.freeze({
  Start: "Start",
  StepStart: "StepStart",
  StepExit: "StepExit",
  WriteReport: "WriteReport",
});

export const GraphProductGateErrorCode = Object.freeze({
  InvalidTransition: "PHASE004_GRAPH_PRODUCT_GATE_INVALID_TRANSITION",
});

class GraphProductGateError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "GraphProductGateError";
    this.code = code;
  }
}

export function transitionGraphProductGateState(state, event, detail = {}) {
  if (state === GraphProductGateState.Pending && event === GraphProductGateEvent.Start) {
    return GraphProductGateState.Running;
  }
  if (
    [GraphProductGateState.Running, GraphProductGateState.StepPassed].includes(state) &&
    event === GraphProductGateEvent.StepStart
  ) {
    return GraphProductGateState.Running;
  }
  if (state === GraphProductGateState.Running && event === GraphProductGateEvent.StepExit) {
    return detail.status === "passed"
      ? GraphProductGateState.StepPassed
      : GraphProductGateState.StepFailed;
  }
  if (
    [GraphProductGateState.StepPassed, GraphProductGateState.StepFailed].includes(state) &&
    event === GraphProductGateEvent.WriteReport
  ) {
    return GraphProductGateState.Reported;
  }
  throw new GraphProductGateError(
    GraphProductGateErrorCode.InvalidTransition,
    `invalid graph product gate transition: ${state}:${event}`,
  );
}

export function buildGraphProductGateCommandPlan() {
  return [
    step("graph_coverage_audit_tests", [
      "npm",
      "run",
      "run:phase004-graph-coverage-audit-tests",
    ]),
    step("graph_coverage_audit", ["npm", "run", "run:phase004-graph-coverage-audit"]),
    step("permission_aware_graph_benchmark", [
      "cargo",
      "test",
      "-p",
      "cabinet-platform",
      "--test",
      "query_performance_benchmarks",
    ]),
    step("self_host_e2e_graph_smoke", ["npm", "run", "run:self-host-e2e-smoke"]),
    step("desktop_remote_graph_smoke", [
      "npm",
      "run",
      "run:desktop-remote-product-smoke",
    ]),
    step("security_log_scanner_tests", ["npm", "run", "run:security-log-scanner-tests"]),
    step("security_log_scan", ["npm", "run", "run:security-log-scanner"]),
  ];
}

export async function runGraphProductGateWithRunner(
  runner,
  {
    startedAt = new Date(),
    completedAtProvider = () => new Date(),
    steps = buildGraphProductGateCommandPlan(),
  } = {},
) {
  let state = transitionGraphProductGateState(
    GraphProductGateState.Pending,
    GraphProductGateEvent.Start,
  );
  const results = [];
  let failedStep = null;
  let failureCategory = "none";

  for (const gateStep of steps) {
    state = transitionGraphProductGateState(state, GraphProductGateEvent.StepStart, {
      stepId: gateStep.id,
    });
    const started = Date.now();
    const execution = await runner(gateStep);
    const status =
      execution.exitCode === 0 && !execution.signal ? "passed" : "failed";
    const result = {
      ...gateStep,
      status,
      exitCode: execution.exitCode,
      signal: execution.signal,
      durationMs: execution.durationMs ?? Date.now() - started,
    };
    results.push(result);
    state = transitionGraphProductGateState(state, GraphProductGateEvent.StepExit, {
      stepId: gateStep.id,
      status,
    });
    if (status === "passed") {
      continue;
    }
    failedStep = gateStep;
    failureCategory = execution.signal ? "command_signal" : "command_exit_nonzero";
    break;
  }

  return {
    phase: "Phase 004.2",
    gate: "Graph Product Gate",
    status: failedStep ? "failed" : "passed",
    state,
    startedAt: startedAt.toISOString(),
    completedAt: completedAtProvider().toISOString(),
    steps: results,
    failedStep,
    failureCategory,
  };
}

export function renderGraphProductGateMarkdown(result) {
  const marker =
    result.status === "passed"
      ? "phase004_graph_product_gate=passed"
      : "phase004_graph_product_gate=failed";
  const lines = [
    "# Phase 004 Graph Product Gate Result",
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
    "- This gate covers Phase 004.2 graph coverage audit, permission-aware graph performance, self-host graph smoke, desktop remote graph smoke, and security log scan.",
    "- It calls lower-level commands and does not duplicate graph usecase, runtime, client, or scanner logic.",
    "- Collaboration, Canvas, mobile collaboration, and final Phase 004 release gates remain separate work.",
  ];
  return `${lines.join("\n")}\n`;
}

async function runCli() {
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const resultPath = path.join(repoRoot, ".tasks/graph-product-gate-result.md");
  const result = await runGraphProductGateWithRunner(
    (gateStep) => spawnCommand(gateStep.command[0], gateStep.command.slice(1), repoRoot),
    { startedAt: new Date() },
  );
  await mkdir(path.dirname(resultPath), { recursive: true });
  await writeFile(resultPath, renderGraphProductGateMarkdown(result));

  if (result.status === "passed") {
    console.log("phase004_graph_product_gate=passed");
    console.log(`step_count=${result.steps.length}`);
    console.log(`result_path=${resultPath}`);
    return;
  }

  console.error("phase004_graph_product_gate=failed");
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
