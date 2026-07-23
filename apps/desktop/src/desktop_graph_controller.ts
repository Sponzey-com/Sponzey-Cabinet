import {
  LocalDesktopCommandClientError,
  createKnowledgeGraphQuery,
  type KnowledgeGraphView,
  type LocalDesktopCommandClient,
} from "@sponzey-cabinet/client-core";
import type {
  DesktopProjectionFreshnessView,
  DesktopProjectionTransport,
} from "./tauri_projection_transport.ts";
import type { DesktopGlobalGraphClient, DesktopGlobalGraphView } from "./tauri_global_graph_transport.ts";

const GLOBAL_GRAPH_PAGE_POLICY = Object.freeze({
  projectionLimit: 2,
  nodeLimit: 1_000,
  edgeLimit: 2_000,
  sessionNodeLimit: 10_000,
  sessionEdgeLimit: 20_000,
});

export const HOME_GRAPH_PROJECTION_LIMIT = 1_000;

export type DesktopGraphSurfaceState = "Idle" | "Loading" | "Ready" | "Empty" | "Stale" | "Repairing" | "Failed";

export interface DesktopGraphQueryState {
  readonly scope: "local" | "global";
  readonly globalCursor?: string;
  readonly centerDocumentId?: string;
  readonly depth: 1 | 2;
  readonly direction: "incoming" | "outgoing" | "both";
  readonly includeUnresolved: boolean;
  readonly includeAssets: boolean;
  readonly nodeLimit: number;
  readonly edgeLimit: number;
}

export interface DesktopGraphSurfaceSnapshot {
  readonly state: DesktopGraphSurfaceState;
  readonly workspaceId: string;
  readonly generation: number;
  readonly query: DesktopGraphQueryState;
  readonly graph?: DesktopGraphView;
  readonly selectedNodeId?: string;
  readonly errorCode?: string;
  readonly retryable?: boolean;
  readonly freshness?: DesktopProjectionFreshnessView;
}

export function createDesktopGraphSnapshot(workspaceId: string): DesktopGraphSurfaceSnapshot {
  return Object.freeze({
    state: "Idle",
    workspaceId,
    generation: 0,
    query: Object.freeze({
      depth: 1,
      scope: "local",
      direction: "both",
      includeUnresolved: true,
      includeAssets: false,
      nodeLimit: 120,
      edgeLimit: 240,
    }),
  });
}

export function requestDesktopGraphLoad(
  snapshot: DesktopGraphSurfaceSnapshot,
  patch: Partial<DesktopGraphQueryState>,
): DesktopGraphSurfaceSnapshot {
  const nextQuery = { ...snapshot.query, ...patch };
  const resetsGlobalPage = nextQuery.scope === "global" && (
    snapshot.query.scope !== "global"
    || patch.includeUnresolved !== undefined && patch.includeUnresolved !== snapshot.query.includeUnresolved
    || patch.includeAssets !== undefined && patch.includeAssets !== snapshot.query.includeAssets
  );
  const query = Object.freeze({
    ...nextQuery,
    ...(resetsGlobalPage ? { globalCursor: undefined } : {}),
  });
  if (query.scope !== "global" && !query.centerDocumentId?.trim()) {
    return Object.freeze({
      ...snapshot,
      state: "Empty",
      generation: snapshot.generation + 1,
      query,
      graph: undefined,
      selectedNodeId: undefined,
      errorCode: undefined,
      retryable: undefined,
    });
  }
  return Object.freeze({
    ...snapshot,
    state: "Loading",
    generation: snapshot.generation + 1,
    query,
    graph: resetsGlobalPage ? undefined : snapshot.graph,
    selectedNodeId: resetsGlobalPage ? undefined : snapshot.selectedNodeId,
    errorCode: undefined,
    retryable: undefined,
  });
}

export async function loadDesktopKnowledgeGraph(
  client: Pick<LocalDesktopCommandClient, "getKnowledgeGraph">,
  loading: DesktopGraphSurfaceSnapshot,
): Promise<DesktopGraphSurfaceSnapshot> {
  const centerDocumentId = loading.query.centerDocumentId;
  if (loading.state !== "Loading" || !centerDocumentId) return loading;
  try {
    const graph = await client.getKnowledgeGraph(createKnowledgeGraphQuery(
      loading.workspaceId,
      centerDocumentId,
      loading.query,
    ));
    return applyDesktopGraphResult(loading, loading.generation, graph);
  } catch (error) {
    const mapped = error instanceof LocalDesktopCommandClientError
      ? { code: error.code, retryable: error.retryable }
      : { code: "COMMAND_BRIDGE_FAILED", retryable: false };
    return applyDesktopGraphFailure(loading, loading.generation, mapped.code, mapped.retryable);
  }
}

