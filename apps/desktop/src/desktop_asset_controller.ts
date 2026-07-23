import {
  LocalDesktopCommandClientError,
  type DocumentAssetsPage,
  type LocalDesktopCommandClient,
} from "@sponzey-cabinet/client-core";
import {
  DesktopAssetImportTransportError,
  type DesktopAssetImportPickerClient,
  type DesktopAssetImportSelection,
  type DesktopAssetDetail,
  type DesktopAssetPreview,
  type DesktopAssetImportStatus,
  type DesktopWorkspaceAssetsPage,
} from "./tauri_asset_import_transport.ts";
import {
  applyAttachmentFileStatus,
  createAttachmentFileSnapshot,
  type AttachmentFileSnapshot,
} from "./attachment_operation_presenter.ts";
import type { DesktopProjectionTransport } from "./tauri_projection_transport.ts";

export type DesktopAssetSurfaceState = "Idle" | "Loading" | "Ready" | "Empty" | "Failed";
export type DesktopAssetImportState = "Idle" | "Selecting" | "Importing" | "Completed" | "Cancelled" | "Failed";
export type DesktopAssetMediaFilter = "all" | "image" | "pdf" | "document" | "other";

export interface DesktopAssetSurfaceSnapshot {
  readonly state: DesktopAssetSurfaceState;
  readonly scope?: "Workspace" | "Document";
  readonly workspaceId: string;
  readonly documentId?: string;
  readonly generation: number;
  readonly importState: DesktopAssetImportState;
  readonly importGeneration: number;
  readonly query: string;
  readonly mediaFilter: DesktopAssetMediaFilter;
  readonly page?: DesktopAssetPage;
  readonly selectedAssetId?: string;
  readonly errorCode?: string;
  readonly retryable?: boolean;
  readonly importErrorCode?: string;
  readonly importOperationId?: string;
  readonly importOperations?: readonly AttachmentFileSnapshot[];
  readonly detailState?: "Idle" | "Loading" | "Ready" | "Failed";
  readonly detail?: DesktopAssetDetail;
  readonly mutationState?: "Idle" | "Linking" | "Unlinking" | "Failed";
  readonly previewState?: "Idle" | "Loading" | "Ready" | "Unsupported" | "Failed";
  readonly previewGeneration?: number;
  readonly preview?: DesktopAssetPreview;
  readonly openState?: "Idle" | "Opening" | "Opened" | "OpenFailed";
  readonly openGeneration?: number;
  readonly openErrorCode?: string;
  readonly dropState?: "Idle" | "Entered";
  readonly dropFileCount?: number;
  readonly requestedCursor?: string;
}

export interface DesktopAssetPage {
  readonly queryName: "list-document-assets" | "list-workspace-assets";
  readonly workspaceId: string;
  readonly documentId?: string;
  readonly assets: DocumentAssetsPage["assets"];
  readonly nextCursor?: string;
}

export interface DesktopAssetPlacementOption {
  readonly identity: string;
  readonly label: string;
}

export interface DesktopAssetImportReadbackPolicy {
  readonly attempts: number;
  readonly intervalMs: number;
  readonly delay: (milliseconds: number) => Promise<void>;
}

const defaultAssetImportReadbackPolicy: DesktopAssetImportReadbackPolicy = Object.freeze({
  attempts: 100,
  intervalMs: 25,
  delay: (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds)),
});

export function createDesktopAssetPlacementOptions(
  snapshot: DesktopAssetSurfaceSnapshot,
): readonly DesktopAssetPlacementOption[] {
  return Object.freeze((snapshot.page?.assets ?? []).map((asset) => Object.freeze({
    identity: asset.assetId,
    label: asset.fileName,
  })));
}

export function createDesktopAssetSnapshot(workspaceId: string): DesktopAssetSurfaceSnapshot {
  return Object.freeze({ state: "Idle", scope: "Workspace", workspaceId, generation: 0, importState: "Idle", importGeneration: 0, query: "", mediaFilter: "all", importOperations: Object.freeze([]), dropState: "Idle", dropFileCount: 0 });
}

export function setDesktopAssetQuery(
  snapshot: DesktopAssetSurfaceSnapshot,
  query: string,
): DesktopAssetSurfaceSnapshot {
  return reconcileDesktopAssetFilterSelection(Object.freeze({ ...snapshot, query }));
}

export function setDesktopAssetMediaFilter(
  snapshot: DesktopAssetSurfaceSnapshot,
  mediaFilter: DesktopAssetMediaFilter,
): DesktopAssetSurfaceSnapshot {
  return reconcileDesktopAssetFilterSelection(Object.freeze({ ...snapshot, mediaFilter }));
}

