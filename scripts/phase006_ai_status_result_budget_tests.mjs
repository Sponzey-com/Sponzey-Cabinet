import assert from "node:assert/strict";
import test from "node:test";

import {
  AiStatusResultBudgetErrorCode,
  evaluateAiStatusResultBudget,
  measureAiStatusResultBudget,
  renderAiStatusResultBudgetMarkdown,
} from "./phase006_ai_status_result_budget.mjs";

test("AI status result budget passes when cached lookups stay under 300ms", () => {
  const result = measureAiStatusResultBudget({
    thresholdMs: 300,
    jobCount: 1000,
    iterations: 200,
  });

  assert.equal(result.passed, true);
  assert.equal(result.marker, "phase006_ai_status_result_budget=passed");
  assert.ok(result.aiStatusReadP95Ms <= 300);
  assert.ok(result.aiResultReadP95Ms <= 300);
});

test("AI status result budget fails when result lookup exceeds threshold", () => {
  const result = evaluateAiStatusResultBudget({
    thresholdMs: 1,
    fixture: { jobCount: 2, iterations: 2 },
    measurements: {
      statusMs: [0.1, 0.2],
      resultMs: [1.5, 2.0],
    },
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, AiStatusResultBudgetErrorCode.ThresholdExceeded);
});

test("AI status result budget markdown excludes prompt answer provider secrets", () => {
  const markdown = renderAiStatusResultBudgetMarkdown(
    evaluateAiStatusResultBudget({
      thresholdMs: 300,
      fixture: { jobCount: 2, iterations: 2 },
      measurements: {
        statusMs: [0.1, 0.2],
        resultMs: [0.1, 0.2],
      },
    }),
  );

  assert.match(markdown, /phase006_ai_status_result_budget=passed/);
  assert.doesNotMatch(markdown, /phase005-ai-prompt-raw-text-should-not-log/);
  assert.doesNotMatch(markdown, /phase005-ai-answer-raw-text-should-not-log/);
  assert.doesNotMatch(markdown, /phase005-retrieval-source-text-should-not-log/);
  assert.doesNotMatch(markdown, /phase005-provider-api-key-should-not-log/);
  assert.doesNotMatch(markdown, /provider_api_key_fixture/);
  assert.doesNotMatch(markdown, /endpoint/);
});
