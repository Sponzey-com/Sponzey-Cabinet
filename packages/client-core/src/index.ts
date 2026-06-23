export type ClientRuntimeKind = "web-local" | "desktop-local" | "remote";

export interface ClientCapabilities {
  readonly runtime: ClientRuntimeKind;
  readonly supportsLocalWorkspace: boolean;
  readonly supportsRemoteWorkspace: boolean;
}

export type DocumentQueryName =
  | "get-current-document"
  | "get-document-history"
  | "search-documents"
  | "get-link-overview"
  | "list-document-assets";

export interface DocumentIdentity {
  readonly workspaceId: string;
  readonly documentId: string;
}

export interface CurrentDocumentQuery extends DocumentIdentity {
  readonly queryName: "get-current-document";
}

export interface DocumentHistoryQuery extends DocumentIdentity {
  readonly queryName: "get-document-history";
  readonly cursor?: string;
  readonly limit: number;
}

export interface SearchDocumentsQuery {
  readonly queryName: "search-documents";
  readonly workspaceId: string;
  readonly text: string;
  readonly limit: number;
}

export interface LinkOverviewQuery extends DocumentIdentity {
  readonly queryName: "get-link-overview";
}

export interface ListDocumentAssetsQuery extends DocumentIdentity {
  readonly queryName: "list-document-assets";
}

export interface CurrentDocumentView extends DocumentIdentity {
  readonly title: string;
  readonly path: string;
  readonly body: string;
  readonly versionId: string;
}

export interface DocumentHistoryEntry {
  readonly versionId: string;
  readonly summary: string;
  readonly author: string;
  readonly createdAt: string;
}

export interface DocumentHistoryPage extends DocumentIdentity {
  readonly entries: readonly DocumentHistoryEntry[];
  readonly nextCursor?: string;
}

export interface SearchResultView extends DocumentIdentity {
  readonly title: string;
  readonly path: string;
  readonly snippet: string;
  readonly score: number;
}

export interface SearchResultsPage {
  readonly queryName: "search-documents";
  readonly workspaceId: string;
  readonly text: string;
  readonly results: readonly SearchResultView[];
}

export interface BacklinkView {
  readonly workspaceId: string;
  readonly sourceDocumentId: string;
  readonly targetDocumentId: string;
  readonly sourceTitle: string;
  readonly sourcePath: string;
}

export interface UnresolvedLinkView {
  readonly workspaceId: string;
  readonly sourceDocumentId: string;
  readonly targetSlug: string;
}

export interface OrphanDocumentView extends DocumentIdentity {
  readonly title: string;
  readonly path: string;
}

export interface LinkOverviewView extends DocumentIdentity {
  readonly queryName: "get-link-overview";
  readonly backlinks: readonly BacklinkView[];
  readonly unresolvedLinks: readonly UnresolvedLinkView[];
  readonly orphanDocuments: readonly OrphanDocumentView[];
}

export type AssetAvailability = "available" | "missing";

export interface AssetView {
  readonly assetId: string;
  readonly label: string;
  readonly fileName: string;
  readonly mediaType: string;
  readonly byteSize: number;
  readonly status: AssetAvailability;
}

export interface DocumentAssetsPage extends DocumentIdentity {
  readonly queryName: "list-document-assets";
  readonly assets: readonly AssetView[];
}

export interface SelectedAssetDraft {
  readonly assetId: string;
  readonly label: string;
  readonly fileName: string;
  readonly mediaType: string;
  readonly byteSize: number;
}

export interface AttachAssetCommand extends DocumentIdentity {
  readonly commandName: "attach-file-to-document";
  readonly asset: SelectedAssetDraft;
}

export interface AttachAssetResult {
  readonly assetId: string;
  readonly status: "attached" | "already-attached";
}

export interface CabinetDocumentClient {
  getCurrentDocument(query: CurrentDocumentQuery): Promise<CurrentDocumentView>;
  getDocumentHistory(query: DocumentHistoryQuery): Promise<DocumentHistoryPage>;
  searchDocuments(query: SearchDocumentsQuery): Promise<SearchResultsPage>;
  getLinkOverview(query: LinkOverviewQuery): Promise<LinkOverviewView>;
  listDocumentAssets(query: ListDocumentAssetsQuery): Promise<DocumentAssetsPage>;
  attachAsset(command: AttachAssetCommand): Promise<AttachAssetResult>;
}

export function createClientCapabilities(runtime: ClientRuntimeKind): ClientCapabilities {
  return {
    runtime,
    supportsLocalWorkspace: runtime === "web-local" || runtime === "desktop-local",
    supportsRemoteWorkspace: runtime === "remote",
  };
}

export function createCurrentDocumentQuery(workspaceId: string, documentId: string): CurrentDocumentQuery {
  return {
    queryName: "get-current-document",
    workspaceId,
    documentId,
  };
}

export function createDocumentHistoryQuery(
  workspaceId: string,
  documentId: string,
  limit: number,
  cursor?: string,
): DocumentHistoryQuery {
  return {
    queryName: "get-document-history",
    workspaceId,
    documentId,
    limit,
    cursor,
  };
}

export function createSearchDocumentsQuery(
  workspaceId: string,
  text: string,
  limit: number,
): SearchDocumentsQuery {
  return {
    queryName: "search-documents",
    workspaceId,
    text,
    limit,
  };
}

export function createLinkOverviewQuery(workspaceId: string, documentId: string): LinkOverviewQuery {
  return {
    queryName: "get-link-overview",
    workspaceId,
    documentId,
  };
}

export function createListDocumentAssetsQuery(
  workspaceId: string,
  documentId: string,
): ListDocumentAssetsQuery {
  return {
    queryName: "list-document-assets",
    workspaceId,
    documentId,
  };
}

export function createAttachAssetClientCommand(
  workspaceId: string,
  documentId: string,
  asset: SelectedAssetDraft,
): AttachAssetCommand {
  return {
    commandName: "attach-file-to-document",
    workspaceId,
    documentId,
    asset,
  };
}
