import type { TauriInvoke } from "./tauri_home_transport.ts";

const importStates = ["selected", "validating", "staging", "hashing", "publishing_object", "persisting_metadata", "preparing_revision", "linking", "associating", "projecting", "verifying", "completed", "validation_failed", "staging_failed", "object_publish_failed", "metadata_persist_failed", "link_failed", "cancelling", "cancelled", "conflict", "recovery_required", "cleanup_required", "failed"] as const;
export type DesktopAssetImportState = typeof importStates[number];

export interface DesktopAssetImportDescriptor {
  readonly handle: string;
  readonly fileName: string;
  readonly mediaType: string;
  readonly byteSize: number;
}

export interface DesktopAssetImportSelection {
  readonly cancelled: boolean;
  readonly files: readonly DesktopAssetImportDescriptor[];
}

export interface DesktopAssetImportPickerClient {
  selectFiles(): Promise<DesktopAssetImportSelection>;
  importFile(request: DesktopAssetImportRequest): Promise<DesktopAssetImportResult>;
  getImportStatus(request: DesktopAssetImportStatusRequest): Promise<DesktopAssetImportStatus>;
  cancelImport(request: DesktopAssetImportStatusRequest): Promise<DesktopAssetImportStatus>;
  getDetail(request: DesktopAssetDetailRequest): Promise<DesktopAssetDetail>;
  getPreview(request: DesktopAssetDetailRequest): Promise<DesktopAssetPreview>;
  openExternal(request: DesktopAssetDetailRequest): Promise<DesktopAssetExternalOpenResult>;
  listWorkspaceAssets(request: DesktopWorkspaceAssetsRequest): Promise<DesktopWorkspaceAssetsPage>;
  link(request: DesktopAssetLinkRequest): Promise<DesktopAssetLinkResult>;
  unlink(request: DesktopAssetUnlinkRequest): Promise<DesktopAssetUnlinkResult>;
}

export interface DesktopAssetImportRequest {
  readonly workspaceId: string;
  readonly documentId: string;
  readonly handle: string;
  readonly label: string;
  readonly attachmentOperationId: string;
  readonly expectedCurrentVersionToken: string;
}

export interface DesktopAssetImportResult {
  readonly operationId: string;
  readonly assetId?: string;
  readonly state: DesktopAssetImportState;
  readonly errorCode?: string;
  readonly retryable: boolean;
  readonly repairRequired: boolean;
}
export interface DesktopAssetImportStatusRequest {
  readonly workspaceId: string;
  readonly operationId: string;
}
export interface DesktopAssetImportStatus extends DesktopAssetImportResult {
  readonly state: DesktopAssetImportState;
}

export interface DesktopAssetDetailRequest {
  readonly workspaceId: string;
  readonly assetId: string;
}
export interface DesktopAssetUnlinkRequest extends DesktopAssetDetailRequest {
  readonly documentId: string;
  readonly operationId: string;
  readonly expectedCurrentVersionToken: string;
}
export interface DesktopAssetLinkRequest extends DesktopAssetUnlinkRequest {
  readonly label: string;
}
export interface DesktopWorkspaceAssetsRequest {
  readonly workspaceId: string;
  readonly cursor?: string;
  readonly limit: number;
}
export interface DesktopWorkspaceAsset {
  readonly assetId: string;
  readonly label: string;
  readonly fileName: string;
  readonly mediaType: string;
  readonly byteSize: number;
  readonly status: string;
}
export interface DesktopWorkspaceAssetsPage {
  readonly workspaceId: string;
  readonly assets: readonly DesktopWorkspaceAsset[];
  readonly nextCursor?: string;
}
export interface DesktopAssetDetail {
  readonly assetId: string;
  readonly fileName: string;
  readonly mediaType: string;
  readonly byteSize: number;
  readonly version: number;
  readonly previewCapability: "image" | "pdf" | "text" | "unsupported";
  readonly extractionStatus: "not_requested" | "pending" | "ready" | "unsupported" | "failed";
  readonly referenceCount: number;
  readonly linkedDocumentIds: readonly string[];
  readonly linkedDocuments: readonly DesktopAssetLinkedDocument[];
}
export interface DesktopAssetLinkedDocument {
  readonly documentId: string;
  readonly title?: string;
  readonly state: "available" | "missing";
}
export interface DesktopAssetPreview {
  readonly assetId: string;
  readonly capability: "image" | "pdf" | "text" | "unsupported";
  readonly mediaType: string;
  readonly presentation: "text" | "data_url" | "unsupported";
  readonly content?: string;
}
export interface DesktopAssetExternalOpenResult {
  readonly opened: true;
}
export interface DesktopAssetUnlinkResult {
  readonly outcome: "fresh" | "replayed" | "no_change";
  readonly delta: "linked" | "relabeled" | "unlinked" | "unchanged";
  readonly revisionNumber: number;
}
export type DesktopAssetLinkResult = DesktopAssetUnlinkResult;

