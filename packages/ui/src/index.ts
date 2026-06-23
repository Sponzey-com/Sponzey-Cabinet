import type {
  ClientCapabilities,
  CurrentDocumentView,
  CurrentDocumentQuery,
  DocumentHistoryEntry,
  DocumentHistoryPage,
  AssetView,
  AttachAssetCommand,
  BacklinkView,
  DocumentAssetsPage,
  LinkOverviewView,
  OrphanDocumentView,
  SearchResultView,
  SearchResultsPage,
  SelectedAssetDraft,
  UnresolvedLinkView,
} from "@sponzey-cabinet/client-core";
import { createCurrentDocumentQuery } from "@sponzey-cabinet/client-core";

export interface ShellDescriptor {
  readonly appName: "Sponzey Cabinet";
  readonly capabilitySummary: string;
}

export function createShellDescriptor(capabilities: ClientCapabilities): ShellDescriptor {
  return {
    appName: "Sponzey Cabinet",
    capabilitySummary: `${capabilities.runtime}:${capabilities.supportsLocalWorkspace}`,
  };
}

export type WorkspaceShellZone =
  | "document-list"
  | "editor"
  | "metadata-panel"
  | "history-panel"
  | "status-bar"
  | "command-palette";

export interface WorkspaceShellPanel {
  readonly zone: WorkspaceShellZone;
  readonly label: string;
  readonly visible: boolean;
}

export interface WorkspaceShellModel {
  readonly appName: "Sponzey Cabinet";
  readonly runtime: ClientCapabilities["runtime"];
  readonly zones: readonly WorkspaceShellPanel[];
}

export function createWorkspaceShellModel(capabilities: ClientCapabilities): WorkspaceShellModel {
  return {
    appName: "Sponzey Cabinet",
    runtime: capabilities.runtime,
    zones: [
      { zone: "document-list", label: "Documents", visible: true },
      { zone: "editor", label: "Editor", visible: true },
      { zone: "metadata-panel", label: "Metadata", visible: true },
      { zone: "history-panel", label: "History", visible: true },
      { zone: "status-bar", label: "Status", visible: true },
      { zone: "command-palette", label: "Command", visible: true },
    ],
  };
}

export interface CurrentDocumentViewModel {
  readonly mode: "current";
  readonly queryName: "get-current-document";
  readonly documentId: string;
  readonly title: string;
  readonly path: string;
  readonly body: string;
  readonly versionId: string;
}

export interface HistoryEntryViewModel {
  readonly versionId: string;
  readonly summary: string;
  readonly author: string;
  readonly createdAt: string;
}

export interface HistoryPanelViewModel {
  readonly mode: "history";
  readonly queryName: "get-document-history";
  readonly documentId: string;
  readonly entries: readonly HistoryEntryViewModel[];
  readonly nextCursor?: string;
}

export function createCurrentDocumentViewModel(document: CurrentDocumentView): CurrentDocumentViewModel {
  return {
    mode: "current",
    queryName: "get-current-document",
    documentId: document.documentId,
    title: document.title,
    path: document.path,
    body: document.body,
    versionId: document.versionId,
  };
}

export function createHistoryPanelViewModel(page: DocumentHistoryPage): HistoryPanelViewModel {
  return {
    mode: "history",
    queryName: "get-document-history",
    documentId: page.documentId,
    entries: page.entries.map(createHistoryEntryViewModel),
    nextCursor: page.nextCursor,
  };
}

function createHistoryEntryViewModel(entry: DocumentHistoryEntry): HistoryEntryViewModel {
  return {
    versionId: entry.versionId,
    summary: entry.summary,
    author: entry.author,
    createdAt: entry.createdAt,
  };
}

export interface SearchResultItemViewModel {
  readonly workspaceId: string;
  readonly documentId: string;
  readonly title: string;
  readonly path: string;
  readonly snippet: string;
  readonly score: number;
}

export interface SearchPanelViewModel {
  readonly mode: "search";
  readonly queryName: "search-documents";
  readonly workspaceId: string;
  readonly text: string;
  readonly results: readonly SearchResultItemViewModel[];
}

export function createSearchPanelViewModel(page: SearchResultsPage): SearchPanelViewModel {
  return {
    mode: "search",
    queryName: "search-documents",
    workspaceId: page.workspaceId,
    text: page.text,
    results: page.results.map(createSearchResultItemViewModel),
  };
}

