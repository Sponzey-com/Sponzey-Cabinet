import assert from "node:assert/strict";
import test from "node:test";

import {
  ProjectionSynchronizationState,
  synchronizeCurrentDocumentProjections,
  transitionProjectionSynchronization,
} from "../src/desktop_projection_synchronizer.ts";

test("projection synchronization transitions through reindex worker and completion", async () => {
  const calls: string[] = [];
  const states: string[] = [];
  const result = await synchronizeCurrentDocumentProjections({
    async requestReindex(workspaceId, documentId) {
      calls.push(`reindex:${workspaceId}:${documentId}`);
      return { enqueuedCount: 1, resetCount: 0, alreadyActiveCount: 0 };
    },
    async runWorker() {
      calls.push("worker");
      return { readyCount: 4, retryScheduledCount: 0, failedCount: 0 };
    },
  }, "workspace-1", "doc-1", (state) => states.push(state));

  assert.deepEqual(calls, ["reindex:workspace-1:doc-1", "worker"]);
  assert.deepEqual(states, ["Reindexing", "Running", "Completed"]);
  assert.deepEqual(result, { state: "Completed", readyCount: 4 });
});

test("projection synchronization fails closed for invalid identity retry or transport failure", async () => {
  let calls = 0;
  const invalid = await synchronizeCurrentDocumentProjections({
    async requestReindex() { calls += 1; throw new Error("not called"); },
    async runWorker() { calls += 1; throw new Error("not called"); },
  }, " ", "doc-1");
  assert.deepEqual(invalid, { state: "Failed", errorCode: "PROJECTION_IDENTITY_INVALID" });
  assert.equal(calls, 0);

  const retry = await synchronizeCurrentDocumentProjections({
    async requestReindex() { return { enqueuedCount: 0, resetCount: 0, alreadyActiveCount: 1 }; },
    async runWorker() { return { readyCount: 0, retryScheduledCount: 1, failedCount: 0 }; },
  }, "workspace-1", "doc-1");
  assert.deepEqual(retry, { state: "Failed", errorCode: "PROJECTION_SYNC_INCOMPLETE" });

  const failed = await synchronizeCurrentDocumentProjections({
    async requestReindex() { throw new Error("private path"); },
    async runWorker() { throw new Error("not called"); },
  }, "workspace-1", "doc-1");
  assert.deepEqual(failed, { state: "Failed", errorCode: "PROJECTION_SYNC_FAILED" });
});

test("projection synchronization rejects out-of-order state transitions", () => {
  assert.equal(transitionProjectionSynchronization(ProjectionSynchronizationState.Idle, "start"), "Reindexing");
  assert.equal(transitionProjectionSynchronization(ProjectionSynchronizationState.Reindexing, "reindexed"), "Running");
  assert.equal(transitionProjectionSynchronization(ProjectionSynchronizationState.Running, "completed"), "Completed");
  assert.equal(transitionProjectionSynchronization(ProjectionSynchronizationState.Idle, "completed"), "Failed");
});