export class DesktopAssetImportTransportError extends Error {
  readonly code: string;
  readonly retryable: boolean;
  readonly repairRequired: boolean;

  constructor(code: string, retryable: boolean, repairRequired = false) {
    super(code);
    this.name = "DesktopAssetImportTransportError";
    this.code = code;
    this.retryable = retryable;
    this.repairRequired = repairRequired;
  }
}

export function createTauriAssetImportTransport(invoke: TauriInvoke): DesktopAssetImportPickerClient {
  return Object.freeze({
    async selectFiles() {
      let response: unknown;
      try {
        response = await invoke("select_desktop_asset_import_files");
      } catch {
        throw new DesktopAssetImportTransportError("COMMAND_BRIDGE_FAILED", false);
      }
      if (isSuccess(response)) return Object.freeze({ cancelled: response.data.cancelled, files: Object.freeze(response.data.files.map((file) => Object.freeze(file))) });
      throw responseError(response);
    },
    async importFile(request) {
      let response: unknown;
      try {
        response = await invoke("import_desktop_asset", { request });
      } catch {
        throw new DesktopAssetImportTransportError("COMMAND_BRIDGE_FAILED", false);
      }
      if (isImportSuccess(response)) {
        return Object.freeze({
          operationId: response.operationId,
          ...(response.assetId ? { assetId: response.assetId } : {}),
          state: response.state,
          ...(response.errorCode ? { errorCode: response.errorCode } : {}),
          retryable: response.retryable === true,
          repairRequired: response.repairRequired === true,
        });
      }
      throw responseError(response);
    },
    async getImportStatus(request) {
      const response = await safeInvoke(invoke, "get_desktop_asset_import_status", { request });
      if (isImportStatusSuccess(response)) return freezeImportStatus(response);
      throw responseError(response);
    },
    async cancelImport(request) {
      const response = await safeInvoke(invoke, "cancel_desktop_asset_import", { request });
      if (isImportStatusSuccess(response)) return freezeImportStatus(response);
      throw responseError(response);
    },
    async getDetail(request) {
      const response = await safeInvoke(invoke, "get_desktop_asset_detail", { request });
      if (isDetailSuccess(response)) return Object.freeze({
        ...response.data,
        linkedDocumentIds: Object.freeze([...response.data.linkedDocumentIds]),
        linkedDocuments: Object.freeze(response.data.linkedDocuments.map((document) => Object.freeze({ ...document }))),
      });
      throw responseError(response);
    },
    async getPreview(request) {
      const response = await safeInvoke(invoke, "get_desktop_asset_preview", { request });
      if (isPreviewSuccess(response)) return Object.freeze({ ...response.data });
      throw responseError(response);
    },
    async openExternal(request) {
      const response = await safeInvoke(invoke, "open_desktop_asset_externally", { request });
      if (isExternalOpenSuccess(response)) return Object.freeze({ opened: true });
      throw responseError(response);
    },
    async listWorkspaceAssets(request) {
      const response = await safeInvoke(invoke, "get_desktop_workspace_assets", { request });
      if (isWorkspacePageSuccess(response)) {
        return Object.freeze({
          workspaceId: response.data.workspaceId,
          assets: Object.freeze(response.data.assets.map((asset) => Object.freeze({ ...asset }))),
          ...(response.data.nextCursor ? { nextCursor: response.data.nextCursor } : {}),
        });
      }
      throw responseError(response);
    },
    async link(request) {
      const response = await safeInvoke(invoke, "link_desktop_asset", { request: { kind: "link", ...request } });
      if (isAttachmentMutationSuccess(response)) return freezeAttachmentMutation(response);
      throw responseError(response);
    },
    async unlink(request) {
      const response = await safeInvoke(invoke, "unlink_desktop_asset", { request: { kind: "unlink", ...request } });
      if (isAttachmentMutationSuccess(response)) return freezeAttachmentMutation(response);
      throw responseError(response);
    },
  });
}

