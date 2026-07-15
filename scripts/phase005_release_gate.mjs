import { spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const Phase005ReleaseGateState = Object.freeze({
  Pending: "Pending",
  Running: "Running",
  StepPassed: "StepPassed",
  StepFailed: "StepFailed",
  Reported: "Reported",
  Failed: "Failed",
});

export const Phase005ReleaseGateEvent = Object.freeze({
  Start: "Start",
  StepStart: "StepStart",
  StepExit: "StepExit",
  WriteReport: "WriteReport",
  Fail: "Fail",
});

export const Phase005ReleaseGateErrorCode = Object.freeze({
  InvalidTransition: "PHASE005_RELEASE_GATE_INVALID_TRANSITION",
});

class Phase005ReleaseGateError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "Phase005ReleaseGateError";
    this.code = code;
  }
}

export function transitionPhase005ReleaseGateState(state, event, detail = {}) {
  if (state === Phase005ReleaseGateState.Pending && event === Phase005ReleaseGateEvent.Start) {
    return { state: Phase005ReleaseGateState.Running };
  }
  if (
    [Phase005ReleaseGateState.Running, Phase005ReleaseGateState.StepPassed].includes(state) &&
    event === Phase005ReleaseGateEvent.StepStart
  ) {
    return { state: Phase005ReleaseGateState.Running, currentStepId: detail.stepId };
  }
  if (state === Phase005ReleaseGateState.Running && event === Phase005ReleaseGateEvent.StepExit) {
    if (detail.status === "passed") {
      return { state: Phase005ReleaseGateState.StepPassed, currentStepId: detail.stepId };
    }
    return {
      state: Phase005ReleaseGateState.StepFailed,
      failedStepId: detail.stepId,
      failureCategory: detail.failureCategory ?? "command_failed",
    };
  }
  if (
    [Phase005ReleaseGateState.StepPassed, Phase005ReleaseGateState.StepFailed].includes(state) &&
    event === Phase005ReleaseGateEvent.WriteReport
  ) {
    return { state: Phase005ReleaseGateState.Reported };
  }
  if (event === Phase005ReleaseGateEvent.Fail) {
    return {
      state: Phase005ReleaseGateState.Failed,
      failedStepId: detail.stepId,
      failureCategory: detail.failureCategory ?? "phase005_release_gate_failed",
    };
  }
  throw new Phase005ReleaseGateError(
    Phase005ReleaseGateErrorCode.InvalidTransition,
    `${Phase005ReleaseGateErrorCode.InvalidTransition}: invalid phase005 release gate transition: ${state}:${event}`,
  );
}

export function buildPhase005ReleaseGateCommandPlan() {
  return [
    step("phase005_plan_validator_tests", ["npm", "run", "run:phase005-plan-validator-tests"]),
    step("phase005_plan_validator", ["npm", "run", "run:phase005-plan-validator"]),
    step("phase005_retrieval_coverage_audit_tests", [
      "npm",
      "run",
      "run:phase005-retrieval-coverage-audit-tests",
    ]),
    step("phase005_retrieval_coverage_audit", [
      "npm",
      "run",
      "run:phase005-retrieval-coverage-audit",
    ]),
    step("phase005_semantic_search_gate_tests", [
      "npm",
      "run",
      "run:phase005-semantic-search-gate-tests",
    ]),
    step("phase005_semantic_search_gate", [
      "npm",
      "run",
      "run:phase005-semantic-search-gate",
    ]),
    step("phase005_ai_answer_product_gate_tests", [
      "npm",
      "run",
      "run:phase005-ai-answer-product-gate-tests",
    ]),
    step("phase005_ai_answer_product_gate", [
      "npm",
      "run",
      "run:phase005-ai-answer-product-gate",
    ]),
    step("phase005_mcp_api_product_gate_tests", [
      "npm",
      "run",
      "run:phase005-mcp-api-product-gate-tests",
    ]),
    step("phase005_mcp_api_product_gate", [
      "npm",
      "run",
      "run:phase005-mcp-api-product-gate",
    ]),
    step("phase005_webhook_connector_product_gate_tests", [
      "npm",
      "run",
      "run:phase005-webhook-connector-product-gate-tests",
    ]),
    step("phase005_webhook_connector_product_gate", [
      "npm",
      "run",
      "run:phase005-webhook-connector-product-gate",
    ]),
    step("phase005_product_smoke_gate_tests", [
      "npm",
      "run",
      "run:phase005-product-smoke-gate-tests",
    ]),
    step("phase005_product_smoke_gate", [
      "npm",
      "run",
      "run:phase005-product-smoke-gate",
    ]),
    step("phase005_observability_matrix_gate_tests", [
      "npm",
      "run",
      "run:phase005-observability-matrix-gate-tests",
    ]),
    step("phase005_observability_matrix_gate", [
      "npm",
      "run",
      "run:phase005-observability-matrix-gate",
    ]),
    step("runbook_validator_tests", ["npm", "run", "run:runbook-validator-tests"]),
    step("runbook_validation", ["npm", "run", "run:runbook-validator"]),
    step("security_log_scanner_tests", ["npm", "run", "run:security-log-scanner-tests"]),
    step("security_log_scan", ["npm", "run", "run:security-log-scanner"]),
  ];
}

