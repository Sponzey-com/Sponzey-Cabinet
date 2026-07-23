import assert from "node:assert/strict";
import test from "node:test";

import {
  aggregateQueryRenderBenchmark,
  benchmarkProcessExitCode,
  evaluateQueryRenderMeasurement,
} from "../src/query_render_benchmark_contract.ts";

test("query render measurement passes only when every semantic and performance assertion passes", () => {
  assert.deepEqual(evaluateQueryRenderMeasurement(validMeasurement()), {
    failureCodes: [],
    queryId: "search",
    status: "Passed",
  });
});

test("query render measurement reports every stable mismatch code", () => {
  assert.deepEqual(evaluateQueryRenderMeasurement({
    ...validMeasurement(),
    markerMatched: false,
    resultCountMatched: false,
    errorCount: 1,
    p95Ms: 300.001,
    sampleCount: 199,
  }), {
    failureCodes: [
      "MARKER_MISMATCH",
      "RESULT_COUNT_MISMATCH",
      "ERROR_COUNT_NON_ZERO",
      "SAMPLE_COUNT_MISMATCH",
      "P95_BUDGET_EXCEEDED",
    ],
    queryId: "search",
    status: "Failed",
  });
});

test("query render measurement rejects invalid identity numeric values and budget", () => {
  assert.deepEqual(evaluateQueryRenderMeasurement({
    ...validMeasurement(),
    queryId: " ",
    errorCount: -1,
    p95Ms: Number.NaN,
    budgetMs: Number.POSITIVE_INFINITY,
    sampleCount: 0,
    expectedSampleCount: 0,
  }), {
    failureCodes: ["MEASUREMENT_INVALID"],
    queryId: "invalid",
    status: "Failed",
  });
});

test("aggregate fails closed for empty duplicate and failed measurements", () => {
  assert.deepEqual(aggregateQueryRenderBenchmark([]), {
    failedQueryIds: [],
    failureCodes: ["MEASUREMENT_SET_EMPTY"],
    queryCount: 0,
    status: "Failed",
  });

  const passed = evaluateQueryRenderMeasurement(validMeasurement());
  assert.deepEqual(aggregateQueryRenderBenchmark([passed, passed]), {
    failedQueryIds: [],
    failureCodes: ["QUERY_ID_DUPLICATE"],
    queryCount: 2,
    status: "Failed",
  });

  const failed = evaluateQueryRenderMeasurement({ ...validMeasurement(), markerMatched: false });
  assert.deepEqual(aggregateQueryRenderBenchmark([failed]), {
    failedQueryIds: ["search"],
    failureCodes: ["QUERY_MEASUREMENT_FAILED"],
    queryCount: 1,
    status: "Failed",
  });
});

test("benchmark process exits zero only for a fully passed aggregate", () => {
  const passed = aggregateQueryRenderBenchmark([evaluateQueryRenderMeasurement(validMeasurement())]);
  const failed = aggregateQueryRenderBenchmark([
    evaluateQueryRenderMeasurement({ ...validMeasurement(), resultCountMatched: false }),
  ]);
  assert.equal(benchmarkProcessExitCode(passed), 0);
  assert.equal(benchmarkProcessExitCode(failed), 2);
});

function validMeasurement() {
  return {
    queryId: "search",
    markerMatched: true,
    resultCountMatched: true,
    errorCount: 0,
    p95Ms: 300,
    budgetMs: 300,
    sampleCount: 200,
    expectedSampleCount: 200,
  } as const;
}
