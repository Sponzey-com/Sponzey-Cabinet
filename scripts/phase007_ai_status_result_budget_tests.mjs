import assert from "node:assert/strict";
import test from "node:test";

import {
  AiStatusResultBudgetErrorCode,
  evaluateAiStatusResultBudget,
  measureAiStatusResultBudget,
  renderAiStatusResultBudgetMarkdown,
} from "./phase007_ai_status_result_budget.mjs";

test("Phase 007 AI cached status and result budget passes under 300ms", () => {
  const result = measureAiStatusResultBudget({
    jobCount: 1000,
    citationCount: 1000,
    iterations: 100,
  });
  const markdown = renderAiStatusResultBudgetMarkdown(result);

  assert.equal(result.passed, true);
  assert.equal(result.marker, "phase007_ai_status_result_budget=passed");
  assert.ok(result.statusReadP95Ms <= 300);
  assert.ok(result.resultReadP95Ms <= 300);
  assert.match(markdown, /fixture_job_count=1000/);
  assert.doesNotMatch(markdown, /ai_prompt_fixture/);
  assert.doesNotMatch(markdown, /ai_answer_fixture/);
  assert.doesNotMatch(markdown, /provider_api_key_fixture/);
});

test("Phase 007 AI cached status and result budget fails when p95 exceeds threshold", () => {
  const result = evaluateAiStatusResultBudget({
    thresholdMs: 300,
    measurements: {
      statusMs: [1, 2, 3],
      resultMs: [301, 302, 303],
    },
    fixture: { jobCount: 10, citationCount: 10, iterations: 3 },
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, AiStatusResultBudgetErrorCode.ThresholdExceeded);
});

test("Phase 007 AI cached status and result budget rejects empty measurements", () => {
  const result = evaluateAiStatusResultBudget({
    thresholdMs: 300,
    measurements: {
      statusMs: [],
      resultMs: [1],
    },
    fixture: { jobCount: 10, citationCount: 10, iterations: 1 },
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, AiStatusResultBudgetErrorCode.EmptyMeasurement);
});
