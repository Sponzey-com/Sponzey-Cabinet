import assert from "node:assert/strict";
import test from "node:test";

import { createPlatformCapabilityMatrix, type AiAnswerResultView } from "../../../packages/client-core/src/index.ts";
import { createAiQueryPanelViewModel } from "../../../packages/ui/src/index.ts";

test("desktop AI product smoke skeleton displays completed answer with citations", () => {
  const matrix = createPlatformCapabilityMatrix();
  const result: AiAnswerResultView = {
    jobId: "answer-job-1",
    state: "Completed",
    answerReference: "answer:answer-job-1",
    freshnessStatus: "fresh",
    citations: [
      {
        sourceId: "document-1",
        sourceKind: "document",
        citationReference: "citation:document-1:1",
        freshness: "fresh",
      },
    ],
  };

  const model = createAiQueryPanelViewModel({
    workspaceId: "workspace-1",
    question: "ai_prompt_fixture",
    result,
  });

  assert.equal(matrix.desktop.aiQuerySupport, "interactive");
  assert.equal(matrix.desktop.aiCitationSupport, "interactive");
  assert.equal(model.displayState, "completed");
  assert.equal(model.canDisplayAnswer, true);
  assert.equal(model.citationCards.length, 1);
  assert.equal(JSON.stringify(model).includes("ai_prompt_fixture"), false);
  assert.equal(JSON.stringify(model).includes("provider_api_key_fixture"), false);
});
