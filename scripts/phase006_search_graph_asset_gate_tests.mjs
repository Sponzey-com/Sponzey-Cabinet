import assert from "node:assert/strict";
import test from "node:test";

import {
  SearchGraphAssetGateErrorCode,
  SearchGraphAssetGateEvent,
  SearchGraphAssetGateState,
  analyzeSearchGraphAssetEvidence,
  renderSearchGraphAssetGateMarkdown,
  transitionSearchGraphAssetGateState,
} from "./phase006_search_graph_asset_gate.mjs";

test("search graph asset gate reports complete evidence as passed", () => {
  const result = analyzeSearchGraphAssetEvidence({ sources: completeSources() });
  const markdown = renderSearchGraphAssetGateMarkdown(result);

  assert.equal(result.passed, true);
  assert.equal(result.marker, "phase006_search_graph_asset_gate=passed");
  assert.match(markdown, /phase006_search_graph_asset_gate=passed/);
  assert.doesNotMatch(markdown, /raw query should not appear/);
  assert.doesNotMatch(markdown, /asset binary content should not leak/);
});

test("search graph asset gate fails when document UX prerequisite is missing", () => {
  const sources = completeSources();
  sources[".tasks/phase006-document-ux-gate-result.md"] = "phase006_document_ux_gate=failed";

  const result = analyzeSearchGraphAssetEvidence({ sources });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, SearchGraphAssetGateErrorCode.RequiredEvidenceMissing);
  assert.equal(result.missingEvidence[0].targetId, "document_ux_prerequisite");
});

test("search graph asset gate fails when discovery budget marker is missing", () => {
  const sources = completeSources();
  sources[".tasks/release/performance-budget-phase006.md"] =
    "phase006_search_graph_asset_budget=failed";

  const result = analyzeSearchGraphAssetEvidence({ sources });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, SearchGraphAssetGateErrorCode.RequiredEvidenceMissing);
  assert.equal(result.missingEvidence[0].targetId, "search_graph_asset_performance_budget");
});

test("search graph asset gate state machine exposes explicit transitions", () => {
  const reading = transitionSearchGraphAssetGateState(
    SearchGraphAssetGateState.Pending,
    SearchGraphAssetGateEvent.Start,
  );
  const validating = transitionSearchGraphAssetGateState(
    reading.state,
    SearchGraphAssetGateEvent.SourcesLoaded,
  );
  const writing = transitionSearchGraphAssetGateState(
    validating.state,
    SearchGraphAssetGateEvent.EvidenceValidated,
  );
  const passed = transitionSearchGraphAssetGateState(
    writing.state,
    SearchGraphAssetGateEvent.ReportWritten,
  );
  const invalid = transitionSearchGraphAssetGateState(
    SearchGraphAssetGateState.Pending,
    SearchGraphAssetGateEvent.ReportWritten,
  );

  assert.equal(reading.state, SearchGraphAssetGateState.ReadingSources);
  assert.equal(validating.state, SearchGraphAssetGateState.ValidatingEvidence);
  assert.equal(writing.state, SearchGraphAssetGateState.WritingReport);
  assert.equal(passed.state, SearchGraphAssetGateState.Passed);
  assert.equal(invalid.errorCode, SearchGraphAssetGateErrorCode.InvalidTransition);
});

function completeSources() {
  return {
    ".tasks/phase006-document-ux-gate-result.md": "phase006_document_ux_gate=passed",
    ".tasks/release/performance-budget-phase006.md": [
      "phase006_search_graph_asset_budget=passed",
      "search_query_p95_ms=3",
      "link_overview_p95_ms=3",
      "asset_metadata_p95_ms=3",
      "graph_lookup_p95_ms=3",
      "canvas_viewport_p95_ms=3",
    ].join("\n"),
    "packages/ui/src/index.ts": [
      "createLocalDiscoveryPanelModel",
      "createGraphPanelViewModel",
      "createCanvasViewportPanelModel",
      "fullWorkspaceScan",
    ].join("\n"),
    "packages/ui/tests/local_discovery_panel_model_tests.ts": [
      "local discovery panel hashes search query",
      "index freshness model exposes reindex action without raw scan fallback",
    ].join("\n"),
    "packages/ui/tests/graph_canvas_panel_model_tests.ts": [
      "graph panel uses neighborhood mode with depth limit",
      "canvas viewport model includes only visible nodes",
    ].join("\n"),
    "apps/desktop/src/index.ts": [
      "createDesktopLocalDiscoveryPanel",
      "createDesktopGraphPanel",
      "createDesktopCanvasViewportPanel",
    ].join("\n"),
    "apps/desktop/tests/desktop_discovery_smoke_tests.ts": [
      "desktop local discovery smoke hides raw query and asset content",
      "desktop graph smoke uses neighborhood contract",
      "desktop canvas smoke filters viewport",
    ].join("\n"),
  };
}
