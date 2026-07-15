import type {
  DesktopBackupClient,
  DesktopBackupDataClass,
  DesktopBackupManifestSummary,
  DesktopBackupOperationStatus,
  DesktopRestoreOperationStatus,
  DesktopStartupRecoveryResult,
} from "./desktop_backup_recovery_controller.ts";
import type { TauriInvoke } from "./tauri_home_transport.ts";

const DATA_CLASSES: readonly DesktopBackupDataClass[] = Object.freeze([
  "current_documents", "version_history", "canvas_records", "asset_metadata",
  "asset_objects", "asset_associations", "graph_rebuild_metadata", "search_rebuild_metadata",
]);

export class DesktopBackupTransportError extends Error {
  readonly code: string;
  readonly retryable: boolean;

  constructor(code: string, retryable: boolean) {
    super(code);
    this.name = "DesktopBackupTransportError";
    this.code = code;
    this.retryable = retryable;
  }
}

export function createTauriBackupRecoveryTransport(invoke: TauriInvoke): DesktopBackupClient {
  async function call(command: string, request: Record<string, unknown>): Promise<Record<string, unknown>> {
    let value: unknown;
    try {
      value = await invoke(command, { request });
    } catch {
      throw bridgeFailure();
    }
    if (!isRecord(value) || hasProhibitedKey(value)) throw bridgeFailure();
    if (value.ok === false && isNonEmptyString(value.errorCode) && typeof value.retryable === "boolean") {
      throw new DesktopBackupTransportError(value.errorCode, value.retryable);
    }
    if (value.ok !== true || value.retryable !== false || !isNonEmptyString(value.state)) throw bridgeFailure();
    return value;
  }

  return Object.freeze({
    async createBackup(input): Promise<DesktopBackupManifestSummary> {
      const response = await call("create_desktop_backup_package", { ...input });
      if (response.state !== "Ready") throw bridgeFailure();
      return parseManifest(response.manifest);
    },

    async startBackupOperation(input) {
      return parseOperation(await call("start_desktop_backup_operation", { ...input }), input.operationId);
    },

    async getBackupOperationStatus(input) {
      return parseOperation(await call("get_desktop_backup_operation_status", { ...input }), input.operationId);
    },

    async cancelBackupOperation(input) {
      return parseOperation(await call("cancel_desktop_backup_operation", { ...input }), input.operationId);
    },

    async startRestoreOperation(input) {
      if (input.confirmed !== true) throw bridgeFailure();
      return parseRestoreOperation(
        await call("start_desktop_restore_operation", { ...input }), input.operationId,
      );
    },

    async getRestoreOperationStatus(input) {
      return parseRestoreOperation(
        await call("get_desktop_restore_operation_status", { ...input }), input.operationId,
      );
    },

    async previewRestore(input) {
      const response = await call("preview_desktop_backup_restore", { ...input });
      if (!["AwaitingConfirmation", "Failed"].includes(String(response.state))) throw bridgeFailure();
      if (typeof response.confirmationReady !== "boolean") throw bridgeFailure();
      if (response.confirmationReady !== (response.state === "AwaitingConfirmation")) throw bridgeFailure();
      return Object.freeze({
        confirmationReady: response.confirmationReady,
        state: response.state as "AwaitingConfirmation" | "Failed",
        manifest: parseManifest(response.manifest),
        errorCode: optionalString(response.errorCode),
      });
    },

    async confirmRestore(input) {
      if (input.confirmed !== true) throw bridgeFailure();
      const response = await call("confirm_desktop_backup_restore", { ...input });
      if (!["Completed", "RolledBack", "CleanupRequired"].includes(String(response.state))) throw bridgeFailure();
      if (response.operationId !== input.operationId) throw bridgeFailure();
      return Object.freeze({
        state: response.state as "Completed" | "RolledBack" | "CleanupRequired",
        errorCode: optionalString(response.errorCode),
      });
    },

    async cancelRestore(input) {
      const response = await call("cancel_desktop_backup_restore", { ...input });
      if (!["Cancelled", "CleanupRequired"].includes(String(response.state))) throw bridgeFailure();
      if (response.operationId !== input.operationId) throw bridgeFailure();
      return Object.freeze({
        state: response.state as "Cancelled" | "CleanupRequired",
        errorCode: optionalString(response.errorCode),
      });
    },

    async recoverStartup(input): Promise<DesktopStartupRecoveryResult> {
      const response = await call("recover_desktop_backup_startup", { ...input });
      if (response.state !== "Completed" || !isRecovery(response.recovery)) throw bridgeFailure();
      return Object.freeze({
        cleanedStagingCount: response.recovery.cleanedStagingCount,
        rolledBackOperationIds: Object.freeze([...response.recovery.rolledBackOperationIds]),
        cleanupRequiredOperationIds: Object.freeze([...response.recovery.cleanupRequiredOperationIds]),
      });
    },
  });
}

