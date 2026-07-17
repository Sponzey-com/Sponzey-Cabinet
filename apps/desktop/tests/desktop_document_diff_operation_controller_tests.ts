import assert from "node:assert/strict";
import test from "node:test";

import type { DocumentDiffQuery, DocumentDiffView } from "@sponzey-cabinet/client-core";

import {
  applyDesktopDocumentDiffOperationCandidate,
  cancelDesktopDocumentDiffOperation,
  createDesktopDocumentDiffOperationSnapshot,
  pollDesktopDocumentDiffOperation,
  retryDesktopDocumentDiffOperation,
  startDesktopDocumentDiffOperation,
  type DesktopDocumentDiffOperationClient,
} from "../src/desktop_document_diff_operation_controller.ts";

const query: DocumentDiffQuery = Object.freeze({
  queryName: "compare-current-document-to-version",
  workspaceId: "workspace-1",
  documentId: "doc-1",
  targetVersionId: "version-secret",
});

test("diff operation controller follows Accepted Running Ready with explicit generation", async () => {
  const client = fakeClient([
    { operationToken: "operation-secret", state: "Accepted" },
    { operationToken: "operation-secret", state: "Running" },
    { operationToken: "operation-secret", state: "Completed", diff: completeDiff() },
  ]);
  const idle = createDesktopDocumentDiffOperationSnapshot();
  const accepted = await startDesktopDocumentDiffOperation(client, idle, query);
  assert.equal(accepted.state, "Accepted");
  assert.equal(accepted.generation, 1);
  const running = await pollDesktopDocumentDiffOperation(client, accepted);
  assert.equal(running.state, "Running");
  const ready = await pollDesktopDocumentDiffOperation(client, running);
  assert.equal(ready.state, "Ready");
  assert.equal(ready.diff?.status, "Complete");
});

test("diff operation controller maps cancelled expired failed and retries stored query", async () => {
  const cancelledClient = fakeClient([
    { operationToken: "operation-secret", state: "Accepted" },
    { operationToken: "operation-secret", state: "Cancelled" },
  ]);
  const accepted = await startDesktopDocumentDiffOperation(
    cancelledClient,
    createDesktopDocumentDiffOperationSnapshot(),
    query,
  );
  assert.equal((await cancelDesktopDocumentDiffOperation(cancelledClient, accepted)).state, "Cancelled");

  const expiredClient = fakeClient([{ operationToken: "new-operation", state: "Accepted" }]);
  const expired = { ...accepted, state: "Expired" as const };
  const retried = await retryDesktopDocumentDiffOperation(expiredClient, expired);
  assert.equal(retried.state, "Accepted");
  assert.equal(retried.generation, accepted.generation + 1);
  assert.equal(retried.operationToken, "new-operation");

  const failedClient = fakeClient([{ operationToken: "operation-secret", state: "Failed", failureCode: "document.diff.failed" }]);
  const failed = await pollDesktopDocumentDiffOperation(failedClient, accepted);
  assert.equal(failed.state, "Failed");
  assert.equal(failed.errorCode, "document.diff.failed");
});

test("diff operation controller rejects token mismatch and stale generation candidates", async () => {
  const mismatch = fakeClient([{ operationToken: "other-operation", state: "Running" }]);
  const accepted = {
    ...createDesktopDocumentDiffOperationSnapshot(),
    state: "Accepted" as const,
    generation: 3,
    query,
    operationToken: "expected-operation",
  };
  const failed = await pollDesktopDocumentDiffOperation(mismatch, accepted);
  assert.equal(failed.state, "Failed");
  assert.equal(failed.errorCode, "DOCUMENT_DIFF_OPERATION_ID_MISMATCH");
  assert.equal(failed.operationToken, undefined);

  const newer = { ...accepted, generation: 4, state: "Running" as const };
  assert.equal(applyDesktopDocumentDiffOperationCandidate(newer, failed), newer);
  assert.equal(
    applyDesktopDocumentDiffOperationCandidate(accepted, {
      ...accepted,
      state: "Running",
      operationToken: "other-operation",
    }),
    accepted,
  );
});

function fakeClient(results: Array<{
  operationToken: string;
  state: "Accepted" | "Running" | "Completed" | "Cancelled" | "Expired" | "Failed";
  diff?: DocumentDiffView;
  failureCode?: string;
}>): DesktopDocumentDiffOperationClient {
  const next = () => {
    const result = results.shift();
    if (!result) throw new Error("missing fake result");
    return Promise.resolve(result);
  };
  return Object.freeze({ start: next, status: next, cancel: next });
}

function completeDiff(): DocumentDiffView {
  return {
    workspaceId: "workspace-1",
    documentId: "doc-1",
    status: "Complete",
    leftVersionId: "current-secret",
    rightVersionId: "version-secret",
    addedCount: 0,
    removedCount: 0,
    attachmentDiff: { status: "Known", added: [], removed: [], relabeled: [], unchangedCount: 0 },
    titleDelta: { kind: "Unchanged" },
    hunks: [],
  };
}
