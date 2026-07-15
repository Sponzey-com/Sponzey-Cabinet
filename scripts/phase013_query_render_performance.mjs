export const PHASE013_QUERY_IDS = Object.freeze([
  "current_document", "history_page", "search", "link_overview", "local_graph",
  "global_graph", "canvas_viewport", "asset_metadata",
]);

const expectedFixture = Object.freeze({
  documentCount: 10_000, historyVersionCount: 1_000, linkCount: 50_000,
  graphNodeCount: 10_000, graphEdgeCount: 50_000, canvasNodeCount: 2_000,
  canvasEdgeCount: 4_000, assetCount: 10_000, pageSize: 50,
});

const transitions = Object.freeze({
  Pending: Object.freeze({ Build: "Building" }),
  Building: Object.freeze({ Native: "MeasuringNative", Fail: "Failed" }),
  MeasuringNative: Object.freeze({ Render: "MeasuringRender", Fail: "Failed" }),
  MeasuringRender: Object.freeze({ Validate: "Validating", Fail: "Failed" }),
  Validating: Object.freeze({ Write: "Writing", Fail: "Failed" }),
  Writing: Object.freeze({ Pass: "Passed", Fail: "Failed" }),
});

export function transitionPhase013Performance(state, event) {
  return transitions[state]?.[event] ?? "Failed";
}

export function validatePhase013QueryRenderPerformance(report, expectedFingerprint) {
  const findingIds = [];
  if (report?.marker !== "phase013_query_render_performance=passed") findingIds.push("marker");
  if (report?.state !== "Passed") findingIds.push("state");
  if (!/^[a-f0-9]{64}$/.test(report?.sourceFingerprint ?? "")) findingIds.push("source_fingerprint");
  if (expectedFingerprint && report?.sourceFingerprint !== expectedFingerprint) findingIds.push("stale_source_fingerprint");
  if (report?.policy?.budgetMs !== 300 || report?.policy?.warmupCount < 30 || report?.policy?.sampleCount < 200) findingIds.push("policy");
  for (const [key, value] of Object.entries(expectedFixture)) {
    if (report?.fixture?.[key] !== value) findingIds.push(`fixture_${key}`);
  }
  const queries = Array.isArray(report?.queries) ? report.queries : [];
  for (const id of PHASE013_QUERY_IDS) {
    const query = queries.find((candidate) => candidate?.queryId === id);
    if (!query) { findingIds.push(`missing_${id}`); continue; }
    const expectedCount = id === "current_document" ? 1 : 50;
    if (query.boundedResultCount !== expectedCount) findingIds.push(`unbounded_${id}`);
    if (!Number.isFinite(query.combinedP95Ms) || query.combinedP95Ms > 300) findingIds.push(`p95_${id}`);
    if (query.errorCount !== 0 || query.markerMatched !== true || query.resultCountMatched !== true) findingIds.push(`invalid_${id}`);
  }
  const serialized = JSON.stringify(report ?? {});
  if (["/Users/", "C:\\Users\\", "documentBody", "raw query", "sessionToken", "provider_api_key"].some((token) => serialized.includes(token))) findingIds.push("sensitive_data");
  return Object.freeze({ passed: findingIds.length === 0, findingIds: Object.freeze([...new Set(findingIds)]) });
}

export const PHASE013_PERFORMANCE_FIXTURE = expectedFixture;
