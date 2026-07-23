export type QueryRenderMeasurementStatus = "Passed" | "Failed";

export type QueryRenderMeasurementFailureCode =
  | "MEASUREMENT_INVALID"
  | "MARKER_MISMATCH"
  | "RESULT_COUNT_MISMATCH"
  | "ERROR_COUNT_NON_ZERO"
  | "SAMPLE_COUNT_MISMATCH"
  | "P95_BUDGET_EXCEEDED";

export interface QueryRenderMeasurementInput {
  readonly queryId: string;
  readonly markerMatched: boolean;
  readonly resultCountMatched: boolean;
  readonly errorCount: number;
  readonly p95Ms: number;
  readonly budgetMs: number;
  readonly sampleCount: number;
  readonly expectedSampleCount: number;
}

export interface QueryRenderMeasurementResult {
  readonly queryId: string;
  readonly status: QueryRenderMeasurementStatus;
  readonly failureCodes: readonly QueryRenderMeasurementFailureCode[];
}

export type QueryRenderAggregateFailureCode =
  | "MEASUREMENT_SET_EMPTY"
  | "QUERY_ID_DUPLICATE"
  | "QUERY_MEASUREMENT_FAILED";

export interface QueryRenderBenchmarkAggregate {
  readonly status: QueryRenderMeasurementStatus;
  readonly queryCount: number;
  readonly failedQueryIds: readonly string[];
  readonly failureCodes: readonly QueryRenderAggregateFailureCode[];
}

export function evaluateQueryRenderMeasurement(
  input: QueryRenderMeasurementInput,
): QueryRenderMeasurementResult {
  const queryId = input.queryId.trim();
  if (!validInput(input, queryId)) {
    return result("invalid", "Failed", ["MEASUREMENT_INVALID"]);
  }

  const failureCodes: QueryRenderMeasurementFailureCode[] = [];
  if (!input.markerMatched) failureCodes.push("MARKER_MISMATCH");
  if (!input.resultCountMatched) failureCodes.push("RESULT_COUNT_MISMATCH");
  if (input.errorCount !== 0) failureCodes.push("ERROR_COUNT_NON_ZERO");
  if (input.sampleCount !== input.expectedSampleCount) failureCodes.push("SAMPLE_COUNT_MISMATCH");
  if (input.p95Ms > input.budgetMs) failureCodes.push("P95_BUDGET_EXCEEDED");
  return result(queryId, failureCodes.length === 0 ? "Passed" : "Failed", failureCodes);
}

export function aggregateQueryRenderBenchmark(
  measurements: readonly QueryRenderMeasurementResult[],
): QueryRenderBenchmarkAggregate {
  const failureCodes: QueryRenderAggregateFailureCode[] = [];
  if (measurements.length === 0) failureCodes.push("MEASUREMENT_SET_EMPTY");
  const ids = measurements.map((measurement) => measurement.queryId);
  if (new Set(ids).size !== ids.length) failureCodes.push("QUERY_ID_DUPLICATE");
  const failedQueryIds = measurements
    .filter((measurement) => measurement.status === "Failed")
    .map((measurement) => measurement.queryId);
  if (failedQueryIds.length > 0) failureCodes.push("QUERY_MEASUREMENT_FAILED");
  return Object.freeze({
    status: failureCodes.length === 0 ? "Passed" : "Failed",
    queryCount: measurements.length,
    failedQueryIds: Object.freeze(failedQueryIds),
    failureCodes: Object.freeze(failureCodes),
  });
}

export function benchmarkProcessExitCode(aggregate: QueryRenderBenchmarkAggregate): 0 | 2 {
  return aggregate.status === "Passed" ? 0 : 2;
}

function validInput(input: QueryRenderMeasurementInput, queryId: string): boolean {
  return queryId.length > 0
    && Number.isInteger(input.errorCount)
    && input.errorCount >= 0
    && Number.isFinite(input.p95Ms)
    && input.p95Ms >= 0
    && Number.isFinite(input.budgetMs)
    && input.budgetMs > 0
    && Number.isInteger(input.sampleCount)
    && input.sampleCount > 0
    && Number.isInteger(input.expectedSampleCount)
    && input.expectedSampleCount > 0;
}

function result(
  queryId: string,
  status: QueryRenderMeasurementStatus,
  failureCodes: readonly QueryRenderMeasurementFailureCode[],
): QueryRenderMeasurementResult {
  return Object.freeze({ queryId, status, failureCodes: Object.freeze([...failureCodes]) });
}
