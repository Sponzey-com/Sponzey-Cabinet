import assert from "node:assert/strict";
import test from "node:test";

import type {
  AiAnswerJobView,
  AiAnswerResultView,
  AiProviderSettingsSummaryView,
  AiRetrievalResultPage,
} from "../../client-core/src/index.ts";
import { createAiQueryPanelViewModel } from "../src/index.ts";

test("AI query panel maps retrieval candidates to citation cards without permission rules", () => {
  const model = createAiQueryPanelViewModel({
    workspaceId: "workspace-1",
    question: "How did the source change?",
    retrieval: retrieval(),
  });

  assert.equal(model.mode, "ai-query");
  assert.equal(model.displayState, "retrieval-ready");
  assert.equal(model.citationCards.length, 1);
  assert.deepEqual(model.citationCards[0], {
    sourceId: "document-1",
    sourceKind: "document",
    sourceTitle: "Source Document",
    citationReference: "citation:document-1:1",
    headingAnchor: "source-heading",
    blockReference: "block-1",
    freshness: "fresh",
    permissionDecision: "allowed",
  });
  assert.equal("permissionRules" in model, false);
});

test("AI query panel keeps provider setup optional and exposes disabled state", () => {
  const notConfigured = createAiQueryPanelViewModel({
    workspaceId: "workspace-1",
    question: "How did the source change?",
    providerSettings: providerSettings("NotConfigured"),
  });
  const disabled = createAiQueryPanelViewModel({
    workspaceId: "workspace-1",
    question: "How did the source change?",
    providerSettings: providerSettings("Disabled"),
  });

  assert.equal(notConfigured.providerState, "NotConfigured");
  assert.equal(notConfigured.providerBlocksLocalWorkspace, false);
  assert.deepEqual(
    notConfigured.providerActions.map((action) => action.id),
    ["open-optional-provider-settings"],
  );
  assert.equal(disabled.displayState, "provider-disabled");
  assert.equal(disabled.providerBlocksLocalWorkspace, false);
});

test("AI query panel maps queued, completed, and refused answer states", () => {
  const queued = createAiQueryPanelViewModel({
    workspaceId: "workspace-1",
    question: "What changed?",
    status: answerStatus("Queued"),
  });
  const completed = createAiQueryPanelViewModel({
    workspaceId: "workspace-1",
    question: "What changed?",
    result: answerResult("Completed"),
  });
  const refused = createAiQueryPanelViewModel({
    workspaceId: "workspace-1",
    question: "What changed?",
    result: answerResult("Refused"),
  });

  assert.equal(queued.displayState, "waiting-for-result");
  assert.equal(completed.displayState, "completed");
  assert.equal(completed.canDisplayAnswer, true);
  assert.equal(refused.displayState, "refused");
  assert.equal(refused.refusalCode, "insufficient_context");
  assert.equal(refused.freshnessStatus, "stale");
});

test("AI query panel does not display completed answer without citations as successful", () => {
  const result: AiAnswerResultView = {
    ...answerResult("Completed"),
    citations: [],
  };

  const model = createAiQueryPanelViewModel({
    workspaceId: "workspace-1",
    question: "What changed?",
    result,
  });

  assert.equal(model.displayState, "invalid-result");
  assert.equal(model.canDisplayAnswer, false);
});

test("AI query panel model excludes prompt, provider, connector, and source raw fixtures", () => {
  const model = createAiQueryPanelViewModel({
    workspaceId: "workspace-1",
    question: "ai_prompt_fixture",
    retrieval: retrieval(),
    result: answerResult("Refused"),
  });
  const serialized = JSON.stringify(model);

  assert.equal(serialized.includes("ai_prompt_fixture"), false);
  assert.equal(serialized.includes("provider_api_key_fixture"), false);
  assert.equal(serialized.includes("connector_access_token_fixture"), false);
  assert.equal(serialized.includes("retrieval_source_text_fixture"), false);
  assert.equal(serialized.includes("providerEndpoint"), false);
});

function retrieval(): AiRetrievalResultPage {
  return {
    queryName: "ai-retrieval",
    workspaceId: "workspace-1",
    textHash: "query-hash-1",
    durationMs: 42,
    candidates: [
      {
        sourceId: "document-1",
        sourceKind: "document",
        sourceTitle: "Source Document",
        citationReference: "citation:document-1:1",
        headingAnchor: "source-heading",
        blockReference: "block-1",
        freshness: "fresh",
        permissionDecision: "allowed",
      },
    ],
  };
}

function answerStatus(state: AiAnswerJobView["state"]): AiAnswerJobView {
  return {
    jobId: "answer-job-1",
    state,
    citationCount: 0,
    freshnessStatus: "unknown",
  };
}

function answerResult(state: AiAnswerResultView["state"]): AiAnswerResultView {
  return {
    jobId: "answer-job-1",
    state,
    answerReference: state === "Completed" ? "answer:answer-job-1" : undefined,
    refusalCode: state === "Refused" ? "insufficient_context" : undefined,
    freshnessStatus: state === "Refused" ? "stale" : "fresh",
    citations: [
      {
        sourceId: "document-1",
        sourceKind: "document",
        sourceTitle: "Source Document",
        citationReference: "citation:document-1:1",
        headingAnchor: "source-heading",
        blockReference: "block-1",
        freshness: state === "Refused" ? "stale" : "fresh",
        permissionDecision: "allowed",
      },
    ],
  };
}

function providerSettings(
  state: AiProviderSettingsSummaryView["state"],
): AiProviderSettingsSummaryView {
  return {
    state,
    credentialHandlePresent: false,
  };
}