export function visibleDesktopAssets(
  snapshot: DesktopAssetSurfaceSnapshot,
): DocumentAssetsPage["assets"] {
  const query = (snapshot.query ?? "").trim().toLocaleLowerCase();
  const mediaFilter = snapshot.mediaFilter ?? "all";
  return Object.freeze((snapshot.page?.assets ?? []).filter((asset) => {
    if (query && !`${asset.fileName}\n${asset.label}`.toLocaleLowerCase().includes(query)) return false;
    if (mediaFilter === "image") return asset.mediaType.startsWith("image/");
    if (mediaFilter === "pdf") return asset.mediaType === "application/pdf";
    if (mediaFilter === "document") return /text|word|document/.test(asset.mediaType);
    if (mediaFilter === "other") return !/image|pdf|text|word|document/.test(asset.mediaType);
    return true;
  }));
}

function reconcileDesktopAssetFilterSelection(
  snapshot: DesktopAssetSurfaceSnapshot,
): DesktopAssetSurfaceSnapshot {
  const visible = visibleDesktopAssets(snapshot);
  if (visible.some((asset) => asset.assetId === snapshot.selectedAssetId)) return snapshot;
  return clearDesktopAssetSelection(snapshot, visible[0]?.assetId);
}

export function applyDesktopAssetDragState(
  snapshot: DesktopAssetSurfaceSnapshot,
  event: { readonly state: "entered" | "left" | "dropped"; readonly fileCount: number },
): DesktopAssetSurfaceSnapshot {
  if (snapshot.scope !== "Document" || snapshot.importState === "Selecting" || snapshot.importState === "Importing") {
    return snapshot;
  }
  const result = Object.freeze({
    ...snapshot,
    dropState: event.state === "entered" ? "Entered" : "Idle",
    dropFileCount: event.state === "entered" ? event.fileCount : 0,
  });
  return result;
}

export function requestDesktopAssetLoad(
  snapshot: DesktopAssetSurfaceSnapshot,
  documentId: string | undefined,
): DesktopAssetSurfaceSnapshot {
  const normalizedDocumentId = documentId?.trim() || undefined;
  if (!normalizedDocumentId) {
    return Object.freeze({
      ...snapshot,
      state: "Loading",
      scope: "Workspace",
      workspaceId: snapshot.workspaceId,
      documentId: undefined,
      generation: snapshot.generation + 1,
      importState: "Idle",
      importErrorCode: undefined,
      dropState: "Idle",
      dropFileCount: 0,
    });
  }
  const documentChanged = snapshot.documentId !== normalizedDocumentId;
  return Object.freeze({
    ...snapshot,
    state: "Loading",
    scope: "Document",
    workspaceId: snapshot.workspaceId,
    documentId: normalizedDocumentId,
    generation: snapshot.generation + 1,
    importState: documentChanged ? "Idle" : snapshot.importState,
    importErrorCode: documentChanged ? undefined : snapshot.importErrorCode,
    importOperations: documentChanged ? Object.freeze([]) : snapshot.importOperations ?? Object.freeze([]),
    dropState: "Idle",
    dropFileCount: 0,
  });
}

export function requestDesktopWorkspaceAssetLoad(
  snapshot: DesktopAssetSurfaceSnapshot,
  targetDocumentId: string | undefined = snapshot.documentId,
): DesktopAssetSurfaceSnapshot {
  return Object.freeze({
    ...snapshot,
    state: "Loading",
    scope: "Workspace",
    documentId: targetDocumentId?.trim() || undefined,
    generation: snapshot.generation + 1,
    errorCode: undefined,
    retryable: undefined,
    requestedCursor: undefined,
  });
}

function clearDesktopAssetSelection(
  snapshot: DesktopAssetSurfaceSnapshot,
  selectedAssetId: string | undefined,
): DesktopAssetSurfaceSnapshot {
  return Object.freeze({
    ...snapshot,
    selectedAssetId,
    detailState: "Idle",
    detail: undefined,
    mutationState: "Idle",
    previewState: "Idle",
    preview: undefined,
    openState: "Idle",
    openErrorCode: undefined,
  });
}

export function requestDesktopWorkspaceAssetNextPage(
  snapshot: DesktopAssetSurfaceSnapshot,
): DesktopAssetSurfaceSnapshot {
  const cursor = snapshot.page?.nextCursor;
  if (snapshot.scope !== "Workspace" || snapshot.state === "Loading" || !cursor) return snapshot;
  return Object.freeze({
    ...snapshot,
    state: "Loading",
    generation: snapshot.generation + 1,
    requestedCursor: cursor,
    errorCode: undefined,
    retryable: undefined,
  });
}

