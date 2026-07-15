import assert from "node:assert/strict";
import test from "node:test";

import {
  AiAssistantGateErrorCode,
  evaluateAiAssistantGate,
  renderAiAssistantGateResult,
} from "./phase007_ai_assistant_gate.mjs";

test("AI assistant gate rejects missing discovery prerequisite", () => {
  const result = evaluateAiAssistantGate({
    discoveryText: "phase007_discovery_gate=failed",
    aiBudgetText: "phase007_ai_status_result_budget=passed",
    aiEvidence: completeAiEvidence(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, AiAssistantGateErrorCode.DiscoveryMissing);
});

test("AI assistant gate rejects missing cached status result budget", () => {
  const result = evaluateAiAssistantGate({
    discoveryText: "phase007_discovery_gate=passed",
    aiBudgetText: "phase007_ai_status_result_budget=failed",
    aiEvidence: completeAiEvidence(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, AiAssistantGateErrorCode.AiBudgetMissing);
});

test("AI assistant gate rejects evidence without provider optional behavior", () => {
  const result = evaluateAiAssistantGate({
    discoveryText: "phase007_discovery_gate=passed",
    aiBudgetText: "phase007_ai_status_result_budget=passed",
    aiEvidence: { ...completeAiEvidence(), providerBlocksLocalWorkspace: true },
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, AiAssistantGateErrorCode.ProviderBlocksWorkspace);
});

test("AI assistant gate passes complete evidence and renders safe marker", () => {
  const result = evaluateAiAssistantGate({
    discoveryText: "phase007_discovery_gate=passed",
    aiBudgetText: "phase007_ai_status_result_budget=passed",
    aiEvidence: completeAiEvidence(),
  });
  const rendered = renderAiAssistantGateResult(result);

  assert.equal(result.passed, true);
  assert.match(rendered, /phase007_ai_assistant_gate=passed/);
  assert.match(rendered, /provider_optional=true/);
  assert.doesNotMatch(rendered, /ai_prompt_fixture/);
  assert.doesNotMatch(rendered, /ai_answer_fixture/);
  assert.doesNotMatch(rendered, /provider_api_key_fixture/);
});

function completeAiEvidence() {
  return {
    providerBlocksLocalWorkspace: false,
    providerSetupActionVisible: true,
    citationHasSourceTitle: true,
    citationHasAnchor: true,
    citationHasFreshness: true,
    citationHasPermissionDecision: true,
    refusalStateSupported: true,
    rawContentExcluded: true,
  };
}
