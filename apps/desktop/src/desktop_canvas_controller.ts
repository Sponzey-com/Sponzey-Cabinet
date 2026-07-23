import {
  DesktopCanvasTransportError,
  type DesktopCanvasClient,
  type DesktopCanvasData,
  type DesktopCanvasMutationDraft,
  type DesktopCanvasMutationRequest,
} from "./tauri_canvas_transport.ts";

export type DesktopCanvasSurfaceState =
  | "Idle"
  | "Loading"
  | "Creating"
  | "Recovering"
  | "Ready"
  | "PreviewingArrange"
  | "ArrangePreview"
  | "Mutating"
  | "Conflict"
  | "RecoveryRequired"
  | "Failed";

export interface DesktopCanvasSurfaceSnapshot {
  readonly state: DesktopCanvasSurfaceState;
  readonly workspaceId: string;
  readonly canvasId?: string;
  readonly generation: number;
  readonly selectedNodeIds: readonly string[];
  readonly selectedEdgeId?: string;
  readonly canvas?: DesktopCanvasData;
  readonly arrangeBaseCanvas?: DesktopCanvasData;
  readonly drag?: DesktopCanvasDragState;
  readonly resize?: DesktopCanvasResizeState;
  readonly pendingRequest?: DesktopCanvasMutationRequest;
  readonly pendingRecoveryOperationId?: string;
  readonly errorCode?: string;
  readonly retryable?: boolean;
}

export interface DesktopCanvasDragState {
  readonly nodeId: string;
  readonly pointerStartX: number;
  readonly pointerStartY: number;
  readonly nodeStartX: number;
  readonly nodeStartY: number;
}

export interface DesktopCanvasResizeState {
  readonly nodeId: string;
  readonly pointerStartX: number;
  readonly pointerStartY: number;
  readonly nodeStartWidth: number;
  readonly nodeStartHeight: number;
}

export const DESKTOP_CANVAS_GEOMETRY_LIMITS = Object.freeze({
  minWidth: 80,
  maxWidth: 1_200,
  minHeight: 60,
  maxHeight: 900,
});

export interface DesktopCanvasViewportPatch {
  readonly deltaX?: number;
  readonly deltaY?: number;
  readonly zoomPercent?: number;
}

export function createDesktopCanvasSnapshot(workspaceId: string): DesktopCanvasSurfaceSnapshot {
  return Object.freeze({ state: "Idle", workspaceId, generation: 0, selectedNodeIds: Object.freeze([]) });
}

export function requestDesktopCanvasLoad(
  snapshot: DesktopCanvasSurfaceSnapshot,
  canvasId: string,
): DesktopCanvasSurfaceSnapshot {
  const normalized = canvasId.trim();
  if (!normalized) return snapshot;
  return Object.freeze({
    ...snapshot,
    state: "Loading",
    canvasId: normalized,
    generation: snapshot.generation + 1,
    canvas: snapshot.canvasId === normalized ? snapshot.canvas : undefined,
    selectedNodeIds: snapshot.canvasId === normalized ? snapshot.selectedNodeIds : Object.freeze([]),
    selectedEdgeId: snapshot.canvasId === normalized ? snapshot.selectedEdgeId : undefined,
    drag: undefined,
    resize: undefined,
    arrangeBaseCanvas: undefined,
    pendingRequest: undefined,
    errorCode: undefined,
    retryable: undefined,
  });
}

export async function loadDesktopCanvas(
  client: DesktopCanvasClient,
  loading: DesktopCanvasSurfaceSnapshot,
): Promise<DesktopCanvasSurfaceSnapshot> {
  if (loading.state !== "Loading" || !loading.canvasId) return loading;
  try {
    const canvas = await client.execute({
      kind: "get_viewport",
      workspaceId: loading.workspaceId,
      canvasId: loading.canvasId,
      surfaceWidth: 1_200,
      surfaceHeight: 720,
      overscan: 120,
      nodeLimit: 250,
      edgeLimit: 500,
    });
    return applyDesktopCanvasResult(loading, loading.generation, canvas);
  } catch (error) {
    return applyDesktopCanvasError(loading, loading.generation, error);
  }
}

