import assert from "node:assert/strict";
import test from "node:test";

import {
  RealtimeCollaborationProductGateErrorCode,
  RealtimeCollaborationProductGateEvent,
  RealtimeCollaborationProductGateState,
  buildRealtimeCollaborationProductGateCommandPlan,
  renderRealtimeCollaborationProductGateMarkdown,
  runRealtimeCollaborationProductGateWithRunner,
  transitionRealtimeCollaborationProductGateState,
} from "./phase004_realtime_collaboration_product_gate.mjs";

test("realtime collaboration product gate command plan includes coverage smoke and security scan", () => {
  const plan = buildRealtimeCollaborationProductGateCommandPlan();

  assert.deepEqual(plan.map((step) => step.id), [
    "collaboration_coverage_audit_tests",
    "collaboration_coverage_audit",
    "realtime_collaboration_smoke_tests",
    "realtime_collaboration_smoke",
    "security_log_scanner_tests",
    "security_log_scan",
  ]);
  assert.deepEqual(plan.at(-1).command, ["npm", "run", "run:security-log-scanner"]);
});

test("realtime collaboration product gate reports passed when every lower-level command passes", async () => {
  const result = await runRealtimeCollaborationProductGateWithRunner(async () => ({
    exitCode: 0,
    signal: null,
    durationMs: 5,
  }));

  assert.equal(result.status, "passed");
  assert.equal(result.phase, "Phase 004.4");
  assert.equal(result.failedStep, null);
  assert.equal(result.steps.length, 6);
});

test("realtime collaboration product gate short-circuits after first failing command", async () => {
  const executed = [];
  const result = await runRealtimeCollaborationProductGateWithRunner(async (step) => {
    executed.push(step.id);
    if (step.id === "realtime_collaboration_smoke") {
      return { exitCode: 1, signal: null, durationMs: 9 };
    }
    return { exitCode: 0, signal: null, durationMs: 4 };
  });

  assert.equal(result.status, "failed");
  assert.equal(result.failedStep.id, "realtime_collaboration_smoke");
  assert.equal(result.failureCategory, "command_exit_nonzero");
  assert.deepEqual(executed, [
    "collaboration_coverage_audit_tests",
    "collaboration_coverage_audit",
    "realtime_collaboration_smoke_tests",
    "realtime_collaboration_smoke",
  ]);
});

test("realtime collaboration product gate state machine rejects invalid transitions", () => {
  assert.equal(
    transitionRealtimeCollaborationProductGateState(
      RealtimeCollaborationProductGateState.Pending,
      RealtimeCollaborationProductGateEvent.Start,
    ),
    RealtimeCollaborationProductGateState.Running,
  );
  assert.equal(
    transitionRealtimeCollaborationProductGateState(
      RealtimeCollaborationProductGateState.Running,
      RealtimeCollaborationProductGateEvent.StepExit,
      { status: "passed" },
    ),
    RealtimeCollaborationProductGateState.StepPassed,
  );
  assert.throws(
    () =>
      transitionRealtimeCollaborationProductGateState(
        RealtimeCollaborationProductGateState.Pending,
        RealtimeCollaborationProductGateEvent.StepExit,
      ),
    (error) => error.code === RealtimeCollaborationProductGateErrorCode.InvalidTransition,
  );
});

test("realtime collaboration product gate markdown records final collaboration evidence", async () => {
  const result = await runRealtimeCollaborationProductGateWithRunner(async () => ({
    exitCode: 0,
    signal: null,
    durationMs: 3,
  }));
  const markdown = renderRealtimeCollaborationProductGateMarkdown(result);

  assert.match(markdown, /Phase 004 Realtime Collaboration Product Gate Result/);
  assert.match(markdown, /phase004_realtime_collaboration_product_gate=passed/);
  assert.match(markdown, /collaboration_coverage_audit/);
  assert.match(markdown, /realtime_collaboration_smoke/);
  assert.match(markdown, /security_log_scan/);
  assert.doesNotMatch(markdown, /document body|selected text|operation text|session-token/i);
});
