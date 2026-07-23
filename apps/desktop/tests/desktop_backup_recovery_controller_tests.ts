import assert from "node:assert/strict";
import test from "node:test";

import {
  cancelDesktopBackupOperation,
  confirmDesktopRestore,
  createDesktopBackupRecoverySnapshot,
  dismissDesktopRestoreConfirmation,
  loadDesktopBackupCatalog,
  pollDesktopBackupOperation,
  pollDesktopRestoreOperation,
  previewDesktopRestore,
  startDesktopBackupOperation,
  startDesktopRestoreOperation,
  selectDesktopBackupCatalogPackage,
  type DesktopBackupClient,
} from "../src/desktop_backup_recovery_controller.ts";

test("backup catalog loads independently and selects only a loaded package", async () => {
  const loaded = await loadDesktopBackupCatalog(fakeClient(), createDesktopBackupRecoverySnapshot("workspace-1"), { limit: 20 });
  assert.equal(loaded.catalogState, "Ready");
  assert.equal(loaded.catalogRecords.length, 1);
  const selected = selectDesktopBackupCatalogPackage(loaded, "package-1");
  assert.equal(selected.state, "Ready");
  assert.equal(selected.manifest?.packageId, "package-1");
  assert.equal(selectDesktopBackupCatalogPackage(loaded, "missing"), loaded);
});

test("durable backup start remains creating until completed status is validated", async () => {
  const client = fakeClient();
  const started = await startDesktopBackupOperation(
    client,
    createDesktopBackupRecoverySnapshot("workspace-1"),
    "backup-operation-1",
  );

  assert.equal(started.state, "Creating");
  assert.equal(started.operationId, "backup-operation-1");
  assert.deepEqual(started.operationProgress, { completedUnits: 0, totalUnits: 8 });
  assert.equal(started.manifest, undefined);

  const completed = await pollDesktopBackupOperation(client, started);
  assert.equal(completed.state, "Ready");
  assert.equal(completed.packageId, "backup-operation-1");
  assert.equal(completed.manifest?.entries.length, 8);
  assert.deepEqual(completed.operationProgress, { completedUnits: 8, totalUnits: 8 });
});

test("durable backup cancellation maps abandoned operation to cancelled UI state", async () => {
  const started = await startDesktopBackupOperation(
    fakeClient(),
    createDesktopBackupRecoverySnapshot("workspace-1"),
    "backup-operation-1",
  );

  const cancelled = await cancelDesktopBackupOperation(fakeClient(), started);
  assert.equal(cancelled.state, "Cancelled");
  assert.equal(cancelled.errorCode, "BACKUP_OPERATION_CANCELLED");
});

test("confirmed restore starts durable staging and polls only to native terminal state", async () => {
  const preview = await previewDesktopRestore(fakeClient(), createDesktopBackupRecoverySnapshot("workspace-1"), "package-1");
  const started = await startDesktopRestoreOperation(fakeClient(), preview, "restore-operation-1");
  assert.equal(started.state, "Applying");
  assert.equal(started.restoreOperationState, "Staging");
  assert.equal(started.operationId, "restore-operation-1");

  const completed = await pollDesktopRestoreOperation(fakeClient(), started);
  assert.equal(completed.state, "Completed");
  assert.equal(completed.restoreOperationState, "Completed");
});

test("restore confirmation cannot call client before validated preview", async () => {
  let confirms = 0;
  const client = fakeClient({ onConfirm: () => { confirms += 1; } });
  const idle = createDesktopBackupRecoverySnapshot("workspace-1");

  const unchanged = await confirmDesktopRestore(client, idle, "operation-1");

  assert.equal(unchanged, idle);
  assert.equal(confirms, 0);
});

