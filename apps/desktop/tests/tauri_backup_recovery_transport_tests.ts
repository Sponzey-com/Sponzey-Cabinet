import assert from "node:assert/strict";
import test from "node:test";

import {
  DesktopBackupTransportError,
  createTauriBackupRecoveryTransport,
} from "../src/tauri_backup_recovery_transport.ts";

test("backup transport maps all commands and returns immutable typed results", async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const transport = createTauriBackupRecoveryTransport(async (command, args) => {
    calls.push({ command, args });
    if (command === "start_desktop_backup_operation") return operation("Queued", 0);
    if (command === "get_desktop_backup_operation_status") return operation("Completed", 8);
    if (command === "cancel_desktop_backup_operation") return operation("Abandoned", 3, "BACKUP_OPERATION_CANCELLED");
    if (command === "start_desktop_restore_operation") return restoreOperation("Staging");
    if (command === "get_desktop_restore_operation_status") return restoreOperation("Completed");
    if (command === "recover_desktop_backup_startup") {
      return { ok: true, state: "Completed", recovery: { cleanedStagingCount: 1, rolledBackOperationIds: [], cleanupRequiredOperationIds: [] }, retryable: false };
    }
    if (command === "list_desktop_backup_catalog") return { ok: true, state: "Ready", records: [manifest()], nextCursor: "1", retryable: false };
    if (command === "confirm_desktop_backup_restore") return { ok: true, state: "Completed", operationId: "operation-1", retryable: false };
    if (command === "cancel_desktop_backup_restore") return { ok: true, state: "Cancelled", operationId: "operation-1", retryable: false };
    return { ok: true, state: command.startsWith("preview") ? "AwaitingConfirmation" : "Ready", confirmationReady: command.startsWith("preview") ? true : undefined, manifest: manifest(), retryable: false };
  });

  const catalog = await transport.listBackups({ workspaceId: "workspace-1", limit: 20 });
  const created = await transport.createBackup({ workspaceId: "workspace-1", packageId: "package-1" });
  const started = await transport.startBackupOperation({ workspaceId: "workspace-1", operationId: "operation-1" });
  const status = await transport.getBackupOperationStatus({ workspaceId: "workspace-1", operationId: "operation-1" });
  const operationCancelled = await transport.cancelBackupOperation({ workspaceId: "workspace-1", operationId: "operation-1" });
  const restoreStarted = await transport.startRestoreOperation({ workspaceId: "workspace-1", packageId: "package-1", operationId: "operation-1", confirmed: true });
  const restoreStatus = await transport.getRestoreOperationStatus({ workspaceId: "workspace-1", operationId: "operation-1" });
  const preview = await transport.previewRestore({ workspaceId: "workspace-1", packageId: "package-1" });
  const confirmed = await transport.confirmRestore({ workspaceId: "workspace-1", packageId: "package-1", operationId: "operation-1", confirmed: true });
  const cancelled = await transport.cancelRestore({ workspaceId: "workspace-1", operationId: "operation-1" });
  const recovery = await transport.recoverStartup({ workspaceId: "workspace-1" });

  assert.equal(created.entries.length, 8);
  assert.equal(catalog.records.length, 1);
  assert.equal(catalog.nextCursor, "1");
  assert.equal(created.createdAtEpochMs, 1_784_064_000_000);
  assert.equal(started.state, "Queued");
  assert.equal(status.state, "Completed");
  assert.equal(operationCancelled.state, "Abandoned");
  assert.equal(operationCancelled.errorCode, "BACKUP_OPERATION_CANCELLED");
  assert.equal(restoreStarted.state, "Staging");
  assert.equal(restoreStatus.state, "Completed");
  assert.equal(preview.confirmationReady, true);
  assert.equal(confirmed.state, "Completed");
  assert.equal(cancelled.state, "Cancelled");
  assert.equal(recovery.cleanedStagingCount, 1);
  assert.deepEqual(calls.map((call) => call.command), [
    "list_desktop_backup_catalog", "create_desktop_backup_package", "start_desktop_backup_operation",
    "get_desktop_backup_operation_status", "cancel_desktop_backup_operation",
    "start_desktop_restore_operation", "get_desktop_restore_operation_status",
    "preview_desktop_backup_restore",
    "confirm_desktop_backup_restore", "cancel_desktop_backup_restore",
    "recover_desktop_backup_startup",
  ]);
  assert.deepEqual(calls[0]?.args, { request: { workspaceId: "workspace-1", limit: 20 } });
  assert.deepEqual(calls[1]?.args, { request: { workspaceId: "workspace-1", packageId: "package-1" } });
  assert.ok(Object.isFrozen(created));
  assert.ok(Object.isFrozen(created.entries));
});