export function beginDesktopAssetImport(snapshot: DesktopAssetSurfaceSnapshot): DesktopAssetSurfaceSnapshot {
  if (!snapshot.documentId || snapshot.importState === "Selecting" || snapshot.importState === "Importing") return snapshot;
  return Object.freeze({
    ...snapshot,
    importState: "Selecting",
    importGeneration: snapshot.importGeneration + 1,
    importErrorCode: undefined,
    importOperations: Object.freeze([]),
    dropState: "Idle",
    dropFileCount: 0,
  });
}

export async function importDesktopDocumentAssets(
  importClient: DesktopAssetImportPickerClient,
  queryClient: Pick<LocalDesktopCommandClient, "getAssetMetadata" | "getCurrentDocument">,
  selecting: DesktopAssetSurfaceSnapshot,
  operationIdSource: () => string,
  onProgress: (snapshot: DesktopAssetSurfaceSnapshot) => void,
  preparedSelection?: DesktopAssetImportSelection,
  readbackPolicy: DesktopAssetImportReadbackPolicy = defaultAssetImportReadbackPolicy,
): Promise<DesktopAssetSurfaceSnapshot> {
  if (selecting.importState !== "Selecting" || !selecting.documentId) return selecting;
  try {
    const selection = preparedSelection ?? await importClient.selectFiles();
    if (selection.cancelled || selection.files.length === 0) {
      return Object.freeze({ ...selecting, importState: "Idle", importErrorCode: undefined });
    }
    let operations = Object.freeze(selection.files.map((file, index) => createAttachmentFileSnapshot({
      generation: selecting.importGeneration,
      operationId: `pending-${index + 1}`,
      fileName: file.fileName,
      byteSize: file.byteSize,
      state: "selected",
    })));
    let importing = Object.freeze({ ...selecting, importState: "Importing" as const, importOperations: operations });
    onProgress(importing);
    const importedAssetIds: string[] = [];
    const importedFileNames: string[] = [];
    const importedIndexes: number[] = [];
    for (const [index, file] of selection.files.entries()) {
      const attachmentOperationId = operationIdSource().trim();
      if (!attachmentOperationId) {
        operations = replaceOperation(operations, index, Object.freeze({
          ...operations[index]!,
          ...applyAttachmentFileStatus(
            Object.freeze({ ...operations[index]!, operationId: "" }),
            { generation: selecting.importGeneration, operationId: "", state: "failed", errorCode: "asset_import.invalid_operation_id" },
          ),
        }));
        importing = Object.freeze({ ...importing, importOperations: operations, importErrorCode: "asset_import.invalid_operation_id" });
        onProgress(importing);
        continue;
      }
      try {
        const current = await queryClient.getCurrentDocument({
          queryName: "get-current-document",
          workspaceId: selecting.workspaceId,
          documentId: selecting.documentId,
        });
        const expectedCurrentVersionToken = current.versionId.trim();
        if (!expectedCurrentVersionToken) {
          throw new DesktopAssetImportTransportError("asset_import.invalid_current_version", false);
        }
        let result: DesktopAssetImportStatus = await importClient.importFile({
          workspaceId: selecting.workspaceId,
          documentId: selecting.documentId,
          handle: file.handle,
          label: file.fileName,
          attachmentOperationId,
          expectedCurrentVersionToken,
        });
        let operationSnapshot = createAttachmentFileSnapshot({
          generation: selecting.importGeneration,
          operationId: result.operationId,
          fileName: file.fileName,
          byteSize: file.byteSize,
          state: result.state === "completed" ? "verifying" : result.state,
        });
        if (result.errorCode) {
          operationSnapshot = applyAttachmentFileStatus(operationSnapshot, {
            generation: selecting.importGeneration,
            operationId: result.operationId,
            state: result.state,
            errorCode: result.errorCode,
            assetId: result.assetId,
          });
        }
        operations = replaceOperation(operations, index, operationSnapshot);
        importing = Object.freeze({
          ...importing,
          importOperationId: result.operationId,
          importOperations: operations,
          importErrorCode: result.errorCode,
        });
        onProgress(importing);
        for (let poll = 0; result.state !== "completed" && poll < 200; poll += 1) {
          const currentOperation = operations[index]!;
          if (currentOperation.terminal || currentOperation.canRepair || currentOperation.canRetry) break;
          result = await importClient.getImportStatus({ workspaceId: selecting.workspaceId, operationId: result.operationId });
          const visibleState = result.state === "completed" ? "verifying" : result.state;
          operations = replaceOperation(operations, index, applyAttachmentFileStatus(
            currentOperation,
            {
              generation: selecting.importGeneration,
              operationId: result.operationId,
              state: visibleState,
              errorCode: result.errorCode,
              assetId: result.assetId,
            },
          ));
          importing = Object.freeze({ ...importing, importOperationId: result.operationId, importOperations: operations });
          onProgress(importing);
          if (result.state !== "completed" && !operations[index]!.terminal && !operations[index]!.canRepair && !operations[index]!.canRetry) {
            await new Promise((resolve) => setTimeout(resolve, 25));
          }
        }
        if (result.state !== "completed") {
          if (!operations[index]!.terminal && !operations[index]!.canRepair && !operations[index]!.canRetry) {
            throw new DesktopAssetImportTransportError("asset_import.status_timeout", true);
          }
          continue;
        }
        if (result.assetId) importedAssetIds.push(result.assetId);
        importedFileNames.push(file.fileName);
        importedIndexes.push(index);
      } catch (error) {
        const mapped = error instanceof DesktopAssetImportTransportError ? error.code : "COMMAND_BRIDGE_FAILED";
        const currentOperation = Object.freeze({ ...operations[index]!, operationId: operations[index]!.operationId.startsWith("pending-") ? attachmentOperationId : operations[index]!.operationId });
        operations = replaceOperation(operations, index, applyAttachmentFileStatus(currentOperation, {
          generation: selecting.importGeneration,
          operationId: currentOperation.operationId,
          state: "failed",
          errorCode: mapped,
        }));
        importing = Object.freeze({ ...importing, importOperations: operations, importErrorCode: mapped });
        onProgress(importing);
      }
    }
    const attempts = Math.max(1, Math.trunc(readbackPolicy.attempts));
    let readback = await loadDesktopDocumentAssets(
      queryClient,
      requestDesktopAssetLoad(importing, selecting.documentId),
    );
    let readbackMatched = assetImportReadbackMatches(readback, importedAssetIds, importedFileNames);
    for (let attempt = 1; !readbackMatched && importedIndexes.length > 0 && attempt < attempts; attempt += 1) {
      await readbackPolicy.delay(Math.max(0, readbackPolicy.intervalMs));
      readback = await loadDesktopDocumentAssets(
        queryClient,
        requestDesktopAssetLoad(importing, selecting.documentId),
      );
      readbackMatched = assetImportReadbackMatches(readback, importedAssetIds, importedFileNames);
    }
    for (const index of importedIndexes) {
      const currentOperation = operations[index]!;
      operations = replaceOperation(operations, index, applyAttachmentFileStatus(currentOperation, {
        generation: selecting.importGeneration,
        operationId: currentOperation.operationId,
        state: readbackMatched ? "completed" : "recovery_required",
        errorCode: readbackMatched ? undefined : "ASSET_IMPORT_READBACK_MISMATCH",
      }));
    }
    const hasFailure = operations.some((operation) => operation.stage !== "Completed");
    const operationErrorCode = operations.find((operation) => operation.stage !== "Completed")?.errorCode;
    if (!readbackMatched || hasFailure) {
      return Object.freeze({
        ...readback,
        importOperations: operations,
        importState: "Failed",
        importErrorCode: readback.errorCode
          ?? (!readbackMatched ? "ASSET_IMPORT_READBACK_MISMATCH" : operationErrorCode ?? importing.importErrorCode ?? "ASSET_IMPORT_PARTIAL_FAILURE"),
      });
    }
    return Object.freeze({
      ...readback,
      importOperations: operations,
      importState: "Completed",
      selectedAssetId: importedAssetIds.at(-1) ?? readback.page?.assets.find((asset) => asset.fileName === importedFileNames.at(-1))?.assetId,
      importErrorCode: undefined,
    });
  } catch (error) {
    const mapped = error instanceof DesktopAssetImportTransportError
      ? error.code
      : "COMMAND_BRIDGE_FAILED";
    return Object.freeze({
      ...selecting,
      importState: mapped === "asset_import.cancelled" ? "Cancelled" : "Failed",
      importErrorCode: mapped,
    });
  }
}

