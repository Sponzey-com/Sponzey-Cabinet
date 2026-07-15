import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import test from "node:test";

import {
  Phase012PlanErrorCode,
  Phase012PlanEvent,
  Phase012PlanState,
  runPhase012PlanValidation,
  transitionPhase012PlanState,
  validatePhase012EvidenceFingerprints,
  validatePhase012PlanText,
} from "./phase012_plan_validator.mjs";

const currentPlan = await readFile(new URL("../.tasks/plan.md", import.meta.url), "utf8");

test("current phase012 plan has all executable contracts", () => {
  const result = validatePhase012PlanText(currentPlan);
  assert.deepEqual(result.findings, []);
  assert.equal(result.phaseCount, 9);
  assert.equal(result.requirementIds.length, 33);
});

test("rejects a missing phase field", () => {
  const changed = currentPlan.replace("* Logging Rules:", "* Removed Logging Rules:");
  const result = validatePhase012PlanText(changed);
  assert.equal(result.findings[0].errorCode, Phase012PlanErrorCode.PhaseFieldMissing);
  assert.equal(result.findings[0].findingId, "Logging Rules");
});

test("rejects a phase number gap", () => {
  const changed = currentPlan.replace("## Phase 012.4.", "## Phase 012.9.");
  const result = validatePhase012PlanText(changed);
  assert.ok(result.findings.some((item) => item.errorCode === Phase012PlanErrorCode.PhaseSequenceInvalid));
});

test("rejects duplicate requirement ids", () => {
  const row = currentPlan.match(/^\| `SCOPE-012-01`.*$/m)[0];
  const result = validatePhase012PlanText(`${currentPlan}\n${row}\n`);
  assert.ok(result.findings.some((item) => item.errorCode === Phase012PlanErrorCode.RequirementInvalid));
});

test("rejects prohibited vague plan language", () => {
  const result = validatePhase012PlanText(`${currentPlan}\n적절히 처리한다\n`);
  assert.ok(result.findings.some((item) => item.errorCode === Phase012PlanErrorCode.VagueLanguage));
});

test("repository plan validation passes current archive and matrix fingerprints", async () => {
  const result = await runPhase012PlanValidation({ root: fileURLToPath(new URL("..", import.meta.url)), writeArtifact: false });
  assert.equal(result.passed, true);
  assert.equal(result.state, Phase012PlanState.Passed);
  assert.equal(result.requirementCount, 33);
});

test("accepts archive-scoped pending evidence before the final release gate", () => {
  assert.equal(validatePhase012EvidenceFingerprints({
    archiveFingerprint: "archive-fingerprint",
    inventory: "source_fingerprint=archive-fingerprint\n",
    matrix: "phase012_requirement_evidence=pending\nsource_fingerprint=archive-fingerprint\n",
    release: "phase012_release_gate=passed\nsource_fingerprint=previous-release\n",
  }), true);
});

test("accepts release-scoped passed evidence and rejects a stale release fingerprint", () => {
  const input = {
    archiveFingerprint: "archive-fingerprint",
    inventory: "source_fingerprint=archive-fingerprint\n",
    matrix: "phase012_requirement_evidence=passed\nsource_fingerprint=release-fingerprint\n",
    release: "phase012_release_gate=passed\nsource_fingerprint=release-fingerprint\n",
  };
  assert.equal(validatePhase012EvidenceFingerprints(input), true);
  assert.equal(validatePhase012EvidenceFingerprints({
    ...input,
    release: "phase012_release_gate=passed\nsource_fingerprint=stale-release\n",
  }), false);
});

test("state machine rejects out-of-order completion", () => {
  let result = transitionPhase012PlanState(Phase012PlanState.NotStarted, Phase012PlanEvent.PrerequisiteAccepted);
  assert.equal(result.state, Phase012PlanState.PrerequisiteValidated);
  result = transitionPhase012PlanState(result.state, Phase012PlanEvent.PlanAccepted);
  assert.equal(result.state, Phase012PlanState.PlanValidated);
  result = transitionPhase012PlanState(result.state, Phase012PlanEvent.EvidenceAccepted);
  assert.equal(result.state, Phase012PlanState.EvidenceValidated);
  result = transitionPhase012PlanState(result.state, Phase012PlanEvent.Complete);
  assert.equal(result.state, Phase012PlanState.Passed);
  const invalid = transitionPhase012PlanState(Phase012PlanState.NotStarted, Phase012PlanEvent.Complete);
  assert.equal(invalid.errorCode, Phase012PlanErrorCode.InvalidTransition);
});
