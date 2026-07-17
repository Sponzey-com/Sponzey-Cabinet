import {
  LocalDesktopCommandClientError,
  type DocumentAssetsPage,
  type LocalDesktopCommandClient,
} from "@sponzey-cabinet/client-core";
import {
  DesktopAssetImportTransportError,
  type DesktopAssetImportPickerClient,
  type DesktopAssetDetail,
  type DesktopAssetPreview,
  type DesktopAssetImportStatus,
  type DesktopWorkspaceAssetsPage,
} from "./tauri_asset_import_transport.ts";

export type DesktopAssetSurfaceState = "Idle" | "Loading" | "Ready" | "Empty" | "Failed";
export type DesktopAssetImportState = "Idle" | "Selecting" | "Importing" | "Completed" | "Cancelled" | "Failed";

export interface DesktopAssetSurfaceSnapshot {
  readonly state: DesktopAssetSurfaceState;
  readonly scope?: "Workspace" | "Document";
  readonly workspaceId: string;
  readonly documentId?: string;
  readonly generation: number;
  readonly importState: DesktopAssetImportState;
  readonly importGeneration: number;
  readonly page?: DesktopAssetPage;
  readonly selectedAssetId?: string;
  readonly errorCode?: string;
  readonly retryable?: boolean;
  readonly importErrorCode?: string;
  readonly importOperationId?: string;
  readonly detailState?: "Idle" | "Loading" | "Ready" | "Failed";
  readonly detail?: DesktopAssetDetail;
  readonly mutationState?: "Idle" | "Linking" | "Unlinking" | "Failed";
  readonly previewState?: "Idle" | "Loading" | "Ready" | "Unsupported" | "Failed";
  readonly previewGeneration?: number;
  readonly preview?: DesktopAssetPreview;
  readonly openState?: "Idle" | "Opening" | "Opened" | "OpenFailed";
  readonly openGeneration?: number;
  readonly openErrorCode?: string;
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

export function createDesktopAssetPlacementOptions(
  snapshot: DesktopAssetSurfaceSnapshot,
): readonly DesktopAssetPlacementOption[] {
  return Object.freeze((snapshot.page?.assets ?? []).map((asset) => Object.freeze({
    identity: asset.assetId,
    label: asset.fileName,
  })));
}

export function createDesktopAssetSnapshot(workspaceId: string): DesktopAssetSurfaceSnapshot {
  return Object.freeze({ state: "Idle", scope: "Workspace", workspaceId, generation: 0, importState: "Idle", importGeneration: 0 });
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
  });
}

export function beginDesktopAssetImport(snapshot: DesktopAssetSurfaceSnapshot): DesktopAssetSurfaceSnapshot {
  if (!snapshot.documentId || snapshot.importState === "Selecting" || snapshot.importState === "Importing") return snapshot;
  return Object.freeze({
    ...snapshot,
    importState: "Selecting",
    importGeneration: snapshot.importGeneration + 1,
    importErrorCode: undefined,
  });
}

export async function importDesktopDocumentAssets(
  importClient: DesktopAssetImportPickerClient,
  queryClient: Pick<LocalDesktopCommandClient, "getAssetMetadata" | "getCurrentDocument">,
  selecting: DesktopAssetSurfaceSnapshot,
  operationIdSource: () => string,
  onProgress: (snapshot: DesktopAssetSurfaceSnapshot) => void,
): Promise<DesktopAssetSurfaceSnapshot> {
  if (selecting.importState !== "Selecting" || !selecting.documentId) return selecting;
  try {
    const selection = await importClient.selectFiles();
    if (selection.cancelled || selection.files.length === 0) {
      return Object.freeze({ ...selecting, importState: "Idle", importErrorCode: undefined });
    }
    const importing = Object.freeze({ ...selecting, importState: "Importing" as const });
    onProgress(importing);
    const importedAssetIds: string[] = [];
    const importedFileNames: string[] = [];
    for (const file of selection.files) {
      const attachmentOperationId = operationIdSource().trim();
      if (!attachmentOperationId) {
        throw new DesktopAssetImportTransportError("asset_import.invalid_operation_id", false);
      }
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
      onProgress(Object.freeze({ ...importing, importOperationId: result.operationId }));
      for (let poll = 0; result.state !== "completed" && poll < 200; poll += 1) {
        result = await importClient.getImportStatus({
          workspaceId: selecting.workspaceId,
          operationId: result.operationId,
        });
        if (!["selected", "validating", "staging", "hashing", "publishing_object", "persisting_metadata", "linking", "completed"].includes(result.state)) {
          throw new DesktopAssetImportTransportError(`asset_import.${result.state}`, false);
        }
        if (result.state !== "completed") await new Promise((resolve) => setTimeout(resolve, 25));
      }
      if (result.state !== "completed") throw new DesktopAssetImportTransportError("asset_import.status_timeout", true);
      if (result.assetId) importedAssetIds.push(result.assetId);
      importedFileNames.push(file.fileName);
    }
    const loading = requestDesktopAssetLoad(importing, selecting.documentId);
    const readback = await loadDesktopDocumentAssets(queryClient, loading);
    if (readback.state === "Failed"
      || importedAssetIds.some((id) => !readback.page?.assets.some((asset) => asset.assetId === id))
      || importedFileNames.some((name) => !readback.page?.assets.some((asset) => asset.fileName === name))) {
      return Object.freeze({
        ...readback,
        importState: "Failed",
        importErrorCode: readback.errorCode ?? "ASSET_IMPORT_READBACK_MISMATCH",
      });
    }
    return Object.freeze({
      ...readback,
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

export async function cancelDesktopAssetImport(
  client: DesktopAssetImportPickerClient,
  snapshot: DesktopAssetSurfaceSnapshot,
): Promise<DesktopAssetSurfaceSnapshot> {
  if (snapshot.importState !== "Importing" || !snapshot.importOperationId) return snapshot;
  try {
    const status = await client.cancelImport({ workspaceId: snapshot.workspaceId, operationId: snapshot.importOperationId });
    return Object.freeze({
      ...snapshot,
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
      limit: 200,
    });
    return applyDesktopAssetResult(loading, loading.generation, {
      queryName: "list-workspace-assets",
      workspaceId: result.workspaceId,
      assets: result.assets,
      ...(result.nextCursor ? { nextCursor: result.nextCursor } : {}),
    });
  } catch (error) {
    const mapped = error instanceof DesktopAssetImportTransportError
      ? { code: error.code, retryable: error.retryable }
      : { code: "COMMAND_BRIDGE_FAILED", retryable: false };
    return applyDesktopAssetFailure(loading, loading.generation, mapped.code, mapped.retryable);
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
  return Object.freeze({
    ...snapshot,
    state: page.assets.length === 0 ? "Empty" : "Ready",
    page,
    selectedAssetId,
    errorCode: undefined,
    retryable: undefined,
  });
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
