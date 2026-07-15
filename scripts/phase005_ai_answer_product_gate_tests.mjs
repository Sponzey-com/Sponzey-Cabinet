import assert from "node:assert/strict";
import test from "node:test";

import {
  AiAnswerGateErrorCode,
  AiAnswerGateEvent,
  AiAnswerGateState,
  analyzeAiAnswerGateSources,
  renderAiAnswerGateMarkdown,
  transitionAiAnswerGateState,
} from "./phase005_ai_answer_product_gate.mjs";

const completeSources = {
  "crates/cabinet-domain/src/ai.rs":
    "AiQuestion AiAnswerResult AiCitation AiRefusal AiFreshnessStatus AiAnswerJobState transition_ai_answer_job AiSummaryReference AiSummaryTarget AiSummaryResult AiRelatedDocumentRecommendation",
  "crates/cabinet-domain/tests/ai_tests.rs":
    "completed_ai_answer_requires_answer_reference_and_citation ai_answer_job_uses_success_refusal_retry_and_failure_transitions",
  "crates/cabinet-domain/tests/ai_summary_tests.rs":
    "ai_summary_result_requires_summary_reference_citation_and_freshness related_document_recommendation_is_reference_only",
  "crates/cabinet-ports/src/ai.rs":
    "AiProviderPort AiProviderRequest AiPromptReference AiProviderPolicy AiProviderResponse AiAnswerResultStorePort",
  "crates/cabinet-usecases/src/ai.rs":
    "AskKnowledgeBaseUsecase BuildAiPromptReferenceUsecase SummarizeRetrievalContextUsecase SuggestRelatedDocumentsUsecase",
  "crates/cabinet-usecases/tests/ai_usecase_tests.rs":
    "ask_knowledge_base_stores_completed_answer_with_valid_citation ask_knowledge_base_schedules_retry_when_provider_times_out",
  "crates/cabinet-usecases/tests/ai_prompt_builder_tests.rs":
    "prompt_reference_builder_does_not_expose_raw_prompt_or_secret_fixture prompt_reference_builder_rejects_secret_like_job_id",
  "crates/cabinet-usecases/tests/ai_summary_usecase_tests.rs":
    "summarize_retrieval_context_reflects_stale_source_freshness suggest_related_documents_returns_document_candidates_only_and_applies_limit",
  "crates/cabinet-adapters/src/fake_ai_provider.rs":
    "FakeAiProvider AiProviderPort",
  "crates/cabinet-adapters/src/local_ai_answer_store.rs":
    "LocalAiAnswerStore AiAnswerResultStorePort",
  "crates/cabinet-adapters/tests/fake_ai_provider_tests.rs":
    "fake_ai_provider_returns_configured_response_and_counts_calls local_ai_answer_store_cached_status_and_result_lookup_stays_under_300ms",
};

test("AI answer gate marks complete fixture as passed", () => {
  const gate = analyzeAiAnswerGateSources({ sources: completeSources });

  assert.equal(gate.status, "passed");
  assert.equal(gate.summary.covered, 4);
  assert.equal(gate.summary.targetsNeedingWork, 0);
});

test("AI answer gate reports missing prompt and cache evidence", () => {
  const {
    "crates/cabinet-usecases/tests/ai_prompt_builder_tests.rs": _promptTests,
    "crates/cabinet-adapters/tests/fake_ai_provider_tests.rs": _adapterTests,
    ...sources
  } = completeSources;

  const gate = analyzeAiAnswerGateSources({ sources });

  assert.equal(gate.status, "failed");
  assert.equal(gate.nextImplementationTarget.id, "ai_answer_port_usecase_contract");
});

test("AI answer gate state machine rejects invalid transitions", () => {
  const running = transitionAiAnswerGateState(AiAnswerGateState.Pending, AiAnswerGateEvent.Start);
  const passed = transitionAiAnswerGateState(running, AiAnswerGateEvent.Pass);
  const reported = transitionAiAnswerGateState(passed, AiAnswerGateEvent.Report);

  assert.equal(running, AiAnswerGateState.Running);
  assert.equal(passed, AiAnswerGateState.Passed);
  assert.equal(reported, AiAnswerGateState.Reported);
  assert.throws(
    () => transitionAiAnswerGateState(AiAnswerGateState.Pending, AiAnswerGateEvent.Report),
    (error) => error.code === AiAnswerGateErrorCode.InvalidTransition,
  );
});

test("AI answer gate markdown records marker without sensitive raw fixtures", () => {
  const gate = analyzeAiAnswerGateSources({ sources: completeSources });
  const markdown = renderAiAnswerGateMarkdown(gate);

  assert.match(markdown, /# Phase 005 AI Answer Product Gate Result/);
  assert.match(markdown, /phase005_ai_answer_product_gate=passed/);
  assert.doesNotMatch(markdown, /provider_api_key_fixture/);
  assert.doesNotMatch(markdown, /connector_access_token_fixture/);
  assert.doesNotMatch(markdown, /ai_answer_fixture/);
  assert.doesNotMatch(markdown, /raw prompt/);
});
