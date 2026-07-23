export type DesktopBackupDataClass =
  | "current_documents" | "version_history" | "canvas_records"
  | "asset_metadata" | "asset_objects" | "asset_associations"
  | "graph_rebuild_metadata" | "search_rebuild_metadata";

export interface DesktopBackupManifestEntry {
  readonly dataClass: DesktopBackupDataClass;
  readonly recordCount: number;
  readonly byteCount: number;
}

export interface DesktopBackupManifestSummary {
  readonly packageId: string;
  readonly schemaVersion: number;
  readonly createdAtEpochMs?: number;
  readonly entries: readonly DesktopBackupManifestEntry[];
}

export interface DesktopBackupCatalogPage {
  readonly records: readonly DesktopBackupManifestSummary[];
  readonly nextCursor?: string;
}

export type DesktopBackupCatalogState = "Idle" | "Loading" | "Ready" | "Empty" | "Failed";

export type DesktopBackupRecoveryState = "Idle" | "Creating" | "Ready" | "Previewing"
  | "AwaitingConfirmation" | "Applying" | "Completed" | "Cancelled" | "RolledBack"
  | "Failed" | "CleanupRequired" | "RecoveryRequired";

export interface DesktopBackupRecoverySnapshot {
  readonly workspaceId: string;
  readonly state: DesktopBackupRecoveryState;
  readonly generation: number;
  readonly packageId?: string;
  readonly operationId?: string;
  readonly operationProgress?: DesktopBackupOperationProgress;
  readonly restoreOperationState?: DesktopRestoreOperationState;
  readonly manifest?: DesktopBackupManifestSummary;
  readonly errorCode?: string;
  readonly retryable?: boolean;
  readonly recovery?: DesktopStartupRecoveryResult;
  readonly catalogState: DesktopBackupCatalogState;
  readonly catalogRecords: readonly DesktopBackupManifestSummary[];
  readonly catalogNextCursor?: string;
  readonly selectedCatalogPackageId?: string;
}

export interface DesktopBackupOperationProgress {
  readonly completedUnits: number;
  readonly totalUnits: number;
}

export type DesktopBackupOperationState = "Queued" | "Running" | "Completed"
  | "Failed" | "Retrying" | "Abandoned";

export interface DesktopBackupOperationStatus {
  readonly operationId: string;
  readonly state: DesktopBackupOperationState;
  readonly progressCompletedUnits: number;
  readonly progressTotalUnits: number;
  readonly errorCode?: string;
  readonly retryable?: boolean;
}

export type DesktopRestoreOperationState = "Staging" | "Applying" | "Reopening"
  | "RollbackRequired" | "RecoveryRequired" | "Completed" | "RolledBack" | "CleanupRequired"
  | "Cancelled" | "Failed";

export interface DesktopRestoreOperationStatus {
  readonly operationId: string;
  readonly state: DesktopRestoreOperationState;
  readonly errorCode?: string;
  readonly retryable?: boolean;
}

export interface DesktopStartupRecoveryResult {
  readonly cleanedStagingCount: number;
  readonly rolledBackOperationIds: readonly string[];
  readonly cleanupRequiredOperationIds: readonly string[];
}

export interface DesktopBackupClient {
  listBackups(input: { workspaceId: string; cursor?: string; limit: number }): Promise<DesktopBackupCatalogPage>;
  createBackup(input: { workspaceId: string; packageId: string }): Promise<DesktopBackupManifestSummary>;
  startBackupOperation(input: { workspaceId: string; operationId: string }): Promise<DesktopBackupOperationStatus>;
  getBackupOperationStatus(input: { workspaceId: string; operationId: string }): Promise<DesktopBackupOperationStatus>;
  cancelBackupOperation(input: { workspaceId: string; operationId: string }): Promise<DesktopBackupOperationStatus>;
  startRestoreOperation(input: { workspaceId: string; packageId: string; operationId: string; confirmed: true }): Promise<DesktopRestoreOperationStatus>;
  getRestoreOperationStatus(input: { workspaceId: string; operationId: string }): Promise<DesktopRestoreOperationStatus>;
  previewRestore(input: { workspaceId: string; packageId: string }): Promise<{
    confirmationReady: boolean;
    state: "AwaitingConfirmation" | "Failed";
    manifest: DesktopBackupManifestSummary;
    errorCode?: string;
  }>;
  confirmRestore(input: { workspaceId: string; packageId: string; operationId: string; confirmed: true }): Promise<{
    state: "Completed" | "RolledBack" | "CleanupRequired" | "RecoveryRequired";
    errorCode?: string;
    retryable?: boolean;
  }>;
  cancelRestore(input: { workspaceId: string; operationId: string }): Promise<{ state: "Cancelled" | "CleanupRequired"; errorCode?: string }>;
  recoverStartup(input: { workspaceId: string }): Promise<DesktopStartupRecoveryResult>;
}

