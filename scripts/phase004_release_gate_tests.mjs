import assert from "node:assert/strict";
import test from "node:test";

import {
  Phase004ReleaseGateErrorCode,
  Phase004ReleaseGateEvent,
  Phase004ReleaseGateState,
  buildPhase004ReleaseGateCommandPlan,
  renderPhase004ReleaseGateMarkdown,
  runPhase004ReleaseGateWithRunner,
  transitionPhase004ReleaseGateState,
} from "./phase004_release_gate.mjs";

test("phase004 release gate command plan includes validators product smoke runbooks and security", () => {
  const plan = buildPhase004ReleaseGateCommandPlan();

  assert.deepEqual(plan.map((step) => step.id), [
    "phase004_plan_validator_tests",
    "phase004_plan_validator",
    "phase004_product_smoke_gate_tests",
    "phase004_product_smoke_gate",
    "runbook_validator_tests",
    "runbook_validation",
    "security_log_scanner_tests",
    "security_log_scan",
  ]);
  assert.deepEqual(plan.at(-1).command, ["npm", "run", "run:security-log-scanner"]);
});

test("phase004 release gate reports complete when all steps pass", async () => {
  const result = await runPhase004ReleaseGateWithRunner(async () => ({
    exitCode: 0,
    signal: null,
    durationMs: 6,
  }));

  assert.equal(result.status, "passed");
  assert.equal(result.phase, "Phase 004");
  assert.equal(result.releaseConclusion, "knowledge graph and realtime collaboration UX expansion complete");
  assert.equal(result.failedStep, null);
  assert.equal(result.steps.length, 8);
});

test("phase004 release gate short-circuits after failed command", async () => {
  const executed = [];
  const result = await runPhase004ReleaseGateWithRunner(async (step) => {
    executed.push(step.id);
    if (step.id === "runbook_validation") {
      return { exitCode: 1, signal: null, durationMs: 9 };
    }
    return { exitCode: 0, signal: null, durationMs: 4 };
  });

  assert.equal(result.status, "failed");
  assert.equal(result.failedStep.id, "runbook_validation");
  assert.equal(result.failureCategory, "command_exit_nonzero");
  assert.deepEqual(executed, [
    "phase004_plan_validator_tests",
    "phase004_plan_validator",
    "phase004_product_smoke_gate_tests",
    "phase004_product_smoke_gate",
    "runbook_validator_tests",
    "runbook_validation",
  ]);
});

test("phase004 release gate state machine rejects invalid transitions", () => {
  assert.equal(
    transitionPhase004ReleaseGateState(
      Phase004ReleaseGateState.Pending,
      Phase004ReleaseGateEvent.Start,
    ).state,
    Phase004ReleaseGateState.Running,
  );
  assert.equal(
    transitionPhase004ReleaseGateState(
      Phase004ReleaseGateState.Running,
      Phase004ReleaseGateEvent.StepExit,
      { stepId: "x", status: "passed" },
    ).state,
    Phase004ReleaseGateState.StepPassed,
  );
  assert.throws(
    () =>
      transitionPhase004ReleaseGateState(
        Phase004ReleaseGateState.Pending,
        Phase004ReleaseGateEvent.WriteReport,
      ),
    (error) => error.code === Phase004ReleaseGateErrorCode.InvalidTransition,
  );
});

test("phase004 release gate markdown records final release evidence", async () => {
  const result = await runPhase004ReleaseGateWithRunner(async () => ({
    exitCode: 0,
    signal: null,
    durationMs: 3,
  }));
  const markdown = renderPhase004ReleaseGateMarkdown(result);

  assert.match(markdown, /Phase 004 Final Release Gate Result/);
  assert.match(markdown, /phase004_release_gate=passed/);
  assert.match(markdown, /knowledge graph and realtime collaboration UX expansion complete/);
  assert.match(markdown, /phase004_product_smoke_gate/);
  assert.match(markdown, /runbook_validation/);
  assert.match(markdown, /security_log_scan/);
});
