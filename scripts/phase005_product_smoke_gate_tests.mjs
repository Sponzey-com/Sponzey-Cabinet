import assert from "node:assert/strict";
import test from "node:test";

import {
  ProductSmokeGateErrorCode,
  ProductSmokeGateEvent,
  ProductSmokeGateState,
  analyzeProductSmokeGateSources,
  renderProductSmokeGateMarkdown,
  transitionProductSmokeGateState,
} from "./phase005_product_smoke_gate.mjs";

const completeSources = {
  "packages/client-core/src/index.ts":
    "CabinetAiApiClient AiRetrievalResultPage AskKnowledgeBaseCommand AiAnswerResultView createPlatformCapabilityMatrix aiQuerySupport aiCitationSupport connectorAdminSupport",
  "packages/client-core/tests/ai_api_client_tests.ts":
    "self-host AI client sends retrieval, answer, status, and result requests through explicit config AI answer result DTO carries citation, refusal, and freshness without provider secrets",
  "packages/client-core/tests/ai_capability_matrix_tests.ts":
    "platform capability matrix documents AI query, citation, and connector admin support",
  "packages/ui/src/index.ts":
    "createAiQueryPanelViewModel AiQueryPanelViewModel AiCitationCardViewModel",
  "packages/ui/tests/ai_query_ui_model_tests.ts":
    "AI query panel maps retrieval candidates to citation cards without permission rules AI query panel does not display completed answer without citations as successful AI query panel model excludes prompt, provider, connector, and source raw fixtures",
  "apps/desktop/tests/desktop_ai_product_smoke_tests.ts":
    "desktop AI product smoke skeleton displays completed answer with citations",
  "apps/mobile/tests/mobile_ai_product_smoke_tests.ts":
    "mobile AI product smoke skeleton displays refusal and citation metadata without connector admin",
  ".tasks/ai-answer-product-gate-result.md": "phase005_ai_answer_product_gate=passed",
  ".tasks/mcp-api-product-gate-result.md": "phase005_mcp_api_product_gate=passed",
  ".tasks/webhook-connector-product-gate-result.md":
    "phase005_webhook_connector_product_gate=passed",
};

test("product smoke gate marks complete fixture as passed", () => {
  const gate = analyzeProductSmokeGateSources({ sources: completeSources });

  assert.equal(gate.status, "passed");
  assert.equal(gate.summary.covered, 4);
  assert.equal(gate.summary.targetsNeedingWork, 0);
});

test("product smoke gate reports missing desktop mobile evidence", () => {
  const {
    "apps/desktop/tests/desktop_ai_product_smoke_tests.ts": _desktop,
    "apps/mobile/tests/mobile_ai_product_smoke_tests.ts": _mobile,
    ...sources
  } = completeSources;

  const gate = analyzeProductSmokeGateSources({ sources });

  assert.equal(gate.status, "failed");
  assert.equal(gate.nextImplementationTarget.id, "desktop_mobile_ai_smoke");
});

test("product smoke gate state machine rejects invalid transitions", () => {
  const running = transitionProductSmokeGateState(
    ProductSmokeGateState.Pending,
    ProductSmokeGateEvent.Start,
  );
  const passed = transitionProductSmokeGateState(running, ProductSmokeGateEvent.Pass);
  const reported = transitionProductSmokeGateState(passed, ProductSmokeGateEvent.Report);

  assert.equal(running, ProductSmokeGateState.Running);
  assert.equal(passed, ProductSmokeGateState.Passed);
  assert.equal(reported, ProductSmokeGateState.Reported);
  assert.throws(
    () =>
      transitionProductSmokeGateState(
        ProductSmokeGateState.Pending,
        ProductSmokeGateEvent.Report,
      ),
    (error) => error.code === ProductSmokeGateErrorCode.InvalidTransition,
  );
});

test("product smoke gate markdown excludes sensitive raw fixtures", () => {
  const gate = analyzeProductSmokeGateSources({ sources: completeSources });
  const markdown = renderProductSmokeGateMarkdown(gate);

  assert.match(markdown, /# Phase 005 Product Smoke Gate Result/);
  assert.match(markdown, /phase005_product_smoke_gate=passed/);
  assert.doesNotMatch(markdown, /ai_prompt_fixture/);
  assert.doesNotMatch(markdown, /ai_answer_fixture/);
  assert.doesNotMatch(markdown, /provider_api_key_fixture/);
  assert.doesNotMatch(markdown, /connector_access_token_fixture/);
  assert.doesNotMatch(markdown, /retrieval_source_text_fixture/);
});
