import assert from "node:assert/strict";
import test from "node:test";

import { createPlatformCapabilityMatrix, type AiAnswerResultView } from "../../../packages/client-core/src/index.ts";
import { createAiQueryPanelViewModel } from "../../../packages/ui/src/index.ts";

test("mobile AI product smoke skeleton displays refusal and citation metadata without connector admin", () => {
  const matrix = createPlatformCapabilityMatrix();
  const result: AiAnswerResultView = {
    jobId: "answer-job-1",
    state: "Refused",
    refusalCode: "insufficient_context",
    freshnessStatus: "stale",
    citations: [
      {
        sourceId: "document-1",
        sourceKind: "document",
        citationReference: "citation:document-1:1",
        freshness: "stale",
      },
    ],
  };

  for (const platform of [matrix.ios, matrix.android]) {
    const model = createAiQueryPanelViewModel({
      workspaceId: "workspace-1",
      question: "mobile ai question",
      result,
    });

    assert.equal(platform.aiQuerySupport, "interactive");
    assert.equal(platform.aiCitationSupport, "view_only");
    assert.equal(platform.connectorAdminSupport, "unsupported");
    assert.equal(model.displayState, "refused");
    assert.equal(model.refusalCode, "insufficient_context");
    assert.equal(JSON.stringify(model).includes("connector_access_token_fixture"), false);
  }
});