function isAttachmentMutationSuccess(value: unknown): value is {
  readonly ok: true;
  readonly outcome: "fresh" | "replayed" | "no_change";
  readonly delta: "linked" | "relabeled" | "unlinked" | "unchanged";
  readonly revisionNumber: number;
} {
  return isRecord(value) && value.ok === true
    && ["fresh", "replayed", "no_change"].includes(String(value.outcome))
    && ["linked", "relabeled", "unlinked", "unchanged"].includes(String(value.delta))
    && Number.isSafeInteger(value.revisionNumber) && Number(value.revisionNumber) > 0
    && !("versionToken" in value) && !("snapshotRef" in value) && !("path" in value);
}

function freezeAttachmentMutation(value: {
  readonly outcome: "fresh" | "replayed" | "no_change";
  readonly delta: "linked" | "relabeled" | "unlinked" | "unchanged";
  readonly revisionNumber: number;
}): DesktopAssetLinkResult {
  return Object.freeze({ outcome: value.outcome, delta: value.delta, revisionNumber: value.revisionNumber });
}

async function safeInvoke(invoke: TauriInvoke, command: string, payload: unknown): Promise<unknown> {
  try { return await invoke(command, payload); }
  catch { throw new DesktopAssetImportTransportError("COMMAND_BRIDGE_FAILED", false); }
}

function responseError(response: unknown): DesktopAssetImportTransportError {
  return isRecord(response) && response.ok === false && typeof response.errorCode === "string" && typeof response.retryable === "boolean"
    ? new DesktopAssetImportTransportError(response.errorCode, response.retryable, response.repairRequired === true)
    : new DesktopAssetImportTransportError("COMMAND_BRIDGE_FAILED", false);
}

function isSuccess(value: unknown): value is { readonly ok: true; readonly data: DesktopAssetImportSelection } {
  return isRecord(value) && value.ok === true && isRecord(value.data)
    && typeof value.data.cancelled === "boolean" && Array.isArray(value.data.files)
    && value.data.files.every(isDescriptor);
}

function isDescriptor(value: unknown): value is DesktopAssetImportDescriptor {
  return isRecord(value) && typeof value.handle === "string" && value.handle.length > 0
    && typeof value.fileName === "string" && typeof value.mediaType === "string"
    && typeof value.byteSize === "number" && value.byteSize > 0 && !("path" in value);
}

function isImportSuccess(value: unknown): value is {
  readonly ok: true;
  readonly operationId: string;
  readonly assetId?: string;
  readonly state: DesktopAssetImportState;
  readonly errorCode?: string;
  readonly retryable?: boolean;
  readonly repairRequired?: boolean;
} {
  return isRecord(value) && value.ok === true
    && typeof value.operationId === "string" && value.operationId.length > 0
    && (value.assetId === undefined || (typeof value.assetId === "string" && /^[a-f0-9]{64}$/.test(value.assetId)))
    && importStates.includes(value.state as DesktopAssetImportState)
    && (value.errorCode === undefined || typeof value.errorCode === "string")
    && (value.retryable === undefined || typeof value.retryable === "boolean")
    && (value.repairRequired === undefined || typeof value.repairRequired === "boolean")
    && (value.state !== "recovery_required" || value.repairRequired === true)
    && !("path" in value) && !("bytes" in value);
}

function isImportStatusSuccess(value: unknown): value is {
  readonly ok: true;
  readonly operationId: string;
  readonly assetId?: string;
  readonly state: DesktopAssetImportStatus["state"];
  readonly errorCode?: string;
  readonly retryable?: boolean;
  readonly repairRequired?: boolean;
} {
  return isRecord(value) && value.ok === true && typeof value.operationId === "string"
    && importStates.includes(value.state as DesktopAssetImportStatus["state"])
    && (value.assetId === undefined || (typeof value.assetId === "string" && /^[a-f0-9]{64}$/.test(value.assetId)))
    && (value.errorCode === undefined || typeof value.errorCode === "string")
    && (value.retryable === undefined || typeof value.retryable === "boolean")
    && (value.repairRequired === undefined || typeof value.repairRequired === "boolean")
    && (value.state !== "recovery_required" || value.repairRequired !== false)
    && !("path" in value) && !("bytes" in value);
}

function freezeImportStatus(value: {
  readonly operationId: string;
  readonly assetId?: string;
  readonly state: DesktopAssetImportState;
  readonly errorCode?: string;
  readonly retryable?: boolean;
  readonly repairRequired?: boolean;
}): DesktopAssetImportStatus {
  return Object.freeze({
    operationId: value.operationId,
    ...(value.assetId ? { assetId: value.assetId } : {}),
    state: value.state,
    ...(value.errorCode ? { errorCode: value.errorCode } : {}),
    retryable: value.retryable === true,
    repairRequired: value.repairRequired === true,
  });
}

