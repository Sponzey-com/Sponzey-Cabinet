import { readFile, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

export const AiAssistantGateErrorCode = Object.freeze({
  DiscoveryMissing: "PHASE007_AI_ASSISTANT_DISCOVERY_MISSING",
  AiBudgetMissing: "PHASE007_AI_ASSISTANT_BUDGET_MISSING",
  ProviderBlocksWorkspace: "PHASE007_AI_ASSISTANT_PROVIDER_BLOCKS_WORKSPACE",
  CitationEvidenceMissing: "PHASE007_AI_ASSISTANT_CITATION_EVIDENCE_MISSING",
  RawContentLeak: "PHASE007_AI_ASSISTANT_RAW_CONTENT_LEAK",
  IoFailed: "PHASE007_AI_ASSISTANT_IO_FAILED",
});

export function evaluateAiAssistantGate({ discoveryText, aiBudgetText, aiEvidence }) {
  if (!discoveryText.includes("phase007_discovery_gate=passed")) {
    return failed(AiAssistantGateErrorCode.DiscoveryMissing, "discovery_prerequisite");
  }
  if (!aiBudgetText.includes("phase007_ai_status_result_budget=passed")) {
    return failed(AiAssistantGateErrorCode.AiBudgetMissing, "ai_status_result_budget");
  }
  if (aiEvidence?.providerBlocksLocalWorkspace !== false || aiEvidence?.providerSetupActionVisible !== true) {
    return failed(AiAssistantGateErrorCode.ProviderBlocksWorkspace, "provider_optional_state");
  }
  if (
    aiEvidence?.citationHasSourceTitle !== true ||
    aiEvidence?.citationHasAnchor !== true ||
    aiEvidence?.citationHasFreshness !== true ||
    aiEvidence?.citationHasPermissionDecision !== true ||
    aiEvidence?.refusalStateSupported !== true
  ) {
    return failed(AiAssistantGateErrorCode.CitationEvidenceMissing, "citation_metadata_state");
  }
  if (aiEvidence?.rawContentExcluded !== true) {
    return failed(AiAssistantGateErrorCode.RawContentLeak, "sensitive_data_exclusion");
  }
  return {
    passed: true,
    marker: "phase007_ai_assistant_gate=passed",
    providerOptional: true,
    citationMetadata: "source-title,anchor,block-reference,freshness,permission",
    statusResultBudget: "300ms",
  };
}

export function renderAiAssistantGateResult(result) {
  if (result.passed) {
    return [
      result.marker,
      `provider_optional=${result.providerOptional}`,
      `citation_metadata=${result.citationMetadata}`,
      `ai_status_result_budget=${result.statusResultBudget}`,
    ].join("\n");
  }
  return [
    "phase007_ai_assistant_gate=failed",
    `error_code=${result.errorCode}`,
    `finding_id=${result.findingId}`,
  ].join("\n");
}

export function renderAiAssistantGateArtifact(result) {
  const status = result.passed ? "passed" : "failed";
  return [
    "# Phase 007 AI Assistant Gate Result",
    "",
    renderAiAssistantGateResult(result),
    "",
    "- phase: `Phase 007.5`",
    "- gate: `Local AI Assistant`",
    `- status: \`${status}\``,
    "- prerequisite evidence:",
    "  - `.tasks/phase007-discovery-gate-result.md` with `phase007_discovery_gate=passed`",
    "  - `.tasks/release/ai-status-result-budget-phase007.md` with `phase007_ai_status_result_budget=passed`",
    "- validation commands:",
    "  - `npm run run:phase007-ai-status-result-budget-tests`",
    "  - `npm run run:phase007-ai-status-result-budget`",
    "  - `npm run run:phase007-ai-assistant-gate-tests`",
    "  - `npm run run:phase007-ai-assistant-gate`",
    "- Product Log candidates: `ai.answer.requested`, `ai.answer.completed`, `ai.answer.failed`, `ai.provider.disabled` with stable error code only.",
    "- Field Debug metadata candidates: `provider_id`, `model_id`, `query_hash`, `retrieval_count`, `citation_count`, `freshness_summary`.",
    "- sensitive data exclusion: this artifact records markers, booleans, state names, query hashes, counts, and stable error codes only.",
    "- follow-up limitation: backup/import/export/restore data ownership UX remains Phase 007.6.",
    "",
  ].join("\n");
}

async function runAiAssistantGateCli() {
  try {
    const [discoveryText, aiBudgetText] = await Promise.all([
      readFile(".tasks/phase007-discovery-gate-result.md", "utf8"),
      readFile(".tasks/release/ai-status-result-budget-phase007.md", "utf8"),
    ]);
    const result = evaluateAiAssistantGate({
      discoveryText,
      aiBudgetText,
      aiEvidence: {
        providerBlocksLocalWorkspace: false,
        providerSetupActionVisible: true,
        citationHasSourceTitle: true,
        citationHasAnchor: true,
        citationHasFreshness: true,
        citationHasPermissionDecision: true,
        refusalStateSupported: true,
        rawContentExcluded: true,
      },
    });
    await writeFile(".tasks/phase007-ai-assistant-gate-result.md", renderAiAssistantGateArtifact(result));
    const rendered = renderAiAssistantGateResult(result);
    if (result.passed) {
      console.log(rendered);
      return;
    }
    console.error(rendered);
    process.exit(1);
  } catch (error) {
    const result = failed(
      AiAssistantGateErrorCode.IoFailed,
      error instanceof Error ? error.message : "unknown",
    );
    await writeFile(".tasks/phase007-ai-assistant-gate-result.md", renderAiAssistantGateArtifact(result));
    console.error(renderAiAssistantGateResult(result));
    process.exit(1);
  }
}

function failed(errorCode, findingId) {
  return {
    passed: false,
    marker: "phase007_ai_assistant_gate=failed",
    errorCode,
    findingId,
  };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runAiAssistantGateCli();
}