function assetImportReadbackMatches(
  readback: DesktopAssetSurfaceSnapshot,
  importedAssetIds: readonly string[],
  importedFileNames: readonly string[],
): boolean {
  return readback.state !== "Failed"
    && importedAssetIds.every((id) => readback.page?.assets.some((asset) => asset.assetId === id))
    && importedFileNames.every((name) => readback.page?.assets.some((asset) => asset.fileName === name));
}

function replaceOperation(
  operations: readonly AttachmentFileSnapshot[],
  index: number,
  operation: AttachmentFileSnapshot,
): readonly AttachmentFileSnapshot[] {
  return Object.freeze(operations.map((current, currentIndex) => currentIndex === index ? operation : current));
}

export async function cancelDesktopAssetImport(
  client: DesktopAssetImportPickerClient,
  snapshot: DesktopAssetSurfaceSnapshot,
): Promise<DesktopAssetSurfaceSnapshot> {
  if (snapshot.importState !== "Importing" || !snapshot.importOperationId) return snapshot;
  try {
    const status = await client.cancelImport({ workspaceId: snapshot.workspaceId, operationId: snapshot.importOperationId });
    const importOperations = Object.freeze((snapshot.importOperations ?? []).map((operation) =>
      operation.operationId === status.operationId
        ? applyAttachmentFileStatus(operation, {
            generation: operation.generation,
            operationId: operation.operationId,
            state: status.state,
          })
        : operation));
    return Object.freeze({
      ...snapshot,
      importOperations,
      importState: status.state === "cancelled" ? "Cancelled" : status.state === "cleanup_required" ? "Failed" : snapshot.importState,
      importErrorCode: status.state === "cleanup_required" ? "asset_import.cleanup_required" : undefined,
    });
  } catch (error) {
    return Object.freeze({
      ...snapshot,
      importState: "Failed",
      importErrorCode: error instanceof DesktopAssetImportTransportError ? error.code : "COMMAND_BRIDGE_FAILED",
    });
  }
}

