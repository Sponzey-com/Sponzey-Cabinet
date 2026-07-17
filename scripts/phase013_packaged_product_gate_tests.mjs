import assert from "node:assert/strict";
import test from "node:test";

import { PHASE013_PACKAGED_JOURNEYS, validatePhase013PackagedProductReport } from "./phase013_packaged_product_gate.mjs";

const fingerprint = "a".repeat(64);

test("complete clean-profile packaged journey passes", () => {
  assert.deepEqual(validatePhase013PackagedProductReport(validReport(), fingerprint), { passed: true, findingIds: [] });
});

test("keyboard document workflow evidence is mandatory", () => {
  const missing = validReport();
  delete missing.keyboardDocumentWorkflowVerified;
  assert.ok(validatePhase013PackagedProductReport(missing, fingerprint).findingIds.includes("keyboard_document_workflow"));
  const failed = validReport();
  failed.keyboardDocumentWorkflowVerified = false;
  assert.ok(validatePhase013PackagedProductReport(failed, fingerprint).findingIds.includes("keyboard_document_workflow"));
});

test("stale incomplete slow and sensitive packaged evidence fails", () => {
  const report = validReport();
  report.sourceFingerprint = "b".repeat(64);
  report.journeys.pop();
  report.p95Ms = 301;
  report.cleanProfile = false;
  report.diagnostics = "/Users/private";
  const result = validatePhase013PackagedProductReport(report, fingerprint);
  for (const id of ["stale_source_fingerprint", "journey_recovery", "p95_ms", "runtime_contract", "sensitive_data"]) assert.ok(result.findingIds.includes(id), id);
});

function validReport() {
  return {
    marker: "phase013_packaged_product_gate=passed",
    sourceFingerprint: fingerprint,
    appFingerprint: "b".repeat(64),
    platform: "macos",
    cleanProfile: true,
    externalRuntimeRequired: false,
    journeys: [...PHASE013_PACKAGED_JOURNEYS],
    sampleCount: 200,
    p95Ms: 80,
    errorCount: 0,
    actionCount: 40,
    durableReadbackCount: 12,
    keyboardDocumentWorkflowVerified: true,
    diagnostics: "sanitized",
  };
}