export async function createDesktopCanvas(
  client: DesktopCanvasClient,
  snapshot: DesktopCanvasSurfaceSnapshot,
  title: string,
): Promise<DesktopCanvasSurfaceSnapshot> {
  if (!snapshot.canvasId || !["Loading", "Failed"].includes(snapshot.state)) return snapshot;
  const creating = Object.freeze({ ...snapshot, state: "Creating" as const, errorCode: undefined, retryable: undefined });
  try {
    const canvas = await client.execute({
      kind: "create",
      workspaceId: creating.workspaceId,
      canvasId: creating.canvasId,
      title,
    });
    return applyDesktopCanvasResult(creating, creating.generation, canvas);
  } catch (error) {
    return applyDesktopCanvasError(creating, creating.generation, error);
  }
}

export function requestDesktopCanvasRecovery(
  snapshot: DesktopCanvasSurfaceSnapshot,
  operationId: string,
): DesktopCanvasSurfaceSnapshot {
  const operation = operationId.trim();
  if (snapshot.state !== "RecoveryRequired" || !snapshot.canvasId || !operation) return snapshot;
  return Object.freeze({
    ...snapshot,
    state: "Recovering",
    generation: snapshot.generation + 1,
    pendingRecoveryOperationId: operation,
    errorCode: undefined,
    retryable: undefined,
  });
}

export async function runDesktopCanvasRecovery(
  client: DesktopCanvasClient,
  recovering: DesktopCanvasSurfaceSnapshot,
): Promise<DesktopCanvasSurfaceSnapshot> {
  if (recovering.state !== "Recovering" || !recovering.canvasId || !recovering.pendingRecoveryOperationId) {
    return recovering;
  }
  try {
    const canvas = await client.execute({
      kind: "recover",
      workspaceId: recovering.workspaceId,
      canvasId: recovering.canvasId,
      operationId: recovering.pendingRecoveryOperationId,
    });
    if (canvas.operationId !== recovering.pendingRecoveryOperationId) return recovering;
    return applyDesktopCanvasResult(recovering, recovering.generation, canvas);
  } catch (error) {
    return applyDesktopCanvasError(recovering, recovering.generation, error);
  }
}

export function requestDesktopCanvasMutation(
  snapshot: DesktopCanvasSurfaceSnapshot,
  draft: DesktopCanvasMutationDraft,
  operationId: string,
): DesktopCanvasSurfaceSnapshot {
  if (snapshot.state !== "Ready" || !snapshot.canvasId || !snapshot.canvas
    || snapshot.canvas.lifecycle === "archived" || !operationId.trim()) return snapshot;
  const pendingRequest = {
    ...draft,
    workspaceId: snapshot.workspaceId,
    canvasId: snapshot.canvasId,
    expectedRevision: snapshot.canvas.revision,
    operationId,
  } as DesktopCanvasMutationRequest;
  return Object.freeze({
    ...snapshot,
    state: "Mutating",
    pendingRequest: Object.freeze(pendingRequest),
    errorCode: undefined,
    retryable: undefined,
  });
}

export function createDesktopCanvasViewportDraft(
  snapshot: DesktopCanvasSurfaceSnapshot,
  patch: DesktopCanvasViewportPatch,
): DesktopCanvasMutationDraft | undefined {
  if (snapshot.state !== "Ready" || !snapshot.canvas || snapshot.canvas.lifecycle === "archived") {
    return undefined;
  }
  const current = snapshot.canvas.viewport;
  const deltaX = Number.isFinite(patch.deltaX) ? Math.round(patch.deltaX ?? 0) : 0;
  const deltaY = Number.isFinite(patch.deltaY) ? Math.round(patch.deltaY ?? 0) : 0;
  const requestedZoom = Number.isFinite(patch.zoomPercent)
    ? Math.round(patch.zoomPercent ?? current.zoomPercent)
    : current.zoomPercent;
  const centerX = clampCoordinate(current.centerX + deltaX);
  const centerY = clampCoordinate(current.centerY + deltaY);
  const zoomPercent = clampSize(requestedZoom, 25, 400);
  if (centerX === current.centerX && centerY === current.centerY && zoomPercent === current.zoomPercent) {
    return undefined;
  }
  return Object.freeze({ kind: "update_viewport", centerX, centerY, zoomPercent });
}

