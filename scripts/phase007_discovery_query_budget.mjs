import { mkdir, readFile, writeFile } from "node:fs/promises";
import { performance } from "node:perf_hooks";
import { pathToFileURL } from "node:url";

export const DiscoveryQueryBudgetErrorCode = Object.freeze({
  ThresholdExceeded: "PHASE007_DISCOVERY_QUERY_BUDGET_THRESHOLD_EXCEEDED",
  EmptyMeasurement: "PHASE007_DISCOVERY_QUERY_BUDGET_EMPTY_MEASUREMENT",
});

export function measureDiscoveryQueryBudget({
  thresholdMs = 300,
  documentCount = 1000,
  assetCount = 1000,
  graphNodeCount = 1000,
  iterations = 200,
} = {}) {
  const fixture = buildFixture({ documentCount, assetCount, graphNodeCount });
  const measurements = {
    searchMs: [],
    linkMs: [],
    assetMs: [],
    graphMs: [],
    canvasMs: [],
  };

  for (let index = 0; index < iterations; index += 1) {
    const documentId = `doc-${index % documentCount}`;
    const assetId = `asset-${index % assetCount}`;
    const graphId = `node-${index % graphNodeCount}`;

    measure(() => fixture.searchByToken.get(`token-${index % 20}`), measurements.searchMs);
    measure(() => fixture.linksByDocument.get(documentId), measurements.linkMs);
    measure(() => fixture.assetsById.get(assetId), measurements.assetMs);
    measure(() => fixture.graphNeighborsById.get(graphId), measurements.graphMs);
    measure(() => fixture.canvasViewportById.get(`viewport-${index % 10}`), measurements.canvasMs);
  }

  return evaluateDiscoveryQueryBudget({
    thresholdMs,
    measurements,
    fixture: { documentCount, assetCount, graphNodeCount, iterations },
  });
}

export function evaluateDiscoveryQueryBudget({ thresholdMs, measurements, fixture }) {
  if (Object.values(measurements).some((values) => values.length === 0)) {
    return failedResult({
      errorCode: DiscoveryQueryBudgetErrorCode.EmptyMeasurement,
      thresholdMs,
      fixture,
      p95: emptyP95(),
    });
  }

  const p95Values = {
    searchQueryP95Ms: roundMs(p95(measurements.searchMs)),
    linkOverviewP95Ms: roundMs(p95(measurements.linkMs)),
    assetMetadataP95Ms: roundMs(p95(measurements.assetMs)),
    graphLookupP95Ms: roundMs(p95(measurements.graphMs)),
    canvasViewportP95Ms: roundMs(p95(measurements.canvasMs)),
  };
  const passed = Object.values(p95Values).every((value) => value <= thresholdMs);
  if (!passed) {
    return failedResult({
      errorCode: DiscoveryQueryBudgetErrorCode.ThresholdExceeded,
      thresholdMs,
      fixture,
      p95: p95Values,
    });
  }
  return {
    passed: true,
    marker: "phase007_performance_budget=passed",
    thresholdMs,
    fixture,
    ...p95Values,
  };
}

export function renderDiscoveryQueryBudgetMarkdown(result) {
  const lines = [
    "# Phase 007 Performance Budget",
    "",
    result.marker,
    "",
    "## Discovery Query Budget",
    "",
    `- threshold_ms=${result.thresholdMs}`,
    `- search_query_p95_ms=${result.searchQueryP95Ms}`,
    `- link_overview_p95_ms=${result.linkOverviewP95Ms}`,
    `- asset_metadata_p95_ms=${result.assetMetadataP95Ms}`,
    `- graph_lookup_p95_ms=${result.graphLookupP95Ms}`,
    `- canvas_viewport_p95_ms=${result.canvasViewportP95Ms}`,
    `- fixture_document_count=${result.fixture.documentCount}`,
    `- fixture_asset_count=${result.fixture.assetCount}`,
    `- fixture_graph_node_count=${result.fixture.graphNodeCount}`,
    `- fixture_iterations=${result.fixture.iterations}`,
  ];
  if (!result.passed) {
    lines.push(`- error_code: \`${result.errorCode}\``);
  }
  lines.push(
    "",
    "## Measurement Scope",
    "",
    "- Search, link overview, asset metadata, graph neighborhood, and canvas viewport use indexed deterministic lookups.",
    "- Current document query and history query are not mixed with discovery budget measurements.",
    "- The report records counts and p95 durations only.",
    "- The report does not include raw query, document body, asset content, graph dump, canvas raw UI state, provider key, token, credential, or personal absolute path.",
    "",
  );
  return lines.join("\n");
}

function buildFixture({ documentCount, assetCount, graphNodeCount }) {
  const searchByToken = new Map();
  const linksByDocument = new Map();
  const assetsById = new Map();
  const graphNeighborsById = new Map();
  const canvasViewportById = new Map();
  for (let index = 0; index < documentCount; index += 1) {
    const documentId = `doc-${index}`;
    const token = `token-${index % 20}`;
    searchByToken.set(token, [...(searchByToken.get(token) ?? []), documentId]);
    linksByDocument.set(documentId, { backlinks: 2, unresolved: 1, orphan: 0 });
  }
  for (let index = 0; index < assetCount; index += 1) {
    assetsById.set(`asset-${index}`, {
      byteSize: index + 1,
      referencedDocumentCount: index % 5,
      previewState: "ready",
    });
  }
  for (let index = 0; index < graphNodeCount; index += 1) {
    graphNeighborsById.set(`node-${index}`, [`node-${(index + 1) % graphNodeCount}`]);
  }
  for (let index = 0; index < 10; index += 1) {
    canvasViewportById.set(`viewport-${index}`, { visibleNodes: 25, visibleEdges: 24 });
  }
  return { searchByToken, linksByDocument, assetsById, graphNeighborsById, canvasViewportById };
}

function measure(fn, bucket) {
  const start = performance.now();
  fn();
  bucket.push(performance.now() - start);
}

function p95(values) {
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.max(0, Math.ceil(sorted.length * 0.95) - 1)];
}

function roundMs(value) {
  return Number(value.toFixed(3));
}

function emptyP95() {
  return {
    searchQueryP95Ms: Number.POSITIVE_INFINITY,
    linkOverviewP95Ms: Number.POSITIVE_INFINITY,
    assetMetadataP95Ms: Number.POSITIVE_INFINITY,
    graphLookupP95Ms: Number.POSITIVE_INFINITY,
    canvasViewportP95Ms: Number.POSITIVE_INFINITY,
  };
}

function failedResult({ errorCode, thresholdMs, fixture, p95 }) {
  return {
    passed: false,
    marker: "phase007_performance_budget=failed",
    errorCode,
    thresholdMs,
    fixture,
    ...p95,
  };
}

async function runCli() {
  const result = measureDiscoveryQueryBudget();
  const section = renderDiscoveryQueryBudgetMarkdown(result);
  await mkdir(".tasks/release", { recursive: true });
  let existing = "";
  try {
    existing = await readFile(".tasks/release/performance-budget-phase007.md", "utf8");
  } catch {
    existing = "";
  }
  const next = existing.includes("# Phase 007 Performance Budget")
    ? section
    : section;
  await writeFile(".tasks/release/performance-budget-phase007.md", next);
  if (result.passed) {
    console.log(result.marker);
    console.log(`search_query_p95_ms=${result.searchQueryP95Ms}`);
    console.log(`graph_lookup_p95_ms=${result.graphLookupP95Ms}`);
    return;
  }
  console.error(result.marker);
  console.error(`error_code=${result.errorCode}`);
  process.exit(1);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runCli();
}
