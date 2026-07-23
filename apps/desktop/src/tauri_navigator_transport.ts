import type {
  AssetSearchResultsPage,
  DocumentNavigatorQuery,
  DocumentNavigatorResult,
  LocalDesktopCommandEnvelope,
  LocalDesktopCommandResponse,
  LocalDesktopCommandTransport,
  SearchAssetsQuery,
  SearchDocumentsQuery,
  SearchResultsPage,
} from "@sponzey-cabinet/client-core";

import type { TauriInvoke } from "./tauri_home_transport.ts";

export function createTauriDocumentNavigatorTransport(
  invoke: TauriInvoke,
): LocalDesktopCommandTransport {
  return async <TData>(
    envelope: LocalDesktopCommandEnvelope,
  ): Promise<LocalDesktopCommandResponse<TData>> => {
    try {
      if (
        envelope.commandName === "local_document_navigator" &&
        isNavigatorQuery(envelope.payload)
      ) {
        const query = envelope.payload;
        const response = await invoke("get_desktop_document_navigator", {
          request: {
            workspace_id: query.workspaceId,
            view: query.view,
            view_key: query.viewKey,
            filter: query.filter,
            limit: query.limit,
            cursor: query.cursor,
          },
        });
        return isNavigatorResponse(response)
          ? (response as LocalDesktopCommandResponse<TData>)
          : bridgeFailure();
      }

      if (envelope.commandName === "search_documents" && isSearchDocumentsQuery(envelope.payload)) {
        const query = envelope.payload;
        const response = await invoke("search_desktop_documents", {
          request: {
            workspace_id: query.workspaceId,
            text: query.text,
            limit: query.limit,
          },
        });
        return isSearchDocumentsResponse(response)
          ? (response as LocalDesktopCommandResponse<TData>)
          : bridgeFailure();
      }

      if (envelope.commandName === "search_assets" && isAssetSearchQuery(envelope.payload)) {
        const query = envelope.payload;
        const response = await invoke("search_desktop_assets", {
          request: {
            workspace_id: query.workspaceId,
            text: query.text,
            limit: query.limit,
          },
        });
        return isAssetSearchResponse(response)
          ? (response as LocalDesktopCommandResponse<TData>)
          : bridgeFailure();
      }

      return bridgeFailure();
    } catch {
      return bridgeFailure();
    }
  };
}

function isNavigatorQuery(
  value: Record<string, unknown>,
): value is DocumentNavigatorQuery & Record<string, unknown> {
  return (
    typeof value.workspaceId === "string" &&
    ["Tree", "Collection", "Tag", "Recent", "Favorite"].includes(String(value.view)) &&
    typeof value.limit === "number" &&
    Number.isInteger(value.limit) &&
    (value.viewKey === undefined || typeof value.viewKey === "string") &&
    (value.filter === undefined || typeof value.filter === "string") &&
    (value.cursor === undefined || typeof value.cursor === "string")
  );
}

function isNavigatorResponse(
  value: unknown,
): value is LocalDesktopCommandResponse<DocumentNavigatorResult> {
  if (!isRecord(value) || typeof value.ok !== "boolean") return false;
  if (!value.ok) {
    return typeof value.errorCode === "string" && typeof value.retryable === "boolean";
  }
  return isNavigatorResult(value.data);
}

function isNavigatorResult(value: unknown): value is DocumentNavigatorResult {
  return (
    isRecord(value) &&
    typeof value.workspaceId === "string" &&
    ["Tree", "Collection", "Tag", "Recent", "Favorite"].includes(String(value.view)) &&
    ["Ready", "EmptyResult", "Degraded"].includes(String(value.state)) &&
    Array.isArray(value.items) &&
    value.items.every(isNavigatorItem) &&
    (value.nextCursor === undefined || value.nextCursor === null || typeof value.nextCursor === "string")
  );
}

function isNavigatorItem(value: unknown): boolean {
  return (
    isRecord(value) &&
    typeof value.documentId === "string" &&
    typeof value.title === "string" &&
    typeof value.path === "string" &&
    Array.isArray(value.collections) &&
    value.collections.every((item) => typeof item === "string") &&
    Array.isArray(value.tags) &&
    value.tags.every((item) => typeof item === "string") &&
    typeof value.favorite === "boolean"
  );
}

function isSearchDocumentsQuery(
  value: Record<string, unknown>,
): value is SearchDocumentsQuery & Record<string, unknown> {
  return (
    value.queryName === "search-documents" &&
    typeof value.workspaceId === "string" &&
    typeof value.text === "string" &&
    typeof value.limit === "number" &&
    Number.isInteger(value.limit)
  );
}

function isSearchDocumentsResponse(
  value: unknown,
): value is LocalDesktopCommandResponse<SearchResultsPage> {
  if (!isRecord(value) || typeof value.ok !== "boolean") return false;
  if (!value.ok) {
    return typeof value.errorCode === "string" && typeof value.retryable === "boolean";
  }
  return (
    isRecord(value.data) &&
    value.data.queryName === "search-documents" &&
    typeof value.data.workspaceId === "string" &&
    typeof value.data.text === "string" &&
    Array.isArray(value.data.results) &&
    value.data.results.every(isSearchDocumentItem)
  );
}

function isSearchDocumentItem(value: unknown): boolean {
  return (
    isRecord(value) &&
    typeof value.documentId === "string" &&
    typeof value.title === "string" &&
    typeof value.path === "string" &&
    typeof value.snippet === "string" &&
    typeof value.score === "number"
  );
}

function isAssetSearchQuery(
  value: Record<string, unknown>,
): value is SearchAssetsQuery & Record<string, unknown> {
  return (
    value.queryName === "search-assets" &&
    typeof value.workspaceId === "string" &&
    typeof value.text === "string" &&
    typeof value.limit === "number" &&
    Number.isInteger(value.limit)
  );
}

function isAssetSearchResponse(
  value: unknown,
): value is LocalDesktopCommandResponse<AssetSearchResultsPage> {
  if (!isRecord(value) || typeof value.ok !== "boolean") return false;
  if (!value.ok) {
    return typeof value.errorCode === "string" && typeof value.retryable === "boolean";
  }
  return (
    isRecord(value.data) &&
    value.data.queryName === "search-assets" &&
    typeof value.data.workspaceId === "string" &&
    typeof value.data.text === "string" &&
    Array.isArray(value.data.results) &&
    value.data.results.every(isAssetSearchItem)
  );
}

function isAssetSearchItem(value: unknown): boolean {
  return (
    isRecord(value) &&
    typeof value.assetId === "string" &&
    typeof value.fileName === "string" &&
    typeof value.mediaType === "string" &&
    typeof value.byteSize === "number" &&
    typeof value.score === "number"
  );
}

function bridgeFailure<TData>(): LocalDesktopCommandResponse<TData> {
  return {
    ok: false,
    errorCode: "COMMAND_BRIDGE_FAILED",
    retryable: false,
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
