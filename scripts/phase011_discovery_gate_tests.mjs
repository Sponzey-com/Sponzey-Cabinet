import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import test from "node:test";

import {
  DiscoveryGateErrorCode,
  analyzeDiscoveryGateEvidence,
  renderDiscoveryGateMarkdown,
  renderDiscoveryPerformanceBudget,
  runDiscoveryGate,
} from "./phase011_discovery_gate.mjs";

test("phase011 discovery gate rejects missing required evidence", () => {
  const result = analyzeDiscoveryGateEvidence({
    sourceFingerprint: "a".repeat(64),
    sources: {
      ".tasks/phase011-history-restore-gate-result.md": "phase011_history_restore_gate=passed",
    },
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, DiscoveryGateErrorCode.RequiredEvidenceMissing);
  assert.equal(result.missingEvidence.length > 0, true);
});

test("phase011 discovery gate passes complete sanitized source evidence", () => {
  const result = analyzeDiscoveryGateEvidence({
    sourceFingerprint: "b".repeat(64),
    sources: completeSources(),
  });
  const artifact = renderDiscoveryGateMarkdown(result);
  const budget = renderDiscoveryPerformanceBudget(result);

  assert.equal(result.passed, true);
  assert.equal(result.marker, "phase011_discovery_gate=passed");
  assert.match(artifact, /requirements=DISC-01,STATE-01,PERF-01,SEC-01/);
  assert.match(artifact, /graph_neighborhood_bounded=true/);
  assert.match(budget, /phase011_performance_budget=passed/);
  assert.match(budget, /search_query/);
  assert.doesNotMatch(artifact, /raw query should not appear/i);
  assert.doesNotMatch(budget, /asset binary content should not leak/i);
});

test("phase011 discovery gate writes marker and performance budget under explicit root", async () => {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase011-discovery-"));
  await mkdir(join(root, ".tasks/release"), { recursive: true });
  for (const [filePath, text] of Object.entries(completeSources())) {
    await mkdir(join(root, filePath.split("/").slice(0, -1).join("/")), { recursive: true });
    await writeFile(join(root, filePath), text);
  }
  await writeFile(
    join(root, ".tasks/phase011-current-implementation-inventory.md"),
    `source_fingerprint=${"c".repeat(64)}\n`,
  );

  const result = await runDiscoveryGate({ root });

  assert.equal(result.passed, true);
  assert.equal(result.sourceFingerprint, "c".repeat(64));
});

function completeSources() {
  return {
    ".tasks/phase011-history-restore-gate-result.md": "phase011_history_restore_gate=passed",
    "packages/ui/src/index.ts": [
      "export type DiscoveryWorkflowState = 'Idle' | 'Searching' | 'ResultsReady' | 'NoResults' | 'IndexStale' | 'Repairing' | 'RepairSucceeded' | 'RepairFailed';",
      "export function createDiscoveryQueryPolicy() {}",
      "workflowState",
      "fullWorkspaceScan: false",
    ].join("\n"),
    "packages/ui/tests/local_discovery_panel_model_tests.ts": [
      "local discovery workflow state exposes stale repairing and failed index states",
      "discovery query policy clamps query and graph limits",
    ].join("\n"),
    "apps/desktop/src/index.ts": [
      "createDesktopLocalDiscoveryPanel",
      "createDesktopGraphPanel",
      "createDesktopCanvasViewportPanel",
    ].join("\n"),
    "apps/desktop/tests/desktop_discovery_smoke_tests.ts": [
      "desktop local discovery smoke hides raw query and asset content",
      "desktop graph smoke uses neighborhood contract",
    ].join("\n"),
    "packages/ui/tests/graph_canvas_panel_model_tests.ts": [
      "graph panel uses neighborhood mode with depth limit",
    ].join("\n"),
    "crates/cabinet-usecases/tests/search_documents_tests.rs": "search_documents_delegates_query_to_search_index",
    "crates/cabinet-usecases/tests/graph_lite_projection_tests.rs": "graph_lite_projection_includes_incoming_outgoing_and_unresolved_depth_one_nodes",
    "crates/cabinet-usecases/tests/list_document_assets_tests.rs": "list_document_assets_returns_metadata_and_reference_without_asset_object_store",
    "crates/cabinet-adapters/tests/local_search_index_tests.rs": "local_search_index",
    "crates/cabinet-adapters/tests/local_link_index_tests.rs": "local_link_index_queries_unresolved_links_and_orphan_documents",
    "crates/cabinet-adapters/tests/local_graph_projection_store_tests.rs": "local_graph_projection_store_keeps_workspace_projections_separate",
    "crates/cabinet-adapters/tests/local_document_asset_repository_tests.rs": "local_document_asset_repository",
  };
}
