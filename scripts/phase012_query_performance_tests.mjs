import assert from "node:assert/strict";
import test from "node:test";

import {
  Phase012QueryPerformanceState,
  advancePhase012QueryPerformanceState,
  parseNativeQueryBenchmarkOutput,
  validatePhase012QueryPerformanceReport,
} from "./phase012_query_performance.mjs";

const REQUIRED_QUERIES = [
  "current_document",
  "history_page",
  "search",
  "link_overview",
  "local_graph",
  "global_graph",
  "canvas_viewport",
  "asset_metadata",
];

test("valid native backend report enforces the complete standard fixture contract", () => {
  const report = validReport();
  assert.deepEqual(validatePhase012QueryPerformanceReport(report, report.sourceFingerprint), {
    passed: true,
    findingIds: [],
  });
});

test("report rejects missing query, stale source, weak sampling, wrong profile and over-budget p95", () => {
  const cases = [
    ["missing_query", (report) => report.queries.pop()],
    ["stale_source_fingerprint", (report) => { report.sourceFingerprint = "c".repeat(64); }],
    ["warmup_count", (report) => { report.warmupCount = 29; }],
    ["sample_count", (report) => { report.sampleCount = 199; }],
    ["build_profile", (report) => { report.buildProfile = "debug"; }],
    ["p95_ms", (report) => { report.queries[0].p95Ms = 300.001; }],
    ["error_count", (report) => { report.queries[0].errorCount = 1; }],
    ["result_count", (report) => { report.queries[0].resultCount = 0; }],
    ["timing_boundary", (report) => { report.timingBoundary = "end_to_end"; }],
  ];
  for (const [findingId, mutate] of cases) {
    const report = validReport();
    mutate(report);
    const result = validatePhase012QueryPerformanceReport(report, "a".repeat(64));
    assert.equal(result.passed, false, findingId);
    assert.ok(result.findingIds.includes(findingId), `${findingId}: ${result.findingIds}`);
  }
});

test("report rejects invalid fixture counts, unbounded paths and sensitive diagnostics", () => {
  for (const [findingId, mutate] of [
    ["fixture_document_count", (report) => { report.fixture.documentCount = 9_999; }],
    ["fixture_history_version_count", (report) => { report.fixture.historyVersionCount = 999; }],
    ["fixture_link_count", (report) => { report.fixture.linkCount = 49_999; }],
    ["fixture_graph_node_count", (report) => { report.fixture.graphNodeCount = 9_999; }],
    ["fixture_graph_edge_count", (report) => { report.fixture.graphEdgeCount = 49_999; }],
    ["fixture_canvas_node_count", (report) => { report.fixture.canvasNodeCount = 1_999; }],
    ["fixture_canvas_edge_count", (report) => { report.fixture.canvasEdgeCount = 3_999; }],
    ["fixture_asset_count", (report) => { report.fixture.assetCount = 9_999; }],
    ["query_path", (report) => { report.queries[0].queryPath = "full_history_scan"; }],
    ["sensitive_data", (report) => { report.diagnostics = "/Users/private/notes.md"; }],
  ]) {
    const report = validReport();
    mutate(report);
    const result = validatePhase012QueryPerformanceReport(report, report.sourceFingerprint);
    assert.equal(result.passed, false, findingId);
    assert.ok(result.findingIds.includes(findingId), `${findingId}: ${result.findingIds}`);
  }
});

test("native benchmark output parser requires exact query rows and numeric measurements", () => {
  const output = [
    "phase012_native_query_benchmark=passed",
    "warmup_count=30",
    "sample_count=200",
    "percentile_method=nearest_rank",
    ...REQUIRED_QUERIES.map((id, index) =>
      `query=${id};p50_ms=${index + 1}.1;p95_ms=${index + 2}.2;max_ms=${index + 3}.3;error_count=0;result_count=50;query_path=bounded_projection`,
    ),
  ].join("\n");
  const parsed = parseNativeQueryBenchmarkOutput(output);
  assert.equal(parsed.queries.length, REQUIRED_QUERIES.length);
  assert.equal(parsed.queries[0].id, "current_document");
  assert.equal(parsed.queries[0].p95Ms, 2.2);
  assert.throws(
    () => parseNativeQueryBenchmarkOutput(output.replace("p95_ms=2.2", "p95_ms=not-a-number")),
    /invalid numeric metric/,
  );
  assert.throws(
    () => parseNativeQueryBenchmarkOutput(output.split("\n").slice(0, -1).join("\n")),
    /required query output missing/,
  );
});

test("evidence runner state machine is deterministic and fail closed", () => {
  let state = Phase012QueryPerformanceState.Pending;
  for (const event of ["BUILD", "MEASURE", "VALIDATE", "WRITE", "PASS"]) {
    state = advancePhase012QueryPerformanceState(state, event);
  }
  assert.equal(state, Phase012QueryPerformanceState.Passed);
  assert.equal(
    advancePhase012QueryPerformanceState(Phase012QueryPerformanceState.Measuring, "FAIL"),
    Phase012QueryPerformanceState.Failed,
  );
  assert.throws(
    () => advancePhase012QueryPerformanceState(Phase012QueryPerformanceState.Pending, "PASS"),
    /invalid performance evidence transition/,
  );
});

function validReport() {
  return {
    marker: "phase012_native_query_performance=passed",
    sourceFingerprint: "a".repeat(64),
    fixtureHash: "b".repeat(64),
    buildProfile: "release",
    timingBoundary: "native_backend_only",
    percentileMethod: "nearest_rank",
    warmupCount: 30,
    sampleCount: 200,
    budgetMs: 300,
    fixture: {
      seed: 12012,
      documentCount: 10_000,
      historyVersionCount: 1_000,
      linkCount: 50_000,
      graphNodeCount: 10_000,
      graphEdgeCount: 50_000,
      canvasNodeCount: 2_000,
      canvasEdgeCount: 4_000,
      assetCount: 10_000,
      pageSize: 50,
    },
    queries: REQUIRED_QUERIES.map((id) => ({
      id,
      p50Ms: 1,
      p95Ms: 2,
      maxMs: 3,
      errorCount: 0,
      resultCount: id === "current_document" ? 1 : 50,
      queryPath: "bounded_index_or_projection",
    })),
    diagnostics: "sanitized",
  };
}