function parseOperation(value: Record<string, unknown>, expectedOperationId: string): DesktopBackupOperationStatus {
  const states = ["Queued", "Running", "Completed", "Failed", "Retrying", "Abandoned"] as const;
  if (value.operationId !== expectedOperationId
    || !states.includes(value.state as typeof states[number])
    || !isCount(value.progressCompletedUnits)
    || !isCount(value.progressTotalUnits)
    || value.progressCompletedUnits > value.progressTotalUnits) {
    throw bridgeFailure();
  }
  return Object.freeze({
    operationId: expectedOperationId,
    state: value.state as DesktopBackupOperationStatus["state"],
    progressCompletedUnits: value.progressCompletedUnits,
    progressTotalUnits: value.progressTotalUnits,
    errorCode: optionalString(value.errorCode),
    retryable: value.retryable,
  });
}

function parseRestoreOperation(
  value: Record<string, unknown>,
  expectedOperationId: string,
): DesktopRestoreOperationStatus {
  const states = ["Staging", "Applying", "Reopening", "RollbackRequired", "Completed", "RolledBack", "CleanupRequired", "Cancelled", "Failed"] as const;
  if (value.operationId !== expectedOperationId
    || !states.includes(value.state as typeof states[number])) throw bridgeFailure();
  return Object.freeze({
    operationId: expectedOperationId,
    state: value.state as DesktopRestoreOperationStatus["state"],
    errorCode: optionalString(value.errorCode),
    retryable: value.retryable,
  });
}

function parseManifest(value: unknown): DesktopBackupManifestSummary {
  if (!isRecord(value) || hasProhibitedKey(value)
    || !isNonEmptyString(value.packageId) || value.schemaVersion !== 1
    || !Array.isArray(value.entries) || value.entries.length !== DATA_CLASSES.length) {
    throw bridgeFailure();
  }
  const entries = value.entries.map((entry, index) => {
    if (!isRecord(entry) || hasProhibitedKey(entry)
      || entry.dataClass !== DATA_CLASSES[index]
      || !isCount(entry.recordCount) || !isCount(entry.byteCount)) {
      throw bridgeFailure();
    }
    return Object.freeze({
      dataClass: entry.dataClass as DesktopBackupDataClass,
      recordCount: entry.recordCount,
      byteCount: entry.byteCount,
    });
  });
  const createdAtEpochMs = optionalPositiveInteger(value.createdAtEpochMs);
  return Object.freeze({
    packageId: value.packageId,
    schemaVersion: value.schemaVersion,
    ...(createdAtEpochMs === undefined ? {} : { createdAtEpochMs }),
    entries: Object.freeze(entries),
  });
}

function optionalPositiveInteger(value: unknown): number | undefined {
  if (value === undefined || value === null) return undefined;
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value <= 0) throw bridgeFailure();
  return value;
}

function isRecovery(value: unknown): value is {
  cleanedStagingCount: number;
  rolledBackOperationIds: string[];
  cleanupRequiredOperationIds: string[];
} {
  return isRecord(value) && !hasProhibitedKey(value)
    && isCount(value.cleanedStagingCount)
    && isStringArray(value.rolledBackOperationIds)
    && isStringArray(value.cleanupRequiredOperationIds);
}

function hasProhibitedKey(value: Record<string, unknown>): boolean {
  const prohibited = new Set(["path", "checksum", "checksumSha256", "documentBody", "objectBytes", "content", "bytes"]);
  return Object.entries(value).some(([key, child]) => prohibited.has(key)
    || (isRecord(child) && hasProhibitedKey(child))
    || (Array.isArray(child) && child.some((item) => isRecord(item) && hasProhibitedKey(item))));
}

function optionalString(value: unknown): string | undefined {
  if (value === undefined || value === null) return undefined;
  if (!isNonEmptyString(value)) throw bridgeFailure();
  return value;
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every(isNonEmptyString);
}
function isCount(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}
function isNonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
function bridgeFailure(): DesktopBackupTransportError {
  return new DesktopBackupTransportError("COMMAND_BRIDGE_FAILED", false);
}
