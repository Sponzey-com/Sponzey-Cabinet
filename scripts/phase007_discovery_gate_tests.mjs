import assert from "node:assert/strict";
import test from "node:test";

import {
  DiscoveryGateErrorCode,
  evaluateDiscoveryGate,
  renderDiscoveryGateResult,
} from "./phase007_discovery_gate.mjs";

test("discovery gate rejects missing local persistence prerequisite", () => {
  const result = evaluateDiscoveryGate({
    localPersistenceText: "phase007_local_persistence_gate=failed",
    performanceBudgetText: "phase007_performance_budget=passed",
    discoveryEvidence: completeDiscoveryEvidence(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, DiscoveryGateErrorCode.LocalPersistenceMissing);
});

test("discovery gate rejects missing Phase 007 performance budget", () => {
  const result = evaluateDiscoveryGate({
    localPersistenceText: "phase007_local_persistence_gate=passed",
    performanceBudgetText: "phase007_performance_budget=failed",
    discoveryEvidence: completeDiscoveryEvidence(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, DiscoveryGateErrorCode.PerformanceBudgetMissing);
});

test("discovery gate rejects full workspace graph scan evidence", () => {
  const result = evaluateDiscoveryGate({
    localPersistenceText: "phase007_local_persistence_gate=passed",
    performanceBudgetText: "phase007_performance_budget=passed",
    discoveryEvidence: { ...completeDiscoveryEvidence(), fullWorkspaceGraphScan: true },
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, DiscoveryGateErrorCode.FullWorkspaceGraphScan);
});

test("discovery gate passes complete evidence and renders safe marker", () => {
  const result = evaluateDiscoveryGate({
    localPersistenceText: "phase007_local_persistence_gate=passed",
    performanceBudgetText: "phase007_performance_budget=passed",
    discoveryEvidence: completeDiscoveryEvidence(),
  });
  const rendered = renderDiscoveryGateResult(result);

  assert.equal(result.passed, true);
  assert.match(rendered, /phase007_discovery_gate=passed/);
  assert.match(rendered, /search_query_budget=300ms/);
  assert.doesNotMatch(rendered, /raw query should not appear/);
  assert.doesNotMatch(rendered, /asset binary content should not leak/);
});

function completeDiscoveryEvidence() {
  return {
    searchQueryName: "search-documents",
    linkQueryName: "get-link-overview",
    assetQueryName: "list-document-assets",
    graphLoadMode: "neighborhood",
    fullWorkspaceGraphScan: false,
    canvasLoadMode: "viewport",
    searchActions: ["open-document", "ask-ai"],
    indexFreshnessVisible: true,
    rawContentExcluded: true,
  };
}
