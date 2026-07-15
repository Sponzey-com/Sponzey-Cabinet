import { readFile, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

export const SearchGraphAssetGateState = Object.freeze({
  Pending: "Pending",
  ReadingSources: "ReadingSources",
  ValidatingEvidence: "ValidatingEvidence",
  WritingReport: "WritingReport",
  Passed: "Passed",
  Failed: "Failed",
});

export const SearchGraphAssetGateEvent = Object.freeze({
  Start: "Start",
  SourcesLoaded: "SourcesLoaded",
  EvidenceValidated: "EvidenceValidated",
  ReportWritten: "ReportWritten",
  Fail: "Fail",
});

export const SearchGraphAssetGateErrorCode = Object.freeze({
  RequiredEvidenceMissing: "PHASE006_SEARCH_GRAPH_ASSET_REQUIRED_EVIDENCE_MISSING",
  SourceReadFailed: "PHASE006_SEARCH_GRAPH_ASSET_SOURCE_READ_FAILED",
  InvalidTransition: "PHASE006_SEARCH_GRAPH_ASSET_INVALID_TRANSITION",
});

const requiredTargets = Object.freeze([
  target("document_ux_prerequisite", "Phase 006 document UX gate prerequisite", {
    requiredFiles: [".tasks/phase006-document-ux-gate-result.md"],
    evidence: ["phase006_document_ux_gate=passed"],
  }),
  target("search_graph_asset_performance_budget", "search graph asset p95 performance budget", {
    requiredFiles: [".tasks/release/performance-budget-phase006.md"],
    evidence: [
      "phase006_search_graph_asset_budget=passed",
      "search_query_p95_ms=",
      "link_overview_p95_ms=",
      "asset_metadata_p95_ms=",
      "graph_lookup_p95_ms=",
      "canvas_viewport_p95_ms=",
    ],
  }),
  target("ui_discovery_graph_canvas_models", "UI discovery graph canvas models", {
    requiredFiles: [
      "packages/ui/src/index.ts",
      "packages/ui/tests/local_discovery_panel_model_tests.ts",
      "packages/ui/tests/graph_canvas_panel_model_tests.ts",
    ],
    evidence: [
      "createLocalDiscoveryPanelModel",
      "createGraphPanelViewModel",
      "createCanvasViewportPanelModel",
      "local discovery panel hashes search query",
      "graph panel uses neighborhood mode with depth limit",
      "canvas viewport model includes only visible nodes",
    ],
  }),
  target("desktop_discovery_smoke", "desktop discovery graph canvas smoke", {
    requiredFiles: [
      "apps/desktop/src/index.ts",
      "apps/desktop/tests/desktop_discovery_smoke_tests.ts",
    ],
    evidence: [
      "createDesktopLocalDiscoveryPanel",
      "createDesktopGraphPanel",
      "createDesktopCanvasViewportPanel",
      "desktop local discovery smoke hides raw query and asset content",
      "desktop graph smoke uses neighborhood contract",
      "desktop canvas smoke filters viewport",
    ],
  }),
]);

export function transitionSearchGraphAssetGateState(currentState, event, detail = {}) {
  if (currentState === SearchGraphAssetGateState.Pending && event === SearchGraphAssetGateEvent.Start) {
    return { state: SearchGraphAssetGateState.ReadingSources };
  }
  if (
    currentState === SearchGraphAssetGateState.ReadingSources &&
    event === SearchGraphAssetGateEvent.SourcesLoaded
  ) {
    return { state: SearchGraphAssetGateState.ValidatingEvidence };
  }
  if (
    currentState === SearchGraphAssetGateState.ValidatingEvidence &&
    event === SearchGraphAssetGateEvent.EvidenceValidated
  ) {
    return { state: SearchGraphAssetGateState.WritingReport };
  }
  if (
    currentState === SearchGraphAssetGateState.WritingReport &&
    event === SearchGraphAssetGateEvent.ReportWritten
  ) {
    return { state: SearchGraphAssetGateState.Passed };
  }
  if (
    [
      SearchGraphAssetGateState.ReadingSources,
      SearchGraphAssetGateState.ValidatingEvidence,
      SearchGraphAssetGateState.WritingReport,
    ].includes(currentState) &&
    event === SearchGraphAssetGateEvent.Fail
  ) {
    return {
      state: SearchGraphAssetGateState.Failed,
      errorCode: detail.errorCode ?? SearchGraphAssetGateErrorCode.RequiredEvidenceMissing,
      findingId: detail.findingId,
    };
  }
  return {
    state: SearchGraphAssetGateState.Failed,
    errorCode: SearchGraphAssetGateErrorCode.InvalidTransition,
  };
}

export function analyzeSearchGraphAssetEvidence({ sources }) {
  if (!sources || Object.keys(sources).length === 0) {
    return failedResult({
      errorCode: SearchGraphAssetGateErrorCode.SourceReadFailed,
      missingEvidence: [{ targetId: "source_set", missing: ["sources"] }],
    });
  }
  const targetResults = requiredTargets.map((entry) => analyzeTarget(entry, sources));
  const missingEvidence = targetResults
    .filter((entry) => entry.missing.length > 0)
    .map((entry) => ({ targetId: entry.id, missing: entry.missing }));
  if (missingEvidence.length > 0) {
    return failedResult({
      errorCode: SearchGraphAssetGateErrorCode.RequiredEvidenceMissing,
      missingEvidence,
      targetResults,
    });
  }
  return {
    passed: true,
    marker: "phase006_search_graph_asset_gate=passed",
    state: SearchGraphAssetGateState.Passed,
    summary: { requiredTargets: requiredTargets.length, missingRequiredEvidence: 0 },
    targetResults,
    missingEvidence: [],
  };
}

export function renderSearchGraphAssetGateMarkdown(result) {
  const lines = [
    "# Phase 006 Search Graph Asset Gate Result",
    "",
    result.marker,
    "",
    "- phase: `Phase 006`",
    "- gate: `Local Search, Graph, Canvas, and Asset Panels`",
    `- status: \`${result.passed ? "passed" : "failed"}\``,
    `- state: \`${result.state}\``,
  ];
  if (!result.passed) {
    lines.push(`- error code: \`${result.errorCode}\``);
  }
  lines.push(
    `- required targets: \`${result.summary.requiredTargets}\``,
    `- missing required evidence: \`${result.summary.missingRequiredEvidence}\``,
    "",
    "## Evidence",
    "",
    "| Target | Status | Description |",
    "| --- | --- | --- |",
  );
  for (const targetResult of result.targetResults ?? []) {
    lines.push(`| \`${targetResult.id}\` | \`${targetResult.status}\` | ${targetResult.description} |`);
  }
  lines.push(
    "",
    "## Sensitive Data Exclusion",
    "",
    "This artifact records target ids, marker status, counts, and stable error codes only. It does not record raw query, document body, asset content, graph dump, canvas raw UI state, provider key, token, credential, or personal absolute path.",
    "",
  );
  return lines.join("\n");
}

export async function runSearchGraphAssetGate({ root = process.cwd() } = {}) {
  let state = transitionSearchGraphAssetGateState(SearchGraphAssetGateState.Pending, SearchGraphAssetGateEvent.Start);
  try {
    const sources = {};
    for (const filePath of collectRequiredFiles()) {
      sources[filePath] = await readFile(`${root}/${filePath}`, "utf8");
    }
    state = transitionSearchGraphAssetGateState(state.state, SearchGraphAssetGateEvent.SourcesLoaded);
    const result = analyzeSearchGraphAssetEvidence({ sources });
    if (!result.passed) {
      state = transitionSearchGraphAssetGateState(state.state, SearchGraphAssetGateEvent.Fail, {
        errorCode: result.errorCode,
        findingId: result.missingEvidence[0]?.targetId,
      });
      return { ...result, state: state.state };
    }
    state = transitionSearchGraphAssetGateState(state.state, SearchGraphAssetGateEvent.EvidenceValidated);
    state = transitionSearchGraphAssetGateState(state.state, SearchGraphAssetGateEvent.ReportWritten);
    return { ...result, state: state.state };
  } catch {
    state = transitionSearchGraphAssetGateState(state.state, SearchGraphAssetGateEvent.Fail, {
      errorCode: SearchGraphAssetGateErrorCode.SourceReadFailed,
    });
    return failedResult({
      errorCode: state.errorCode,
      state: state.state,
      missingEvidence: [{ targetId: "source_read", missing: ["required source file"] }],
    });
  }
}

function analyzeTarget(entry, sources) {
  const texts = entry.requiredFiles.map((filePath) => sources[filePath] ?? "");
  const missingFiles = entry.requiredFiles.filter((filePath) => !(filePath in sources));
  const missingEvidence = entry.evidence.filter((needle) => !texts.some((text) => text.includes(needle)));
  const missing = [...missingFiles, ...missingEvidence];
  return {
    id: entry.id,
    description: entry.description,
    status: missing.length === 0 ? "covered" : "missing",
    missing,
  };
}

function failedResult({ errorCode, state = SearchGraphAssetGateState.Failed, missingEvidence, targetResults = [] }) {
  return {
    passed: false,
    marker: "phase006_search_graph_asset_gate=failed",
    state,
    errorCode,
    summary: { requiredTargets: requiredTargets.length, missingRequiredEvidence: missingEvidence.length },
    targetResults,
    missingEvidence,
  };
}

function target(id, description, { requiredFiles, evidence }) {
  return { id, description, requiredFiles, evidence };
}

function collectRequiredFiles() {
  return [...new Set(requiredTargets.flatMap((entry) => entry.requiredFiles))];
}

async function runCli() {
  const result = await runSearchGraphAssetGate();
  await writeFile(".tasks/phase006-search-graph-asset-gate-result.md", renderSearchGraphAssetGateMarkdown(result));
  if (result.passed) {
    console.log(result.marker);
    console.log(`gate_state=${result.state}`);
    console.log(`required_targets=${result.summary.requiredTargets}`);
    return;
  }
  console.error(result.marker);
  console.error(`gate_state=${result.state}`);
  console.error(`error_code=${result.errorCode}`);
  process.exit(1);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runCli();
}
