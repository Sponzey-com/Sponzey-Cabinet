import assert from "node:assert/strict";
import test from "node:test";

import {
  DocumentQueryBudgetErrorCode,
  evaluateDocumentQueryBudget,
  measureDocumentQueryBudget,
  renderDocumentQueryBudgetMarkdown,
} from "./phase006_document_query_budget.mjs";

test("document query budget passes when current and history p95 stay under 300ms", () => {
  const result = evaluateDocumentQueryBudget({
    thresholdMs: 300,
    currentReadDurationsMs: [1, 2, 3, 4, 5],
    historyReadDurationsMs: [2, 3, 4, 5, 6],
    fixture: {
      documentCount: 1000,
      historyEntriesPerDocument: 20,
      iterations: 5,
    },
  });
  const markdown = renderDocumentQueryBudgetMarkdown(result);

  assert.equal(result.passed, true);
  assert.equal(result.marker, "phase006_document_query_budget=passed");
  assert.equal(result.currentReadP95Ms, 5);
  assert.equal(result.historyReadP95Ms, 6);
  assert.match(markdown, /current_document_read_p95_ms=5/);
  assert.match(markdown, /history_read_p95_ms=6/);
  assert.doesNotMatch(markdown, /phase006-raw-document-body-should-not-log/);
  assert.doesNotMatch(markdown, /provider_api_key_fixture/);
});

test("document query budget fails when current read exceeds threshold", () => {
  const result = evaluateDocumentQueryBudget({
    thresholdMs: 300,
    currentReadDurationsMs: [1, 2, 301, 302],
    historyReadDurationsMs: [1, 2, 3, 4],
    fixture: {
      documentCount: 1000,
      historyEntriesPerDocument: 20,
      iterations: 4,
    },
  });

  assert.equal(result.passed, false);
  assert.equal(result.marker, "phase006_document_query_budget=failed");
  assert.equal(result.errorCode, DocumentQueryBudgetErrorCode.ThresholdExceeded);
});

test("document query budget measurement uses deterministic in-memory indexed reads", () => {
  const result = measureDocumentQueryBudget({
    thresholdMs: 300,
    documentCount: 300,
    historyEntriesPerDocument: 10,
    iterations: 60,
  });

  assert.equal(result.fixture.documentCount, 300);
  assert.equal(result.fixture.historyEntriesPerDocument, 10);
  assert.equal(result.fixture.iterations, 60);
  assert.equal(result.currentReadP95Ms <= 300, true);
  assert.equal(result.historyReadP95Ms <= 300, true);
});
