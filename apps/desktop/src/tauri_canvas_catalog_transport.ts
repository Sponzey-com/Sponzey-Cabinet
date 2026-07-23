import type { TauriInvoke } from "./tauri_home_transport.ts";

export type DesktopCanvasCatalogLifecycle = "draft" | "saved" | "embedded" | "updated" | "archived";
export type DesktopCanvasSelectionSource = "last_used" | "fallback" | "empty";

export interface DesktopCanvasCatalogQuery {
  readonly workspaceId: string;
  readonly limit: number;
  readonly includeArchived: boolean;
}

export interface DesktopCanvasCatalogEntryView {
  readonly canvasId: string;
  readonly title: string;
  readonly lifecycle: DesktopCanvasCatalogLifecycle;
  readonly revision: number;
}

export interface DesktopCanvasCatalogView {
  readonly entries: readonly DesktopCanvasCatalogEntryView[];
  readonly selectedCanvasId?: string;
  readonly selectionSource: DesktopCanvasSelectionSource;
}

export interface DesktopCanvasCatalogClient {
  getCatalog(query: DesktopCanvasCatalogQuery): Promise<DesktopCanvasCatalogView>;
  selectCanvas(workspaceId: string, canvasId: string): Promise<string>;
}

export class DesktopCanvasCatalogTransportError extends Error {
  readonly code: string;
  readonly retryable: boolean;

  constructor(code: string, retryable: boolean) {
    super(code);
    this.name = "DesktopCanvasCatalogTransportError";
    this.code = code;
    this.retryable = retryable;
  }
}

export function createTauriCanvasCatalogTransport(invoke: TauriInvoke): DesktopCanvasCatalogClient {
  return Object.freeze({
    async getCatalog(query: DesktopCanvasCatalogQuery): Promise<DesktopCanvasCatalogView> {
      if (!isNonEmptyString(query.workspaceId) || !isPositiveInteger(query.limit)) {
        throw bridgeFailure();
      }
      const response = await invokeSafely(invoke, "get_desktop_canvas_catalog", {
        request: {
          workspaceId: query.workspaceId,
          limit: query.limit,
          includeArchived: query.includeArchived,
        },
      });
      if (isFailure(response)) throw nativeFailure(response);
      if (!isCatalogSuccess(response)) throw bridgeFailure();
      return freezeCatalog(response.data);
    },

    async selectCanvas(workspaceId: string, canvasId: string): Promise<string> {
      if (!isNonEmptyString(workspaceId) || !isNonEmptyString(canvasId)) throw bridgeFailure();
      const response = await invokeSafely(invoke, "select_desktop_canvas", {
        request: { workspaceId, canvasId },
      });
      if (isFailure(response)) throw nativeFailure(response);
      if (!isSelectionSuccess(response) || response.selectedCanvasId !== canvasId) {
        throw bridgeFailure();
      }
      return response.selectedCanvasId;
    },
  });
}

async function invokeSafely(
  invoke: TauriInvoke,
  command: string,
  args: Record<string, unknown>,
): Promise<unknown> {
  try {
    return await invoke(command, args);
  } catch {
    throw bridgeFailure();
  }
}

function freezeCatalog(data: CatalogData): DesktopCanvasCatalogView {
  return Object.freeze({
    entries: Object.freeze(data.entries.map((entry) => Object.freeze({ ...entry }))),
    selectedCanvasId: data.selectedCanvasId ?? undefined,
    selectionSource: data.selectionSource,
  });
}

type CatalogData = {
  readonly entries: readonly DesktopCanvasCatalogEntryView[];
  readonly selectedCanvasId: string | null;
  readonly selectionSource: DesktopCanvasSelectionSource;
};

function isCatalogSuccess(value: unknown): value is {
  readonly ok: true;
  readonly data: CatalogData;
  readonly selectedCanvasId: string | null;
  readonly retryable: false;
} {
  if (!isRecord(value) || hasProhibitedKeyDeep(value) || value.ok !== true || value.retryable !== false) {
    return false;
  }
  if (!isCatalogData(value.data)) return false;
  return value.selectedCanvasId === value.data.selectedCanvasId;
}

function isCatalogData(value: unknown): value is CatalogData {
  if (!isRecord(value) || hasProhibitedKeyDeep(value) || !Array.isArray(value.entries)) return false;
  if (!value.entries.every(isCatalogEntry)) return false;
  if (!isSelectionSource(value.selectionSource)) return false;
  const selected = value.selectedCanvasId;
  if (selected !== null && !isNonEmptyString(selected)) return false;
  if (value.selectionSource === "empty") return selected === null;
  return typeof selected === "string" && value.entries.some(
    (entry) => entry.canvasId === selected && entry.lifecycle !== "archived",
  );
}

function isCatalogEntry(value: unknown): value is DesktopCanvasCatalogEntryView {
  return isRecord(value) && !hasProhibitedKeyDeep(value)
    && isNonEmptyString(value.canvasId)
    && isNonEmptyString(value.title)
    && isLifecycle(value.lifecycle)
    && isPositiveInteger(value.revision);
}

function isSelectionSuccess(value: unknown): value is {
  readonly ok: true;
  readonly data: null;
  readonly selectedCanvasId: string;
  readonly retryable: false;
} {
  return isRecord(value) && !hasProhibitedKeyDeep(value)
    && value.ok === true && value.data === null
    && isNonEmptyString(value.selectedCanvasId) && value.retryable === false;
}

function isFailure(value: unknown): value is {
  readonly ok: false;
  readonly errorCode: string;
  readonly retryable: boolean;
} {
  return isRecord(value) && !hasProhibitedKeyDeep(value)
    && value.ok === false && isNonEmptyString(value.errorCode)
    && typeof value.retryable === "boolean";
}

function nativeFailure(value: { readonly errorCode: string; readonly retryable: boolean }): DesktopCanvasCatalogTransportError {
  return new DesktopCanvasCatalogTransportError(value.errorCode, value.retryable);
}

function bridgeFailure(): DesktopCanvasCatalogTransportError {
  return new DesktopCanvasCatalogTransportError("COMMAND_BRIDGE_FAILED", false);
}

const PROHIBITED_KEYS = new Set([
  "path",
  "absolutePath",
  "snapshotPath",
  "checksum",
  "content",
  "bytes",
  "documentBody",
]);

function hasProhibitedKeyDeep(value: unknown): boolean {
  if (Array.isArray(value)) return value.some(hasProhibitedKeyDeep);
  if (!isRecord(value)) return false;
  return Object.entries(value).some(([key, child]) =>
    PROHIBITED_KEYS.has(key) || hasProhibitedKeyDeep(child),
  );
}

function isLifecycle(value: unknown): value is DesktopCanvasCatalogLifecycle {
  return ["draft", "saved", "embedded", "updated", "archived"].includes(String(value));
}

function isSelectionSource(value: unknown): value is DesktopCanvasSelectionSource {
  return ["last_used", "fallback", "empty"].includes(String(value));
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

function isPositiveInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value > 0;
}
