import assert from "node:assert/strict";
import test from "node:test";

import {
  DiscoveryQueryBudgetErrorCode,
  evaluateDiscoveryQueryBudget,
  measureDiscoveryQueryBudget,
  renderDiscoveryQueryBudgetMarkdown,
} from "./phase007_discovery_query_budget.mjs";

test("Phase 007 discovery query budget passes indexed deterministic lookups under 300ms", () => {
  const result = measureDiscoveryQueryBudget({
    documentCount: 1000,
    assetCount: 1000,
    graphNodeCount: 1000,
    iterations: 100,
  });
  const markdown = renderDiscoveryQueryBudgetMarkdown(result);

  assert.equal(result.passed, true);
  assert.equal(result.marker, "phase007_performance_budget=passed");
  assert.ok(result.searchQueryP95Ms <= 300);
  assert.ok(result.linkOverviewP95Ms <= 300);
  assert.ok(result.assetMetadataP95Ms <= 300);
  assert.ok(result.graphLookupP95Ms <= 300);
  assert.ok(result.canvasViewportP95Ms <= 300);
  assert.match(markdown, /fixture_document_count=1000/);
  assert.doesNotMatch(markdown, /raw query should not appear/);
  assert.doesNotMatch(markdown, /asset binary content should not leak/);
});

test("Phase 007 discovery query budget fails when any p95 exceeds threshold", () => {
  const result = evaluateDiscoveryQueryBudget({
    thresholdMs: 300,
    measurements: {
      searchMs: [1, 2, 3],
      linkMs: [1, 2, 3],
      assetMs: [1, 2, 3],
      graphMs: [301, 302, 303],
      canvasMs: [1, 2, 3],
    },
    fixture: { documentCount: 10, assetCount: 10, graphNodeCount: 10, iterations: 3 },
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, DiscoveryQueryBudgetErrorCode.ThresholdExceeded);
  assert.equal(result.marker, "phase007_performance_budget=failed");
});

test("Phase 007 discovery query budget rejects empty measurement buckets", () => {
  const result = evaluateDiscoveryQueryBudget({
    thresholdMs: 300,
    measurements: {
      searchMs: [],
      linkMs: [1],
      assetMs: [1],
      graphMs: [1],
      canvasMs: [1],
    },
    fixture: { documentCount: 10, assetCount: 10, graphNodeCount: 10, iterations: 1 },
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, DiscoveryQueryBudgetErrorCode.EmptyMeasurement);
});