export async function startDesktopRestoreOperation(
  client: DesktopBackupClient,
  snapshot: DesktopBackupRecoverySnapshot,
  operationId: string,
): Promise<DesktopBackupRecoverySnapshot> {
  if (snapshot.state !== "AwaitingConfirmation" || !snapshot.packageId) return snapshot;
  try {
    const result = await client.startRestoreOperation({
      workspaceId: snapshot.workspaceId,
      packageId: snapshot.packageId,
      operationId,
      confirmed: true,
    });
    if (result.operationId !== operationId || !["Staging", "Applying", "Reopening"].includes(result.state)) {
      return failed(snapshot, { code: "RESTORE_OPERATION_INVALID_STATE", retryable: false });
    }
    return restoreOperationSnapshot(snapshot, result);
  } catch (error) {
    return failed(snapshot, error);
  }
}

export async function pollDesktopRestoreOperation(
  client: DesktopBackupClient,
  snapshot: DesktopBackupRecoverySnapshot,
): Promise<DesktopBackupRecoverySnapshot> {
  if (snapshot.state !== "Applying" || !snapshot.operationId) return snapshot;
  try {
    const result = await client.getRestoreOperationStatus({
      workspaceId: snapshot.workspaceId,
      operationId: snapshot.operationId,
    });
    if (result.operationId !== snapshot.operationId) {
      return failed(snapshot, { code: "RESTORE_OPERATION_ID_MISMATCH", retryable: false });
    }
    return restoreOperationSnapshot(snapshot, result);
  } catch (error) {
    return failed(snapshot, error);
  }
}

export async function startDesktopBackupOperation(
  client: DesktopBackupClient,
  snapshot: DesktopBackupRecoverySnapshot,
  operationId: string,
): Promise<DesktopBackupRecoverySnapshot> {
  if (snapshot.state === "Creating" || snapshot.state === "Applying") return snapshot;
  try {
    const result = await client.startBackupOperation({ workspaceId: snapshot.workspaceId, operationId });
    if (result.operationId !== operationId || !["Queued", "Running", "Retrying"].includes(result.state)) {
      return failed(snapshot, { code: "BACKUP_OPERATION_INVALID_STATE", retryable: false });
    }
    return operationSnapshot(snapshot, result, "Creating");
  } catch (error) {
    return failed({ ...snapshot, generation: snapshot.generation + 1 }, error);
  }
}

export async function pollDesktopBackupOperation(
  client: DesktopBackupClient,
  snapshot: DesktopBackupRecoverySnapshot,
): Promise<DesktopBackupRecoverySnapshot> {
  if (snapshot.state !== "Creating" || !snapshot.operationId) return snapshot;
  try {
    const result = await client.getBackupOperationStatus({
      workspaceId: snapshot.workspaceId,
      operationId: snapshot.operationId,
    });
    if (result.operationId !== snapshot.operationId) {
      return failed(snapshot, { code: "BACKUP_OPERATION_ID_MISMATCH", retryable: false });
    }
    if (["Queued", "Running", "Retrying"].includes(result.state)) {
      return operationSnapshot(snapshot, result, "Creating");
    }
    if (result.state === "Abandoned") return operationSnapshot(snapshot, result, "Cancelled");
    if (result.state === "Failed") return operationSnapshot(snapshot, result, "Failed");

    const preview = await client.previewRestore({
      workspaceId: snapshot.workspaceId,
      packageId: snapshot.operationId,
    });
    if (!preview.confirmationReady || preview.state !== "AwaitingConfirmation") {
      return failed(snapshot, { code: preview.errorCode ?? "BACKUP_PACKAGE_VALIDATION_FAILED", retryable: false });
    }
    return Object.freeze({
      ...operationSnapshot(snapshot, result, "Ready"),
      packageId: preview.manifest.packageId,
      manifest: preview.manifest,
    });
  } catch (error) {
    return failed(snapshot, error);
  }
}

