import assert from "node:assert/strict";
import test from "node:test";

import {
  DataOwnershipGateErrorCode,
  evaluateDataOwnershipGate,
  renderDataOwnershipGateResult,
} from "./phase007_data_ownership_gate.mjs";

test("data ownership gate rejects missing AI assistant prerequisite", () => {
  const result = evaluateDataOwnershipGate({
    aiAssistantText: "phase007_ai_assistant_gate=failed",
    ownershipEvidence: completeOwnershipEvidence(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, DataOwnershipGateErrorCode.AiAssistantMissing);
});

test("data ownership gate rejects secret inclusion evidence", () => {
  const result = evaluateDataOwnershipGate({
    aiAssistantText: "phase007_ai_assistant_gate=passed",
    ownershipEvidence: { ...completeOwnershipEvidence(), secretsExcluded: false },
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, DataOwnershipGateErrorCode.SecretExclusionMissing);
});

test("data ownership gate rejects restore without confirmation guard", () => {
  const result = evaluateDataOwnershipGate({
    aiAssistantText: "phase007_ai_assistant_gate=passed",
    ownershipEvidence: { ...completeOwnershipEvidence(), restoreRequiresConfirmation: false },
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, DataOwnershipGateErrorCode.RestoreConfirmationMissing);
});

test("data ownership gate passes complete evidence and renders safe marker", () => {
  const result = evaluateDataOwnershipGate({
    aiAssistantText: "phase007_ai_assistant_gate=passed",
    ownershipEvidence: completeOwnershipEvidence(),
  });
  const rendered = renderDataOwnershipGateResult(result);

  assert.equal(result.passed, true);
  assert.match(rendered, /phase007_data_ownership_gate=passed/);
  assert.match(rendered, /backup_default_path=platform-default/);
  assert.doesNotMatch(rendered, /raw markdown body should not leak/);
  assert.doesNotMatch(rendered, /asset binary content should not leak/);
  assert.doesNotMatch(rendered, /provider_api_key_fixture/);
  assert.doesNotMatch(rendered, /\/Users\/example\/private/);
});

function completeOwnershipEvidence() {
  return {
    backupDefaultPath: "platform-default",
    backupBlocksStartup: false,
    secretsExcluded: true,
    importConflictPolicies: ["rename", "skip"],
    restoreRequiresConfirmation: true,
    restoreStagingIsolated: true,
    rawContentExcluded: true,
  };
}
