import { readFile, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

export const AiUxGateState = Object.freeze({
  Pending: "Pending",
  ReadingSources: "ReadingSources",
  ValidatingEvidence: "ValidatingEvidence",
  WritingReport: "WritingReport",
  Passed: "Passed",
  Failed: "Failed",
});

export const AiUxGateEvent = Object.freeze({
  Start: "Start",
  SourcesLoaded: "SourcesLoaded",
  EvidenceValidated: "EvidenceValidated",
  ReportWritten: "ReportWritten",
  Fail: "Fail",
});

export const AiUxGateErrorCode = Object.freeze({
  RequiredEvidenceMissing: "PHASE006_AI_UX_REQUIRED_EVIDENCE_MISSING",
  SourceReadFailed: "PHASE006_AI_UX_SOURCE_READ_FAILED",
  InvalidTransition: "PHASE006_AI_UX_INVALID_TRANSITION",
});

const requiredTargets = Object.freeze([
  target("search_graph_asset_prerequisite", "Phase 006 search graph asset gate prerequisite", {
    requiredFiles: [".tasks/phase006-search-graph-asset-gate-result.md"],
    evidence: ["phase006_search_graph_asset_gate=passed"],
  }),
  target("phase005_fake_provider_baseline", "Phase 005 fake provider and cached answer baseline", {
    requiredFiles: [".tasks/phase005/ai-answer-product-gate-result.md"],
    evidence: [
      "phase005_ai_answer_product_gate=passed",
      "deterministic fake provider and local cached answer store complete",
    ],
  }),
  target("ai_status_result_performance_budget", "AI status and result p95 performance budget", {
    requiredFiles: [".tasks/release/performance-budget-phase006.md"],
    evidence: [
      "phase006_ai_status_result_budget=passed",
      "ai_status_read_p95_ms=",
      "ai_result_read_p95_ms=",
    ],
  }),
  target("client_core_ai_contract", "client-core AI DTO and optional provider contract", {
    requiredFiles: [
      "packages/client-core/src/index.ts",
      "packages/client-core/tests/ai_api_client_tests.ts",
    ],
    evidence: [
      "AiAnswerResultView",
      "LocalAiToolDescriptorView",
      "AiProviderSettingsSummaryView",
      "AI API client config does not require provider endpoint, model, or key",
      "AI answer result DTO carries citation, refusal, and freshness without provider secrets",
    ],
  }),
  target("ai_query_citation_tool_scope_ui_models", "AI query citation tool scope UI models", {
    requiredFiles: [
      "packages/ui/src/index.ts",
      "packages/ui/tests/ai_query_ui_model_tests.ts",
      "packages/ui/tests/ai_citation_tool_scope_model_tests.ts",
    ],
    evidence: [
      "createAiQueryPanelViewModel",
      "createAiCitationSourceOpenAction",
      "createLocalAiToolScopeViewModel",
      "createAiProviderSettingsViewModel",
      "AI query panel does not display completed answer without citations as successful",
      "AI query panel model excludes prompt, provider, connector, and source raw fixtures",
      "AI citation source open action separates current document and version reads",
      "local AI tool scope view hides server admin and destructive tools",
      "AI provider settings model is optional and excludes credentials",
    ],
  }),
  target("desktop_ai_local_ux_smoke", "desktop local AI UX smoke", {
    requiredFiles: [
      "apps/desktop/src/index.ts",
      "apps/desktop/tests/desktop_ai_product_smoke_tests.ts",
      "apps/desktop/tests/desktop_ai_local_ux_smoke_tests.ts",
    ],
    evidence: [
      "createDesktopAiCitationSourceOpenAction",
      "createDesktopLocalAiToolScope",
      "createDesktopAiProviderSettings",
      "desktop AI product smoke skeleton displays completed answer with citations",
      "desktop AI local UX smoke separates citation source current and history opens",
      "desktop AI local UX smoke exposes read-only tool scope",
      "desktop AI local UX smoke keeps provider setup optional and secret-free",
    ],
  }),
]);

export function transitionAiUxGateState(currentState, event, detail = {}) {
  if (currentState === AiUxGateState.Pending && event === AiUxGateEvent.Start) {
    return { state: AiUxGateState.ReadingSources };
  }
  if (currentState === AiUxGateState.ReadingSources && event === AiUxGateEvent.SourcesLoaded) {
    return { state: AiUxGateState.ValidatingEvidence };
  }
  if (
    currentState === AiUxGateState.ValidatingEvidence &&
    event === AiUxGateEvent.EvidenceValidated
  ) {
    return { state: AiUxGateState.WritingReport };
  }
  if (currentState === AiUxGateState.WritingReport && event === AiUxGateEvent.ReportWritten) {
    return { state: AiUxGateState.Passed };
  }
  if (
    [AiUxGateState.ReadingSources, AiUxGateState.ValidatingEvidence, AiUxGateState.WritingReport].includes(
      currentState,
    ) &&
    event === AiUxGateEvent.Fail
  ) {
    return {
      state: AiUxGateState.Failed,
      errorCode: detail.errorCode ?? AiUxGateErrorCode.RequiredEvidenceMissing,
      findingId: detail.findingId,
    };
  }
  return {
    state: AiUxGateState.Failed,
    errorCode: AiUxGateErrorCode.InvalidTransition,
  };
}

export function analyzeAiUxEvidence({ sources }) {
  if (!sources || Object.keys(sources).length === 0) {
    return failedResult({
      errorCode: AiUxGateErrorCode.SourceReadFailed,
      missingEvidence: [{ targetId: "source_set", missing: ["sources"] }],
    });
  }
  const targetResults = requiredTargets.map((entry) => analyzeTarget(entry, sources));
  const missingEvidence = targetResults
    .filter((entry) => entry.missing.length > 0)
    .map((entry) => ({ targetId: entry.id, missing: entry.missing }));
  if (missingEvidence.length > 0) {
    return failedResult({
      errorCode: AiUxGateErrorCode.RequiredEvidenceMissing,
      missingEvidence,
      targetResults,
    });
  }
  return {
    passed: true,
    marker: "phase006_ai_ux_gate=passed",
    state: AiUxGateState.Passed,
    summary: { requiredTargets: requiredTargets.length, missingRequiredEvidence: 0 },
    targetResults,
    missingEvidence: [],
  };
}

export function renderAiUxGateMarkdown(result) {
  const lines = [
    "# Phase 006 AI UX Gate Result",
    "",
    result.marker,
    "",
    "- phase: `Phase 006`",
    "- gate: `Local AI Query, Citation, and Tool Scope UX`",
    `- status: \`${result.passed ? "passed" : "failed"}\``,
    `- state: \`${result.state}\``,
  ];
  if (!result.passed) {
    lines.push(`- error code: \`${result.errorCode}\``);
  }
  lines.push(
    `- required targets: \`${result.summary.requiredTargets}\``,
    `- missing required evidence: \`${result.summary.missingRequiredEvidence}\``,
    "",
    "## Evidence",
    "",
    "| Target | Status | Description |",
    "| --- | --- | --- |",
  );
  for (const targetResult of result.targetResults ?? []) {
    lines.push(`| \`${targetResult.id}\` | \`${targetResult.status}\` | ${targetResult.description} |`);
  }
  lines.push(
    "",
    "## Sensitive Data Exclusion",
    "",
    "This artifact records target ids, marker status, counts, and stable error codes only. It does not record raw prompt, raw generated response, retrieval source text, provider secret, credential, token, provider network address, or personal absolute path.",
    "",
    "## Manual Provider Boundary",
    "",
    "- External provider validation is optional and manual.",
    "- The default gate uses deterministic local/fake provider evidence and does not call external AI services.",
    "",
  );
  return lines.join("\n");
}

export async function runAiUxGate({ root = process.cwd() } = {}) {
  let state = transitionAiUxGateState(AiUxGateState.Pending, AiUxGateEvent.Start);
  try {
    const sources = {};
    for (const filePath of collectRequiredFiles()) {
      sources[filePath] = await readFile(`${root}/${filePath}`, "utf8");
    }
    state = transitionAiUxGateState(state.state, AiUxGateEvent.SourcesLoaded);
    const result = analyzeAiUxEvidence({ sources });
    if (!result.passed) {
      state = transitionAiUxGateState(state.state, AiUxGateEvent.Fail, {
        errorCode: result.errorCode,
        findingId: result.missingEvidence[0]?.targetId,
      });
      return { ...result, state: state.state };
    }
    state = transitionAiUxGateState(state.state, AiUxGateEvent.EvidenceValidated);
    state = transitionAiUxGateState(state.state, AiUxGateEvent.ReportWritten);
    return { ...result, state: state.state };
  } catch {
    state = transitionAiUxGateState(state.state, AiUxGateEvent.Fail, {
      errorCode: AiUxGateErrorCode.SourceReadFailed,
    });
    return failedResult({
      errorCode: state.errorCode,
      state: state.state,
      missingEvidence: [{ targetId: "source_read", missing: ["required source file"] }],
    });
  }
}

function analyzeTarget(entry, sources) {
  const texts = entry.requiredFiles.map((filePath) => sources[filePath] ?? "");
  const missingFiles = entry.requiredFiles.filter((filePath) => !(filePath in sources));
  const missingEvidence = entry.evidence.filter((needle) => !texts.some((text) => text.includes(needle)));
  const missing = [...missingFiles, ...missingEvidence];
  return {
    id: entry.id,
    description: entry.description,
    status: missing.length === 0 ? "covered" : "missing",
    missing,
  };
}

function failedResult({ errorCode, state = AiUxGateState.Failed, missingEvidence, targetResults = [] }) {
  return {
    passed: false,
    marker: "phase006_ai_ux_gate=failed",
    state,
    errorCode,
    summary: { requiredTargets: requiredTargets.length, missingRequiredEvidence: missingEvidence.length },
    targetResults,
    missingEvidence,
  };
}

function target(id, description, { requiredFiles, evidence }) {
  return { id, description, requiredFiles, evidence };
}

function collectRequiredFiles() {
  return [...new Set(requiredTargets.flatMap((entry) => entry.requiredFiles))];
}

async function runCli() {
  const result = await runAiUxGate();
  await writeFile(".tasks/phase006-ai-ux-gate-result.md", renderAiUxGateMarkdown(result));
  if (result.passed) {
    console.log(result.marker);
    console.log(`gate_state=${result.state}`);
    console.log(`required_targets=${result.summary.requiredTargets}`);
    return;
  }
  console.error(result.marker);
  console.error(`gate_state=${result.state}`);
  console.error(`error_code=${result.errorCode}`);
  process.exit(1);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runCli();
}