export async function cancelDesktopBackupOperation(
  client: DesktopBackupClient,
  snapshot: DesktopBackupRecoverySnapshot,
): Promise<DesktopBackupRecoverySnapshot> {
  if (snapshot.state !== "Creating" || !snapshot.operationId) return snapshot;
  try {
    const result = await client.cancelBackupOperation({
      workspaceId: snapshot.workspaceId,
      operationId: snapshot.operationId,
    });
    if (result.operationId !== snapshot.operationId || result.state !== "Abandoned") {
      return failed(snapshot, { code: "BACKUP_OPERATION_CANCEL_FAILED", retryable: false });
    }
    return operationSnapshot(snapshot, result, "Cancelled");
  } catch (error) {
    return failed(snapshot, error);
  }
}

export function createDesktopBackupRecoverySnapshot(workspaceId: string): DesktopBackupRecoverySnapshot {
  return Object.freeze({ workspaceId, state: "Idle", generation: 0, catalogState: "Idle", catalogRecords: Object.freeze([]) });
}

export async function loadDesktopBackupCatalog(
  client: DesktopBackupClient,
  snapshot: DesktopBackupRecoverySnapshot,
  input: { readonly cursor?: string; readonly limit: number },
): Promise<DesktopBackupRecoverySnapshot> {
  const pending = Object.freeze({ ...snapshot, generation: snapshot.generation + 1, catalogState: "Loading" as const });
  try {
    const page = await client.listBackups({ workspaceId: snapshot.workspaceId, ...input });
    const records = input.cursor ? [...snapshot.catalogRecords, ...page.records] : [...page.records];
    return Object.freeze({
      ...pending,
      catalogState: records.length === 0 ? "Empty" as const : "Ready" as const,
      catalogRecords: Object.freeze(records),
      catalogNextCursor: page.nextCursor,
    });
  } catch (error) {
    const failure = failed(pending, error);
    return Object.freeze({ ...failure, catalogState: "Failed" as const });
  }
}

export function selectDesktopBackupCatalogPackage(
  snapshot: DesktopBackupRecoverySnapshot,
  packageId: string,
): DesktopBackupRecoverySnapshot {
  const manifest = snapshot.catalogRecords.find((record) => record.packageId === packageId);
  if (!manifest || snapshot.catalogState !== "Ready") return snapshot;
  return Object.freeze({
    ...snapshot,
    state: "Ready",
    packageId,
    manifest,
    selectedCatalogPackageId: packageId,
    errorCode: undefined,
  });
}

export async function createDesktopBackup(
  client: DesktopBackupClient,
  snapshot: DesktopBackupRecoverySnapshot,
  packageId: string,
): Promise<DesktopBackupRecoverySnapshot> {
  if (snapshot.state === "Creating" || snapshot.state === "Applying") return snapshot;
  const pending = { ...snapshot, state: "Creating" as const, generation: snapshot.generation + 1, errorCode: undefined };
  try {
    const manifest = await client.createBackup({ workspaceId: snapshot.workspaceId, packageId });
    return Object.freeze({ ...pending, state: "Ready", packageId: manifest.packageId, manifest });
  } catch (error) {
    return failed(pending, error);
  }
}

export async function previewDesktopRestore(
  client: DesktopBackupClient,
  snapshot: DesktopBackupRecoverySnapshot,
  packageId: string,
): Promise<DesktopBackupRecoverySnapshot> {
  if (snapshot.state === "Creating" || snapshot.state === "Applying") return snapshot;
  const pending = { ...snapshot, state: "Previewing" as const, generation: snapshot.generation + 1, packageId, errorCode: undefined };
  try {
    const result = await client.previewRestore({ workspaceId: snapshot.workspaceId, packageId });
    return Object.freeze({
      ...pending,
      state: result.confirmationReady && result.state === "AwaitingConfirmation" ? "AwaitingConfirmation" : "Failed",
      manifest: result.manifest,
      errorCode: result.errorCode,
    });
  } catch (error) {
    return failed(pending, error);
  }
}