export async function runPhase005ReleaseGateWithRunner(
  runner,
  {
    startedAt = new Date(),
    completedAtProvider = () => new Date(),
    steps = buildPhase005ReleaseGateCommandPlan(),
  } = {},
) {
  let state = transitionPhase005ReleaseGateState(
    Phase005ReleaseGateState.Pending,
    Phase005ReleaseGateEvent.Start,
  );
  const stepResults = [];
  let failedStep = null;
  let failureCategory = "none";

  for (const gateStep of steps) {
    state = transitionPhase005ReleaseGateState(
      state.state,
      Phase005ReleaseGateEvent.StepStart,
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
    state = transitionPhase005ReleaseGateState(
      state.state,
      Phase005ReleaseGateEvent.StepExit,
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
    phase: "Phase 005",
    gate: "Final Release Gate",
    status: failedStep ? "failed" : "passed",
    state: state.state,
    releaseConclusion: failedStep
      ? "AI and external integration platform incomplete"
      : "AI and external integration platform complete",
    startedAt: startedAt.toISOString(),
    completedAt: completedAtProvider().toISOString(),
    steps: stepResults,
    failedStep,
    failureCategory,
  };
}

export function renderPhase005ReleaseGateMarkdown(result) {
  const releaseMarker =
    result.status === "passed" ? "phase005_release_gate=passed" : "phase005_release_gate=failed";
  const lines = [
    "# Phase 005 Final Release Gate Result",
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
    "- AI and external integration platform complete",
    "- phase005_plan_validation",
    "- phase005_retrieval_coverage_audit",
    "- phase005_semantic_search_gate",
    "- phase005_ai_answer_product_gate",
    "- phase005_mcp_api_product_gate",
    "- phase005_webhook_connector_product_gate",
    "- phase005_product_smoke_gate",
    "- phase005_observability_matrix_gate",
    "- runbook_validation",
    "- security_log_scan",
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
    "- This gate proves Phase 005 planning, retrieval, semantic search, AI answer, MCP/API, webhook/connector, cross-platform AI product smoke, observability, runbook validation, and security scanning are release-reviewed together.",
    "- The default gate uses deterministic local/fake providers and does not call external AI provider, MCP client, webhook endpoint, Slack, Teams, or Jira services.",
    "- The report records step ids, markers, status, and durations only; it does not record prompt, answer, source text, provider key, connector token, or payload values.",
  ];
  return `${lines.join("\n")}\n`;
}

export function renderPhase005ReleaseGatePlaceholder() {
  return [
    "# Phase 005 Final Release Gate Result",
    "",
    "phase005_release_gate=running",
    "",
    "- phase: `Phase 005`",
    "- gate: `Final Release Gate`",
    "- status: `running`",
    "- release conclusion: `final gate running`",
    "",
    "## Review Notes",
    "",
    "- Placeholder exists so security scan can include the final artifact path during the release gate run.",
    "- This file is overwritten with the final passed or failed result before the command exits.",
  ].join("\n") + "\n";
}

async function runCli() {
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const resultPath = path.join(repoRoot, ".tasks/phase005-release-gate-result.md");
  await mkdir(path.dirname(resultPath), { recursive: true });
  await writeFile(resultPath, renderPhase005ReleaseGatePlaceholder(), "utf8");

  const result = await runPhase005ReleaseGateWithRunner((gateStep) =>
    spawnCommand(gateStep.command[0], gateStep.command.slice(1), repoRoot),
  );
  const reported = transitionPhase005ReleaseGateState(
    result.state,
    Phase005ReleaseGateEvent.WriteReport,
  );
  const finalResult = { ...result, state: reported.state };
  await writeFile(resultPath, renderPhase005ReleaseGateMarkdown(finalResult), "utf8");

  if (finalResult.status === "passed") {
    console.log("phase005_release_gate=passed");
    console.log(`release_conclusion=${finalResult.releaseConclusion}`);
    console.log(`command_count=${finalResult.steps.length}`);
    console.log(`result_path=${resultPath}`);
    return;
  }

  console.error("phase005_release_gate=failed");
  console.error(`failed_step=${finalResult.failedStep?.id ?? "unknown"}`);
  console.error(`failure_category=${finalResult.failureCategory}`);
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
