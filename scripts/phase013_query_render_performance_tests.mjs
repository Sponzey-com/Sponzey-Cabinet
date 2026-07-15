import assert from "node:assert/strict";
import test from "node:test";

import { PHASE013_PERFORMANCE_FIXTURE, PHASE013_QUERY_IDS, transitionPhase013Performance, validatePhase013QueryRenderPerformance } from "./phase013_query_render_performance.mjs";

const fingerprint = "a".repeat(64);

test("Phase 013 performance state machine rejects implicit transitions", () => {
  assert.equal(transitionPhase013Performance("Pending", "Build"), "Building");
  assert.equal(transitionPhase013Performance("Building", "Native"), "MeasuringNative");
  assert.equal(transitionPhase013Performance("Pending", "Pass"), "Failed");
});

test("complete bounded report passes", () => {
  assert.deepEqual(validatePhase013QueryRenderPerformance(validReport(), fingerprint), { passed: true, findingIds: [] });
});

test("stale weak unbounded and sensitive report fails", () => {
  const report = validReport();
  report.sourceFingerprint = "b".repeat(64);
  report.policy.sampleCount = 10;
  report.fixture.canvasNodeCount = 20;
  report.queries.find((query) => query.queryId === "canvas_viewport").boundedResultCount = 500;
  report.queries.find((query) => query.queryId === "search").combinedP95Ms = 301;
  report.diagnostics = "/Users/private";
  const result = validatePhase013QueryRenderPerformance(report, fingerprint);
  for (const id of ["stale_source_fingerprint", "policy", "fixture_canvasNodeCount", "unbounded_canvas_viewport", "p95_search", "sensitive_data"]) assert.ok(result.findingIds.includes(id), id);
});

function validReport() {
  return {
    marker: "phase013_query_render_performance=passed",
    state: "Passed",
    sourceFingerprint: fingerprint,
    policy: { budgetMs: 300, warmupCount: 30, sampleCount: 200 },
    fixture: { ...PHASE013_PERFORMANCE_FIXTURE },
    queries: PHASE013_QUERY_IDS.map((queryId) => ({ queryId, boundedResultCount: queryId === "current_document" ? 1 : 50, combinedP95Ms: 42, errorCount: 0, markerMatched: true, resultCountMatched: true })),
    diagnostics: "sanitized",
  };
}
