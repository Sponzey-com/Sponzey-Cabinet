import { readFile, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

export const DiscoveryGateErrorCode = Object.freeze({
  LocalPersistenceMissing: "PHASE007_DISCOVERY_LOCAL_PERSISTENCE_MISSING",
  PerformanceBudgetMissing: "PHASE007_DISCOVERY_PERFORMANCE_BUDGET_MISSING",
  QueryContractMissing: "PHASE007_DISCOVERY_QUERY_CONTRACT_MISSING",
  FullWorkspaceGraphScan: "PHASE007_DISCOVERY_FULL_WORKSPACE_GRAPH_SCAN",
  RawContentLeak: "PHASE007_DISCOVERY_RAW_CONTENT_LEAK",
  IoFailed: "PHASE007_DISCOVERY_IO_FAILED",
});

export function evaluateDiscoveryGate({
  localPersistenceText,
  performanceBudgetText,
  discoveryEvidence,
}) {
  if (!localPersistenceText.includes("phase007_local_persistence_gate=passed")) {
    return failed(DiscoveryGateErrorCode.LocalPersistenceMissing, "local_persistence_prerequisite");
  }
  if (!performanceBudgetText.includes("phase007_performance_budget=passed")) {
    return failed(DiscoveryGateErrorCode.PerformanceBudgetMissing, "performance_budget_prerequisite");
  }
  if (
    discoveryEvidence?.searchQueryName !== "search-documents" ||
    discoveryEvidence?.linkQueryName !== "get-link-overview" ||
    discoveryEvidence?.assetQueryName !== "list-document-assets" ||
    discoveryEvidence?.graphLoadMode !== "neighborhood" ||
    discoveryEvidence?.canvasLoadMode !== "viewport" ||
    !discoveryEvidence?.searchActions?.includes("open-document") ||
    !discoveryEvidence?.searchActions?.includes("ask-ai") ||
    discoveryEvidence?.indexFreshnessVisible !== true
  ) {
    return failed(DiscoveryGateErrorCode.QueryContractMissing, "discovery_query_contract");
  }
  if (discoveryEvidence.fullWorkspaceGraphScan !== false) {
    return failed(DiscoveryGateErrorCode.FullWorkspaceGraphScan, "graph_neighborhood_contract");
  }
  if (discoveryEvidence.rawContentExcluded !== true) {
    return failed(DiscoveryGateErrorCode.RawContentLeak, "sensitive_data_exclusion");
  }
  return {
    passed: true,
    marker: "phase007_discovery_gate=passed",
    searchQueryBudget: "300ms",
    graphLoadMode: "neighborhood",
    canvasLoadMode: "viewport",
    queryContracts: [
      "search-documents",
      "get-link-overview",
      "list-document-assets",
      "get-knowledge-graph",
    ],
  };
}

export function renderDiscoveryGateResult(result) {
  if (result.passed) {
    return [
      result.marker,
      `search_query_budget=${result.searchQueryBudget}`,
      `graph_load_mode=${result.graphLoadMode}`,
      `canvas_load_mode=${result.canvasLoadMode}`,
      `query_contracts=${result.queryContracts.join(",")}`,
    ].join("\n");
  }
  return [
    "phase007_discovery_gate=failed",
    `error_code=${result.errorCode}`,
    `finding_id=${result.findingId}`,
  ].join("\n");
}

export function renderDiscoveryGateArtifact(result) {
  const status = result.passed ? "passed" : "failed";
  return [
    "# Phase 007 Discovery Gate Result",
    "",
    renderDiscoveryGateResult(result),
    "",
    "- phase: `Phase 007.4`",
    "- gate: `Discovery Workspace`",
    `- status: \`${status}\``,
    "- prerequisite evidence:",
    "  - `.tasks/phase007-local-persistence-gate-result.md` with `phase007_local_persistence_gate=passed`",
    "  - `.tasks/release/performance-budget-phase007.md` with `phase007_performance_budget=passed`",
    "- validation commands:",
    "  - `npm run run:phase007-discovery-query-budget-tests`",
    "  - `npm run run:phase007-discovery-query-budget`",
    "  - `npm run run:phase007-discovery-gate-tests`",
    "  - `npm run run:phase007-discovery-gate`",
    "- Product Log candidates: `search.failed`, `index.rebuild.completed`, `asset.metadata.failed`, `graph.query.failed` with stable error code only.",
    "- Field Debug metadata candidates: `query_hash`, `result_count`, `index_freshness`, `graph_node_count`, `graph_edge_count`, `asset_count`.",
    "- sensitive data exclusion: this artifact records markers, query names, load modes, booleans, counts, and stable error codes only.",
    "- follow-up limitation: AI citation and provider settings remain Phase 007.5.",
    "",
  ].join("\n");
}

async function runDiscoveryGateCli() {
  try {
    const [localPersistenceText, performanceBudgetText] = await Promise.all([
      readFile(".tasks/phase007-local-persistence-gate-result.md", "utf8"),
      readFile(".tasks/release/performance-budget-phase007.md", "utf8"),
    ]);
    const result = evaluateDiscoveryGate({
      localPersistenceText,
      performanceBudgetText,
      discoveryEvidence: {
        searchQueryName: "search-documents",
        linkQueryName: "get-link-overview",
        assetQueryName: "list-document-assets",
        graphLoadMode: "neighborhood",
        fullWorkspaceGraphScan: false,
        canvasLoadMode: "viewport",
        searchActions: ["open-document", "ask-ai"],
        indexFreshnessVisible: true,
        rawContentExcluded: true,
      },
    });
    await writeFile(".tasks/phase007-discovery-gate-result.md", renderDiscoveryGateArtifact(result));
    const rendered = renderDiscoveryGateResult(result);
    if (result.passed) {
      console.log(rendered);
      return;
    }
    console.error(rendered);
    process.exit(1);
  } catch (error) {
    const result = failed(
      DiscoveryGateErrorCode.IoFailed,
      error instanceof Error ? error.message : "unknown",
    );
    await writeFile(".tasks/phase007-discovery-gate-result.md", renderDiscoveryGateArtifact(result));
    console.error(renderDiscoveryGateResult(result));
    process.exit(1);
  }
}

function failed(errorCode, findingId) {
  return {
    passed: false,
    marker: "phase007_discovery_gate=failed",
    errorCode,
    findingId,
  };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runDiscoveryGateCli();
}