export async function repairDesktopAttachmentProjection(
  projection: Pick<DesktopProjectionTransport, "startRepair" | "runRepair">,
  query: Pick<LocalDesktopCommandClient, "getAssetMetadata">,
  snapshot: DesktopAssetSurfaceSnapshot,
  attachmentOperationId: string,
  onProgress: (snapshot: DesktopAssetSurfaceSnapshot) => void = () => {},
): Promise<DesktopAssetSurfaceSnapshot> {
  if (snapshot.scope !== "Document" || !snapshot.documentId) return snapshot;
  const operationIndex = (snapshot.importOperations ?? []).findIndex((item) =>
    item.operationId === attachmentOperationId && item.canRepair);
  if (operationIndex < 0) return snapshot;
  const original = snapshot.importOperations![operationIndex]!;
  let operations = replaceOperation(snapshot.importOperations!, operationIndex, applyAttachmentFileStatus(original, {
    generation: original.generation,
    operationId: original.operationId,
    state: "projecting",
  }));
  let working = Object.freeze({
    ...snapshot,
    importState: "Importing" as const,
    importOperationId: attachmentOperationId,
    importOperations: operations,
    importErrorCode: undefined,
  });
  onProgress(working);

  try {
    const started = await projection.startRepair(snapshot.workspaceId, snapshot.documentId);
    const repaired = await projection.runRepair(snapshot.workspaceId, started.operationId);
    if (repaired.state !== "succeeded") {
      return attachmentRepairFailure(working, operationIndex, "ATTACHMENT_PROJECTION_RECOVERY_REQUIRED");
    }
    const projecting = operations[operationIndex]!;
    operations = replaceOperation(operations, operationIndex, applyAttachmentFileStatus(projecting, {
      generation: projecting.generation,
      operationId: projecting.operationId,
      state: "verifying",
    }));
    working = Object.freeze({ ...working, importOperations: operations });
    onProgress(working);
    const loading = requestDesktopAssetLoad(working, snapshot.documentId);
    const readback = await loadDesktopDocumentAssets(query, loading);
    const assets = readback.page?.assets ?? [];
    const matched = readback.state !== "Failed" && assets.some((asset) =>
      original.assetId ? asset.assetId === original.assetId : asset.fileName === original.displayName);
    if (!matched) {
      return attachmentRepairFailure(
        { ...readback, importOperations: operations },
        operationIndex,
        "ASSET_IMPORT_READBACK_MISMATCH",
      );
    }
    const verifying = operations[operationIndex]!;
    operations = replaceOperation(operations, operationIndex, applyAttachmentFileStatus(verifying, {
      generation: verifying.generation,
      operationId: verifying.operationId,
      state: "completed",
      assetId: original.assetId,
    }));
    const allCompleted = operations.every((item) => item.stage === "Completed");
    return Object.freeze({
      ...readback,
      importOperations: operations,
      importState: allCompleted ? "Completed" : "Failed",
      importErrorCode: allCompleted ? undefined : "ASSET_IMPORT_PARTIAL_FAILURE",
    });
  } catch {
    return attachmentRepairFailure(working, operationIndex, "ATTACHMENT_PROJECTION_RECOVERY_REQUIRED");
  }
}

