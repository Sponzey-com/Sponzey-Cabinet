import assert from "node:assert/strict";
import test from "node:test";

import {
  ProductSmokeGateErrorCode,
  ProductSmokeGateEvent,
  ProductSmokeGateState,
  planProductSmokeGateCommands,
  renderProductSmokeGateMarkdown,
  runProductSmokeGate,
  transitionProductSmokeGateState,
} from "./phase003_product_smoke_gate.mjs";

test("product smoke gate command plan includes coverage audit and all product smokes", () => {
  const steps = planProductSmokeGateCommands();

  assert.deepEqual(
    steps.map((step) => step.id),
    [
      "product_smoke_coverage_audit_tests",
      "product_smoke_coverage_audit",
      "self_host_e2e_smoke",
      "browser_smoke",
      "desktop_remote_product_smoke",
      "mobile_read_product_smoke",
    ],
  );
  assert.deepEqual(steps.at(-1).command, [
    "npm",
    "run",
    "run:mobile-read-product-smoke",
  ]);
});

test("product smoke gate returns passed result when every command exits successfully", async () => {
  const result = await runProductSmokeGate({
    commandRunner: async () => ({ exitCode: 0, signal: null }),
    startedAt: new Date("2026-01-01T00:00:00.000Z"),
    completedAtProvider: () => new Date("2026-01-01T00:00:01.000Z"),
  });

  assert.equal(result.status, "passed");
  assert.equal(result.commandResults.length, 6);
  assert.equal(result.failedStepId, null);
  assert.equal(result.failureCategory, "none");
});

test("product smoke gate stops after first failing command", async () => {
  const executed = [];
  const result = await runProductSmokeGate({
    commandRunner: async (_command, args) => {
      const stepName = args.at(-1);
      executed.push(stepName);
      return stepName === "run:browser-smoke"
        ? { exitCode: 1, signal: null }
        : { exitCode: 0, signal: null };
    },
    startedAt: new Date("2026-01-01T00:00:00.000Z"),
    completedAtProvider: () => new Date("2026-01-01T00:00:01.000Z"),
  });

  assert.equal(result.status, "failed");
  assert.equal(result.failedStepId, "browser_smoke");
  assert.equal(result.failureCategory, "command_exit_nonzero");
  assert.deepEqual(executed, [
    "run:phase003-product-smoke-coverage-audit-tests",
    "run:phase003-product-smoke-coverage-audit",
    "run:self-host-e2e-smoke",
    "run:browser-smoke",
  ]);
});

test("product smoke gate state machine rejects invalid transitions", () => {
  assert.deepEqual(
    transitionProductSmokeGateState(
      ProductSmokeGateState.Pending,
      ProductSmokeGateEvent.Start,
    ),
    { state: ProductSmokeGateState.Running },
  );
  assert.deepEqual(
    transitionProductSmokeGateState(
      ProductSmokeGateState.Running,
      ProductSmokeGateEvent.StepExit,
      {
        stepId: "self_host_e2e_smoke",
        status: "passed",
      },
    ),
    { state: ProductSmokeGateState.StepPassed, currentStepId: "self_host_e2e_smoke" },
  );
  assert.throws(
    () =>
      transitionProductSmokeGateState(
        ProductSmokeGateState.Pending,
        ProductSmokeGateEvent.StepExit,
      ),
    (error) => error.code === ProductSmokeGateErrorCode.InvalidTransition,
  );
});

test("product smoke gate markdown records every step and final status", async () => {
  const result = await runProductSmokeGate({
    commandRunner: async () => ({ exitCode: 0, signal: null }),
    startedAt: new Date("2026-01-01T00:00:00.000Z"),
    completedAtProvider: () => new Date("2026-01-01T00:00:01.000Z"),
  });
  const markdown = renderProductSmokeGateMarkdown(result);

  assert.match(markdown, /# Phase 003 Product Smoke Gate Result/);
  assert.match(markdown, /status: `passed`/);
  assert.match(markdown, /browser_smoke/);
  assert.match(markdown, /mobile_read_product_smoke/);
});
