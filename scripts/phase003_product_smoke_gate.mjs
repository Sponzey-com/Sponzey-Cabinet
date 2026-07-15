import { spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const ProductSmokeGateState = Object.freeze({
  Pending: "Pending",
  Running: "Running",
  StepPassed: "StepPassed",
  StepFailed: "StepFailed",
  Reported: "Reported",
  Failed: "Failed",
});

export const ProductSmokeGateEvent = Object.freeze({
  Start: "Start",
  StepStart: "StepStart",
  StepExit: "StepExit",
  WriteReport: "WriteReport",
  Fail: "Fail",
});

export const ProductSmokeGateErrorCode = Object.freeze({
  InvalidTransition: "PHASE003_PRODUCT_SMOKE_GATE_INVALID_TRANSITION",
});

class ProductSmokeGateError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "ProductSmokeGateError";
    this.code = code;
  }
}

export function transitionProductSmokeGateState(state, event, detail = {}) {
  if (state === ProductSmokeGateState.Pending && event === ProductSmokeGateEvent.Start) {
    return { state: ProductSmokeGateState.Running };
  }
  if (
    [ProductSmokeGateState.Running, ProductSmokeGateState.StepPassed].includes(state) &&
    event === ProductSmokeGateEvent.StepStart
  ) {
    return { state: ProductSmokeGateState.Running, currentStepId: detail.stepId };
  }
  if (state === ProductSmokeGateState.Running && event === ProductSmokeGateEvent.StepExit) {
    if (detail.status === "passed") {
      return { state: ProductSmokeGateState.StepPassed, currentStepId: detail.stepId };
    }
    return {
      state: ProductSmokeGateState.StepFailed,
      failedStepId: detail.stepId,
      failureCategory: detail.failureCategory ?? "command_failed",
    };
  }
  if (
    [ProductSmokeGateState.StepPassed, ProductSmokeGateState.StepFailed].includes(state) &&
    event === ProductSmokeGateEvent.WriteReport
  ) {
    return { state: ProductSmokeGateState.Reported };
  }
  if (event === ProductSmokeGateEvent.Fail) {
    return {
      state: ProductSmokeGateState.Failed,
      failedStepId: detail.stepId,
      failureCategory: detail.failureCategory ?? "product_smoke_gate_failed",
    };
  }
  throw new ProductSmokeGateError(
    ProductSmokeGateErrorCode.InvalidTransition,
    `invalid product smoke gate transition: ${state}:${event}`,
  );
}

export function planProductSmokeGateCommands() {
  return [
    step("product_smoke_coverage_audit_tests", [
      "npm",
      "run",
      "run:phase003-product-smoke-coverage-audit-tests",
    ]),
    step("product_smoke_coverage_audit", [
      "npm",
      "run",
      "run:phase003-product-smoke-coverage-audit",
    ]),
    step("self_host_e2e_smoke", ["npm", "run", "run:self-host-e2e-smoke"]),
    step("browser_smoke", ["npm", "run", "run:browser-smoke"]),
    step("desktop_remote_product_smoke", [
      "npm",
      "run",
      "run:desktop-remote-product-smoke",
    ]),
    step("mobile_read_product_smoke", [
      "npm",
      "run",
      "run:mobile-read-product-smoke",
    ]),
  ];
}

export async function runProductSmokeGate({
  commandRunner,
  startedAt = new Date(),
  completedAtProvider = () => new Date(),
  root = process.cwd(),
  steps = planProductSmokeGateCommands(),
}) {
  let state = transitionProductSmokeGateState(
    ProductSmokeGateState.Pending,
    ProductSmokeGateEvent.Start,
  );
  const commandResults = [];
  let failedStepId = null;
  let failureCategory = "none";

  for (const gateStep of steps) {
    state = transitionProductSmokeGateState(
      state.state,
      ProductSmokeGateEvent.StepStart,
      { stepId: gateStep.id },
    );
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
    state = transitionProductSmokeGateState(
      state.state,
      ProductSmokeGateEvent.StepExit,
      {
        stepId: gateStep.id,
        status,
        failureCategory: execution.signal ? "command_signal" : "command_exit_nonzero",
      },
    );

    if (status === "passed") {
      continue;
    }

    failedStepId = gateStep.id;
    failureCategory = execution.signal ? "command_signal" : "command_exit_nonzero";
    break;
  }

  return {
    phase: "Phase 003.3",
    gate: "Product Smoke Gate",
    status: failedStepId ? "failed" : "passed",
    state: state.state,
    startedAt: startedAt.toISOString(),
    completedAt: completedAtProvider().toISOString(),
    commandResults,
    failedStepId,
    failureCategory,
  };
}

export function renderProductSmokeGateMarkdown(result) {
  const lines = [
    "# Phase 003 Product Smoke Gate Result",
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
    "- This gate covers Phase 003.3 product smoke coverage, self-host E2E, Web browser, desktop remote, and mobile read product smokes.",
    "- Packaging, install once, upgrade, security scanner, and runbook gates remain separate Phase 003 hardening work.",
  ];
  return `${lines.join("\n")}\n`;
}

async function runCli() {
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const resultPath = path.join(
    repoRoot,
    ".tasks/phase003/product-smoke-gate-result.md",
  );
  const result = await runProductSmokeGate({
    root: repoRoot,
    commandRunner: spawnCommand,
    startedAt: new Date(),
  });
  await mkdir(path.dirname(resultPath), { recursive: true });
  await writeFile(resultPath, renderProductSmokeGateMarkdown(result));

  if (result.status === "passed") {
    console.log("phase003_product_smoke_gate=passed");
    console.log(`command_count=${result.commandResults.length}`);
    console.log(`result_path=${resultPath}`);
    return;
  }

  console.error("phase003_product_smoke_gate=failed");
  console.error(`failed_step=${result.failedStepId}`);
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
