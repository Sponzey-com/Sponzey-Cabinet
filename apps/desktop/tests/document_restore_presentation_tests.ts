import assert from "node:assert/strict";
import test from "node:test";

import {
  beginRestoreApply,
  beginRestorePreview,
  cancelRestoreConfirmation,
  completeRestorePreview,
  failRestoreApply,
  refreshRestoreConflict,
  requestRestoreConfirmation,
  retryRestoreRecovery,
} from "../src/document_restore_presentation.ts";

const diff = {
  workspaceId: "opaque-workspace",
  documentId: "opaque-document",
  status: "Complete",
  leftVersionId: "opaque-current",
  rightVersionId: "opaque-target",
  addedCount: 1,
  removedCount: 1,
  attachmentDiff: { status: "Known", added: [], removed: [], relabeled: [], unchangedCount: 0 },
  titleDelta: { kind: "Unchanged" },
  hunks: [],
} as const;

test("restore presentation distinguishes preview ready and missing asset blocking", () => {
  const previewing = beginRestorePreview("opaque-target");
  const ready = completeRestorePreview(previewing, {
    targetVersionId: "opaque-target",
    expectedCurrentVersionId: "opaque-current",
    targetVersionLabel: "버전 3",
    changedLineCount: 4,
    missingAssetLabels: [],
    canRestore: true,
    diff,
  });
  const blocked = completeRestorePreview(previewing, {
    targetVersionId: "opaque-target",
    expectedCurrentVersionId: "opaque-current",
    targetVersionLabel: "버전 3",
    changedLineCount: 0,
    missingAssetLabels: ["회의 자료"],
    canRestore: false,
    diff,
  });

  assert.equal(ready.status, "PreviewReady");
  assert.equal(blocked.status, "BlockedMissingAsset");
  assert.deepEqual(blocked.missingAssetLabels, ["회의 자료"]);
});

test("restore recovery preserves operation identity while conflict refreshes preview", () => {
  const ready = completeRestorePreview(beginRestorePreview("opaque-target"), {
    targetVersionId: "opaque-target",
    expectedCurrentVersionId: "opaque-current",
    targetVersionLabel: "버전 3",
    changedLineCount: 2,
    missingAssetLabels: [],
    canRestore: true,
    diff,
  });
  const confirming = requestRestoreConfirmation(ready);
  const applying = beginRestoreApply(confirming, "opaque-operation");
  const recovery = failRestoreApply(applying, {
    code: "DOCUMENT_RESTORE_RECOVERY_REQUIRED",
    retryable: true,
    repairRequired: true,
  });
  const retry = retryRestoreRecovery(recovery);
  const conflict = failRestoreApply(applying, {
    code: "DOCUMENT_RESTORE_VERSION_CONFLICT",
    retryable: false,
    repairRequired: false,
  });
  const refreshed = refreshRestoreConflict(conflict);

  assert.equal(recovery.status, "RecoveryRequired");
  assert.equal(retry.status, "Applying");
  assert.equal(retry.operationId, "opaque-operation");
  assert.equal(conflict.status, "Conflict");
  assert.deepEqual(refreshed, { status: "Previewing", targetVersionId: "opaque-target" });
});

test("restore failure classification does not expose raw error as state content", () => {
  const ready = completeRestorePreview(beginRestorePreview("opaque-target"), {
    targetVersionId: "opaque-target",
    expectedCurrentVersionId: "opaque-current",
    targetVersionLabel: "버전 3",
    changedLineCount: 1,
    missingAssetLabels: [],
    canRestore: true,
    diff,
  });
  const applying = beginRestoreApply(requestRestoreConfirmation(ready), "opaque-operation");
  const missing = failRestoreApply(applying, {
    code: "DOCUMENT_RESTORE_MISSING_DEPENDENCY",
    retryable: false,
    repairRequired: false,
  });
  const failed = failRestoreApply(applying, {
    code: "COMMAND_BRIDGE_FAILED",
    retryable: false,
    repairRequired: false,
  });

  assert.equal(missing.status, "BlockedMissingAsset");
  assert.equal(failed.status, "Failed");
  assert.equal(failed.errorCategory, "unavailable");
});

test("restore requires explicit confirmation and cancel preserves the preview", () => {
  const ready = completeRestorePreview(beginRestorePreview("opaque-target"), {
    targetVersionId: "opaque-target",
    expectedCurrentVersionId: "opaque-current",
    targetVersionLabel: "버전 3",
    changedLineCount: 2,
    missingAssetLabels: [],
    canRestore: true,
    diff,
  });

  const refused = beginRestoreApply(ready, "must-not-dispatch");
  const confirming = requestRestoreConfirmation(ready);
  const cancelled = cancelRestoreConfirmation(confirming);
  const applying = beginRestoreApply(confirming, "opaque-operation");

  assert.equal(refused.status, "PreviewReady");
  assert.equal(confirming.status, "Confirming");
  assert.equal(cancelled.status, "PreviewReady");
  assert.equal(cancelled.diff, diff);
  assert.equal(applying.status, "Applying");
  assert.equal(applying.operationId, "opaque-operation");
});

test("too-large restore preview is blocked before confirmation or operation creation", () => {
  const tooLarge = completeRestorePreview(beginRestorePreview("opaque-target"), {
    targetVersionId: "opaque-target",
    expectedCurrentVersionId: "opaque-current",
    targetVersionLabel: "버전 3",
    changedLineCount: 20_000,
    missingAssetLabels: [],
    canRestore: true,
    diff: { ...diff, status: "TooLarge", limitReason: "hunks" },
  });
  const confirmationRefused = requestRestoreConfirmation(tooLarge);
  const applyRefused = beginRestoreApply(tooLarge, "must-not-exist");
  const missingWins = completeRestorePreview(beginRestorePreview("opaque-target"), {
    targetVersionId: "opaque-target",
    expectedCurrentVersionId: "opaque-current",
    targetVersionLabel: "버전 3",
    changedLineCount: 20_000,
    missingAssetLabels: ["누락된 자료"],
    canRestore: false,
    diff: { ...diff, status: "TooLarge", limitReason: "hunks" },
  });

  assert.equal(tooLarge.status, "BlockedLargeDiff");
  assert.equal(confirmationRefused.status, "BlockedLargeDiff");
  assert.equal(applyRefused.status, "BlockedLargeDiff");
  assert.equal(missingWins.status, "BlockedMissingAsset");
  assert.equal("operationId" in tooLarge, false);
});
