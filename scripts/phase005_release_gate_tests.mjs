import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  Phase005ReleaseGateEvent,
  Phase005ReleaseGateState,
  buildPhase005ReleaseGateCommandPlan,
  renderPhase005ReleaseGateMarkdown,
  runPhase005ReleaseGateWithRunner,
  transitionPhase005ReleaseGateState,
} from "./phase005_release_gate.mjs";

test("phase005 release gate command plan includes required gates", () => {
  const stepIds = buildPhase005ReleaseGateCommandPlan().map((step) => step.id);

  for (const requiredStep of [
    "phase005_plan_validator_tests",
    "phase005_plan_validator",
    "phase005_retrieval_coverage_audit_tests",
    "phase005_retrieval_coverage_audit",
    "phase005_semantic_search_gate_tests",
    "phase005_semantic_search_gate",
    "phase005_ai_answer_product_gate_tests",
    "phase005_ai_answer_product_gate",
    "phase005_mcp_api_product_gate_tests",
    "phase005_mcp_api_product_gate",
    "phase005_webhook_connector_product_gate_tests",
    "phase005_webhook_connector_product_gate",
    "phase005_product_smoke_gate_tests",
    "phase005_product_smoke_gate",
    "phase005_observability_matrix_gate_tests",
    "phase005_observability_matrix_gate",
    "runbook_validator_tests",
    "runbook_validation",
    "security_log_scanner_tests",
    "security_log_scan",
  ]) {
    assert.ok(stepIds.includes(requiredStep), requiredStep);
  }
});

test("phase005 release gate renders passed marker when all steps pass", async () => {
  const result = await runPhase005ReleaseGateWithRunner(async () => ({
    exitCode: 0,
    signal: null,
    durationMs: 3,
  }), {
    startedAt: new Date("2026-07-01T00:00:00.000Z"),
    completedAtProvider: () => new Date("2026-07-01T00:00:10.000Z"),
    steps: [
      { id: "phase005_plan_validator", command: ["npm", "run", "run:phase005-plan-validator"] },
      { id: "security_log_scan", command: ["npm", "run", "run:security-log-scanner"] },
    ],
  });
  const markdown = renderPhase005ReleaseGateMarkdown(result);

  assert.equal(result.status, "passed");
  assert.equal(result.state, Phase005ReleaseGateState.StepPassed);
  assert.match(markdown, /phase005_release_gate=passed/);
  assert.match(markdown, /AI and external integration platform complete/);
});

test("phase005 release gate short-circuits on first failed step", async () => {
  const executed = [];
  const result = await runPhase005ReleaseGateWithRunner(async (step) => {
    executed.push(step.id);
    return {
      exitCode: step.id === "phase005_product_smoke_gate" ? 1 : 0,
      signal: null,
      durationMs: 4,
    };
  }, {
    steps: [
      { id: "phase005_plan_validator", command: ["npm", "run", "run:phase005-plan-validator"] },
      { id: "phase005_product_smoke_gate", command: ["npm", "run", "run:phase005-product-smoke-gate"] },
      { id: "security_log_scan", command: ["npm", "run", "run:security-log-scanner"] },
    ],
  });

  assert.equal(result.status, "failed");
  assert.equal(result.state, Phase005ReleaseGateState.StepFailed);
  assert.equal(result.failedStep.id, "phase005_product_smoke_gate");
  assert.deepEqual(executed, ["phase005_plan_validator", "phase005_product_smoke_gate"]);
});

test("phase005 release gate rejects invalid transition with stable code", () => {
  assert.throws(
    () =>
      transitionPhase005ReleaseGateState(
        Phase005ReleaseGateState.Pending,
        Phase005ReleaseGateEvent.WriteReport,
      ),
    /PHASE005_RELEASE_GATE_INVALID_TRANSITION/,
  );
});

test("phase005 release gate markdown excludes sensitive fixture values", async () => {
  const manifest = JSON.parse(
    await readFile(".tasks/release/security-log-policy-manifest.json", "utf8"),
  );
  const sensitiveValues = manifest.deniedFixtures
    .filter((fixture) =>
      [
        "ai_prompt_fixture",
        "ai_answer_fixture",
        "retrieval_source_text_fixture",
        "embedding_input_fixture",
        "provider_api_key_fixture",
        "connector_access_token_fixture",
        "connector_refresh_token_fixture",
        "connector_client_secret_fixture",
        "webhook_secret_fixture",
        "webhook_payload_body_fixture",
      ].includes(fixture.id),
    )
    .map((fixture) => fixture.value);
  const result = await runPhase005ReleaseGateWithRunner(async () => ({
    exitCode: 0,
    signal: null,
    durationMs: 1,
  }), {
    steps: [
      { id: "phase005_plan_validator", command: ["npm", "run", "run:phase005-plan-validator"] },
    ],
  });
  const markdown = renderPhase005ReleaseGateMarkdown(result);

  for (const value of sensitiveValues) {
    assert.doesNotMatch(markdown, new RegExp(value));
  }
});