function attachmentRepairFailure(
  snapshot: DesktopAssetSurfaceSnapshot,
  operationIndex: number,
  errorCode: string,
): DesktopAssetSurfaceSnapshot {
  const operations = snapshot.importOperations ?? [];
  const operation = operations[operationIndex];
  if (!operation) return Object.freeze({ ...snapshot, importState: "Failed", importErrorCode: errorCode });
  const failed = applyAttachmentFileStatus(operation, {
    generation: operation.generation,
    operationId: operation.operationId,
    state: "recovery_required",
    errorCode,
    assetId: operation.assetId,
  });
  return Object.freeze({
    ...snapshot,
    importOperations: replaceOperation(operations, operationIndex, failed),
    importState: "Failed",
    importErrorCode: errorCode,
  });
}

export async function loadDesktopDocumentAssets(
  client: Pick<LocalDesktopCommandClient, "getAssetMetadata">,
  loading: DesktopAssetSurfaceSnapshot,
): Promise<DesktopAssetSurfaceSnapshot> {
  if (loading.state !== "Loading" || loading.scope !== "Document" || !loading.documentId) return loading;
  try {
    const page = await client.getAssetMetadata({
      queryName: "list-document-assets",
      workspaceId: loading.workspaceId,
      documentId: loading.documentId,
    });
    return applyDesktopAssetResult(loading, loading.generation, page);
  } catch (error) {
    const mapped = error instanceof LocalDesktopCommandClientError
      ? { code: error.code, retryable: error.retryable }
      : { code: "COMMAND_BRIDGE_FAILED", retryable: false };
    return applyDesktopAssetFailure(loading, loading.generation, mapped.code, mapped.retryable);
  }
}

export async function loadDesktopWorkspaceAssets(
  client: Pick<DesktopAssetImportPickerClient, "listWorkspaceAssets">,
  loading: DesktopAssetSurfaceSnapshot,
): Promise<DesktopAssetSurfaceSnapshot> {
  if (loading.state !== "Loading" || loading.scope !== "Workspace") return loading;
  try {
    const result: DesktopWorkspaceAssetsPage = await client.listWorkspaceAssets({
      workspaceId: loading.workspaceId,
      ...(loading.requestedCursor ? { cursor: loading.requestedCursor } : {}),
      limit: 200,
    });
    const assets = loading.requestedCursor
      ? mergeWorkspaceAssets(loading.page?.assets ?? [], result.assets)
      : result.assets;
    return applyDesktopAssetResult(loading, loading.generation, {
      queryName: "list-workspace-assets",
      workspaceId: result.workspaceId,
      assets,
      ...(result.nextCursor ? { nextCursor: result.nextCursor } : {}),
    });
  } catch (error) {
    const mapped = error instanceof DesktopAssetImportTransportError
      ? { code: error.code, retryable: error.retryable }
      : { code: "COMMAND_BRIDGE_FAILED", retryable: false };
    return loading.requestedCursor
      ? Object.freeze({ ...loading, state: "Failed", errorCode: mapped.code, retryable: mapped.retryable })
      : applyDesktopAssetFailure(loading, loading.generation, mapped.code, mapped.retryable);
  }
}

export function applyDesktopAssetResult(
  snapshot: DesktopAssetSurfaceSnapshot,
  generation: number,
  page: DesktopAssetPage,
): DesktopAssetSurfaceSnapshot {
  if (generation !== snapshot.generation) return snapshot;
  const selectedAssetId = page.assets.some((asset) => asset.assetId === snapshot.selectedAssetId)
    ? snapshot.selectedAssetId
    : page.assets[0]?.assetId;
  const result = Object.freeze({
    ...snapshot,
    state: page.assets.length === 0 ? "Empty" : "Ready",
    page,
    selectedAssetId,
    errorCode: undefined,
    retryable: undefined,
    requestedCursor: undefined,
  });
  return selectedAssetId === snapshot.selectedAssetId
    ? result
    : clearDesktopAssetSelection(result, selectedAssetId);
}

