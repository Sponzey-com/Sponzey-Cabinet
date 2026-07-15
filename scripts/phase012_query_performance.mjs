const REQUIRED_QUERY_IDS = Object.freeze([
  "current_document",
  "history_page",
  "search",
  "link_overview",
  "local_graph",
  "global_graph",
  "canvas_viewport",
  "asset_metadata",
]);

const EXPECTED_FIXTURE = Object.freeze({
  documentCount: 10_000,
  historyVersionCount: 1_000,
  linkCount: 50_000,
  graphNodeCount: 10_000,
  graphEdgeCount: 50_000,
  canvasNodeCount: 2_000,
  canvasEdgeCount: 4_000,
  assetCount: 10_000,
  pageSize: 50,
});

const SENSITIVE_PATTERNS = Object.freeze([
  /\/Users\//,
  /[A-Z]:\\Users\\/i,
  /document[_ ]?body/i,
  /raw[_ ]?query/i,
  /secret|token|credential|api[_-]?key/i,
  /\.md\b/i,
]);

export const Phase012QueryPerformanceState = Object.freeze({
  Pending: "Pending",
  Building: "Building",
  Measuring: "Measuring",
  Validating: "Validating",
  Writing: "Writing",
  Passed: "Passed",
  Failed: "Failed",
});

const TRANSITIONS = Object.freeze({
  Pending: Object.freeze({ BUILD: "Building", FAIL: "Failed" }),
  Building: Object.freeze({ MEASURE: "Measuring", FAIL: "Failed" }),
  Measuring: Object.freeze({ VALIDATE: "Validating", FAIL: "Failed" }),
  Validating: Object.freeze({ WRITE: "Writing", FAIL: "Failed" }),
  Writing: Object.freeze({ PASS: "Passed", FAIL: "Failed" }),
});

export function advancePhase012QueryPerformanceState(state, event) {
  const next = TRANSITIONS[state]?.[event];
  if (!next) throw new Error(`invalid performance evidence transition: ${state} + ${event}`);
  return next;
}

export function parseNativeQueryBenchmarkOutput(stdout) {
  const lines = String(stdout).trim().split(/\r?\n/).filter(Boolean);
  const scalar = new Map();
  const queries = [];
  for (const line of lines) {
    if (line.startsWith("query=")) {
      const fields = Object.fromEntries(line.split(";").map(splitPair));
      queries.push({
        id: fields.query,
        p50Ms: numeric(fields.p50_ms),
        p95Ms: numeric(fields.p95_ms),
        maxMs: numeric(fields.max_ms),
        errorCount: integer(fields.error_count),
        resultCount: integer(fields.result_count),
        queryPath: fields.query_path,
      });
    } else {
      const [key, value] = splitPair(line);
      scalar.set(key, value);
    }
  }
  for (const id of REQUIRED_QUERY_IDS) {
    if (!queries.some((entry) => entry.id === id)) {
      throw new Error(`required query output missing: ${id}`);
    }
  }
  return {
    marker: scalar.get("phase012_native_query_benchmark"),
    warmupCount: integer(scalar.get("warmup_count")),
    sampleCount: integer(scalar.get("sample_count")),
    percentileMethod: scalar.get("percentile_method"),
    queries,
  };
}

export function validatePhase012QueryPerformanceReport(report, expectedSourceFingerprint) {
  const findingIds = [];
  if (report?.marker !== "phase012_native_query_performance=passed") findingIds.push("marker");
  if (!isSha256(report?.sourceFingerprint)) findingIds.push("source_fingerprint");
  if (expectedSourceFingerprint && report?.sourceFingerprint !== expectedSourceFingerprint) {
    findingIds.push("stale_source_fingerprint");
  }
  if (!isSha256(report?.fixtureHash)) findingIds.push("fixture_hash");
  if (report?.buildProfile !== "release") findingIds.push("build_profile");
  if (report?.timingBoundary !== "native_backend_only") findingIds.push("timing_boundary");
  if (report?.percentileMethod !== "nearest_rank") findingIds.push("percentile_method");
  if (!Number.isInteger(report?.warmupCount) || report.warmupCount < 30) findingIds.push("warmup_count");
  if (!Number.isInteger(report?.sampleCount) || report.sampleCount < 200) findingIds.push("sample_count");
  if (report?.budgetMs !== 300) findingIds.push("budget_ms");
  for (const [key, expected] of Object.entries(EXPECTED_FIXTURE)) {
    if (report?.fixture?.[key] !== expected) findingIds.push(`fixture_${camelToSnake(key)}`);
  }
  const queries = Array.isArray(report?.queries) ? report.queries : [];
  if (REQUIRED_QUERY_IDS.some((id) => !queries.some((query) => query?.id === id))) {
    findingIds.push("missing_query");
  }
  for (const query of queries) {
    if (!Number.isFinite(query?.p95Ms) || query.p95Ms > 300) findingIds.push("p95_ms");
    if (!Number.isFinite(query?.p50Ms) || !Number.isFinite(query?.maxMs)) findingIds.push("metric");
    if (query?.errorCount !== 0) findingIds.push("error_count");
    if (!query?.queryPath || /scan|unbounded/i.test(query.queryPath)) findingIds.push("query_path");
  }
  const expectedCounts = new Map([
    ["current_document", 1],
    ["history_page", 50],
    ["search", 50],
    ["link_overview", 50],
    ["local_graph", 50],
    ["global_graph", 50],
    ["canvas_viewport", 50],
    ["asset_metadata", 50],
  ]);
  for (const [id, count] of expectedCounts) {
    if (queries.find((query) => query?.id === id)?.resultCount !== count) {
      findingIds.push("result_count");
    }
  }
  if (SENSITIVE_PATTERNS.some((pattern) => pattern.test(JSON.stringify(report ?? {})))) {
    findingIds.push("sensitive_data");
  }
  return { passed: findingIds.length === 0, findingIds: [...new Set(findingIds)] };
}

function splitPair(value) {
  const index = value.indexOf("=");
  if (index <= 0) throw new Error(`invalid benchmark output field: ${value}`);
  return [value.slice(0, index), value.slice(index + 1)];
}

function numeric(value) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) throw new Error(`invalid numeric metric: ${value}`);
  return parsed;
}

function integer(value) {
  const parsed = numeric(value);
  if (!Number.isInteger(parsed)) throw new Error(`invalid integer metric: ${value}`);
  return parsed;
}

function isSha256(value) {
  return /^[a-f0-9]{64}$/.test(value ?? "");
}

function camelToSnake(value) {
  return value.replace(/[A-Z]/g, (letter) => `_${letter.toLowerCase()}`);
}
