import assert from "node:assert/strict";
import test from "node:test";

import {
  PHASE012_RENDER_QUERY_IDS,
  nearestRankPercentile,
  parseDesktopQueryRenderBenchmarkOutput,
  transitionQueryRenderPerformanceState,
  validateQueryRenderPerformanceReport,
} from "./phase012_query_render_performance.mjs";

const fingerprint = "a".repeat(64);
const nativeFingerprint = "b".repeat(64);
const fixtureHash = "c".repeat(64);

test("nearest-rank percentile is deterministic at boundaries", () => {
  assert.equal(nearestRankPercentile([4, 1, 3, 2], 0.5), 2);
  assert.equal(nearestRankPercentile([4, 1, 3, 2], 0.95), 4);
  assert.throws(() => nearestRankPercentile([], 0.95), /samples/);
  assert.throws(() => nearestRankPercentile([1], 0), /percentile/);
});

test("runner transition rejects an out-of-order event", () => {
  assert.equal(transitionQueryRenderPerformanceState("Pending", "LoadNative").state, "LoadingNative");
  assert.equal(transitionQueryRenderPerformanceState("LoadingNative", "Measure").state, "Measuring");
  assert.equal(transitionQueryRenderPerformanceState("Measuring", "Validate").state, "Validating");
  assert.equal(transitionQueryRenderPerformanceState("Validating", "Write").state, "Writing");
  assert.equal(transitionQueryRenderPerformanceState("Writing", "Pass").state, "Passed");
  assert.equal(transitionQueryRenderPerformanceState("Pending", "Pass").state, "Failed");
});

test("desktop benchmark parser requires all eight numeric query rows", () => {
  const output = PHASE012_RENDER_QUERY_IDS.map((queryId) => [
    `query=${queryId}`,
    "standard_fixture_count=10000",
    "bounded_result_count=50",
    "marker_matched=true",
    "result_count_matched=true",
    "sample_count=200",
    "error_count=0",
    "p50_ms=1.0",
    "p95_ms=2.0",
    "max_ms=3.0",
  ].join(";")).join("\n");
  assert.equal(parseDesktopQueryRenderBenchmarkOutput(output).length, 8);
  assert.throws(() => parseDesktopQueryRenderBenchmarkOutput(output.replace("p95_ms=2.0", "p95_ms=bad")), /numeric/);
  assert.throws(() => parseDesktopQueryRenderBenchmarkOutput(output.split("\n").slice(1).join("\n")), /required desktop query/);
});

test("validator accepts complete current-source desktop render evidence", () => {
  const validation = validateQueryRenderPerformanceReport(validReport(), {
    sourceFingerprint: fingerprint,
    nativeSourceFingerprint: nativeFingerprint,
    fixtureHash,
  });
  assert.deepEqual(validation, { passed: true, findingIds: [] });
});

test("validator rejects missing query stale evidence errors and budget excess", () => {
  const report = validReport();
  report.queries = report.queries.slice(1);
  report.sourceFingerprint = "d".repeat(64);
  report.queries[0].errorCount = 1;
  report.queries[1].combinedP95Ms = 300.001;
  report.queries[2].resultCountMatched = false;
  const validation = validateQueryRenderPerformanceReport(report, {
    sourceFingerprint: fingerprint,
    nativeSourceFingerprint: nativeFingerprint,
    fixtureHash,
  });
  assert.equal(validation.passed, false);
  assert.ok(validation.findingIds.includes("query_current_document_missing"));
  assert.ok(validation.findingIds.includes("stale_source_fingerprint"));
  assert.ok(validation.findingIds.some((id) => id.endsWith("_errors")));
  assert.ok(validation.findingIds.some((id) => id.endsWith("_budget")));
  assert.ok(validation.findingIds.some((id) => id.endsWith("_result_count_mismatch")));
});

test("validator rejects false end-to-end claims and sensitive data", () => {
  const report = validReport();
  report.packagedEndToEndMeasured = true;
  report.environment = { platform: "macos", profile: "release", leaked: "/Users/private/file.md" };
  const validation = validateQueryRenderPerformanceReport(report, {
    sourceFingerprint: fingerprint,
    nativeSourceFingerprint: nativeFingerprint,
    fixtureHash,
  });
  assert.equal(validation.passed, false);
  assert.ok(validation.findingIds.includes("packaged_end_to_end_claim"));
  assert.ok(validation.findingIds.includes("sensitive_data"));
});

function validReport() {
  return {
    marker: "phase012_query_render_performance=passed",
    state: "Passed",
    sourceFingerprint: fingerprint,
    nativeSourceFingerprint: nativeFingerprint,
    fixtureHash,
    timingBoundary: "desktop_controller_dispatch_to_rendered_markup_marker",
    combinedMethod: "native_p95_plus_desktop_render_p95_plus_ipc_allowance",
    packagedEndToEndMeasured: false,
    followUp: "phase012_packaged_end_to_end",
    policy: {
      warmupCount: 30,
      measuredCount: 200,
      percentile: "nearest_rank",
      budgetMs: 300,
      ipcAllowanceMs: 25,
    },
    environment: { platform: "macos", profile: "release" },
    queries: PHASE012_RENDER_QUERY_IDS.map((queryId) => ({
      queryId,
      standardFixtureCount: 10_000,
      boundedResultCount: 50,
      markerMatched: true,
      resultCountMatched: true,
      sampleCount: 200,
      errorCount: 0,
      nativeP95Ms: 20,
      desktopRenderP50Ms: 1,
      desktopRenderP95Ms: 2,
      desktopRenderMaxMs: 3,
      ipcAllowanceMs: 25,
      combinedP95Ms: 47,
    })),
  };
}