function mergeWorkspaceAssets(
  current: DocumentAssetsPage["assets"],
  incoming: DocumentAssetsPage["assets"],
): DocumentAssetsPage["assets"] {
  const identities = new Set(current.map((asset) => asset.assetId));
  return Object.freeze([
    ...current,
    ...incoming.filter((asset) => {
      if (identities.has(asset.assetId)) return false;
      identities.add(asset.assetId);
      return true;
    }),
  ]);
}

export async function linkDesktopSelectedAsset(
  lifecycleClient: Pick<DesktopAssetImportPickerClient, "link">,
  queryClient: Pick<LocalDesktopCommandClient, "getAssetMetadata" | "getCurrentDocument">,
  snapshot: DesktopAssetSurfaceSnapshot,
  operationIdSource: () => string,
): Promise<DesktopAssetSurfaceSnapshot> {
  if (!snapshot.documentId || !snapshot.selectedAssetId || snapshot.mutationState === "Linking") return snapshot;
  const asset = snapshot.page?.assets.find((item) => item.assetId === snapshot.selectedAssetId);
  if (!asset) return snapshot;
  const linking = Object.freeze({ ...snapshot, mutationState: "Linking" as const });
  try {
    const operationId = operationIdSource().trim();
    if (!operationId) return Object.freeze({ ...snapshot, mutationState: "Failed" });
    const current = await queryClient.getCurrentDocument({
      queryName: "get-current-document",
      workspaceId: snapshot.workspaceId,
      documentId: snapshot.documentId,
    });
    await lifecycleClient.link({
      workspaceId: snapshot.workspaceId,
      documentId: snapshot.documentId,
      assetId: asset.assetId,
      label: asset.label || asset.fileName,
      operationId,
      expectedCurrentVersionToken: current.versionId,
    });
    const readback = await loadDesktopDocumentAssets(
      queryClient,
      requestDesktopAssetLoad(linking, snapshot.documentId),
    );
    if (!readback.page?.assets.some((item) => item.assetId === asset.assetId)) {
      return Object.freeze({ ...readback, mutationState: "Failed" });
    }
    return Object.freeze({ ...readback, selectedAssetId: asset.assetId, mutationState: "Idle" });
  } catch {
    return Object.freeze({ ...snapshot, mutationState: "Failed" });
  }
}

export function applyDesktopAssetFailure(
  snapshot: DesktopAssetSurfaceSnapshot,
  generation: number,
  errorCode: string,
  retryable: boolean,
): DesktopAssetSurfaceSnapshot {
  if (generation !== snapshot.generation) return snapshot;
  return Object.freeze({
    ...snapshot,
    state: "Failed",
    page: undefined,
    selectedAssetId: undefined,
    errorCode,
    retryable,
  });
}

export function selectDesktopAsset(
  snapshot: DesktopAssetSurfaceSnapshot,
  assetId: string,
): DesktopAssetSurfaceSnapshot {
  if (!snapshot.page?.assets.some((asset) => asset.assetId === assetId)) return snapshot;
  return Object.freeze({
    ...snapshot,
    selectedAssetId: assetId,
    detailState: "Loading",
    detail: undefined,
    mutationState: "Idle",
    previewState: "Idle",
    preview: undefined,
    openState: "Idle",
    openErrorCode: undefined,
    openGeneration: (snapshot.openGeneration ?? 0) + 1,
  });
}

export function requestDesktopAssetOpen(snapshot: DesktopAssetSurfaceSnapshot): DesktopAssetSurfaceSnapshot {
  if (!snapshot.selectedAssetId || snapshot.openState === "Opening") return snapshot;
  return Object.freeze({
    ...snapshot,
    openState: "Opening",
    openGeneration: (snapshot.openGeneration ?? 0) + 1,
    openErrorCode: undefined,
  });
}

export async function openDesktopSelectedAsset(
  client: Pick<DesktopAssetImportPickerClient, "openExternal">,
  opening: DesktopAssetSurfaceSnapshot,
): Promise<DesktopAssetSurfaceSnapshot> {
  if (opening.openState !== "Opening" || !opening.selectedAssetId) return opening;
  try {
    const result = await client.openExternal({
      workspaceId: opening.workspaceId,
      assetId: opening.selectedAssetId,
    });
    return Object.freeze({
      ...opening,
      openState: result.opened ? "Opened" : "OpenFailed",
      openErrorCode: result.opened ? undefined : "ASSET_EXTERNAL_OPEN_FAILED",
    });
  } catch (error) {
    return Object.freeze({
      ...opening,
      openState: "OpenFailed",
      openErrorCode: error instanceof DesktopAssetImportTransportError
        ? error.code
        : "COMMAND_BRIDGE_FAILED",
    });
  }
}

