import type {
  DesktopAssetImportDescriptor,
  DesktopAssetImportSelection,
} from "./tauri_asset_import_transport.ts";

export const ASSET_DRAG_STATE_EVENT = "cabinet-asset-drag-state";
export const ASSET_DROP_SELECTION_EVENT = "cabinet-asset-drop-selection";

export interface DesktopAssetDragState {
  readonly state: "entered" | "left" | "dropped";
  readonly fileCount: number;
}

export type TauriEventListen = (
  eventName: string,
  handler: (event: { readonly payload: unknown }) => void,
) => Promise<() => void>;

export interface DesktopAssetDropCallbacks {
  readonly onState: (state: DesktopAssetDragState) => void;
  readonly onSelection: (selection: DesktopAssetImportSelection) => void;
  readonly onError: (errorCode: string) => void;
}

export async function subscribeTauriAssetDrop(
  listen: TauriEventListen,
  callbacks: DesktopAssetDropCallbacks,
): Promise<() => void> {
  let unlistenState: (() => void) | undefined;
  let unlistenSelection: (() => void) | undefined;
  try {
    unlistenState = await listen(ASSET_DRAG_STATE_EVENT, ({ payload }) => {
      const state = parseDragState(payload);
      if (!state) return callbacks.onError("ASSET_DRAG_STATE_INVALID");
      callbacks.onState(state);
    });
    unlistenSelection = await listen(ASSET_DROP_SELECTION_EVENT, ({ payload }) => {
      const selection = parseSelection(payload);
      if (selection) return callbacks.onSelection(selection);
      callbacks.onError(selectionFailureCode(payload));
    });
  } catch {
    unlistenState?.();
    unlistenSelection?.();
    callbacks.onError("ASSET_DROP_EVENT_BRIDGE_FAILED");
    return () => {};
  }
  return () => {
    unlistenState?.();
    unlistenSelection?.();
  };
}

export function getGlobalTauriEventListen(): TauriEventListen | undefined {
  const tauri = (globalThis as unknown as {
    readonly __TAURI__?: { readonly event?: { readonly listen?: TauriEventListen } };
  }).__TAURI__;
  return typeof tauri?.event?.listen === "function" ? tauri.event.listen.bind(tauri.event) : undefined;
}

function parseDragState(value: unknown): DesktopAssetDragState | undefined {
  if (!isRecord(value) || "path" in value || "paths" in value || "fileName" in value) return undefined;
  if (!(["entered", "left", "dropped"] as const).includes(value.state as DesktopAssetDragState["state"])) return undefined;
  if (!Number.isSafeInteger(value.fileCount) || Number(value.fileCount) < 0 || Number(value.fileCount) > 1000) return undefined;
  return Object.freeze({ state: value.state as DesktopAssetDragState["state"], fileCount: Number(value.fileCount) });
}

function parseSelection(value: unknown): DesktopAssetImportSelection | undefined {
  if (!isRecord(value) || value.ok !== true || !isRecord(value.data)) return undefined;
  if (typeof value.data.cancelled !== "boolean" || !Array.isArray(value.data.files)) return undefined;
  const files: DesktopAssetImportDescriptor[] = [];
  for (const candidate of value.data.files) {
    if (!isDescriptor(candidate)) return undefined;
    files.push(Object.freeze({
      handle: candidate.handle,
      fileName: candidate.fileName,
      mediaType: candidate.mediaType,
      byteSize: candidate.byteSize,
    }));
  }
  return Object.freeze({ cancelled: value.data.cancelled, files: Object.freeze(files) });
}

function isDescriptor(value: unknown): value is DesktopAssetImportDescriptor {
  return isRecord(value)
    && typeof value.handle === "string" && value.handle.length > 0
    && typeof value.fileName === "string" && value.fileName.length > 0
    && typeof value.mediaType === "string" && value.mediaType.length > 0
    && Number.isSafeInteger(value.byteSize) && Number(value.byteSize) > 0
    && !("path" in value) && !("paths" in value) && !("bytes" in value);
}

function selectionFailureCode(value: unknown): string {
  return isRecord(value) && value.ok === false && typeof value.errorCode === "string"
    ? value.errorCode
    : "ASSET_DROP_EVENT_INVALID";
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