export function requestDesktopCanvasArrangePreview(
  snapshot: DesktopCanvasSurfaceSnapshot,
): DesktopCanvasSurfaceSnapshot {
  if (snapshot.state !== "Ready" || !snapshot.canvasId || !snapshot.canvas
    || snapshot.canvas.lifecycle === "archived") return snapshot;
  return Object.freeze({
    ...snapshot,
    state: "PreviewingArrange",
    arrangeBaseCanvas: snapshot.canvas,
    errorCode: undefined,
    retryable: undefined,
  });
}

export async function runDesktopCanvasArrangePreview(
  client: DesktopCanvasClient,
  snapshot: DesktopCanvasSurfaceSnapshot,
): Promise<DesktopCanvasSurfaceSnapshot> {
  const base = snapshot.arrangeBaseCanvas;
  if (snapshot.state !== "PreviewingArrange" || !snapshot.canvasId || !base) return snapshot;
  try {
    const preview = await client.execute({
      kind: "preview_auto_arrange",
      workspaceId: snapshot.workspaceId,
      canvasId: snapshot.canvasId,
      expectedRevision: base.revision,
    });
    return applyDesktopCanvasArrangePreview(snapshot, preview);
  } catch (error) {
    return applyDesktopCanvasError(snapshot, snapshot.generation, error);
  }
}

export function applyDesktopCanvasArrangePreview(
  snapshot: DesktopCanvasSurfaceSnapshot,
  preview: DesktopCanvasData,
): DesktopCanvasSurfaceSnapshot {
  const base = snapshot.arrangeBaseCanvas;
  if (snapshot.state !== "PreviewingArrange" || !base || preview.canvasId !== base.canvasId
    || preview.revision !== base.revision) return snapshot;
  return Object.freeze({ ...snapshot, state: "ArrangePreview", canvas: preview });
}

export function cancelDesktopCanvasArrangePreview(
  snapshot: DesktopCanvasSurfaceSnapshot,
): DesktopCanvasSurfaceSnapshot {
  if (snapshot.state !== "ArrangePreview" || !snapshot.arrangeBaseCanvas) return snapshot;
  return Object.freeze({
    ...snapshot,
    state: "Ready",
    canvas: snapshot.arrangeBaseCanvas,
    arrangeBaseCanvas: undefined,
  });
}

export function requestDesktopCanvasArrangeApply(
  snapshot: DesktopCanvasSurfaceSnapshot,
  operationId: string,
): DesktopCanvasSurfaceSnapshot {
  if (snapshot.state !== "ArrangePreview" || !snapshot.arrangeBaseCanvas) return snapshot;
  return requestDesktopCanvasMutation(Object.freeze({
    ...snapshot,
    state: "Ready",
    canvas: snapshot.arrangeBaseCanvas,
    arrangeBaseCanvas: undefined,
  }), { kind: "auto_arrange" }, operationId);
}

export function selectDesktopCanvasNode(
  snapshot: DesktopCanvasSurfaceSnapshot,
  nodeId: string,
): DesktopCanvasSurfaceSnapshot {
  if (snapshot.state !== "Ready" || !snapshot.canvas?.nodes.some((node) => node.nodeId === nodeId)) {
    return snapshot;
  }
  const selected = snapshot.selectedNodeIds.includes(nodeId)
    ? snapshot.selectedNodeIds.filter((id) => id !== nodeId)
    : snapshot.selectedNodeIds.length < 2 ? [...snapshot.selectedNodeIds, nodeId] : snapshot.selectedNodeIds;
  if (selected === snapshot.selectedNodeIds) return snapshot;
  return Object.freeze({ ...snapshot, selectedNodeIds: Object.freeze(selected), selectedEdgeId: undefined });
}

