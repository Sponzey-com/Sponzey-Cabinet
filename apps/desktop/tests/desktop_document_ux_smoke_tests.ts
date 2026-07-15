import assert from "node:assert/strict";
import test from "node:test";

import type { CurrentDocumentView, DocumentHistoryPage } from "../../../packages/client-core/src/index.ts";
import {
  createDesktopDocumentReadingWorkspace,
  createDesktopRestoreApplyCommand,
  createDesktopRestorePreviewModel,
  createDesktopRestorePreviewRequest,
} from "../src/index.ts";

test("desktop document UX smoke renders markdown preview table and keeps read paths split", () => {
  const reading = createDesktopDocumentReadingWorkspace(currentDocument(), historyPage());
  const table = reading.preview.blocks.find((block) => block.kind === "table");

  assert.equal(reading.mode, "document-reading-workspace");
  assert.equal(reading.current.queryName, "get-current-document");
  assert.equal(reading.history.queryName, "get-document-history");
  assert.equal(reading.querySeparation.currentReadQueryName, "get-current-document");
  assert.equal(reading.querySeparation.historyReadQueryName, "get-document-history");
  assert.equal(table?.kind, "table");
  assert.deepEqual(table?.headers, ["항목", "내용", "상태"]);
  assert.deepEqual(table?.alignments, ["left", "center", "right"]);
});

test("desktop restore smoke creates preview request and confirmed command without body payload", () => {
  const reading = createDesktopDocumentReadingWorkspace(currentDocument(), historyPage());
  const entry = reading.history.entries[0];
  assert.ok(entry);

  const request = createDesktopRestorePreviewRequest("workspace-1", "doc-1", entry);
  const preview = createDesktopRestorePreviewModel({
    workspaceId: request.workspaceId,
    documentId: request.documentId,
    targetVersionId: request.targetVersionId,
    canRestore: true,
    lines: [
      { kind: "removed", text: "current line" },
      { kind: "added", text: "restored line" },
    ],
  });
  const command = createDesktopRestoreApplyCommand(preview, {
    confirmed: true,
    expectedCurrentVersionId: "version-current",
    restoredVersionId: "version-restore-1",
    restoredSnapshotRef: "snapshot-restore-1",
    author: "local-user",
    summary: "Restore version-1",
  });
  const serializedRequest = JSON.stringify(request);
  const serializedCommand = JSON.stringify(command);

  assert.equal(request.queryName, "preview-document-restore");
  assert.equal(preview.state, "PreviewReady");
  assert.equal(command.status, "created");
  assert.equal(command.command?.commandName, "restore-document-version");
  assert.equal(command.command?.expectedCurrentVersionId, "version-current");
  assert.equal(serializedRequest.includes("문서 원문이 request에 들어가면 안 됩니다"), false);
  assert.equal(serializedCommand.includes("문서 원문이 request에 들어가면 안 됩니다"), false);
  assert.equal(serializedRequest.includes("server-url"), false);
  assert.equal(serializedCommand.includes("pull request"), false);
});

test("desktop restore apply command requires expected current version guard", () => {
  const preview = createDesktopRestorePreviewModel({
    workspaceId: "workspace-1",
    documentId: "doc-1",
    targetVersionId: "version-1",
    canRestore: true,
    lines: [],
  });
  const command = createDesktopRestoreApplyCommand(preview, {
    confirmed: true,
    expectedCurrentVersionId: "",
    restoredVersionId: "version-restore-1",
    restoredSnapshotRef: "snapshot-restore-1",
    author: "local-user",
    summary: "Restore version-1",
  });

  assert.equal(command.status, "not-created");
  assert.equal(command.errorCode, "RESTORE_INVALID_TRANSITION");
});

function currentDocument(): CurrentDocumentView {
  return {
    workspaceId: "workspace-1",
    documentId: "doc-1",
    title: "Source Document",
    path: "docs/source.md",
    body: [
      "# Source Document",
      "",
      "문서 원문이 request에 들어가면 안 됩니다.",
      "",
      "| 항목 | 내용 | 상태 |",
      "| :--- | :---: | ---: |",
      "| 1번 그리드 | 좌측 정렬 | 우측 정렬 |",
    ].join("\n"),
    versionId: "version-current",
  };
}

function historyPage(): DocumentHistoryPage {
  return {
    workspaceId: "workspace-1",
    documentId: "doc-1",
    entries: [
      {
        versionId: "version-1",
        summary: "Created document",
        author: "local-user",
        createdAt: "2026-07-09T00:00:00Z",
      },
    ],
  };
}
