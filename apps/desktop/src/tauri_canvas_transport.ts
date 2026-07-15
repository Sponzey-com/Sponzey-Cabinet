import type { TauriInvoke } from "./tauri_home_transport.ts";

export type DesktopCanvasLifecycle = "draft" | "updated" | "archived";
export type DesktopCanvasTargetKind = "document" | "attachment" | "external" | "text";

export interface DesktopCanvasViewport {
  readonly centerX: number;
  readonly centerY: number;
  readonly zoomPercent: number;
}

export interface DesktopCanvasNode {
  readonly nodeId: string;
  readonly targetKind: DesktopCanvasTargetKind;
  readonly targetId: string;
  readonly displayLabel: string;
  readonly targetStatus: "available" | "missing";
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

export interface DesktopCanvasEdge {
  readonly edgeId: string;
  readonly sourceNodeId: string;
  readonly targetNodeId: string;
}

export interface DesktopCanvasData {
  readonly canvasId: string;
  readonly title: string;
  readonly revision: number;
  readonly lifecycle: DesktopCanvasLifecycle;
  readonly viewport: DesktopCanvasViewport;
  readonly nodes: readonly DesktopCanvasNode[];
  readonly edges: readonly DesktopCanvasEdge[];
  readonly totalNodeCount?: number;
  readonly totalEdgeCount?: number;
  readonly matchingNodeCount?: number;
  readonly matchingEdgeCount?: number;
  readonly truncated?: boolean;
  readonly operationId?: string;
}

interface CanvasIdentityRequest {
  readonly workspaceId: string;
  readonly canvasId: string;
}
interface CanvasRevisionRequest extends CanvasIdentityRequest {
  readonly expectedRevision: number;
  readonly operationId: string;
}
interface CanvasGeometryFields {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

export type DesktopCanvasRequest =
  | ({ readonly kind: "create"; readonly title: string } & CanvasIdentityRequest)
  | ({ readonly kind: "get" } & CanvasIdentityRequest)
  | ({ readonly kind: "recover"; readonly operationId: string } & CanvasIdentityRequest)
  | ({ readonly kind: "get_viewport"; readonly centerX?: number; readonly centerY?: number; readonly zoomPercent?: number; readonly surfaceWidth: number; readonly surfaceHeight: number; readonly overscan: number; readonly nodeLimit: number; readonly edgeLimit: number } & CanvasIdentityRequest)
  | ({ readonly kind: "preview_auto_arrange"; readonly expectedRevision: number } & CanvasIdentityRequest)
  | ({ readonly kind: "rename"; readonly title: string } & CanvasRevisionRequest)
  | ({ readonly kind: "archive" } & CanvasRevisionRequest)
  | ({ readonly kind: "add_document_node"; readonly nodeId: string; readonly documentId: string } & CanvasRevisionRequest & CanvasGeometryFields)
  | ({ readonly kind: "add_asset_node"; readonly nodeId: string; readonly assetId: string } & CanvasRevisionRequest & CanvasGeometryFields)
  | ({ readonly kind: "add_text_node"; readonly nodeId: string; readonly text: string } & CanvasRevisionRequest & CanvasGeometryFields)
  | ({ readonly kind: "connect_edge"; readonly edgeId: string; readonly sourceNodeId: string; readonly targetNodeId: string } & CanvasRevisionRequest)
  | ({ readonly kind: "remove_node"; readonly nodeId: string } & CanvasRevisionRequest)
  | ({ readonly kind: "remove_edge"; readonly edgeId: string } & CanvasRevisionRequest)
  | ({ readonly kind: "update_node_geometry"; readonly nodeId: string } & CanvasRevisionRequest & CanvasGeometryFields)
  | ({ readonly kind: "update_viewport"; readonly centerX: number; readonly centerY: number; readonly zoomPercent: number } & CanvasRevisionRequest)
  | ({ readonly kind: "auto_arrange" } & CanvasRevisionRequest);

export type DesktopCanvasMutationRequest = Exclude<DesktopCanvasRequest, { readonly kind: "create" | "get" | "recover" | "get_viewport" | "preview_auto_arrange" }>;
export type DesktopCanvasMutationDraft = DesktopCanvasMutationRequest extends infer Request
  ? Request extends DesktopCanvasMutationRequest
    ? Omit<Request, "workspaceId" | "canvasId" | "expectedRevision" | "operationId">
    : never
  : never;

export interface DesktopCanvasClient {
  execute(request: DesktopCanvasRequest): Promise<DesktopCanvasData>;
}

export class DesktopCanvasTransportError extends Error {
  readonly code: string;
  readonly retryable: boolean;
  readonly recoveryRequired: boolean;

