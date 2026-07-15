import assert from "node:assert/strict";
import test from "node:test";

import {
  GraphProductGateErrorCode,
  GraphProductGateEvent,
  GraphProductGateState,
  buildGraphProductGateCommandPlan,
  renderGraphProductGateMarkdown,
  runGraphProductGateWithRunner,
  transitionGraphProductGateState,
} from "./phase004_graph_product_gate.mjs";

test("graph product gate command plan includes graph coverage performance smoke and security scan", () => {
  const plan = buildGraphProductGateCommandPlan();

  assert.deepEqual(plan.map((step) => step.id), [
    "graph_coverage_audit_tests",
    "graph_coverage_audit",
    "permission_aware_graph_benchmark",
    "self_host_e2e_graph_smoke",
    "desktop_remote_graph_smoke",
    "security_log_scanner_tests",
    "security_log_scan",
  ]);
  assert.deepEqual(plan.at(-1).command, ["npm", "run", "run:security-log-scanner"]);
});

test("graph product gate reports passed when every lower-level command passes", async () => {
  const result = await runGraphProductGateWithRunner(async () => ({
    exitCode: 0,
    signal: null,
    durationMs: 5,
  }));

  assert.equal(result.status, "passed");
  assert.equal(result.phase, "Phase 004.2");
  assert.equal(result.failedStep, null);
  assert.equal(result.steps.length, 7);
});

test("graph product gate short-circuits after first failing command", async () => {
  const executed = [];
  const result = await runGraphProductGateWithRunner(async (step) => {
    executed.push(step.id);
    if (step.id === "self_host_e2e_graph_smoke") {
      return { exitCode: 1, signal: null, durationMs: 9 };
    }
    return { exitCode: 0, signal: null, durationMs: 4 };
  });

  assert.equal(result.status, "failed");
  assert.equal(result.failedStep.id, "self_host_e2e_graph_smoke");
  assert.equal(result.failureCategory, "command_exit_nonzero");
  assert.deepEqual(executed, [
    "graph_coverage_audit_tests",
    "graph_coverage_audit",
    "permission_aware_graph_benchmark",
    "self_host_e2e_graph_smoke",
  ]);
});

test("graph product gate state machine rejects invalid transitions", () => {
  assert.equal(
    transitionGraphProductGateState(
      GraphProductGateState.Pending,
      GraphProductGateEvent.Start,
    ),
    GraphProductGateState.Running,
  );
  assert.equal(
    transitionGraphProductGateState(
      GraphProductGateState.Running,
      GraphProductGateEvent.StepExit,
      { status: "passed" },
    ),
    GraphProductGateState.StepPassed,
  );
  assert.throws(
    () =>
      transitionGraphProductGateState(
        GraphProductGateState.Pending,
        GraphProductGateEvent.StepExit,
      ),
    (error) => error.code === GraphProductGateErrorCode.InvalidTransition,
  );
});

test("graph product gate markdown records final graph evidence", async () => {
  const result = await runGraphProductGateWithRunner(async () => ({
    exitCode: 0,
    signal: null,
    durationMs: 3,
  }));
  const markdown = renderGraphProductGateMarkdown(result);

  assert.match(markdown, /Phase 004 Graph Product Gate Result/);
  assert.match(markdown, /phase004_graph_product_gate=passed/);
  assert.match(markdown, /graph_coverage_audit/);
  assert.match(markdown, /permission_aware_graph_benchmark/);
  assert.match(markdown, /security_log_scan/);
});
