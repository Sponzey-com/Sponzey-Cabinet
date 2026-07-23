import assert from "node:assert/strict";
import test from "node:test";

import {
  AttachmentStatusPresentationError,
  applyAttachmentFileStatus,
  createAttachmentFileSnapshot,
  presentAttachmentWireState,
  sanitizeAttachmentDisplayName,
} from "../src/attachment_operation_presenter.ts";

test("every current and planned wire state maps to an explicit presentation stage", () => {
  const cases = [
    ["selected", "Selected"], ["validating", "Validating"], ["staging", "Staging"],
    ["hashing", "Hashing"], ["publishing_object", "PublishingObject"],
    ["persisting_metadata", "PersistingMetadata"], ["preparing_revision", "PreparingRevision"],
    ["linking", "Associating"], ["associating", "Associating"], ["projecting", "Projecting"],
    ["verifying", "Verifying"], ["completed", "Completed"], ["cancelling", "Cancelling"],
    ["cancelled", "Cancelled"], ["validation_failed", "Failed"], ["staging_failed", "Failed"],
    ["object_publish_failed", "Failed"], ["metadata_persist_failed", "RecoveryRequired"],
    ["link_failed", "RecoveryRequired"], ["conflict", "Conflict"],
    ["recovery_required", "RecoveryRequired"], ["cleanup_required", "RecoveryRequired"],
    ["failed", "Failed"],
  ] as const;

  for (const [wire, stage] of cases) {
    assert.equal(presentAttachmentWireState(wire).stage, stage, wire);
  }
  assert.throws(
    () => presentAttachmentWireState("unknown"),
    (error) => error instanceof AttachmentStatusPresentationError
      && error.code === "ATTACHMENT_STATUS_UNSUPPORTED",
  );
});

test("presentation capabilities distinguish cancel retry repair and terminal outcomes", () => {
  assert.deepEqual(capabilities("validating"), [true, false, false, false]);
  assert.deepEqual(capabilities("associating"), [false, false, false, false]);
  assert.deepEqual(capabilities("conflict"), [false, true, false, false]);
  assert.deepEqual(capabilities("recovery_required"), [false, false, true, false]);
  assert.deepEqual(capabilities("failed"), [false, false, false, true]);
  assert.deepEqual(capabilities("completed"), [false, false, false, false]);
});

test("file snapshot excludes opaque handle and sanitizes path-like display names", () => {
  assert.equal(sanitizeAttachmentDisplayName("/Users/private/report.pdf"), "report.pdf");
  assert.equal(sanitizeAttachmentDisplayName("C:\\private\\draft.txt"), "draft.txt");
  assert.equal(sanitizeAttachmentDisplayName("\u0000\n"), "첨부 파일");

  const snapshot = createAttachmentFileSnapshot({
    generation: 3,
    operationId: "operation-internal-1",
    fileName: "/Users/private/report.pdf",
    byteSize: 42,
    state: "selected",
  });
  assert.equal(snapshot.displayName, "report.pdf");
  assert.equal(JSON.stringify(snapshot).includes("/Users/"), false);
  assert.equal(JSON.stringify(snapshot).includes("handle"), false);
  assert.equal(snapshot.userLabel.includes("operation-internal-1"), false);
});

test("status updates ignore stale generation operation mismatch and state regression", () => {
  const initial = createAttachmentFileSnapshot({ generation: 2, operationId: "op-1", fileName: "note.txt", byteSize: 10, state: "selected" });
  const hashing = applyAttachmentFileStatus(initial, { generation: 2, operationId: "op-1", state: "hashing", completedBytes: 6, totalBytes: 10 });
  assert.equal(hashing.stage, "Hashing");
  assert.equal(hashing.progressPercent, 60);
  assert.strictEqual(applyAttachmentFileStatus(hashing, { generation: 1, operationId: "op-1", state: "completed" }), hashing);
  assert.strictEqual(applyAttachmentFileStatus(hashing, { generation: 2, operationId: "other", state: "completed" }), hashing);
  assert.strictEqual(applyAttachmentFileStatus(hashing, { generation: 2, operationId: "op-1", state: "validating" }), hashing);
});

test("terminal replay is identity preserving and terminal mutation is rejected", () => {
  const initial = createAttachmentFileSnapshot({ generation: 1, operationId: "op-1", fileName: "note.txt", byteSize: 10, state: "selected" });
  const completed = applyAttachmentFileStatus(initial, { generation: 1, operationId: "op-1", state: "completed", assetId: "asset-internal" });
  assert.equal(completed.stage, "Completed");
  assert.strictEqual(applyAttachmentFileStatus(completed, { generation: 1, operationId: "op-1", state: "completed", assetId: "asset-internal" }), completed);
  assert.strictEqual(applyAttachmentFileStatus(completed, { generation: 1, operationId: "op-1", state: "failed", errorCode: "LATE_FAILURE" }), completed);
  assert.equal(completed.userLabel.includes("asset-internal"), false);
});

test("recovery accepts only an explicit repair progression and clears the prior error", () => {
  const projecting = createAttachmentFileSnapshot({ generation: 4, operationId: "op-repair", fileName: "repair.pdf", byteSize: 10, state: "projecting" });
  const recovery = applyAttachmentFileStatus(projecting, {
    generation: 4,
    operationId: "op-repair",
    state: "recovery_required",
    errorCode: "asset_graph_reindex.repository_unavailable",
    assetId: "asset-1",
  });
  const repairing = applyAttachmentFileStatus(recovery, {
    generation: 4,
    operationId: "op-repair",
    state: "projecting",
  });
  const verifying = applyAttachmentFileStatus(repairing, {
    generation: 4,
    operationId: "op-repair",
    state: "verifying",
  });
  const completed = applyAttachmentFileStatus(verifying, {
    generation: 4,
    operationId: "op-repair",
    state: "completed",
  });

  assert.equal(repairing.stage, "Projecting");
  assert.equal(repairing.errorCode, undefined);
  assert.equal(verifying.stage, "Verifying");
  assert.equal(completed.stage, "Completed");
  assert.strictEqual(applyAttachmentFileStatus(recovery, { generation: 3, operationId: "op-repair", state: "projecting" }), recovery);
  assert.strictEqual(applyAttachmentFileStatus(recovery, { generation: 4, operationId: "other", state: "projecting" }), recovery);
});

function capabilities(state: string): readonly boolean[] {
  const value = presentAttachmentWireState(state);
  return [value.canCancel, value.canRetry, value.canRepair, value.canStartNewAttempt];
}