export function selectDesktopCanvasEdge(
  snapshot: DesktopCanvasSurfaceSnapshot,
  edgeId: string,
): DesktopCanvasSurfaceSnapshot {
  if (snapshot.state !== "Ready" || !snapshot.canvas?.edges.some((edge) => edge.edgeId === edgeId)) {
    return snapshot;
  }
  return Object.freeze({
    ...snapshot,
    selectedNodeIds: Object.freeze([]),
    selectedEdgeId: snapshot.selectedEdgeId === edgeId ? undefined : edgeId,
  });
}

export function beginDesktopCanvasDrag(
  snapshot: DesktopCanvasSurfaceSnapshot,
  nodeId: string,
  pointerX: number,
  pointerY: number,
): DesktopCanvasSurfaceSnapshot {
  if (snapshot.state !== "Ready" || snapshot.canvas?.lifecycle === "archived") return snapshot;
  const node = snapshot.canvas?.nodes.find((candidate) => candidate.nodeId === nodeId);
  if (!node || !Number.isFinite(pointerX) || !Number.isFinite(pointerY)) return snapshot;
  return Object.freeze({
    ...snapshot,
    drag: Object.freeze({
      nodeId,
      pointerStartX: pointerX,
      pointerStartY: pointerY,
      nodeStartX: node.x,
      nodeStartY: node.y,
    }),
  });
}

export function finishDesktopCanvasDrag(
  snapshot: DesktopCanvasSurfaceSnapshot,
  nodeId: string,
  pointerX: number,
  pointerY: number,
): { readonly snapshot: DesktopCanvasSurfaceSnapshot; readonly draft?: DesktopCanvasMutationDraft } {
  const drag = snapshot.drag;
  const node = snapshot.canvas?.nodes.find((candidate) => candidate.nodeId === nodeId);
  const cleared = Object.freeze({ ...snapshot, drag: undefined });
  if (!drag || drag.nodeId !== nodeId || !node || !Number.isFinite(pointerX) || !Number.isFinite(pointerY)) {
    return Object.freeze({ snapshot: cleared });
  }
  const scale = Math.max(0.25, (snapshot.canvas?.viewport.zoomPercent ?? 100) / 100);
  const x = clampCoordinate(Math.round(drag.nodeStartX + (pointerX - drag.pointerStartX) / scale));
  const y = clampCoordinate(Math.round(drag.nodeStartY + (pointerY - drag.pointerStartY) / scale));
  if (x === node.x && y === node.y) return Object.freeze({ snapshot: cleared });
  return Object.freeze({
    snapshot: cleared,
    draft: Object.freeze({
      kind: "update_node_geometry" as const,
      nodeId,
      x,
      y,
      width: node.width,
      height: node.height,
    }),
  });
}

export function beginDesktopCanvasResize(
  snapshot: DesktopCanvasSurfaceSnapshot,
  nodeId: string,
  pointerX: number,
  pointerY: number,
): DesktopCanvasSurfaceSnapshot {
  if (snapshot.state !== "Ready" || snapshot.canvas?.lifecycle === "archived") return snapshot;
  const node = snapshot.canvas?.nodes.find((candidate) => candidate.nodeId === nodeId);
  if (!node || !Number.isFinite(pointerX) || !Number.isFinite(pointerY)) return snapshot;
  return Object.freeze({
    ...snapshot,
    resize: Object.freeze({
      nodeId,
      pointerStartX: pointerX,
      pointerStartY: pointerY,
      nodeStartWidth: node.width,
      nodeStartHeight: node.height,
    }),
  });
}

