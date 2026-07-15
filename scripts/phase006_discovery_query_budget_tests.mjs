import assert from "node:assert/strict";
import test from "node:test";

import {
  DiscoveryQueryBudgetErrorCode,
  evaluateDiscoveryQueryBudget,
  measureDiscoveryQueryBudget,
  renderDiscoveryQueryBudgetMarkdown,
} from "./phase006_discovery_query_budget.mjs";

test("discovery query budget passes when every p95 stays under 300ms", () => {
  const result = evaluateDiscoveryQueryBudget({
    thresholdMs: 300,
    measurements: {
      searchMs: [1, 2, 3],
      linkMs: [1, 2, 3],
      assetMs: [1, 2, 3],
      graphMs: [1, 2, 3],
      canvasMs: [1, 2, 3],
    },
    fixture: { documentCount: 1000, assetCount: 1000, graphNodeCount: 1000, iterations: 3 },
  });
  const markdown = renderDiscoveryQueryBudgetMarkdown(result);

  assert.equal(result.passed, true);
  assert.equal(result.marker, "phase006_search_graph_asset_budget=passed");
  assert.match(markdown, /search_query_p95_ms=3/);
  assert.match(markdown, /graph_lookup_p95_ms=3/);
  assert.doesNotMatch(markdown, /raw query should not appear/);
  assert.doesNotMatch(markdown, /asset binary content should not leak/);
  assert.doesNotMatch(markdown, /canvas_raw_ui_state/);
});

test("discovery query budget fails when graph lookup exceeds threshold", () => {
  const result = evaluateDiscoveryQueryBudget({
    thresholdMs: 300,
    measurements: {
      searchMs: [1, 2, 3],
      linkMs: [1, 2, 3],
      assetMs: [1, 2, 3],
      graphMs: [301, 302, 303],
      canvasMs: [1, 2, 3],
    },
    fixture: { documentCount: 1000, assetCount: 1000, graphNodeCount: 1000, iterations: 3 },
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, DiscoveryQueryBudgetErrorCode.ThresholdExceeded);
});

test("discovery query budget measurement uses indexed deterministic fixtures", () => {
  const result = measureDiscoveryQueryBudget({
    thresholdMs: 300,
    documentCount: 300,
    assetCount: 300,
    graphNodeCount: 300,
    iterations: 60,
  });

  assert.equal(result.passed, true);
  assert.equal(result.fixture.documentCount, 300);
  assert.equal(result.fixture.iterations, 60);
});
