import type {
  DocumentAssetsPage,
  KnowledgeGraphQuery,
  KnowledgeGraphView,
  ListDocumentAssetsQuery,
  LocalDesktopCommandEnvelope,
  LocalDesktopCommandResponse,
  LocalDesktopCommandTransport,
} from "@sponzey-cabinet/client-core";

import type { TauriInvoke } from "./tauri_home_transport.ts";

export function createTauriDiscoveryTransport(invoke: TauriInvoke): LocalDesktopCommandTransport {
  return async <TData>(
    envelope: LocalDesktopCommandEnvelope,
  ): Promise<LocalDesktopCommandResponse<TData>> => {
    if (envelope.commandName === "list_document_assets" && isDocumentAssetsQuery(envelope.payload)) {
      return invokeDocumentAssets(invoke, envelope) as Promise<LocalDesktopCommandResponse<TData>>;
    }
    if (envelope.commandName !== "get_graph_projection" || !isKnowledgeGraphQuery(envelope.payload)) return bridgeFailure();

    const query = envelope.payload;
    try {
      const response = await invoke("get_desktop_knowledge_graph", {
        request: {
          command_name: envelope.commandName,
          payload: {
            kind: "graph_projection",
            workspace_id: query.workspaceId,
            document_id: query.documentId,
            depth: query.depth,
            direction: query.direction,
            include_unresolved: query.includeUnresolved,
            include_assets: query.includeAssets,
            node_limit: query.nodeLimit,
            edge_limit: query.edgeLimit,
          },
        },
      });
      return isKnowledgeGraphCommandResponse(response)
        ? (response as LocalDesktopCommandResponse<TData>)
        : bridgeFailure();
    } catch {
      return bridgeFailure();
    }
  };
}

async function invokeDocumentAssets(
  invoke: TauriInvoke,
  envelope: LocalDesktopCommandEnvelope,
): Promise<LocalDesktopCommandResponse<DocumentAssetsPage>> {
  const query = envelope.payload;
  if (!isDocumentAssetsQuery(query)) return bridgeFailure();
  try {
    const response = await invoke("get_desktop_document_assets", {
      request: {
        command_name: envelope.commandName,
        payload: {
          kind: "document_identity",
          workspace_id: query.workspaceId,
          document_id: query.documentId,
        },
      },
    });
    return isDocumentAssetsResponse(response) ? response : bridgeFailure();
  } catch {
    return bridgeFailure();
  }
}

function isDocumentAssetsQuery(value: Record<string, unknown>): value is ListDocumentAssetsQuery & Record<string, unknown> {
  return value.queryName === "list-document-assets" && isNonEmptyString(value.workspaceId) && isNonEmptyString(value.documentId);
}

function isDocumentAssetsResponse(value: unknown): value is LocalDesktopCommandResponse<DocumentAssetsPage> {
  if (!isRecord(value) || typeof value.ok !== "boolean") return false;
  if (!value.ok) return typeof value.errorCode === "string" && typeof value.retryable === "boolean";
  const data = value.data;
  return isRecord(data)
    && data.queryName === "list-document-assets"
    && isNonEmptyString(data.workspaceId)
    && isNonEmptyString(data.documentId)
    && Array.isArray(data.assets)
    && data.assets.every((asset) => isRecord(asset)
      && isNonEmptyString(asset.assetId)
      && isNonEmptyString(asset.label)
      && isNonEmptyString(asset.fileName)
      && isNonEmptyString(asset.mediaType)
      && isNonNegativeInteger(asset.byteSize)
      && ["available", "missing", "metadata_only"].includes(String(asset.status)));
}

function isKnowledgeGraphQuery(
  value: Record<string, unknown>,
): value is KnowledgeGraphQuery & Record<string, unknown> {
  return (
    value.queryName === "get-knowledge-graph" &&
    isNonEmptyString(value.workspaceId) &&
    isNonEmptyString(value.documentId) &&
    (value.depth === 1 || value.depth === 2) &&
    ["incoming", "outgoing", "both"].includes(String(value.direction)) &&
    typeof value.includeUnresolved === "boolean" &&
    typeof value.includeAssets === "boolean" &&
    isBoundedInteger(value.nodeLimit, 1, 500) &&
    isBoundedInteger(value.edgeLimit, 1, 1_000)
  );
}

function isKnowledgeGraphCommandResponse(
  value: unknown,
): value is LocalDesktopCommandResponse<KnowledgeGraphView> {
  if (!isRecord(value) || typeof value.ok !== "boolean") return false;
  if (!value.ok) {
    return typeof value.errorCode === "string" && typeof value.retryable === "boolean";
  }
  return isKnowledgeGraphView(value.data);
}

function isKnowledgeGraphView(value: unknown): value is KnowledgeGraphView {
  return (
    isRecord(value) &&
    isNonEmptyString(value.centerDocumentId) &&
    ["clean", "reindex_requested", "reindexing", "degraded"].includes(String(value.status)) &&
    Array.isArray(value.nodes) &&
    value.nodes.every(isGraphNode) &&
    Array.isArray(value.edges) &&
    value.edges.every(isGraphEdge) &&
    isGraphStats(value.stats) &&
    isNonEmptyString(value.freshnessRevision)
  );
}

function isGraphNode(value: unknown): boolean {
  return (
    isRecord(value) &&
    isNonEmptyString(value.id) &&
    ["document", "unresolved_link", "attachment", "external_link"].includes(String(value.kind))
  );
}

function isGraphEdge(value: unknown): boolean {
  return (
    isRecord(value) &&
    isNonEmptyString(value.id) &&
    isNonEmptyString(value.sourceId) &&
    isNonEmptyString(value.targetId) &&
    ["document_link", "attachment_reference", "external_reference", "canvas_relation"].includes(String(value.kind))
  );
}

function isGraphStats(value: unknown): boolean {
  return (
    isRecord(value) &&
    isNonNegativeInteger(value.candidateCount) &&
    isNonNegativeInteger(value.filteredCount)
  );
}

function isBoundedInteger(value: unknown, minimum: number, maximum: number): boolean {
  return typeof value === "number" && Number.isInteger(value) && value >= minimum && value <= maximum;
}

function isNonNegativeInteger(value: unknown): boolean {
  return typeof value === "number" && Number.isInteger(value) && value >= 0;
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

function bridgeFailure<TData>(): LocalDesktopCommandResponse<TData> {
  return { ok: false, errorCode: "COMMAND_BRIDGE_FAILED", retryable: false };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
