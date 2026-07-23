import type { TauriInvoke } from "./tauri_home_transport.ts";
export interface DesktopGlobalGraphQuery { readonly workspaceId: string; readonly cursor?: string; readonly includeUnresolved: boolean; readonly includeAssets: boolean; readonly projectionLimit: number; readonly nodeLimit: number; readonly edgeLimit: number }
export interface DesktopGlobalGraphView { readonly status: "clean" | "degraded"; readonly nodes: readonly { readonly id: string; readonly kind: "document" | "unresolved_link" | "attachment" | "external_link"; readonly label: string; readonly breadcrumbLabel: string; readonly availability: "available" | "missing"; readonly canNavigate: boolean }[]; readonly edges: readonly { readonly id: string; readonly sourceId: string; readonly targetId: string; readonly kind: "document_link" | "attachment_reference" | "external_reference" | "canvas_relation" }[]; readonly candidateCount: number; readonly nextCursor?: string }
export interface DesktopGlobalGraphClient { getGlobalGraph(query: DesktopGlobalGraphQuery): Promise<DesktopGlobalGraphView> }
export class DesktopGlobalGraphTransportError extends Error {
  readonly code: string;
  readonly retryable: boolean;

  constructor(code: string, retryable: boolean) {
    super(code);
    this.name = "DesktopGlobalGraphTransportError";
    this.code = code;
    this.retryable = retryable;
  }
}
export function createTauriGlobalGraphTransport(invoke: TauriInvoke): DesktopGlobalGraphClient { return Object.freeze({ async getGlobalGraph(query) { let response: unknown; const request = { workspaceId: query.workspaceId, ...(query.cursor ? { cursor: query.cursor } : {}), includeUnresolved: query.includeUnresolved, includeAssets: query.includeAssets, projectionLimit: query.projectionLimit, nodeLimit: query.nodeLimit, edgeLimit: query.edgeLimit }; try { response = await invoke("get_desktop_global_knowledge_graph", { request }); } catch { throw new DesktopGlobalGraphTransportError("COMMAND_BRIDGE_FAILED", false); } if (!isResponse(response)) { if (isRecord(response) && response.ok === false && typeof response.errorCode === "string" && typeof response.retryable === "boolean") throw new DesktopGlobalGraphTransportError(response.errorCode, response.retryable); throw new DesktopGlobalGraphTransportError("COMMAND_BRIDGE_FAILED", false); } return Object.freeze({ ...response.data, nextCursor: typeof response.data.nextCursor === "string" ? response.data.nextCursor : undefined }); } }); }
function isResponse(value: unknown): value is { ok: true; data: DesktopGlobalGraphView & { nextCursor?: string | null } } { return isRecord(value) && value.ok === true && isRecord(value.data) && (value.data.status === "clean" || value.data.status === "degraded") && Array.isArray(value.data.nodes) && value.data.nodes.every(isGraphNode) && Array.isArray(value.data.edges) && typeof value.data.candidateCount === "number"; }
function isGraphNode(value: unknown): boolean { return isRecord(value) && typeof value.id === "string" && typeof value.kind === "string" && typeof value.label === "string" && typeof value.breadcrumbLabel === "string" && (value.availability === "available" || value.availability === "missing") && typeof value.canNavigate === "boolean"; }
function isRecord(value: unknown): value is Record<string, unknown> { return typeof value === "object" && value !== null && !Array.isArray(value); }
