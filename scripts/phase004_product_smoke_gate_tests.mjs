import assert from "node:assert/strict";
import test from "node:test";

import {
  Phase004ProductSmokeGateErrorCode,
  Phase004ProductSmokeGateEvent,
  Phase004ProductSmokeGateState,
  buildPhase004ProductSmokeGateCommandPlan,
  renderPhase004ProductSmokeGateMarkdown,
  runPhase004ProductSmokeGateWithRunner,
  transitionPhase004ProductSmokeGateState,
} from "./phase004_product_smoke_gate.mjs";

test("phase004 product smoke gate command plan includes graph realtime canvas mobile and security", () => {
  const plan = buildPhase004ProductSmokeGateCommandPlan();

  assert.deepEqual(plan.map((step) => step.id), [
    "graph_product_gate_tests",
    "graph_product_gate",
    "realtime_collaboration_product_gate_tests",
    "realtime_collaboration_product_gate",
    "canvas_coverage_audit_tests",
    "canvas_coverage_audit",
    "canvas_product_smoke",
    "mobile_capability_audit_tests",
    "mobile_product_smoke",
    "mobile_capability_audit",
    "security_log_scanner_tests",
    "security_log_scan",
  ]);
  assert.deepEqual(plan.at(-1).command, ["npm", "run", "run:security-log-scanner"]);
});

test("phase004 product smoke gate reports passed when every lower-level command passes", async () => {
  const result = await runPhase004ProductSmokeGateWithRunner(async () => ({
    exitCode: 0,
    signal: null,
    durationMs: 7,
  }));

  assert.equal(result.status, "passed");
  assert.equal(result.phase, "Phase 004.7");
  assert.equal(result.failedStep, null);
  assert.equal(result.steps.length, 12);
});

test("phase004 product smoke gate short-circuits after first failing command", async () => {
  const executed = [];
  const result = await runPhase004ProductSmokeGateWithRunner(async (step) => {
    executed.push(step.id);
    if (step.id === "canvas_product_smoke") {
      return { exitCode: 1, signal: null, durationMs: 11 };
    }
    return { exitCode: 0, signal: null, durationMs: 5 };
  });

  assert.equal(result.status, "failed");
  assert.equal(result.failedStep.id, "canvas_product_smoke");
  assert.equal(result.failureCategory, "command_exit_nonzero");
  assert.deepEqual(executed, [
    "graph_product_gate_tests",
    "graph_product_gate",
    "realtime_collaboration_product_gate_tests",
    "realtime_collaboration_product_gate",
    "canvas_coverage_audit_tests",
    "canvas_coverage_audit",
    "canvas_product_smoke",
  ]);
});

test("phase004 product smoke gate state machine rejects invalid transitions", () => {
  assert.equal(
    transitionPhase004ProductSmokeGateState(
      Phase004ProductSmokeGateState.Pending,
      Phase004ProductSmokeGateEvent.Start,
    ),
    Phase004ProductSmokeGateState.Running,
  );
  assert.equal(
    transitionPhase004ProductSmokeGateState(
      Phase004ProductSmokeGateState.Running,
      Phase004ProductSmokeGateEvent.StepExit,
      { status: "passed" },
    ),
    Phase004ProductSmokeGateState.StepPassed,
  );
  assert.throws(
    () =>
      transitionPhase004ProductSmokeGateState(
        Phase004ProductSmokeGateState.Pending,
        Phase004ProductSmokeGateEvent.StepExit,
      ),
    (error) => error.code === Phase004ProductSmokeGateErrorCode.InvalidTransition,
  );
});

test("phase004 product smoke gate markdown records final product smoke evidence", async () => {
  const result = await runPhase004ProductSmokeGateWithRunner(async () => ({
    exitCode: 0,
    signal: null,
    durationMs: 3,
  }));
  const markdown = renderPhase004ProductSmokeGateMarkdown(result);

  assert.match(markdown, /Phase 004 Product Smoke Gate Result/);
  assert.match(markdown, /phase004_product_smoke_gate=passed/);
  assert.match(markdown, /graph_product_gate/);
  assert.match(markdown, /realtime_collaboration_product_gate/);
  assert.match(markdown, /canvas_product_smoke/);
  assert.match(markdown, /mobile_product_smoke/);
  assert.match(markdown, /security_log_scan/);
});