export function requestDesktopAssetPreview(snapshot: DesktopAssetSurfaceSnapshot): DesktopAssetSurfaceSnapshot {
  if (!snapshot.selectedAssetId || snapshot.previewState === "Loading") return snapshot;
  return Object.freeze({
    ...snapshot,
    previewState: "Loading",
    previewGeneration: (snapshot.previewGeneration ?? 0) + 1,
    preview: undefined,
  });
}

export async function loadDesktopAssetPreview(
  client: Pick<DesktopAssetImportPickerClient, "getPreview">,
  snapshot: DesktopAssetSurfaceSnapshot,
): Promise<DesktopAssetSurfaceSnapshot> {
  if (snapshot.previewState !== "Loading" || !snapshot.selectedAssetId) return snapshot;
  try {
    const preview = await client.getPreview({ workspaceId: snapshot.workspaceId, assetId: snapshot.selectedAssetId });
    if (preview.assetId !== snapshot.selectedAssetId) return Object.freeze({ ...snapshot, previewState: "Failed", preview: undefined });
    return Object.freeze({ ...snapshot, previewState: preview.presentation === "unsupported" ? "Unsupported" : "Ready", preview });
  } catch {
    return Object.freeze({ ...snapshot, previewState: "Failed", preview: undefined });
  }
}

export function closeDesktopAssetPreview(snapshot: DesktopAssetSurfaceSnapshot): DesktopAssetSurfaceSnapshot {
  if (!snapshot.previewState || snapshot.previewState === "Idle") return snapshot;
  return Object.freeze({ ...snapshot, previewState: "Idle", preview: undefined });
}

export async function loadDesktopAssetDetail(
  client: DesktopAssetImportPickerClient,
  snapshot: DesktopAssetSurfaceSnapshot,
): Promise<DesktopAssetSurfaceSnapshot> {
  if (snapshot.detailState !== "Loading" || !snapshot.selectedAssetId) return snapshot;
  try {
    const detail = await client.getDetail({ workspaceId: snapshot.workspaceId, assetId: snapshot.selectedAssetId });
    if (detail.assetId !== snapshot.selectedAssetId) return Object.freeze({ ...snapshot, detailState: "Failed" });
    return Object.freeze({ ...snapshot, detailState: "Ready", detail });
  } catch {
    return Object.freeze({ ...snapshot, detailState: "Failed", detail: undefined });
  }
}

export function applyDesktopAssetDetailResult(
  current: DesktopAssetSurfaceSnapshot,
  result: DesktopAssetSurfaceSnapshot,
): DesktopAssetSurfaceSnapshot {
  if (!result.selectedAssetId || current.selectedAssetId !== result.selectedAssetId) return current;
  return Object.freeze({
    ...current,
    detailState: result.detailState,
    detail: result.detail,
  });
}

export async function unlinkDesktopSelectedAsset(
  lifecycleClient: DesktopAssetImportPickerClient,
  queryClient: Pick<LocalDesktopCommandClient, "getAssetMetadata" | "getCurrentDocument">,
  snapshot: DesktopAssetSurfaceSnapshot,
  operationIdSource: () => string,
): Promise<DesktopAssetSurfaceSnapshot> {
  if (!snapshot.documentId || !snapshot.selectedAssetId || snapshot.mutationState === "Unlinking") return snapshot;
  const assetId = snapshot.selectedAssetId;
  const unlinking = Object.freeze({ ...snapshot, mutationState: "Unlinking" as const });
  try {
    const operationId = operationIdSource().trim();
    if (!operationId) return Object.freeze({ ...snapshot, mutationState: "Failed" });
    const current = await queryClient.getCurrentDocument({ queryName: "get-current-document", workspaceId: snapshot.workspaceId, documentId: snapshot.documentId });
    await lifecycleClient.unlink({ workspaceId: snapshot.workspaceId, documentId: snapshot.documentId, assetId, operationId, expectedCurrentVersionToken: current.versionId });
    const readback = await loadDesktopDocumentAssets(queryClient, requestDesktopAssetLoad(unlinking, snapshot.documentId));
    if (readback.page?.assets.some((asset) => asset.assetId === assetId)) {
      return Object.freeze({ ...readback, mutationState: "Failed" });
    }
    return Object.freeze({ ...readback, mutationState: "Idle", detailState: "Idle", detail: undefined });
  } catch {
    return Object.freeze({ ...snapshot, mutationState: "Failed" });
  }
}
