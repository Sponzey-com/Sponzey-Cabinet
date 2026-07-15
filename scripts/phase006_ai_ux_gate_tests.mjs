import assert from "node:assert/strict";
import test from "node:test";

import {
  AiUxGateErrorCode,
  AiUxGateEvent,
  AiUxGateState,
  analyzeAiUxEvidence,
  renderAiUxGateMarkdown,
  transitionAiUxGateState,
} from "./phase006_ai_ux_gate.mjs";

test("AI UX gate reports complete evidence as passed", () => {
  const result = analyzeAiUxEvidence({ sources: completeSources() });
  const markdown = renderAiUxGateMarkdown(result);

  assert.equal(result.passed, true);
  assert.equal(result.marker, "phase006_ai_ux_gate=passed");
  assert.match(markdown, /phase006_ai_ux_gate=passed/);
  assert.doesNotMatch(markdown, /phase005-ai-prompt-raw-text-should-not-log/);
  assert.doesNotMatch(markdown, /phase005-ai-answer-raw-text-should-not-log/);
  assert.doesNotMatch(markdown, /phase005-provider-api-key-should-not-log/);
  assert.doesNotMatch(markdown, /providerEndpoint/);
});

test("AI UX gate fails when search graph asset prerequisite is missing", () => {
  const sources = completeSources();
  sources[".tasks/phase006-search-graph-asset-gate-result.md"] =
    "phase006_search_graph_asset_gate=failed";

  const result = analyzeAiUxEvidence({ sources });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, AiUxGateErrorCode.RequiredEvidenceMissing);
  assert.equal(result.missingEvidence[0].targetId, "search_graph_asset_prerequisite");
});

test("AI UX gate fails when AI budget marker is missing", () => {
  const sources = completeSources();
  sources[".tasks/release/performance-budget-phase006.md"] =
    "phase006_ai_status_result_budget=failed";

  const result = analyzeAiUxEvidence({ sources });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, AiUxGateErrorCode.RequiredEvidenceMissing);
  assert.equal(result.missingEvidence[0].targetId, "ai_status_result_performance_budget");
});

test("AI UX gate fails when citation source or tool scope evidence is missing", () => {
  const sources = completeSources();
  sources["packages/ui/src/index.ts"] = "createAiQueryPanelViewModel";

  const result = analyzeAiUxEvidence({ sources });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, AiUxGateErrorCode.RequiredEvidenceMissing);
  assert.equal(result.missingEvidence[0].targetId, "ai_query_citation_tool_scope_ui_models");
});

test("AI UX gate state machine exposes explicit transitions", () => {
  const reading = transitionAiUxGateState(AiUxGateState.Pending, AiUxGateEvent.Start);
  const validating = transitionAiUxGateState(reading.state, AiUxGateEvent.SourcesLoaded);
  const writing = transitionAiUxGateState(validating.state, AiUxGateEvent.EvidenceValidated);
  const passed = transitionAiUxGateState(writing.state, AiUxGateEvent.ReportWritten);
  const invalid = transitionAiUxGateState(AiUxGateState.Pending, AiUxGateEvent.ReportWritten);

  assert.equal(reading.state, AiUxGateState.ReadingSources);
  assert.equal(validating.state, AiUxGateState.ValidatingEvidence);
  assert.equal(writing.state, AiUxGateState.WritingReport);
  assert.equal(passed.state, AiUxGateState.Passed);
  assert.equal(invalid.errorCode, AiUxGateErrorCode.InvalidTransition);
});

function completeSources() {
  return {
    ".tasks/phase006-search-graph-asset-gate-result.md": "phase006_search_graph_asset_gate=passed",
    ".tasks/phase005/ai-answer-product-gate-result.md": [
      "phase005_ai_answer_product_gate=passed",
      "deterministic fake provider and local cached answer store complete",
    ].join("\n"),
    ".tasks/release/performance-budget-phase006.md": [
      "phase006_ai_status_result_budget=passed",
      "ai_status_read_p95_ms=3",
      "ai_result_read_p95_ms=3",
    ].join("\n"),
    "packages/client-core/src/index.ts": [
      "AiAnswerResultView",
      "LocalAiToolDescriptorView",
      "AiProviderSettingsSummaryView",
    ].join("\n"),
    "packages/client-core/tests/ai_api_client_tests.ts": [
      "AI API client config does not require provider endpoint, model, or key",
      "AI answer result DTO carries citation, refusal, and freshness without provider secrets",
    ].join("\n"),
    "packages/ui/src/index.ts": [
      "createAiQueryPanelViewModel",
      "createAiCitationSourceOpenAction",
      "createLocalAiToolScopeViewModel",
      "createAiProviderSettingsViewModel",
    ].join("\n"),
    "packages/ui/tests/ai_query_ui_model_tests.ts": [
      "AI query panel does not display completed answer without citations as successful",
      "AI query panel model excludes prompt, provider, connector, and source raw fixtures",
    ].join("\n"),
    "packages/ui/tests/ai_citation_tool_scope_model_tests.ts": [
      "AI citation source open action separates current document and version reads",
      "local AI tool scope view hides server admin and destructive tools",
      "AI provider settings model is optional and excludes credentials",
    ].join("\n"),
    "apps/desktop/src/index.ts": [
      "createDesktopAiCitationSourceOpenAction",
      "createDesktopLocalAiToolScope",
      "createDesktopAiProviderSettings",
    ].join("\n"),
    "apps/desktop/tests/desktop_ai_product_smoke_tests.ts": [
      "desktop AI product smoke skeleton displays completed answer with citations",
    ].join("\n"),
    "apps/desktop/tests/desktop_ai_local_ux_smoke_tests.ts": [
      "desktop AI local UX smoke separates citation source current and history opens",
      "desktop AI local UX smoke exposes read-only tool scope",
      "desktop AI local UX smoke keeps provider setup optional and secret-free",
    ].join("\n"),
  };
}
