export const PHASE012_RENDER_QUERY_IDS = Object.freeze([
  "current_document",
  "history_page",
  "search",
  "link_overview",
  "local_graph",
  "global_graph",
  "canvas_viewport",
  "asset_metadata",
]);

const transitions = Object.freeze({
  Pending: Object.freeze({ LoadNative: "LoadingNative" }),
  LoadingNative: Object.freeze({ Measure: "Measuring" }),
  Measuring: Object.freeze({ Validate: "Validating" }),
  Validating: Object.freeze({ Write: "Writing" }),
  Writing: Object.freeze({ Pass: "Passed" }),
});

export function transitionQueryRenderPerformanceState(state, event) {
  return Object.freeze({ state: transitions[state]?.[event] ?? "Failed" });
}

export function nearestRankPercentile(samples, percentile) {
  if (!Array.isArray(samples) || samples.length === 0) throw new Error("samples must not be empty");
  if (!(percentile > 0 && percentile <= 1)) throw new Error("percentile must be in (0, 1]");
  const sorted = samples.toSorted((left, right) => left - right);
  return sorted[Math.ceil(sorted.length * percentile) - 1];
}

export function parseDesktopQueryRenderBenchmarkOutput(stdout) {
  const queries = String(stdout).trim().split(/\r?\n/).filter(Boolean).map((line) => {
    if (!line.startsWith("query=")) throw new Error(`invalid desktop benchmark row: ${line}`);
    const fields = Object.fromEntries(line.split(";").map((field) => {
      const separator = field.indexOf("=");
      if (separator <= 0) throw new Error(`invalid desktop benchmark field: ${field}`);
      return [field.slice(0, separator), field.slice(separator + 1)];
    }));
    return Object.freeze({
      queryId: fields.query,
      standardFixtureCount: integer(fields.standard_fixture_count),
      boundedResultCount: integer(fields.bounded_result_count),
      markerMatched: fields.marker_matched === "true",
      resultCountMatched: fields.result_count_matched === "true",
      sampleCount: integer(fields.sample_count),
      errorCount: integer(fields.error_count),
      desktopRenderP50Ms: numeric(fields.p50_ms),
      desktopRenderP95Ms: numeric(fields.p95_ms),
      desktopRenderMaxMs: numeric(fields.max_ms),
    });
  });
  for (const queryId of PHASE012_RENDER_QUERY_IDS) {
    if (!queries.some((query) => query.queryId === queryId)) throw new Error(`required desktop query output missing: ${queryId}`);
  }
  return Object.freeze(queries);
}

export function validateQueryRenderPerformanceReport(report, expected = {}) {
  const findingIds = [];
  if (report?.marker !== "phase012_query_render_performance=passed") findingIds.push("marker");
  if (report?.state !== "Passed") findingIds.push("state");
  validateHash(report?.sourceFingerprint, "source_fingerprint", findingIds);
  validateHash(report?.nativeSourceFingerprint, "native_source_fingerprint", findingIds);
  validateHash(report?.fixtureHash, "fixture_hash", findingIds);
  if (expected.sourceFingerprint && report?.sourceFingerprint !== expected.sourceFingerprint) findingIds.push("stale_source_fingerprint");
  if (expected.nativeSourceFingerprint && report?.nativeSourceFingerprint !== expected.nativeSourceFingerprint) findingIds.push("stale_native_source_fingerprint");
  if (expected.fixtureHash && report?.fixtureHash !== expected.fixtureHash) findingIds.push("stale_fixture_hash");
  if (report?.timingBoundary !== "desktop_controller_dispatch_to_rendered_markup_marker") findingIds.push("timing_boundary");
  if (report?.combinedMethod !== "native_p95_plus_desktop_render_p95_plus_ipc_allowance") findingIds.push("combined_method");
  if (report?.packagedEndToEndMeasured !== false) findingIds.push("packaged_end_to_end_claim");
  if (report?.followUp !== "phase012_packaged_end_to_end") findingIds.push("follow_up");
  if (report?.environment?.platform !== "macos" || report?.environment?.profile !== "release") findingIds.push("environment");

  const policy = report?.policy ?? {};
  if (!(policy.warmupCount >= 30)) findingIds.push("warmup_count");
  if (!(policy.measuredCount >= 30)) findingIds.push("measured_count");
  if (policy.percentile !== "nearest_rank") findingIds.push("percentile");
  if (policy.budgetMs !== 300) findingIds.push("budget_policy");
  if (!(policy.ipcAllowanceMs >= 0)) findingIds.push("ipc_allowance");

  const queries = Array.isArray(report?.queries) ? report.queries : [];
  for (const queryId of PHASE012_RENDER_QUERY_IDS) {
    const query = queries.find((candidate) => candidate?.queryId === queryId);
    if (!query) {
      findingIds.push(`query_${queryId}_missing`);
      continue;
    }
    if (!(query.standardFixtureCount > 0)) findingIds.push(`query_${queryId}_fixture`);
    if (!(query.boundedResultCount > 0)) findingIds.push(`query_${queryId}_bounded_result`);
    if (query.markerMatched !== true) findingIds.push(`query_${queryId}_marker`);
    if (query.resultCountMatched !== true) findingIds.push(`query_${queryId}_result_count_mismatch`);
    if (!(query.sampleCount >= policy.measuredCount)) findingIds.push(`query_${queryId}_samples`);
    if (query.errorCount !== 0) findingIds.push(`query_${queryId}_errors`);
    for (const field of ["nativeP95Ms", "desktopRenderP50Ms", "desktopRenderP95Ms", "desktopRenderMaxMs", "combinedP95Ms"]) {
      if (!(Number.isFinite(query[field]) && query[field] >= 0)) findingIds.push(`query_${queryId}_${field}`);
    }
    const expectedCombined = query.nativeP95Ms + query.desktopRenderP95Ms + query.ipcAllowanceMs;
    if (Math.abs(query.combinedP95Ms - expectedCombined) > 0.001) findingIds.push(`query_${queryId}_combined_math`);
    if (!(query.combinedP95Ms <= policy.budgetMs)) findingIds.push(`query_${queryId}_budget`);
  }
  if (queries.some((query) => !PHASE012_RENDER_QUERY_IDS.includes(query?.queryId))) findingIds.push("unknown_query");
  if (containsSensitiveData(JSON.stringify(report ?? {}))) findingIds.push("sensitive_data");
  return Object.freeze({ passed: findingIds.length === 0, findingIds: Object.freeze(findingIds) });
}

function validateHash(value, findingId, findingIds) {
  if (!/^[a-f0-9]{64}$/.test(value ?? "")) findingIds.push(findingId);
}

function containsSensitiveData(text) {
  return [
    "/Users/",
    "C:\\Users\\",
    "raw document body",
    "provider_api_key",
    "sessionToken",
    "source.md",
  ].some((token) => text.includes(token));
}

function numeric(value) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) throw new Error(`invalid numeric benchmark value: ${value}`);
  return parsed;
}

function integer(value) {
  const parsed = numeric(value);
  if (!Number.isInteger(parsed)) throw new Error(`invalid integer benchmark value: ${value}`);
  return parsed;
}
