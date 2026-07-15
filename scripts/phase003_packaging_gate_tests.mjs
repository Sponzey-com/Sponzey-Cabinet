import assert from "node:assert/strict";
import test from "node:test";

import {
  PackagingGateErrorCode,
  PackagingGateEvent,
  PackagingGateState,
  buildPackagingGateCommandPlan,
  renderPackagingGateMarkdown,
  runPackagingGateWithRunner,
  transitionPackagingGateState,
} from "./phase003_packaging_gate.mjs";

test("packaging gate command plan runs audit before smoke commands", () => {
  const plan = buildPackagingGateCommandPlan();

  assert.deepEqual(plan.map((step) => step.id), [
    "packaging_coverage_audit_tests",
    "packaging_coverage_audit",
    "self_host_server_package_smoke_tests",
    "self_host_server_package_smoke",
    "self_host_upgrade_smoke_tests",
    "self_host_upgrade_smoke",
    "browser_smoke",
    "desktop_package_smoke",
    "desktop_packaged_app_smoke",
    "desktop_dist_browser_smoke",
  ]);
});

test("packaging gate reports all pass when every command exits zero", async () => {
  const result = await runPackagingGateWithRunner(async () => ({
    exitCode: 0,
    signal: null,
    durationMs: 7,
  }));

  assert.equal(result.status, "passed");
  assert.equal(result.failedStep, null);
  assert.equal(result.steps.length, 10);
  assert.equal(result.steps.every((step) => step.status === "passed"), true);
});

test("packaging gate short-circuits after failed command", async () => {
  const executed = [];
  const result = await runPackagingGateWithRunner(async (step) => {
    executed.push(step.id);
    if (step.id === "self_host_upgrade_smoke") {
      return { exitCode: 1, signal: null, durationMs: 9 };
    }
    return { exitCode: 0, signal: null, durationMs: 5 };
  });

  assert.equal(result.status, "failed");
  assert.equal(result.failedStep.id, "self_host_upgrade_smoke");
  assert.equal(executed.includes("browser_smoke"), false);
});

test("packaging gate state machine rejects invalid transitions", () => {
  assert.throws(
    () => transitionPackagingGateState(PackagingGateState.Pending, PackagingGateEvent.WriteReport),
    (error) => error.code === PackagingGateErrorCode.InvalidTransition,
  );
});

test("packaging gate markdown records every step and final status", async () => {
  const result = await runPackagingGateWithRunner(async () => ({
    exitCode: 0,
    signal: null,
    durationMs: 3,
  }));
  const markdown = renderPackagingGateMarkdown(result);

  assert.match(markdown, /Phase 003 Packaging Gate Result/);
  assert.match(markdown, /status: `passed`/);
  assert.match(markdown, /self_host_server_package_smoke/);
  assert.match(markdown, /self_host_upgrade_smoke/);
  assert.match(markdown, /desktop_packaged_app_smoke/);
});