test("validated preview exposes all backup classes and confirmation state", async () => {
  const client = fakeClient();
  const idle = createDesktopBackupRecoverySnapshot("workspace-1");

  const preview = await previewDesktopRestore(client, idle, "package-1");

  assert.equal(preview.state, "AwaitingConfirmation");
  assert.equal(preview.manifest?.entries.length, 8);
  assert.deepEqual(preview.manifest?.entries.map((entry) => entry.dataClass), [
    "current_documents", "version_history", "canvas_records", "asset_metadata",
    "asset_objects", "asset_associations", "graph_rebuild_metadata", "search_rebuild_metadata",
  ]);
});

test("reopen rollback is explicit and does not become completed", async () => {
  const client = fakeClient({ confirmState: "RolledBack", confirmError: "RESTORE_REOPEN_FAILED" });
  const preview = await previewDesktopRestore(client, createDesktopBackupRecoverySnapshot("workspace-1"), "package-1");

  const result = await confirmDesktopRestore(client, preview, "operation-1");

  assert.equal(result.state, "RolledBack");
  assert.equal(result.errorCode, "RESTORE_REOPEN_FAILED");
});

test("rollback failure remains recovery required and retryable", async () => {
  const client = fakeClient({ confirmState: "RecoveryRequired", confirmError: "RESTORE_ROLLBACK_FAILED" });
  const preview = await previewDesktopRestore(client, createDesktopBackupRecoverySnapshot("workspace-1"), "package-1");

  const result = await confirmDesktopRestore(client, preview, "operation-1");

  assert.equal(result.state, "RecoveryRequired");
  assert.equal(result.errorCode, "RESTORE_ROLLBACK_FAILED");
});

test("dismiss closes only a validated confirmation without native I/O", async () => {
  const preview = await previewDesktopRestore(fakeClient(), createDesktopBackupRecoverySnapshot("workspace-1"), "package-1");
  const dismissed = dismissDesktopRestoreConfirmation(preview);
  assert.equal(dismissed.state, "Ready");
  assert.equal(dismissed.packageId, "package-1");
  assert.equal(dismissDesktopRestoreConfirmation(createDesktopBackupRecoverySnapshot("workspace-1")).state, "Idle");
});

function fakeClient(options: { onConfirm?: () => void; confirmState?: "Completed" | "RolledBack" | "RecoveryRequired"; confirmError?: string } = {}): DesktopBackupClient {
  return {
    async listBackups() { return { records: Object.freeze([manifest()]) }; },
    async createBackup() { return manifest(); },
    async startBackupOperation(input) { return { operationId: input.operationId, state: "Queued", progressCompletedUnits: 0, progressTotalUnits: 8 }; },
    async getBackupOperationStatus(input) { return { operationId: input.operationId, state: "Completed", progressCompletedUnits: 8, progressTotalUnits: 8 }; },
    async cancelBackupOperation(input) { return { operationId: input.operationId, state: "Abandoned", progressCompletedUnits: 0, progressTotalUnits: 8, errorCode: "BACKUP_OPERATION_CANCELLED", retryable: false }; },
    async startRestoreOperation(input) { return { operationId: input.operationId, state: "Staging" }; },
    async getRestoreOperationStatus(input) { return { operationId: input.operationId, state: "Completed" }; },
    async previewRestore(input) { return { confirmationReady: true, state: "AwaitingConfirmation", manifest: manifest(input.packageId) }; },
    async confirmRestore() { options.onConfirm?.(); return { state: options.confirmState ?? "Completed", errorCode: options.confirmError }; },
    async cancelRestore() { return { state: "Cancelled" }; },
    async recoverStartup() { return { cleanedStagingCount: 0, rolledBackOperationIds: [], cleanupRequiredOperationIds: [] }; },
  };
}

function manifest(packageId = "package-1") {
  const classes = ["current_documents", "version_history", "canvas_records", "asset_metadata", "asset_objects", "asset_associations", "graph_rebuild_metadata", "search_rebuild_metadata"] as const;
  return { packageId, schemaVersion: 1, entries: classes.map((dataClass) => ({ dataClass, recordCount: 1, byteCount: 10 })) };
}
