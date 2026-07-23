import assert from "node:assert/strict";
import test from "node:test";

import {
  nearestRankPercentile,
  validateTopologyScaleReport,
  type TopologyScaleReport,
} from "../src/topology_scale_budget.ts";

const fingerprint = "a".repeat(64);

test("topology scale report requires complete sanitized 1k 5k and 10k evidence", () => {
  assert.deepEqual(validateTopologyScaleReport(validReport(), fingerprint), { passed: true, findingIds: [] });
  const missing = validReport();
  missing.profiles = missing.profiles.filter((profile) => profile.nodeCount !== 5_000);
  assert.ok(validateTopologyScaleReport(missing, fingerprint).findingIds.includes("profile_5000"));
});

test("topology scale report fails closed on budget resource and sensitive evidence", () => {
  const report = validReport();
  report.sourceFingerprint = "b".repeat(64);
  report.profiles[2].mappingP95Ms = 301;
  report.profiles[2].editorInputDelayMs = 101;
  report.profiles[2].browserErrorCount = 1;
  report.diagnostics = "/Users/private/notes/source.md" as "sanitized";
  const findings = validateTopologyScaleReport(report, fingerprint).findingIds;
  for (const finding of ["stale_source_fingerprint", "mapping_p95_10000", "editor_input_delay_10000", "browser_error_10000", "sensitive_data"]) {
    assert.ok(findings.includes(finding), finding);
  }
});

test("nearest-rank percentile is deterministic and rejects invalid input", () => {
  assert.equal(nearestRankPercentile([5, 1, 3, 2, 4], 0.5), 3);
  assert.equal(nearestRankPercentile([5, 1, 3, 2, 4], 0.95), 5);
  assert.equal(nearestRankPercentile([], 0.95), 0);
  assert.throws(() => nearestRankPercentile([1], 0), /TOPOLOGY_PERCENTILE_INVALID/);
});

function validReport(): MutableReport {
  return {
    marker: "topology_scale_budget=passed",
    sourceFingerprint: fingerprint,
    diagnostics: "sanitized",
    profiles: [1_000, 5_000, 10_000].map((nodeCount) => ({
      nodeCount,
      edgeCount: nodeCount * 2,
      sampleCount: 30,
      mappingP50Ms: 10,
      mappingP95Ms: 20,
      mappingMaxMs: 30,
      browserReadyMs: 800,
      editorInputDelayMs: 20,
      rendererCanvasCount: 7,
      browserErrorCount: 0,
      unhandledRejectionCount: 0,
    })),
  };
}

type MutableReport = {
  -readonly [Key in keyof TopologyScaleReport]: Key extends "profiles"
    ? Array<{ -readonly [ProfileKey in keyof TopologyScaleReport["profiles"][number]]: TopologyScaleReport["profiles"][number][ProfileKey] }>
    : TopologyScaleReport[Key];
};