  constructor(code: string, retryable: boolean, recoveryRequired: boolean) {
    super(code);
    this.name = "DesktopCanvasTransportError";
    this.code = code;
    this.retryable = retryable;
    this.recoveryRequired = recoveryRequired;
  }
}

export function createTauriCanvasTransport(invoke: TauriInvoke): DesktopCanvasClient {
  return Object.freeze({
    async execute(request: DesktopCanvasRequest): Promise<DesktopCanvasData> {
      let response: unknown;
      try {
        response = await invoke("execute_desktop_canvas", { request });
      } catch {
        throw bridgeFailure();
      }
      if (isSuccess(response)) {
        if (isOperationRequest(request)) {
          if (response.operationId !== request.operationId) throw bridgeFailure();
          return freezeCanvas({ ...response.data, operationId: response.operationId });
        }
        if (response.operationId !== undefined && response.operationId !== null) throw bridgeFailure();
        return freezeCanvas(response.data);
      }
      if (isFailure(response)) {
        throw new DesktopCanvasTransportError(
          response.errorCode,
          response.retryable,
          response.recoveryRequired,
        );
      }
      throw bridgeFailure();
    },
  });
}

function freezeCanvas(data: DesktopCanvasData): DesktopCanvasData {
  return Object.freeze({
    ...data,
    viewport: Object.freeze({ ...data.viewport }),
    nodes: Object.freeze(data.nodes.map((node) => Object.freeze({ ...node }))),
    edges: Object.freeze(data.edges.map((edge) => Object.freeze({ ...edge }))),
  });
}

function bridgeFailure(): DesktopCanvasTransportError {
  return new DesktopCanvasTransportError("COMMAND_BRIDGE_FAILED", false, false);
}

function isSuccess(value: unknown): value is { readonly ok: true; readonly data: DesktopCanvasData; readonly operationId?: unknown } {
  return isRecord(value) && value.ok === true && isCanvasData(value.data)
    && value.retryable === false && value.recoveryRequired === false;
}

function isMutationRequest(request: DesktopCanvasRequest): request is DesktopCanvasMutationRequest {
  return request.kind !== "create" && request.kind !== "get" && request.kind !== "recover" && request.kind !== "get_viewport"
    && request.kind !== "preview_auto_arrange";
}

function isOperationRequest(
  request: DesktopCanvasRequest,
): request is DesktopCanvasMutationRequest | Extract<DesktopCanvasRequest, { readonly kind: "recover" }> {
  return request.kind === "recover" || isMutationRequest(request);
}

function isFailure(value: unknown): value is {
  readonly ok: false;
  readonly errorCode: string;
  readonly retryable: boolean;
  readonly recoveryRequired: boolean;
} {
  return isRecord(value) && value.ok === false && typeof value.errorCode === "string"
    && typeof value.retryable === "boolean" && typeof value.recoveryRequired === "boolean";
}

function isCanvasData(value: unknown): value is DesktopCanvasData {
  return isRecord(value) && !hasProhibitedKey(value)
    && isNonEmptyString(value.canvasId) && isNonEmptyString(value.title)
    && isPositiveInteger(value.revision)
    && ["draft", "updated", "archived"].includes(String(value.lifecycle))
    && isViewport(value.viewport)
    && Array.isArray(value.nodes) && value.nodes.every(isNode)
    && Array.isArray(value.edges) && value.edges.every(isEdge)
    && optionalCount(value.totalNodeCount) && optionalCount(value.totalEdgeCount)
    && optionalCount(value.matchingNodeCount) && optionalCount(value.matchingEdgeCount)
    && (value.truncated === undefined || typeof value.truncated === "boolean");
}

function isViewport(value: unknown): value is DesktopCanvasViewport {
  return isRecord(value) && isInteger(value.centerX) && isInteger(value.centerY)
    && isPositiveInteger(value.zoomPercent);
}

function isNode(value: unknown): value is DesktopCanvasNode {
  return isRecord(value) && !hasProhibitedKey(value)
    && isNonEmptyString(value.nodeId) && isNonEmptyString(value.targetId)
    && isNonEmptyString(value.displayLabel)
    && ["available", "missing"].includes(String(value.targetStatus))
    && ["document", "attachment", "external", "text"].includes(String(value.targetKind))
    && isInteger(value.x) && isInteger(value.y)
    && isPositiveInteger(value.width) && isPositiveInteger(value.height);
}

function isEdge(value: unknown): value is DesktopCanvasEdge {
  return isRecord(value) && isNonEmptyString(value.edgeId)
    && isNonEmptyString(value.sourceNodeId) && isNonEmptyString(value.targetNodeId);
}

function hasProhibitedKey(value: Record<string, unknown>): boolean {
  return ["path", "bytes", "objectBytes", "documentBody"].some((key) => key in value);
}
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
function isNonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}
function isInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value);
}
function isPositiveInteger(value: unknown): value is number {
  return isInteger(value) && value > 0;
}
function optionalCount(value: unknown): boolean {
  return value === undefined || (isInteger(value) && value >= 0);
}
