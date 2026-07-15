import assert from "node:assert/strict";
import test from "node:test";

import {
  assertSensitiveOutputClean,
  buildSelfHostUpgradeSmokePlan,
  renderUpgradeSmokeSummary,
  validateUpgradeSmokeResults,
} from "./run_self_host_upgrade_smoke.mjs";

test("upgrade smoke plan verifies migration state machine before persistence smokes", () => {
  const plan = buildSelfHostUpgradeSmokePlan();

  assert.deepEqual(plan.steps.map((step) => step.id), [
    "migration_state_machine",
    "data_preservation_smoke",
    "phase002_migration_fixture_smoke",
  ]);
  assert.deepEqual(plan.steps[0].command, [
    "cargo",
    "test",
    "-p",
    "cabinet-core",
    "--test",
    "migration_tests",
  ]);
});

test("upgrade smoke validator rejects failed commands and missing pass markers", () => {
  const plan = buildSelfHostUpgradeSmokePlan();
  const passed = plan.steps.map((step) => ({
    step,
    exitCode: 0,
    stdout: `${step.successMarker}\ntest result: ok`,
    stderr: "",
  }));

  assert.equal(validateUpgradeSmokeResults(passed).passed, true);

  assert.throws(
    () =>
      validateUpgradeSmokeResults([
        { step: plan.steps[0], exitCode: 1, stdout: "", stderr: "failed" },
      ]),
    /upgrade smoke step failed/,
  );
  assert.throws(
    () =>
      validateUpgradeSmokeResults([
        { step: plan.steps[0], exitCode: 0, stdout: "test result: ok", stderr: "" },
      ]),
    /upgrade smoke marker was not found/,
  );
});

test("upgrade smoke summary renders stable pass markers", () => {
  const plan = buildSelfHostUpgradeSmokePlan();
  const result = validateUpgradeSmokeResults(
    plan.steps.map((step) => ({
      step,
      exitCode: 0,
      stdout: `${step.successMarker}\ntest result: ok`,
      stderr: "",
    })),
  );
  const summary = renderUpgradeSmokeSummary(result);

  assert.match(summary, /run_self_host_upgrade_smoke=true/);
  assert.match(summary, /migration_state_machine=verified/);
  assert.match(summary, /upgrade_migration_smoke=passed/);
});

test("upgrade smoke sensitive output scanner rejects secrets and document bodies", () => {
  assertSensitiveOutputClean("upgrade_migration_smoke=passed");

  assert.throws(
    () => assertSensitiveOutputClean("upgrade_migration_smoke=passed\nsecret-access-key"),
    /sensitive output detected/,
  );
  assert.throws(
    () =>
      assertSensitiveOutputClean(
        "upgrade_migration_smoke=passed\nfixture document body should not be logged",
      ),
    /sensitive output detected/,
  );
});