export async function confirmDesktopRestore(
  client: DesktopBackupClient,
  snapshot: DesktopBackupRecoverySnapshot,
  operationId: string,
): Promise<DesktopBackupRecoverySnapshot> {
  if (snapshot.state !== "AwaitingConfirmation" || !snapshot.packageId) return snapshot;
  const pending = { ...snapshot, state: "Applying" as const, generation: snapshot.generation + 1, operationId, errorCode: undefined };
  try {
    const result = await client.confirmRestore({ workspaceId: snapshot.workspaceId, packageId: snapshot.packageId, operationId, confirmed: true });
    return Object.freeze({ ...pending, state: result.state, errorCode: result.errorCode, retryable: result.retryable });
  } catch (error) {
    return failed(pending, error);
  }
}

export async function cancelDesktopRestore(client: DesktopBackupClient, snapshot: DesktopBackupRecoverySnapshot): Promise<DesktopBackupRecoverySnapshot> {
  if (!snapshot.operationId || !["Applying", "CleanupRequired"].includes(snapshot.state)) return snapshot;
  try {
    const result = await client.cancelRestore({ workspaceId: snapshot.workspaceId, operationId: snapshot.operationId });
    return Object.freeze({ ...snapshot, state: result.state, restoreOperationState: result.state, errorCode: result.errorCode });
  } catch (error) {
    return failed(snapshot, error);
  }
}

export function dismissDesktopRestoreConfirmation(
  snapshot: DesktopBackupRecoverySnapshot,
): DesktopBackupRecoverySnapshot {
  if (snapshot.state !== "AwaitingConfirmation") return snapshot;
  return Object.freeze({ ...snapshot, state: "Ready", errorCode: undefined });
}

export async function recoverDesktopBackupStartup(client: DesktopBackupClient, snapshot: DesktopBackupRecoverySnapshot): Promise<DesktopBackupRecoverySnapshot> {
  try {
    const recovery = await client.recoverStartup({ workspaceId: snapshot.workspaceId });
    return Object.freeze({
      ...snapshot,
      generation: snapshot.generation + 1,
      state: recovery.cleanupRequiredOperationIds.length > 0 ? "CleanupRequired" : recovery.rolledBackOperationIds.length > 0 ? "RolledBack" : snapshot.state,
      recovery,
      errorCode: recovery.cleanupRequiredOperationIds.length > 0 ? "RESTORE_CLEANUP_REQUIRED" : undefined,
    });
  } catch (error) {
    return failed(snapshot, error);
  }
}

function failed(snapshot: DesktopBackupRecoverySnapshot, error: unknown): DesktopBackupRecoverySnapshot {
  const value = error as { code?: unknown; retryable?: unknown };
  return Object.freeze({
    ...snapshot,
    state: "Failed",
    errorCode: typeof value?.code === "string" ? value.code : "BACKUP_COMMAND_FAILED",
    retryable: value?.retryable === true,
  });
}

function operationSnapshot(
  snapshot: DesktopBackupRecoverySnapshot,
  result: DesktopBackupOperationStatus,
  state: DesktopBackupRecoveryState,
): DesktopBackupRecoverySnapshot {
  return Object.freeze({
    ...snapshot,
    state,
    generation: snapshot.generation + 1,
    operationId: result.operationId,
    packageId: result.operationId,
    operationProgress: Object.freeze({
      completedUnits: result.progressCompletedUnits,
      totalUnits: result.progressTotalUnits,
    }),
    errorCode: result.errorCode,
    retryable: result.retryable,
  });
}

function restoreOperationSnapshot(
  snapshot: DesktopBackupRecoverySnapshot,
  result: DesktopRestoreOperationStatus,
): DesktopBackupRecoverySnapshot {
  const state: DesktopBackupRecoveryState = ["Staging", "Applying", "Reopening", "RollbackRequired"].includes(result.state)
    ? "Applying"
    : result.state;
  return Object.freeze({
    ...snapshot,
    state,
    generation: snapshot.generation + 1,
    operationId: result.operationId,
    restoreOperationState: result.state,
    errorCode: result.errorCode,
    retryable: result.retryable,
  });
}