test("backup transport rejects native failure, malformed classes, and sensitive fields", async () => {
  const failure = createTauriBackupRecoveryTransport(async () => ({ ok: false, state: "Failed", errorCode: "BACKUP_PACKAGE_STORAGE_UNAVAILABLE", retryable: true }));
  const malformed = createTauriBackupRecoveryTransport(async () => ({ ok: true, state: "Ready", manifest: { ...manifest(), entries: manifest().entries.slice(0, 7) }, retryable: false }));
  const leaking = createTauriBackupRecoveryTransport(async () => ({ ok: true, state: "Ready", manifest: { ...manifest(), path: "/Users/private" }, retryable: false }));
  const malformedTime = createTauriBackupRecoveryTransport(async () => ({ ok: true, state: "Ready", manifest: { ...manifest(), createdAtEpochMs: 0 }, retryable: false }));

  await assert.rejects(() => failure.createBackup({ workspaceId: "workspace-1", packageId: "package-1" }), (error: unknown) => error instanceof DesktopBackupTransportError && error.code === "BACKUP_PACKAGE_STORAGE_UNAVAILABLE" && error.retryable);
  await assert.rejects(() => malformed.createBackup({ workspaceId: "workspace-1", packageId: "package-1" }), /COMMAND_BRIDGE_FAILED/);
  await assert.rejects(() => leaking.createBackup({ workspaceId: "workspace-1", packageId: "package-1" }), /COMMAND_BRIDGE_FAILED/);
  await assert.rejects(() => malformedTime.createBackup({ workspaceId: "workspace-1", packageId: "package-1" }), /COMMAND_BRIDGE_FAILED/);
});

test("backup transport accepts legacy manifest without creation time", async () => {
  const legacy = createTauriBackupRecoveryTransport(async () => {
    const { createdAtEpochMs: _omitted, ...value } = manifest();
    return { ok: true, state: "Ready", manifest: value, retryable: false };
  });
  assert.equal(
    (await legacy.createBackup({ workspaceId: "workspace-1", packageId: "package-1" })).createdAtEpochMs,
    undefined,
  );
});

test("backup transport preserves explicit restore recovery-required state", async () => {
  const transport = createTauriBackupRecoveryTransport(async (command) => {
    if (command === "confirm_desktop_backup_restore") {
      return { ok: true, state: "RecoveryRequired", operationId: "operation-1", errorCode: "RESTORE_ROLLBACK_FAILED", retryable: true };
    }
    return restoreOperation("RecoveryRequired");
  });

  const confirmed = await transport.confirmRestore({ workspaceId: "workspace-1", packageId: "package-1", operationId: "operation-1", confirmed: true });
  const status = await transport.getRestoreOperationStatus({ workspaceId: "workspace-1", operationId: "operation-1" });

  assert.equal(confirmed.state, "RecoveryRequired");
  assert.equal(confirmed.errorCode, "RESTORE_ROLLBACK_FAILED");
  assert.equal(status.state, "RecoveryRequired");
});

test("backup catalog transport rejects duplicate, unsorted, and sensitive records", async () => {
  const duplicate = createTauriBackupRecoveryTransport(async () => ({
    ok: true, state: "Ready", records: [manifest(), manifest()], retryable: false,
  }));
  const unsorted = createTauriBackupRecoveryTransport(async () => ({
    ok: true, state: "Ready", records: [
      { ...manifest(), packageId: "older", createdAtEpochMs: 100 },
      { ...manifest(), packageId: "newer", createdAtEpochMs: 200 },
    ], retryable: false,
  }));
  const leaking = createTauriBackupRecoveryTransport(async () => ({
    ok: true, state: "Ready", records: [{ ...manifest(), path: "/private/backup" }], retryable: false,
  }));

  await assert.rejects(() => duplicate.listBackups({ workspaceId: "workspace-1", limit: 20 }), /COMMAND_BRIDGE_FAILED/);
  await assert.rejects(() => unsorted.listBackups({ workspaceId: "workspace-1", limit: 20 }), /COMMAND_BRIDGE_FAILED/);
  await assert.rejects(() => leaking.listBackups({ workspaceId: "workspace-1", limit: 20 }), /COMMAND_BRIDGE_FAILED/);
});

function manifest() {
  const classes = ["current_documents", "version_history", "canvas_records", "asset_metadata", "asset_objects", "asset_associations", "graph_rebuild_metadata", "search_rebuild_metadata"] as const;
  return { packageId: "package-1", schemaVersion: 1, createdAtEpochMs: 1_784_064_000_000, entries: classes.map((dataClass) => ({ dataClass, recordCount: 1, byteCount: 10 })) };
}

function operation(state: string, completed: number, errorCode?: string) {
  return { ok: true, operationId: "operation-1", state, progressCompletedUnits: completed, progressTotalUnits: 8, errorCode, retryable: false };
}

function restoreOperation(state: string) {
  return { ok: true, operationId: "operation-1", state, retryable: false };
}
