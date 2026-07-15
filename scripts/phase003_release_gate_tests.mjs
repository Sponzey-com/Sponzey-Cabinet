import assert from "node:assert/strict";
import test from "node:test";

import {
  Phase003ReleaseGateErrorCode,
  Phase003ReleaseGateEvent,
  Phase003ReleaseGateState,
  buildPhase003ReleaseGateCommandPlan,
  renderPhase003ReleaseGateMarkdown,
  runPhase003ReleaseGateWithRunner,
  transitionPhase003ReleaseGateState,
} from "./phase003_release_gate.mjs";

test("phase003 release gate command plan includes lower gates and hardening checks", () => {
  const plan = buildPhase003ReleaseGateCommandPlan();

  assert.deepEqual(plan.map((step) => step.id), [
    "runtime_persistence_gate_tests",
    "runtime_persistence_gate",
    "product_smoke_gate_tests",
    "product_smoke_gate",
    "packaging_gate_tests",
    "packaging_gate",
    "security_log_scanner_tests",
    "security_log_scan",
    "runbook_validator_tests",
    "runbook_validation",
    "hardening_coverage_audit_tests",
    "hardening_coverage_audit",
  ]);
});

test("phase003 release gate reports production hardening complete when all steps pass", async () => {
  const result = await runPhase003ReleaseGateWithRunner(async () => ({
    exitCode: 0,
    signal: null,
    durationMs: 6,
  }));

  assert.equal(result.status, "passed");
  assert.equal(result.releaseConclusion, "production hardening complete");
  assert.equal(result.failedStep, null);
  assert.equal(result.steps.length, 12);
});

test("phase003 release gate short-circuits after failed command", async () => {
  const executed = [];
  const result = await runPhase003ReleaseGateWithRunner(async (step) => {
    executed.push(step.id);
    if (step.id === "packaging_gate") {
      return { exitCode: 1, signal: null, durationMs: 9 };
    }
    return { exitCode: 0, signal: null, durationMs: 4 };
  });

  assert.equal(result.status, "failed");
  assert.equal(result.failedStep.id, "packaging_gate");
  assert.equal(executed.includes("security_log_scanner"), false);
});

test("phase003 release gate state machine rejects invalid transitions", () => {
  assert.throws(
    () =>
      transitionPhase003ReleaseGateState(
        Phase003ReleaseGateState.Pending,
        Phase003ReleaseGateEvent.WriteReport,
      ),
    (error) => error.code === Phase003ReleaseGateErrorCode.InvalidTransition,
  );
});

test("phase003 release gate markdown records final hardening evidence", async () => {
  const result = await runPhase003ReleaseGateWithRunner(async () => ({
    exitCode: 0,
    signal: null,
    durationMs: 3,
  }));
  const markdown = renderPhase003ReleaseGateMarkdown(result);

  assert.match(markdown, /Phase 003 Final Release Gate Result/);
  assert.match(markdown, /phase003_release_gate=passed/);
  assert.match(markdown, /production hardening complete/);
  assert.match(markdown, /security_log_scan/);
  assert.match(markdown, /runbook_validation/);
  assert.match(markdown, /product_smoke_gate/);
  assert.match(markdown, /packaging_gate/);
});