export function createOpenSearchResultCommand(result: SearchResultItemViewModel): CurrentDocumentQuery {
  return createCurrentDocumentQuery(result.workspaceId, result.documentId);
}

function createSearchResultItemViewModel(result: SearchResultView): SearchResultItemViewModel {
  return {
    workspaceId: result.workspaceId,
    documentId: result.documentId,
    title: result.title,
    path: result.path,
    snippet: result.snippet,
    score: result.score,
  };
}

export interface BacklinkItemViewModel {
  readonly workspaceId: string;
  readonly sourceDocumentId: string;
  readonly targetDocumentId: string;
  readonly sourceTitle: string;
  readonly sourcePath: string;
}

export interface UnresolvedLinkItemViewModel {
  readonly workspaceId: string;
  readonly sourceDocumentId: string;
  readonly targetSlug: string;
}

export interface OrphanDocumentItemViewModel {
  readonly workspaceId: string;
  readonly documentId: string;
  readonly title: string;
  readonly path: string;
}

export interface LinkPanelViewModel {
  readonly mode: "links";
  readonly queryName: "get-link-overview";
  readonly workspaceId: string;
  readonly documentId: string;
  readonly backlinks: readonly BacklinkItemViewModel[];
  readonly unresolvedLinks: readonly UnresolvedLinkItemViewModel[];
  readonly orphanDocuments: readonly OrphanDocumentItemViewModel[];
}

export function createLinkPanelViewModel(overview: LinkOverviewView): LinkPanelViewModel {
  return {
    mode: "links",
    queryName: "get-link-overview",
    workspaceId: overview.workspaceId,
    documentId: overview.documentId,
    backlinks: overview.backlinks.map(createBacklinkItemViewModel),
    unresolvedLinks: overview.unresolvedLinks.map(createUnresolvedLinkItemViewModel),
    orphanDocuments: overview.orphanDocuments.map(createOrphanDocumentItemViewModel),
  };
}

export function createOpenBacklinkCommand(backlink: BacklinkItemViewModel): CurrentDocumentQuery {
  return createCurrentDocumentQuery(backlink.workspaceId, backlink.sourceDocumentId);
}

export function createOpenOrphanDocumentCommand(orphan: OrphanDocumentItemViewModel): CurrentDocumentQuery {
  return createCurrentDocumentQuery(orphan.workspaceId, orphan.documentId);
}

function createBacklinkItemViewModel(backlink: BacklinkView): BacklinkItemViewModel {
  return {
    workspaceId: backlink.workspaceId,
    sourceDocumentId: backlink.sourceDocumentId,
    targetDocumentId: backlink.targetDocumentId,
    sourceTitle: backlink.sourceTitle,
    sourcePath: backlink.sourcePath,
  };
}

function createUnresolvedLinkItemViewModel(link: UnresolvedLinkView): UnresolvedLinkItemViewModel {
  return {
    workspaceId: link.workspaceId,
    sourceDocumentId: link.sourceDocumentId,
    targetSlug: link.targetSlug,
  };
}

function createOrphanDocumentItemViewModel(orphan: OrphanDocumentView): OrphanDocumentItemViewModel {
  return {
    workspaceId: orphan.workspaceId,
    documentId: orphan.documentId,
    title: orphan.title,
    path: orphan.path,
  };
}

export interface AssetItemViewModel {
  readonly assetId: string;
  readonly label: string;
  readonly fileName: string;
  readonly mediaType: string;
  readonly byteSize: number;
  readonly status: "available" | "missing";
}

export interface AssetPanelViewModel {
  readonly mode: "assets";
  readonly queryName: "list-document-assets";
  readonly workspaceId: string;
  readonly documentId: string;
  readonly assets: readonly AssetItemViewModel[];
}

export function createAssetPanelViewModel(page: DocumentAssetsPage): AssetPanelViewModel {
  return {
    mode: "assets",
    queryName: "list-document-assets",
    workspaceId: page.workspaceId,
    documentId: page.documentId,
    assets: page.assets.map(createAssetItemViewModel),
  };
}

export function createAttachAssetCommand(
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

function createAssetItemViewModel(asset: AssetView): AssetItemViewModel {
  return {
    assetId: asset.assetId,
    label: asset.label,
    fileName: asset.fileName,
    mediaType: asset.mediaType,
    byteSize: asset.byteSize,
    status: asset.status,
  };
}
