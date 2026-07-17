import assert from "node:assert/strict";
import test from "node:test";

import type { DocumentDiffView } from "@sponzey-cabinet/client-core";

import { presentDesktopDocumentDiffOperation } from "../src/document_diff_operation_presenter.ts";

const target = Object.freeze({ targetVersionId: "version-secret", targetVersionLabel: "버전 3" });

test("background diff presenter maps operation states without exposing token", () => {
  const accepted = presentDesktopDocumentDiffOperation({
    state: "Accepted",
    generation: 2,
    operationToken: "operation-token-secret",
  }, target);
  const expired = presentDesktopDocumentDiffOperation({
    state: "Expired",
    generation: 2,
    query: {
      queryName: "compare-current-document-to-version",
      workspaceId: "workspace-secret",
      documentId: "document-secret",
      targetVersionId: "version-secret",
    },
  }, target);

  assert.deepEqual(accepted, { status: "Accepted", ...target });
  assert.deepEqual(expired, { status: "Expired", ...target });
  assert.doesNotMatch(JSON.stringify([accepted, expired]), /operation-token-secret|workspace-secret|document-secret/);
});

test("background diff presenter maps only complete result to ready workspace", () => {
  const diff: DocumentDiffView = {
    status: "Complete",
    queryName: "compare-current-document-to-version",
    workspaceId: "workspace-secret",
    documentId: "document-secret",
    addedCount: 1,
    removedCount: 0,
    attachmentDiff: { status: "Known", added: [], removed: [], relabeled: [], unchangedCount: 0 },
    titleDelta: { kind: "Unchanged" },
    hunks: [{
      oldStartLine: 1,
      newStartLine: 1,
      addedCount: 1,
      removedCount: 0,
      lines: [{ kind: "Added", text: "새 내용", newLineNumber: 1 }],
    }],
  };
  const ready = presentDesktopDocumentDiffOperation({ state: "Ready", generation: 3, diff }, target);
  const failed = presentDesktopDocumentDiffOperation({
    state: "Failed",
    generation: 3,
    errorCode: "DOCUMENT_DIFF_OPERATION_FAILED",
    retryable: false,
  }, target);

  assert.equal(ready.status, "Ready");
  assert.equal(ready.status === "Ready" && ready.hunks[0]?.lines[0]?.text, "새 내용");
  assert.deepEqual(failed, { status: "Failed", ...target, errorCode: "DOCUMENT_DIFF_OPERATION_FAILED", canRetry: true });
  assert.doesNotMatch(JSON.stringify(ready), /workspace-secret|document-secret/);
});
