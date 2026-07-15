import assert from "node:assert/strict";
import test from "node:test";

import type { HistoryEntryViewModel } from "../src/index.ts";
import {
  RestoreFlowErrorCode,
  RestoreFlowEvent,
  RestoreFlowState,
  createRestoreApplyCommand,
  createRestorePreviewModel,
  createRestorePreviewRequestFromHistoryEntry,
  transitionRestoreFlowState,
} from "../src/index.ts";

test("restore flow state machine accepts only preview before apply", () => {
  const previewing = transitionRestoreFlowState(
    RestoreFlowState.Idle,
    RestoreFlowEvent.PreviewRequested,
  );
  const previewReady = transitionRestoreFlowState(
    previewing.state,
    RestoreFlowEvent.PreviewLoaded,
  );
  const applying = transitionRestoreFlowState(
    previewReady.state,
    RestoreFlowEvent.ApplyRequested,
  );
  const completed = transitionRestoreFlowState(
    applying.state,
    RestoreFlowEvent.ApplySucceeded,
  );
  const invalidApply = transitionRestoreFlowState(
    RestoreFlowState.Idle,
    RestoreFlowEvent.ApplyRequested,
  );

  assert.equal(previewing.state, RestoreFlowState.Previewing);
  assert.equal(previewReady.state, RestoreFlowState.PreviewReady);
  assert.equal(applying.state, RestoreFlowState.Applying);
  assert.equal(completed.state, RestoreFlowState.Completed);
  assert.equal(invalidApply.state, RestoreFlowState.Failed);
  assert.equal(invalidApply.errorCode, RestoreFlowErrorCode.InvalidTransition);
});

test("history entry creates restore preview request without document body", () => {
  const entry: HistoryEntryViewModel = {
    versionId: "version-1",
    summary: "Created document",
    author: "local-user",
    createdAt: "2026-07-09T00:00:00Z",
  };

  const request = createRestorePreviewRequestFromHistoryEntry("workspace-1", "doc-1", entry);
  const serialized = JSON.stringify(request);

  assert.equal(request.queryName, "preview-document-restore");
  assert.equal(request.workspaceId, "workspace-1");
  assert.equal(request.documentId, "doc-1");
  assert.equal(request.targetVersionId, "version-1");
  assert.equal(serialized.includes("document body should not be here"), false);
  assert.equal(serialized.includes("provider_api_key_fixture"), false);
});

test("restore preview model creates confirmed command only when restore is allowed", () => {
  const preview = createRestorePreviewModel({
    workspaceId: "workspace-1",
    documentId: "doc-1",
    targetVersionId: "version-1",
    canRestore: true,
    lines: [
      { kind: "unchanged", text: "line 1" },
      { kind: "removed", text: "line current" },
      { kind: "added", text: "line restored" },
    ],
  });
  const unconfirmed = createRestoreApplyCommand(preview, {
    confirmed: false,
    expectedCurrentVersionId: "version-current",
    restoredVersionId: "version-restore-1",
    restoredSnapshotRef: "snapshot-restore-1",
    author: "local-user",
    summary: "Restore version-1",
  });
  const confirmed = createRestoreApplyCommand(preview, {
    confirmed: true,
    expectedCurrentVersionId: "version-current",
    restoredVersionId: "version-restore-1",
    restoredSnapshotRef: "snapshot-restore-1",
    author: "local-user",
    summary: "Restore version-1",
  });
  const blockedPreview = createRestorePreviewModel({
    workspaceId: "workspace-1",
    documentId: "doc-1",
    targetVersionId: "version-2",
    canRestore: false,
    lines: [],
  });
  const blocked = createRestoreApplyCommand(blockedPreview, {
    confirmed: true,
    expectedCurrentVersionId: "version-current",
    restoredVersionId: "version-restore-2",
    restoredSnapshotRef: "snapshot-restore-2",
    author: "local-user",
    summary: "Restore version-2",
  });

  assert.equal(preview.state, RestoreFlowState.PreviewReady);
  assert.equal(preview.productLogEvent, "document.restore.previewed");
  assert.deepEqual(
    preview.lines.map((line) => line.kind),
    ["unchanged", "removed", "added"],
  );
  assert.equal(unconfirmed.status, "not-created");
  assert.equal(unconfirmed.errorCode, RestoreFlowErrorCode.ConfirmationRequired);
  assert.equal(confirmed.status, "created");
  assert.equal(confirmed.command?.commandName, "restore-document-version");
  assert.equal(confirmed.command?.targetVersionId, "version-1");
  assert.equal(blocked.status, "not-created");
  assert.equal(blocked.errorCode, RestoreFlowErrorCode.RestoreNotAllowed);
});
