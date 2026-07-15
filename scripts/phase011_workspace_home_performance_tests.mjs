import assert from "node:assert/strict";
import test from "node:test";

import { validateWorkspaceHomePerformanceReport } from "./phase011_workspace_home_performance.mjs";

test("performance report enforces standard fixture and p95 contract", () => {
  assert.equal(validateWorkspaceHomePerformanceReport(validReport()).passed, true);

  for (const [field, value] of [
    ["warmupCount", 19],
    ["sampleCount", 199],
    ["p95Ms", 300.01],
    ["currentDocumentCount", 9999],
    ["totalVersionCount", 99999],
    ["buildProfile", "debug"],
  ]) {
    const report = validReport();
    report[field] = value;
    assert.equal(validateWorkspaceHomePerformanceReport(report).passed, false, field);
  }
});

test("performance report rejects raw-scan and sensitive artifacts", () => {
  const scan = validReport();
  scan.queryPath = "raw_document_full_scan";
  assert.equal(validateWorkspaceHomePerformanceReport(scan).passed, false);
  const sensitive = validReport();
  sensitive.diagnostics = "notes/private.md raw document body";
  assert.equal(validateWorkspaceHomePerformanceReport(sensitive).passed, false);
});

function validReport() {
  return {
    marker: "phase011_workspace_home_performance=passed",
    sourceFingerprint: "a".repeat(64),
    fixtureHash: "b".repeat(64),
    currentDocumentCount: 10000,
    totalVersionCount: 100000,
    warmupCount: 20,
    sampleCount: 200,
    p50Ms: 1.2,
    p95Ms: 2.5,
    maxMs: 4.2,
    buildProfile: "release",
    queryPath: "bounded_workspace_home_projection",
    diagnostics: "sanitized",
  };
}
