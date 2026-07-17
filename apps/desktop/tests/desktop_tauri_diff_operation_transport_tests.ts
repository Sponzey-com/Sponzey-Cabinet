import assert from "node:assert/strict";
import test from "node:test";

import { DesktopDocumentDiffOperationTransportError, createTauriDocumentDiffOperationTransport } from "../src/tauri_document_diff_operation_transport.ts";

const query = Object.freeze({
  queryName: "compare-current-document-to-version" as const,
  workspaceId: "workspace-1",
  documentId: "doc-1",
  targetVersionId: "history-secret",
});

test("background diff transport maps start status and cancel without exposing native shape", async () => {
  const calls: Array<{ command: string; request: unknown }> = [];
  const client = createTauriDocumentDiffOperationTransport(async (command, args) => {
    calls.push({ command, request: args?.request });
    if (command === "start_desktop_document_diff_operation") {
      return success("opaque-operation-secret", "accepted");
    }
    if (command === "get_desktop_document_diff_operation_status") {
      return success("opaque-operation-secret", "completed", nativeDiff());
    }
    return success("opaque-operation-secret", "cancelled");
  });

  const accepted = await client.start(query);
  assert.deepEqual(accepted, { operationToken: "opaque-operation-secret", state: "Accepted" });
  const completed = await client.status({
    operationToken: accepted.operationToken,
    workspaceId: query.workspaceId,
    documentId: query.documentId,
  });
  assert.equal(completed.state, "Completed");
  assert.equal(completed.diff?.status, "Complete");
  assert.equal(completed.diff?.attachmentDiff.status, "Known");
  const cancelled = await client.cancel({ operationToken: accepted.operationToken });
  assert.equal(cancelled.state, "Cancelled");

  assert.deepEqual(calls, [
    {
      command: "start_desktop_document_diff_operation",
      request: {
        kind: "current_to_version",
        workspaceId: "workspace-1",
        documentId: "doc-1",
        versionToken: "history-secret",
      },
    },
    {
      command: "get_desktop_document_diff_operation_status",
      request: { operationToken: "opaque-operation-secret" },
    },
    {
      command: "cancel_desktop_document_diff_operation",
      request: { operationToken: "opaque-operation-secret" },
    },
  ]);
});

test("background diff transport rejects token mismatch prohibited keys and invalid state payload", async () => {
  const mismatch = createTauriDocumentDiffOperationTransport(async () => success("other-token", "running"));
  await assert.rejects(
    mismatch.status({ operationToken: "expected-token", workspaceId: "workspace-1", documentId: "doc-1" }),
    (error) => error instanceof DesktopDocumentDiffOperationTransportError && error.code === "COMMAND_BRIDGE_FAILED",
  );

  const prohibited = createTauriDocumentDiffOperationTransport(async () => ({
    ...success("expected-token", "completed", nativeDiff()),
    data: { ...success("expected-token", "completed", nativeDiff()).data, assetId: "must-not-cross" },
  }));
  await assert.rejects(
    prohibited.status({ operationToken: "expected-token", workspaceId: "workspace-1", documentId: "doc-1" }),
    DesktopDocumentDiffOperationTransportError,
  );

  const invalid = createTauriDocumentDiffOperationTransport(async () => success("expected-token", "running", nativeDiff()));
  await assert.rejects(
    invalid.status({ operationToken: "expected-token", workspaceId: "workspace-1", documentId: "doc-1" }),
    DesktopDocumentDiffOperationTransportError,
  );
});

test("background diff transport maps stable native failures without raw response content", async () => {
  const client = createTauriDocumentDiffOperationTransport(async () => ({
    ok: false,
    retryable: true,
    repairRequired: false,
    errorCode: "DOCUMENT_DIFF_OPERATION_RUNTIME_UNAVAILABLE",
  }));
  await assert.rejects(
    client.start(query),
    (error) => error instanceof DesktopDocumentDiffOperationTransportError
      && error.code === "DOCUMENT_DIFF_OPERATION_RUNTIME_UNAVAILABLE"
      && error.retryable,
  );
});

function success(operationToken: string, state: string, diff?: unknown) {
  return {
    ok: true,
    retryable: false,
    repairRequired: false,
    data: { operationToken, state, ...(diff === undefined ? {} : { diff }) },
  };
}

function nativeDiff() {
  return {
    kind: "complete",
    leftVersionToken: "current-secret",
    rightVersionToken: "history-secret",
    addedCount: 1,
    removedCount: 1,
    attachmentDiff: {
      kind: "known",
      added: [{ label: "새 자료", availability: "available" }],
      removed: [],
      relabeled: [],
      unchangedCount: 0,
    },
    titleDelta: { kind: "changed", before: "현재", after: "이전" },
    hunks: [{
      oldStartLine: 1,
      newStartLine: 1,
      addedCount: 1,
      removedCount: 1,
      lines: [
        { kind: "removed", text: "현재", oldLineNumber: 1 },
        { kind: "added", text: "이전", newLineNumber: 1 },
      ],
    }],
  };
}