export async function loadDesktopGlobalKnowledgeGraph(
  client: DesktopGlobalGraphClient,
  loading: DesktopGraphSurfaceSnapshot,
  projectionLimit: number = GLOBAL_GRAPH_PAGE_POLICY.projectionLimit,
): Promise<DesktopGraphSurfaceSnapshot> {
  if (loading.state !== "Loading" || loading.query.scope !== "global") return loading;
  try {
    const graph = await client.getGlobalGraph({
      workspaceId: loading.workspaceId,
      ...(loading.query.globalCursor ? { cursor: loading.query.globalCursor } : {}),
      includeUnresolved: loading.query.includeUnresolved,
      includeAssets: loading.query.includeAssets,
      projectionLimit,
      nodeLimit: GLOBAL_GRAPH_PAGE_POLICY.nodeLimit,
      edgeLimit: GLOBAL_GRAPH_PAGE_POLICY.edgeLimit,
    });
    const isContinuation = loading.query.globalCursor !== undefined
      && loading.query.globalCursor === loading.graph?.nextCursor;
    const merged = mergeGlobalGraphPage(isContinuation ? loading.graph : undefined, graph);
    if (typeof merged === "string") {
      return Object.freeze({
        ...loading,
        state: "Failed",
        errorCode: merged === "limit_exceeded"
          ? "GLOBAL_GRAPH_SESSION_LIMIT_EXCEEDED"
          : "GLOBAL_GRAPH_PAGE_CONFLICT",
        retryable: false,
      });
    }
    return applyDesktopGraphResult(loading, loading.generation, merged);
  } catch (error) {
    const mapped = safeProjectionError(error);
    return applyDesktopGraphFailure(loading, loading.generation, mapped.code, mapped.retryable);
  }
}

function mergeGlobalGraphPage(
  current: DesktopGraphView | undefined,
  page: DesktopGlobalGraphView,
): DesktopGraphView | "conflict" | "limit_exceeded" {
  const nodes = new Map(current?.nodes.map((node) => [node.id, node]) ?? []);
  for (const node of page.nodes) {
    const existing = nodes.get(node.id);
    if (existing && !sameFlatRecord(existing, node)) return "conflict";
    nodes.set(node.id, node);
  }
  const edges = new Map(current?.edges.map((edge) => [edge.id, edge]) ?? []);
  for (const edge of page.edges) {
    const existing = edges.get(edge.id);
    if (existing && !sameFlatRecord(existing, edge)) return "conflict";
    edges.set(edge.id, edge);
  }
  if (
    nodes.size > GLOBAL_GRAPH_PAGE_POLICY.sessionNodeLimit
    || edges.size > GLOBAL_GRAPH_PAGE_POLICY.sessionEdgeLimit
  ) {
    return "limit_exceeded";
  }
  const nodeIds = new Set(nodes.keys());
  if ([...edges.values()].some((edge) => !nodeIds.has(edge.sourceId) || !nodeIds.has(edge.targetId))) {
    return "conflict";
  }
  return Object.freeze({
    status: current?.status === "degraded" || page.status === "degraded" ? "degraded" : "clean",
    nodes: Object.freeze([...nodes.values()]),
    edges: Object.freeze([...edges.values()]),
    candidateCount: (current?.candidateCount ?? 0) + page.candidateCount,
    nextCursor: page.nextCursor,
  });
}

function sameFlatRecord(
  left: object,
  right: object,
): boolean {
  const leftRecord = left as Readonly<Record<string, unknown>>;
  const rightRecord = right as Readonly<Record<string, unknown>>;
  const leftKeys = Object.keys(left).sort();
  const rightKeys = Object.keys(right).sort();
  return leftKeys.length === rightKeys.length
    && leftKeys.every((key, index) => key === rightKeys[index] && leftRecord[key] === rightRecord[key]);
}

export async function loadDesktopKnowledgeGraphWithFreshness(
  graphClient: Pick<LocalDesktopCommandClient, "getKnowledgeGraph">,
  projectionClient: Pick<DesktopProjectionTransport, "getFreshness">,
  loading: DesktopGraphSurfaceSnapshot,
): Promise<DesktopGraphSurfaceSnapshot> {
  const graphResult = await loadDesktopKnowledgeGraph(graphClient, loading);
  if (graphResult.state === "Failed" || !loading.query.centerDocumentId) return graphResult;
  try {
    const freshness = await projectionClient.getFreshness(loading.workspaceId, loading.query.centerDocumentId);
    return applyDesktopProjectionFreshness(graphResult, loading.generation, freshness);
  } catch (error) {
    const mapped = safeProjectionError(error);
    return applyDesktopGraphFailure(graphResult, loading.generation, mapped.code, mapped.retryable);
  }
}