export function finishDesktopCanvasResize(
  snapshot: DesktopCanvasSurfaceSnapshot,
  nodeId: string,
  pointerX: number,
  pointerY: number,
): { readonly snapshot: DesktopCanvasSurfaceSnapshot; readonly draft?: DesktopCanvasMutationDraft } {
  const resize = snapshot.resize;
  const node = snapshot.canvas?.nodes.find((candidate) => candidate.nodeId === nodeId);
  const cleared = Object.freeze({ ...snapshot, resize: undefined });
  if (!resize || resize.nodeId !== nodeId || !node || !Number.isFinite(pointerX) || !Number.isFinite(pointerY)) {
    return Object.freeze({ snapshot: cleared });
  }
  const scale = Math.max(0.25, (snapshot.canvas?.viewport.zoomPercent ?? 100) / 100);
  const width = clampSize(
    Math.round(resize.nodeStartWidth + (pointerX - resize.pointerStartX) / scale),
    DESKTOP_CANVAS_GEOMETRY_LIMITS.minWidth,
    DESKTOP_CANVAS_GEOMETRY_LIMITS.maxWidth,
  );
  const height = clampSize(
    Math.round(resize.nodeStartHeight + (pointerY - resize.pointerStartY) / scale),
    DESKTOP_CANVAS_GEOMETRY_LIMITS.minHeight,
    DESKTOP_CANVAS_GEOMETRY_LIMITS.maxHeight,
  );
  if (width === node.width && height === node.height) return Object.freeze({ snapshot: cleared });
  return Object.freeze({
    snapshot: cleared,
    draft: Object.freeze({
      kind: "update_node_geometry" as const,
      nodeId,
      x: node.x,
      y: node.y,
      width,
      height,
    }),
  });
}

export async function runDesktopCanvasMutation(
  client: DesktopCanvasClient,
  mutating: DesktopCanvasSurfaceSnapshot,
): Promise<DesktopCanvasSurfaceSnapshot> {
  if (mutating.state !== "Mutating" || !mutating.pendingRequest) return mutating;
  try {
    const canvas = await client.execute(mutating.pendingRequest);
    if (canvas.operationId !== mutating.pendingRequest.operationId) return mutating;
    return applyDesktopCanvasResult(mutating, mutating.generation, canvas);
  } catch (error) {
    return applyDesktopCanvasError(mutating, mutating.generation, error);
  }
}

export function applyDesktopCanvasResult(
  snapshot: DesktopCanvasSurfaceSnapshot,
  generation: number,
  canvas: DesktopCanvasData,
): DesktopCanvasSurfaceSnapshot {
  if (generation !== snapshot.generation) return snapshot;
  const selectedNodeIds = snapshot.pendingRequest?.kind === "connect_edge"
    ? []
    : snapshot.selectedNodeIds.filter((nodeId) => canvas.nodes.some((node) => node.nodeId === nodeId));
  const selectedEdgeId = snapshot.pendingRequest?.kind === "remove_edge"
    ? undefined
    : canvas.edges.some((edge) => edge.edgeId === snapshot.selectedEdgeId)
      ? snapshot.selectedEdgeId
      : undefined;
  return Object.freeze({
    ...snapshot,
    state: "Ready",
    canvasId: canvas.canvasId,
    canvas,
    selectedNodeIds: Object.freeze(selectedNodeIds),
    selectedEdgeId,
    drag: undefined,
    resize: undefined,
    arrangeBaseCanvas: undefined,
    pendingRequest: undefined,
    pendingRecoveryOperationId: undefined,
    errorCode: undefined,
    retryable: undefined,
  });
}

function clampCoordinate(value: number): number {
  return Math.max(-10_000, Math.min(10_000, value));
}

function clampSize(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

export function applyDesktopCanvasError(
  snapshot: DesktopCanvasSurfaceSnapshot,
  generation: number,
  error: unknown,
): DesktopCanvasSurfaceSnapshot {
  if (generation !== snapshot.generation) return snapshot;
  const mapped = error instanceof DesktopCanvasTransportError
    ? error
    : new DesktopCanvasTransportError("COMMAND_BRIDGE_FAILED", false, false);
  const state: DesktopCanvasSurfaceState = mapped.recoveryRequired
    ? "RecoveryRequired"
    : mapped.code === "CANVAS_VERSION_CONFLICT" ? "Conflict" : "Failed";
  return Object.freeze({
    ...snapshot,
    state,
    canvas: state === "RecoveryRequired" ? undefined : snapshot.canvas,
    arrangeBaseCanvas: undefined,
    resize: undefined,
    pendingRequest: undefined,
    pendingRecoveryOperationId: undefined,
    errorCode: mapped.code,
    retryable: mapped.retryable,
  });
}
