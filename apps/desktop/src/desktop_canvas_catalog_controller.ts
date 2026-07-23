import {
  DesktopCanvasCatalogTransportError,
  type DesktopCanvasCatalogClient,
  type DesktopCanvasCatalogEntryView,
  type DesktopCanvasCatalogView,
  type DesktopCanvasSelectionSource,
} from "./tauri_canvas_catalog_transport.ts";

export type DesktopCanvasCatalogState = "Idle" | "Loading" | "Ready" | "Empty" | "Selecting" | "Failed";

export interface DesktopCanvasCatalogSnapshot {
  readonly state: DesktopCanvasCatalogState;
  readonly workspaceId: string;
  readonly generation: number;
  readonly entries: readonly DesktopCanvasCatalogEntryView[];
  readonly selectedCanvasId?: string;
  readonly selectionSource?: DesktopCanvasSelectionSource;
  readonly pendingCanvasId?: string;
  readonly errorCode?: string;
  readonly retryable?: boolean;
}

export function resolveDesktopCanvasMenuTarget(input: {
  readonly catalogState: DesktopCanvasCatalogState;
  readonly selectedCanvasId?: string;
  readonly entries: readonly DesktopCanvasCatalogEntryView[];
  readonly displayedCanvasId?: string;
  readonly displayedLifecycle?: string;
}): string | undefined {
  if (input.catalogState === "Ready" && input.selectedCanvasId) return input.selectedCanvasId;
  if (input.displayedLifecycle !== "archived" || !input.displayedCanvasId) return undefined;
  return input.entries.some((entry) => entry.canvasId === input.displayedCanvasId)
    ? input.displayedCanvasId
    : undefined;
}

export function createDesktopCanvasCatalogSnapshot(workspaceId: string): DesktopCanvasCatalogSnapshot {
  const normalized = workspaceId.trim();
  if (!normalized) throw new Error("CANVAS_CATALOG_INVALID_WORKSPACE");
  return Object.freeze({
    state: "Idle",
    workspaceId: normalized,
    generation: 0,
    entries: Object.freeze([]),
  });
}

export function requestDesktopCanvasCatalogLoad(
  snapshot: DesktopCanvasCatalogSnapshot,
): DesktopCanvasCatalogSnapshot {
  return Object.freeze({
    ...snapshot,
    state: "Loading",
    generation: snapshot.generation + 1,
    pendingCanvasId: undefined,
    errorCode: undefined,
    retryable: undefined,
  });
}

export async function loadDesktopCanvasCatalog(
  client: DesktopCanvasCatalogClient,
  loading: DesktopCanvasCatalogSnapshot,
  limit: number,
): Promise<DesktopCanvasCatalogSnapshot> {
  if (loading.state !== "Loading") return loading;
  try {
    const result = await client.getCatalog({
      workspaceId: loading.workspaceId,
      limit,
      includeArchived: true,
    });
    return applyDesktopCanvasCatalogResult(loading, loading.generation, result);
  } catch (error) {
    return applyDesktopCanvasCatalogError(loading, loading.generation, error);
  }
}

export function applyDesktopCanvasCatalogResult(
  snapshot: DesktopCanvasCatalogSnapshot,
  generation: number,
  result: DesktopCanvasCatalogView,
): DesktopCanvasCatalogSnapshot {
  if (snapshot.generation !== generation || snapshot.state !== "Loading") return snapshot;
  const entries = Object.freeze(result.entries.map((entry) => Object.freeze({ ...entry })));
  return Object.freeze({
    state: result.selectionSource === "empty" ? "Empty" : "Ready",
    workspaceId: snapshot.workspaceId,
    generation,
    entries,
    selectedCanvasId: result.selectedCanvasId,
    selectionSource: result.selectionSource,
  });
}

export function requestDesktopCanvasSelection(
  snapshot: DesktopCanvasCatalogSnapshot,
  canvasId: string,
): DesktopCanvasCatalogSnapshot {
  const normalized = canvasId.trim();
  const entry = snapshot.entries.find((candidate) => candidate.canvasId === normalized);
  if (snapshot.state !== "Ready" || !entry || entry.lifecycle === "archived") return snapshot;
  if (snapshot.selectedCanvasId === normalized && snapshot.selectionSource === "last_used") return snapshot;
  return Object.freeze({
    ...snapshot,
    state: "Selecting",
    generation: snapshot.generation + 1,
    pendingCanvasId: normalized,
    errorCode: undefined,
    retryable: undefined,
  });
}

export async function selectDesktopCanvas(
  client: DesktopCanvasCatalogClient,
  selecting: DesktopCanvasCatalogSnapshot,
): Promise<DesktopCanvasCatalogSnapshot> {
  if (selecting.state !== "Selecting" || !selecting.pendingCanvasId) return selecting;
  const generation = selecting.generation;
  const requestedCanvasId = selecting.pendingCanvasId;
  try {
    const selectedCanvasId = await client.selectCanvas(selecting.workspaceId, requestedCanvasId);
    if (selectedCanvasId !== requestedCanvasId) return catalogBridgeFailure(selecting, generation);
    return Object.freeze({
      ...selecting,
      state: "Ready",
      selectedCanvasId,
      selectionSource: "last_used",
      pendingCanvasId: undefined,
      errorCode: undefined,
      retryable: undefined,
    });
  } catch (error) {
    return applyDesktopCanvasCatalogError(selecting, generation, error);
  }
}

function applyDesktopCanvasCatalogError(
  snapshot: DesktopCanvasCatalogSnapshot,
  generation: number,
  error: unknown,
): DesktopCanvasCatalogSnapshot {
  if (snapshot.generation !== generation || !["Loading", "Selecting"].includes(snapshot.state)) {
    return snapshot;
  }
  if (error instanceof DesktopCanvasCatalogTransportError) {
    return Object.freeze({
      ...snapshot,
      state: "Failed",
      pendingCanvasId: undefined,
      errorCode: error.code,
      retryable: error.retryable,
    });
  }
  return catalogBridgeFailure(snapshot, generation);
}

function catalogBridgeFailure(
  snapshot: DesktopCanvasCatalogSnapshot,
  generation: number,
): DesktopCanvasCatalogSnapshot {
  if (snapshot.generation !== generation) return snapshot;
  return Object.freeze({
    ...snapshot,
    state: "Failed",
    pendingCanvasId: undefined,
    errorCode: "COMMAND_BRIDGE_FAILED",
    retryable: false,
  });
}