export function applyDesktopGraphResult(
  snapshot: DesktopGraphSurfaceSnapshot,
  generation: number,
  graph: DesktopGraphView,
): DesktopGraphSurfaceSnapshot {
  if (generation !== snapshot.generation) return snapshot;
  const state = graph.nodes.length === 0
    ? "Empty"
    : graph.status === "clean"
      ? "Ready"
      : "Stale";
  return Object.freeze({
    ...snapshot,
    state,
    graph,
    selectedNodeId: graph.nodes.some((node) => node.id === snapshot.selectedNodeId)
      ? snapshot.selectedNodeId
      : graph.centerDocumentId ?? graph.nodes[0]?.id,
    errorCode: undefined,
    retryable: undefined,
  });
}

export type DesktopGraphView = Pick<KnowledgeGraphView, "status" | "nodes" | "edges"> & {
  readonly centerDocumentId?: string;
  readonly stats?: KnowledgeGraphView["stats"];
  readonly freshnessRevision?: string;
  readonly candidateCount?: number;
  readonly nextCursor?: string;
};

export function applyDesktopGraphFailure(
  snapshot: DesktopGraphSurfaceSnapshot,
  generation: number,
  errorCode: string,
  retryable: boolean,
): DesktopGraphSurfaceSnapshot {
  if (generation !== snapshot.generation) return snapshot;
  return Object.freeze({
    ...snapshot,
    state: errorCode === "GRAPH_PROJECTION_NOT_FOUND" ? "Empty" : "Failed",
    graph: undefined,
    selectedNodeId: undefined,
    errorCode,
    retryable,
  });
}

export function applyDesktopProjectionFreshness(
  snapshot: DesktopGraphSurfaceSnapshot,
  generation: number,
  freshness: DesktopProjectionFreshnessView,
): DesktopGraphSurfaceSnapshot {
  if (generation !== snapshot.generation) return snapshot;
  const state = freshness.state === "ready"
    ? snapshot.graph?.nodes.length === 0 ? "Empty" : "Ready"
    : freshness.state === "stale" ? "Stale"
      : freshness.state === "repairing" ? "Repairing" : "Failed";
  return Object.freeze({
    ...snapshot,
    state,
    freshness,
    errorCode: state === "Failed" ? "PROJECTION_REPAIR_FAILED" : undefined,
    retryable: state === "Failed" ? true : undefined,
  });
}

export function requestDesktopGraphRepair(
  snapshot: DesktopGraphSurfaceSnapshot,
): DesktopGraphSurfaceSnapshot {
  if (snapshot.query.scope === "global" || !snapshot.query.centerDocumentId) return snapshot;
  return Object.freeze({
    ...snapshot,
    state: "Repairing",
    generation: snapshot.generation + 1,
    errorCode: undefined,
    retryable: undefined,
  });
}

export async function repairDesktopKnowledgeGraph(
  projectionClient: Pick<DesktopProjectionTransport, "startRepair" | "runRepair" | "getRepairStatus" | "getFreshness">,
  graphClient: Pick<LocalDesktopCommandClient, "getKnowledgeGraph">,
  repairing: DesktopGraphSurfaceSnapshot,
): Promise<DesktopGraphSurfaceSnapshot> {
  const documentId = repairing.query.centerDocumentId;
  if (repairing.state !== "Repairing" || !documentId) return repairing;
  try {
    const started = await projectionClient.startRepair(repairing.workspaceId, documentId);
    await projectionClient.runRepair(repairing.workspaceId, started.operationId);
    const status = await projectionClient.getRepairStatus(repairing.workspaceId, started.operationId);
    if (status.state !== "succeeded") {
      return status.state === "queued" || status.state === "running" || status.state === "publishing" || status.state === "cancel_pending"
        ? Object.freeze({ ...repairing, freshness: undefined })
        : applyDesktopGraphFailure(repairing, repairing.generation, `projection_repair.${status.state}`, status.state === "failed_retryable");
    }
    const freshness = await projectionClient.getFreshness(repairing.workspaceId, documentId);
    const graph = await graphClient.getKnowledgeGraph(createKnowledgeGraphQuery(
      repairing.workspaceId,
      documentId,
      repairing.query,
    ));
    return applyDesktopProjectionFreshness(
      applyDesktopGraphResult(repairing, repairing.generation, graph),
      repairing.generation,
      freshness,
    );
  } catch (error) {
    const mapped = safeProjectionError(error);
    return applyDesktopGraphFailure(repairing, repairing.generation, mapped.code, mapped.retryable);
  }
}

export function selectDesktopGraphNode(
  snapshot: DesktopGraphSurfaceSnapshot,
  nodeId: string,
): DesktopGraphSurfaceSnapshot {
  if (!snapshot.graph?.nodes.some((node) => node.id === nodeId)) return snapshot;
  return Object.freeze({ ...snapshot, selectedNodeId: nodeId });
}

function safeProjectionError(error: unknown): { readonly code: string; readonly retryable: boolean } {
  if (typeof error === "object" && error !== null
    && "code" in error && typeof error.code === "string"
    && "retryable" in error && typeof error.retryable === "boolean") {
    return { code: error.code, retryable: error.retryable };
  }
  return { code: "COMMAND_BRIDGE_FAILED", retryable: false };
}