function isDetailSuccess(value: unknown): value is { readonly ok: true; readonly data: DesktopAssetDetail } {
  if (!isRecord(value) || value.ok !== true || !isRecord(value.data)
    || "path" in value.data || "bytes" in value.data || "body" in value.data
    || "versionToken" in value.data || "snapshotRef" in value.data) return false;
  const data = value.data;
  return typeof data.assetId === "string" && /^[a-f0-9]{64}$/.test(data.assetId)
    && typeof data.fileName === "string" && typeof data.mediaType === "string"
    && isNonNegativeInteger(data.byteSize) && isNonNegativeInteger(data.version) && data.version > 0
    && ["image", "pdf", "text", "unsupported"].includes(String(data.previewCapability))
    && ["not_requested", "pending", "ready", "unsupported", "failed"].includes(String(data.extractionStatus))
    && isNonNegativeInteger(data.referenceCount)
    && Array.isArray(data.linkedDocumentIds) && data.linkedDocumentIds.length <= 200
    && data.linkedDocumentIds.every(isSafeDocumentIdentity)
    && Array.isArray(data.linkedDocuments)
    && data.linkedDocuments.length === data.linkedDocumentIds.length
    && data.linkedDocuments.every((document, index) => isLinkedDocument(document)
      && document.documentId === data.linkedDocumentIds[index]);
}

function isLinkedDocument(value: unknown): value is DesktopAssetLinkedDocument {
  if (!isRecord(value) || !isSafeDocumentIdentity(value.documentId)
    || !["available", "missing"].includes(String(value.state))
    || "path" in value || "body" in value || "versionToken" in value || "snapshotRef" in value) return false;
  if (value.state === "missing") return value.title === undefined;
  return typeof value.title === "string"
    && value.title.trim() === value.title
    && value.title.length > 0
    && Array.from(value.title).length <= 120
    && !Array.from(value.title).some((character) => /[\u0000-\u001f\u007f]/.test(character));
}

function isSafeDocumentIdentity(value: unknown): value is string {
  return typeof value === "string" && value.length > 0 && value.length <= 256 && !/[\u0000-\u001f\u007f]/.test(value);
}

function isPreviewSuccess(value: unknown): value is { readonly ok: true; readonly data: DesktopAssetPreview } {
  if (!isRecord(value) || value.ok !== true || !isRecord(value.data) || "path" in value.data || "bytes" in value.data) return false;
  const data = value.data;
  if (typeof data.assetId !== "string" || !/^[a-f0-9]{64}$/.test(data.assetId)
    || typeof data.mediaType !== "string"
    || !["image", "pdf", "text", "unsupported"].includes(String(data.capability))
    || !["text", "data_url", "unsupported"].includes(String(data.presentation))) return false;
  if (data.presentation === "unsupported") return data.content == null;
  if (typeof data.content !== "string" || data.content.length > 3_000_000) return false;
  return data.presentation === "text" || data.content.startsWith(`data:${data.mediaType};base64,`);
}

function isExternalOpenSuccess(value: unknown): value is { readonly ok: true; readonly opened: true } {
  return isRecord(value)
    && value.ok === true
    && value.opened === true
    && !("path" in value)
    && !("objectKey" in value)
    && !("bytes" in value);
}

function isWorkspacePageSuccess(value: unknown): value is {
  readonly ok: true;
  readonly data: { readonly workspaceId: string; readonly assets: DesktopWorkspaceAsset[]; readonly nextCursor?: string };
} {
  if (!isRecord(value) || value.ok !== true || !isRecord(value.data)) return false;
  const data = value.data;
  return typeof data.workspaceId === "string"
    && Array.isArray(data.assets) && data.assets.every(isWorkspaceAsset)
    && (data.nextCursor == null || (typeof data.nextCursor === "string" && /^[a-f0-9]{64}$/.test(data.nextCursor)));
}

function isWorkspaceAsset(value: unknown): value is DesktopWorkspaceAsset {
  return isRecord(value) && typeof value.assetId === "string" && /^[a-f0-9]{64}$/.test(value.assetId)
    && typeof value.label === "string" && typeof value.fileName === "string"
    && typeof value.mediaType === "string" && isNonNegativeInteger(value.byteSize)
    && typeof value.status === "string" && !("path" in value) && !("bytes" in value);
}

function isNonNegativeInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
