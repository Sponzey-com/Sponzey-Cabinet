import assert from "node:assert/strict";
import test from "node:test";

import {
  PHASE013_REQUIREMENT_FAMILIES,
  validatePhase013FinalReleaseReport,
} from "./phase013_final_release_gate.mjs";

const fingerprint = "a".repeat(64);

function validReport() {
  return {
    marker: "phase013_final_release_gate=passed",
    state: "Passed",
    sourceFingerprint: fingerprint,
    taskCount: 46,
    diagnostics: "sanitized",
    requirementEvidence: PHASE013_REQUIREMENT_FAMILIES.map((family) => ({
      family,
      state: "Passed",
      evidence: `release/${family.toLowerCase()}-phase013.json`,
    })),
    commands: [{ name: "desktop-tests", state: "Passed" }],
  };
}

test("accepts complete sanitized current Phase 013 evidence", () => {
  assert.deepEqual(validatePhase013FinalReleaseReport(validReport(), fingerprint), {
    passed: true,
    findingIds: [],
  });
});

test("rejects missing, failed, duplicate, and stale requirement evidence", () => {
  const missing = validReport();
  missing.requirementEvidence = missing.requirementEvidence.slice(1);
  assert.ok(validatePhase013FinalReleaseReport(missing, fingerprint).findingIds.includes("requirement_family_missing"));

  const failed = validReport();
  failed.requirementEvidence[0] = { ...failed.requirementEvidence[0], state: "Failed" };
  assert.ok(validatePhase013FinalReleaseReport(failed, fingerprint).findingIds.includes("requirement_evidence_failed"));

  const duplicate = validReport();
  duplicate.requirementEvidence.push(duplicate.requirementEvidence[0]);
  assert.ok(validatePhase013FinalReleaseReport(duplicate, fingerprint).findingIds.includes("requirement_family_duplicate"));

  assert.ok(validatePhase013FinalReleaseReport(validReport(), "b".repeat(64)).findingIds.includes("stale_source_fingerprint"));
});

test("rejects failed commands and sensitive or absolute-path evidence", () => {
  const failedCommand = validReport();
  failedCommand.commands[0] = { name: "desktop-tests", state: "Failed" };
  assert.ok(validatePhase013FinalReleaseReport(failedCommand, fingerprint).findingIds.includes("command_failed"));

  const sensitive = validReport();
  sensitive.requirementEvidence[0] = {
    ...sensitive.requirementEvidence[0],
    documentBody: "private body",
  };
  assert.ok(validatePhase013FinalReleaseReport(sensitive, fingerprint).findingIds.includes("sensitive_evidence"));

  const absolute = validReport();
  absolute.requirementEvidence[0] = {
    ...absolute.requirementEvidence[0],
    evidence: "/Users/example/private.json",
  };
  assert.ok(validatePhase013FinalReleaseReport(absolute, fingerprint).findingIds.includes("absolute_path"));
});
