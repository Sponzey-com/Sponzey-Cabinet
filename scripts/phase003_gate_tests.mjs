import assert from "node:assert/strict";
import test from "node:test";

import {
  Phase003GateErrorCode,
  Phase003GateEvent,
  Phase003GateState,
  planPhase003GateCommands,
  renderPhase003GateMarkdown,
  runPhase003Gate,
  transitionPhase003GateState,
} from "./phase003_gate.mjs";

test("phase003 gate command plan includes runtime persistence and durable dependency audits", () => {
  const steps = planPhase003GateCommands();

  assert.deepEqual(
    steps.map((step) => step.id),
    [
      "runtime_wiring_audit_tests",
      "runtime_wiring_audit",
      "persistence_gap_audit_tests",
      "persistence_gap_audit",
      "durable_dependency_manifest_audit_tests",
      "durable_dependency_manifest_audit",
      "recovery_coverage_audit_tests",
      "recovery_coverage_audit",
    ],
  );
  assert.deepEqual(steps.at(-1).command, [
    "npm",
    "run",
    "run:phase003-recovery-coverage-audit",
  ]);
});

test("phase003 gate returns passed result when every command exits successfully", async () => {
  const result = await runPhase003Gate({
    commandRunner: async () => ({ exitCode: 0, signal: null }),
    startedAt: new Date("2026-01-01T00:00:00.000Z"),
    completedAtProvider: () => new Date("2026-01-01T00:00:01.000Z"),
  });

  assert.equal(result.status, "passed");
  assert.equal(result.commandResults.length, 8);
  assert.equal(result.failedStepId, null);
  assert.equal(result.failureCategory, "none");
});

test("phase003 gate stops after first failing command", async () => {
  const executed = [];
  const result = await runPhase003Gate({
    commandRunner: async (_command, args) => {
      const stepName = args.at(-1);
      executed.push(stepName);
      return stepName === "run:phase003-persistence-gap-audit"
        ? { exitCode: 1, signal: null }
        : { exitCode: 0, signal: null };
    },
    startedAt: new Date("2026-01-01T00:00:00.000Z"),
    completedAtProvider: () => new Date("2026-01-01T00:00:01.000Z"),
  });

  assert.equal(result.status, "failed");
  assert.equal(result.failedStepId, "persistence_gap_audit");
  assert.equal(result.failureCategory, "command_exit_nonzero");
  assert.deepEqual(executed, [
    "run:phase003-runtime-wiring-audit-tests",
    "run:phase003-runtime-wiring-audit",
    "run:phase003-persistence-gap-audit-tests",
    "run:phase003-persistence-gap-audit",
  ]);
});

test("phase003 gate state machine rejects invalid transitions", () => {
  assert.deepEqual(
    transitionPhase003GateState(Phase003GateState.Pending, Phase003GateEvent.Start),
    { state: Phase003GateState.Running },
  );
  assert.deepEqual(
    transitionPhase003GateState(Phase003GateState.Running, Phase003GateEvent.StepExit, {
      stepId: "runtime_wiring_audit",
      status: "passed",
    }),
    { state: Phase003GateState.StepPassed, currentStepId: "runtime_wiring_audit" },
  );
  assert.throws(
    () => transitionPhase003GateState(Phase003GateState.Pending, Phase003GateEvent.StepExit),
    (error) => error.code === Phase003GateErrorCode.InvalidTransition,
  );
});

test("phase003 gate markdown records every step and final status", async () => {
  const result = await runPhase003Gate({
    commandRunner: async () => ({ exitCode: 0, signal: null }),
    startedAt: new Date("2026-01-01T00:00:00.000Z"),
    completedAtProvider: () => new Date("2026-01-01T00:00:01.000Z"),
  });
  const markdown = renderPhase003GateMarkdown(result);

  assert.match(markdown, /# Phase 003 Runtime Persistence Gate Result/);
  assert.match(markdown, /status: `passed`/);
  assert.match(markdown, /runtime_wiring_audit/);
  assert.match(markdown, /durable_dependency_manifest_audit/);
  assert.match(markdown, /recovery_coverage_audit/);
});
