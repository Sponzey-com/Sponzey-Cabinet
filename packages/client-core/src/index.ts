export type ClientRuntimeKind = "web-local" | "desktop-local" | "remote";

export type PlatformFeatureSupport = "unsupported" | "view_only" | "interactive";

export interface ClientCapabilities {
  readonly runtime: ClientRuntimeKind;
  readonly supportsLocalWorkspace: boolean;
  readonly supportsRemoteWorkspace: boolean;
}

export type CurrentProductScope = "personal_local_desktop";

export type PersonalLocalDesktopPlatform = "windows" | "macos" | "linux";

export type PersonalLocalDesktopActionId =
  | "open-home"
  | "new-document"
  | "quick-search"
  | "open-graph"
  | "open-assets"
  | "ask-ai"
  | "create-backup"
  | "import-markdown"
  | "export-package"
  | "open-settings";

export type ForbiddenPersonalLocalDesktopActionId =
  | "server-url"
  | "tenant-admin"
  | "organization-admin"
  | "team-invite"
  | "tenant-settings"
  | "sso-settings"
  | "server-workspace-connect"
  | "billing"
  | "admin-console";

export interface PersonalLocalDesktopAction {
  readonly id: PersonalLocalDesktopActionId;
  readonly label: string;
}

export interface PersonalLocalDesktopCapabilityProfile extends ClientCapabilities {
  readonly productScope: CurrentProductScope;
  readonly runtime: "desktop-local";
  readonly supportsLocalWorkspace: true;
  readonly supportsRemoteWorkspace: false;
  readonly platforms: readonly PersonalLocalDesktopPlatform[];
  readonly actions: readonly PersonalLocalDesktopAction[];
}

export type DocumentQueryName =
  | "get-current-document"
  | "get-document-history"
  | "get-document-version"
  | "preview-document-restore"
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

export interface DocumentVersionQuery extends DocumentIdentity {
  readonly queryName: "get-document-version";
  readonly versionId: string;
}

export interface DocumentVersionView extends DocumentIdentity {
  readonly versionId: string;
  readonly body: string;
}

export type RestoreDiffLineKindView = "unchanged" | "removed" | "added";

export interface RestoreDiffLineView {
  readonly kind: RestoreDiffLineKindView;
  readonly text: string;
}

export interface RestorePreviewQuery extends DocumentIdentity {
  readonly queryName: "preview-document-restore";
  readonly targetVersionId: string;
  readonly expectedCurrentVersionId: string;
}

export interface RestorePreviewResult extends DocumentIdentity {
  readonly targetVersionId: string;
  readonly expectedCurrentVersionId: string;
  readonly canRestore: boolean;
  readonly lines: readonly RestoreDiffLineView[];
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

export interface KnowledgeGraphQuery extends DocumentIdentity {
  readonly queryName: "get-knowledge-graph";
  readonly depth: 1 | 2;
  readonly direction: "incoming" | "outgoing" | "both";
  readonly includeUnresolved: boolean;
  readonly includeAssets: boolean;
  readonly nodeLimit: number;
  readonly edgeLimit: number;
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

export type AiRetrievalSourceKindView =
  | "document"
  | "attachment"
  | "canvas"
  | "comment"
  | "review"
  | "graph";

export type AiFreshnessStatusView = "fresh" | "stale" | "unknown";

export type AiPermissionDecisionSummaryView = "allowed" | "filtered";

export interface AiRetrievalQuery {
  readonly workspaceId: string;
  readonly text: string;
  readonly limit: number;
}

export interface AiRetrievalCandidateView {
  readonly sourceId: string;
  readonly sourceKind: AiRetrievalSourceKindView;
  readonly sourceTitle?: string;
  readonly citationReference: string;
  readonly headingAnchor?: string;
  readonly blockReference?: string;
  readonly freshness: AiFreshnessStatusView;
  readonly permissionDecision: AiPermissionDecisionSummaryView;
}

export interface AiRetrievalResultPage {
  readonly queryName: "ai-retrieval";
  readonly workspaceId: string;
  readonly textHash: string;
  readonly candidates: readonly AiRetrievalCandidateView[];
  readonly durationMs: number;
}

export interface AskKnowledgeBaseCommand {
  readonly workspaceId: string;
  readonly question: string;
  readonly retrievalLimit: number;
}

export type AiAnswerJobStateView =
  | "Queued"
  | "RetrievalPreparing"
  | "ProviderRequested"
  | "CitationValidating"
  | "Completed"
  | "Refused"
  | "RetryScheduled"
  | "Failed";

export interface AiAnswerJobQuery {
  readonly workspaceId: string;
  readonly jobId: string;
}

export interface AiAnswerJobView {
  readonly jobId: string;
  readonly state: AiAnswerJobStateView;
  readonly citationCount: number;
  readonly freshnessStatus: AiFreshnessStatusView;
}

export interface AiCitationView {
  readonly sourceId: string;
  readonly sourceKind: AiRetrievalSourceKindView;
  readonly sourceTitle?: string;
  readonly citationReference: string;
  readonly headingAnchor?: string;
  readonly blockReference?: string;
  readonly freshness: AiFreshnessStatusView;
  readonly permissionDecision?: AiPermissionDecisionSummaryView;
}

export interface AiAnswerResultView {
  readonly jobId: string;
  readonly state: "Completed" | "Refused" | "Failed";
  readonly answerReference?: string;
  readonly refusalCode?: string;
  readonly freshnessStatus: AiFreshnessStatusView;
  readonly citations: readonly AiCitationView[];
}

export type LocalAiToolOperationView =
  | "read-document"
  | "search-documents"
  | "open-citation"
  | "ask-ai"
  | "read-asset-metadata"
  | "read-graph"
  | "write-document"
  | "delete-document"
  | "admin"
  | "server"
  | "team"
  | "billing"
  | "sso"
  | "connector-setup";

export interface LocalAiToolDescriptorView {
  readonly id: string;
  readonly label: string;
  readonly operation: LocalAiToolOperationView;
}

export type AiProviderSettingsStateView =
  | "NotConfigured"
  | "Configured"
  | "ValidationQueued"
  | "Valid"
  | "Invalid"
  | "Disabled";

export interface AiProviderSettingsSummaryView {
  readonly state: AiProviderSettingsStateView;
  readonly providerName?: string;
  readonly modelName?: string;
  readonly credentialHandlePresent: boolean;
  readonly validationCode?: string;
}

export type BackupArtifactOperationView = "backup" | "export";

export interface BackupArtifactManifestSummaryView {
  readonly artifactId: string;
  readonly operation: BackupArtifactOperationView;
  readonly documentCount: number;
  readonly assetCount: number;
  readonly versionCount: number;
  readonly byteSizeBucket: string;
  readonly createdAtIso: string;
  readonly sealed: boolean;
  readonly excludedSecretCategories: readonly string[];
}

export type RestoreStagingStateView =
  | "Staging"
  | "Validating"
  | "ReadyToApply"
  | "Applying"
  | "Completed"
  | "Failed";

export interface RestoreStagingIssueView {
  readonly code: string;
  readonly severity: "warning" | "error";
}

export type ImportSourceKindView = "markdown-folder" | "obsidian-vault";

export type ImportPreviewStateView =
  | "Selected"
  | "Scanning"
  | "PreviewReady"
  | "Applying"
  | "Completed"
  | "Failed";

export interface ImportPreviewSummaryView {
  readonly sourceKind: ImportSourceKindView;
  readonly sourceHash: string;
  readonly scannedDocumentCount: number;
  readonly assetReferenceCount: number;
  readonly linkCount: number;
  readonly unsupportedItemCount?: number;
  readonly estimatedByteSizeBucket: string;
}

export interface ImportConflictItemView {
  readonly code: string;
  readonly severity: "info" | "warning" | "blocking";
  readonly count: number;
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

export type AssetAvailability = "available" | "missing" | "metadata_only";
export type AssetPreviewStatus = "ready" | "unavailable" | "pending";
export type AssetOcrStatus = "not-indexed" | "indexed" | "unavailable";
export type AssetIndexStatus = "Fresh" | "Stale" | "Rebuilding" | "RebuildFailed";

export interface AssetView {
  readonly assetId: string;
  readonly label: string;
  readonly fileName: string;
  readonly mediaType: string;
  readonly byteSize: number;
  readonly status: AssetAvailability;
  readonly referencedDocumentCount?: number;
  readonly previewState?: AssetPreviewStatus;
  readonly ocrState?: AssetOcrStatus;
  readonly indexState?: AssetIndexStatus;
}

export interface DocumentAssetsPage extends DocumentIdentity {
  readonly queryName: "list-document-assets";
  readonly assets: readonly AssetView[];
}

export type KnowledgeGraphStatusView =
  | "clean"
  | "reindex_requested"
  | "reindexing"
  | "degraded";

export type KnowledgeGraphNodeKindView =
  | "document"
  | "unresolved_link"
  | "attachment"
  | "external_link";

export type KnowledgeGraphEdgeKindView =
  | "document_link"
  | "attachment_reference"
  | "external_reference"
  | "canvas_relation";

export interface KnowledgeGraphNodeView {
  readonly id: string;
  readonly kind: KnowledgeGraphNodeKindView;
}

export interface KnowledgeGraphEdgeView {
  readonly id: string;
  readonly sourceId: string;
  readonly targetId: string;
  readonly kind: KnowledgeGraphEdgeKindView;
}

export interface KnowledgeGraphStatsView {
  readonly candidateCount: number;
  readonly filteredCount: number;
}

export interface KnowledgeGraphPerformanceView {
  readonly targetMs: number;
  readonly observedMs: number;
}

export interface KnowledgeGraphView {
  readonly centerDocumentId: string;
  readonly status: KnowledgeGraphStatusView;
  readonly nodes: readonly KnowledgeGraphNodeView[];
  readonly edges: readonly KnowledgeGraphEdgeView[];
  readonly stats: KnowledgeGraphStatsView;
  readonly freshnessRevision: string;
  readonly performance?: KnowledgeGraphPerformanceView;
}

export type CanvasLifecycleStateView = "draft" | "saved" | "embedded" | "updated" | "archived";

export type CanvasNodeTargetKindView = "document" | "attachment" | "external_link" | "text_card";

export interface CanvasNodeView {
  readonly id: string;
  readonly targetKind: CanvasNodeTargetKindView;
  readonly x: number;
  readonly y: number;
}

export interface CanvasEdgeView {
  readonly id: string;
  readonly sourceId: string;
  readonly targetId: string;
}

export interface CanvasView {
  readonly canvasId: string;
  readonly state: CanvasLifecycleStateView;
  readonly nodes: readonly CanvasNodeView[];
  readonly edges: readonly CanvasEdgeView[];
  readonly embedReference?: string;
}

export interface CanvasCommandView {
  readonly canvasId: string;
  readonly state: CanvasLifecycleStateView;
  readonly nodeCount: number;
  readonly edgeCount: number;
  readonly productLogEvent: string;
}

export interface CanvasEmbedView {
  readonly reference: string;
  readonly productLogEvent: string;
}

export interface CreateCanvasCommand {
  readonly workspaceId: string;
  readonly canvasId: string;
}

export type AddCanvasNodeTargetCommand =
  | { readonly kind: "document"; readonly documentId: string }
  | { readonly kind: "attachment"; readonly assetSha256Hex: string }
  | { readonly kind: "external_link"; readonly url: string }
  | { readonly kind: "text_card"; readonly text: string };

export interface AddCanvasNodeCommand {
  readonly workspaceId: string;
  readonly canvasId: string;
  readonly nodeId: string;
  readonly target: AddCanvasNodeTargetCommand;
  readonly x: number;
  readonly y: number;
}

export interface EmbedCanvasCommand extends DocumentIdentity {
  readonly canvasId: string;
}

export type CollaborationRealtimeEventName =
  | "join-document-room"
  | "broadcast-operation"
  | "broadcast-presence"
  | "request-replay";

export interface JoinDocumentRoomRealtimeCommand extends DocumentIdentity {
  readonly sessionId: string;
  readonly actorUserId: string;
}

export interface BroadcastOperationRealtimeCommand extends DocumentIdentity {
  readonly sessionId: string;
  readonly actorUserId: string;
  readonly operationId: string;
  readonly baseRevision: number;
  readonly currentRevision: number;
  readonly startOffset: number;
  readonly endOffset: number;
  readonly insertedText: string;
}

export interface BroadcastPresenceRealtimeCommand extends DocumentIdentity {
  readonly sessionId: string;
  readonly actorUserId: string;
  readonly cursorStart: number;
  readonly cursorEnd: number;
  readonly selectedText?: string;
  readonly documentBody?: string;
  readonly token?: string;
}

export interface ReplayLocalChangesRealtimeCommand extends DocumentIdentity {
  readonly sessionId: string;
  readonly lastAcknowledgedSequence?: number;
}

export interface JoinDocumentRoomRealtimeRequest extends JoinDocumentRoomRealtimeCommand {
  readonly eventName: "join-document-room";
}

export interface BroadcastOperationRealtimeRequest extends BroadcastOperationRealtimeCommand {
  readonly eventName: "broadcast-operation";
}

export interface BroadcastPresenceRealtimeRequest extends DocumentIdentity {
  readonly eventName: "broadcast-presence";
  readonly sessionId: string;
  readonly actorUserId: string;
  readonly cursorStart: number;
  readonly cursorEnd: number;
}

export interface ReplayLocalChangesRealtimeRequest extends ReplayLocalChangesRealtimeCommand {
  readonly eventName: "request-replay";
}

export type CollaborationRealtimeRequest =
  | JoinDocumentRoomRealtimeRequest
  | BroadcastOperationRealtimeRequest
  | BroadcastPresenceRealtimeRequest
  | ReplayLocalChangesRealtimeRequest;

export interface CollaborationRealtimeAcceptedResponse extends DocumentIdentity {
  readonly status: "accepted";
}

export interface CollaborationRealtimeRejectedResponse extends DocumentIdentity {
  readonly status: "rejected";
  readonly errorCode: string;
}

export type CollaborationRealtimeResponse =
  | CollaborationRealtimeAcceptedResponse
  | CollaborationRealtimeRejectedResponse;

export type CollaborationRealtimeTransport = (
  request: CollaborationRealtimeRequest,
) => Promise<CollaborationRealtimeResponse>;

export interface CollaborationRealtimeClient {
  joinDocumentRoom(command: JoinDocumentRoomRealtimeCommand): Promise<CollaborationRealtimeAcceptedResponse>;
  broadcastOperation(
    command: BroadcastOperationRealtimeCommand,
  ): Promise<CollaborationRealtimeAcceptedResponse>;
  broadcastPresence(
    command: BroadcastPresenceRealtimeCommand,
  ): Promise<CollaborationRealtimeAcceptedResponse>;
  requestReplay(
    command: ReplayLocalChangesRealtimeCommand,
  ): Promise<CollaborationRealtimeAcceptedResponse>;
}

export class CabinetRealtimeClientError extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = "CabinetRealtimeClientError";
    this.code = code;
  }
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

export const PHASE009_LOCAL_DESKTOP_COMMAND_NAMES = [
  "local_workspace_bootstrap",
  "local_workspace_home",
  "local_document_navigator",
  "create_document",
  "rename_document",
  "save_document_revision",
  "get_current_document",
  "update_current_document",
  "get_document_history",
  "get_document_version",
  "preview_document_restore",
  "restore_document_version",
  "search_documents",
  "get_link_overview",
  "get_graph_projection",
  "list_document_assets",
  "attach_document_asset",
  "create_backup",
  "preview_import",
  "preview_restore",
  "apply_restore",
] as const;

export type LocalDesktopCommandName = (typeof PHASE009_LOCAL_DESKTOP_COMMAND_NAMES)[number];

export type LocalDesktopCommandErrorCode =
  | "LOCAL_WORKSPACE_NOT_READY"
  | "DOCUMENT_NOT_FOUND"
  | "VERSION_CONFLICT"
  | "STORE_UNAVAILABLE"
  | "INDEX_STALE"
  | "ASSET_NOT_FOUND"
  | "COMMAND_BRIDGE_FAILED"
  | "DOCUMENT_NAVIGATOR_INVALID_INPUT"
  | "DOCUMENT_NAVIGATOR_PROJECTION_UNAVAILABLE"
  | "DOCUMENT_AUTHORING_INVALID_INPUT"
  | "DOCUMENT_AUTHORING_NOT_FOUND"
  | "DOCUMENT_AUTHORING_VERSION_CONFLICT"
  | "DOCUMENT_AUTHORING_POINTER_MISSING"
  | "DOCUMENT_AUTHORING_POINTER_UPDATE_FAILED"
  | "DOCUMENT_AUTHORING_STORAGE_UNAVAILABLE"
  | "DOCUMENT_AUTHORING_RUNTIME_UNAVAILABLE"
  | "DOCUMENT_RESTORE_VERSION_CONFLICT"
  | "DOCUMENT_RESTORE_NOT_FOUND"
  | "DOCUMENT_RESTORE_STORAGE_UNAVAILABLE"
  | "GRAPH_INVALID_INPUT"
  | "GRAPH_PROJECTION_NOT_FOUND"
  | "GRAPH_PROJECTION_UNAVAILABLE"
  | "GRAPH_PROJECTION_CORRUPTED"
  | "ASSET_INVALID_INPUT"
  | "ASSET_DOCUMENT_NOT_FOUND"
  | "ASSET_METADATA_UNAVAILABLE"
  | "COMMAND_INVALID_TRANSITION";

export type LocalDesktopSetupHealthView =
  | "Ready"
  | "Initializing"
  | "MigrationRequired"
  | "RepairRequired"
  | "Failed";

export interface LocalDesktopCommandEnvelope<TPayload = Record<string, unknown>> {
  readonly commandName: LocalDesktopCommandName;
  readonly payload: TPayload;
  readonly correlationId?: string;
}

export type LocalDesktopCommandResponse<TData> =
  | {
      readonly ok: true;
      readonly data: TData;
    }
  | {
      readonly ok: false;
      readonly errorCode: LocalDesktopCommandErrorCode;
      readonly retryable: boolean;
      readonly repairRequired?: boolean;
      readonly message?: string;
    };

export type LocalDesktopCommandTransport = <TData>(
  request: LocalDesktopCommandEnvelope,
) => Promise<LocalDesktopCommandResponse<TData>>;

export interface OpenDefaultWorkspaceResult {
  readonly workspaceId: string;
  readonly displayName: string;
  readonly setupHealth: LocalDesktopSetupHealthView;
}

export interface WorkspaceHomeQuery {
  readonly workspaceId: string;
  readonly recentDocuments: number;
  readonly favorites: number;
  readonly tags: number;
  readonly recentChanges: number;
  readonly unfinishedItems: number;
}

export interface WorkspaceHomeDocumentItem {
  readonly documentId: string;
  readonly title: string;
  readonly path: string;
}

export interface WorkspaceHomeTagItem {
  readonly label: string;
  readonly documentCount: number;
}

export interface WorkspaceHomeChangeItem {
  readonly documentId: string;
  readonly summary: string;
}

export interface WorkspaceHomeUnfinishedItem {
  readonly documentId: string;
  readonly label: string;
}

export interface WorkspaceHomeResult {
  readonly workspaceId: string;
  readonly state: "Ready" | "Empty" | "Degraded";
  readonly recentDocuments: readonly WorkspaceHomeDocumentItem[];
  readonly favorites: readonly WorkspaceHomeDocumentItem[];
  readonly tags: readonly WorkspaceHomeTagItem[];
  readonly recentChanges: readonly WorkspaceHomeChangeItem[];
  readonly unfinishedItems: readonly WorkspaceHomeUnfinishedItem[];
  readonly backupStatus: "NeverCreated" | "Fresh" | "Stale" | "Failed";
  readonly healthStatus: "Healthy" | "Degraded" | "ReadOnlyRecovery";
}

export type DocumentNavigatorView =
  | "Tree"
  | "Collection"
  | "Tag"
  | "Recent"
  | "Favorite";

export interface DocumentNavigatorQuery {
  readonly workspaceId: string;
  readonly view: DocumentNavigatorView;
  readonly viewKey?: string;
  readonly filter?: string;
  readonly limit: number;
  readonly cursor?: string;
}

export interface DocumentNavigatorItem {
  readonly documentId: string;
  readonly title: string;
  readonly path: string;
  readonly collections: readonly string[];
  readonly tags: readonly string[];
  readonly favorite: boolean;
}

export interface DocumentNavigatorResult {
  readonly workspaceId: string;
  readonly view: DocumentNavigatorView;
  readonly state: "Ready" | "EmptyResult" | "Degraded";
  readonly items: readonly DocumentNavigatorItem[];
  readonly nextCursor?: string | null;
}

export interface SaveCurrentDocumentCommand extends DocumentIdentity {
  readonly title: string;
  readonly path: string;
  readonly body: string;
  readonly expectedVersionId: string;
}

export interface SaveCurrentDocumentResult extends DocumentIdentity {
  readonly status: "saved-local";
  readonly currentVersionId: string;
  readonly versionAppended: true;
}

export interface CreateLocalDocumentCommand extends DocumentIdentity {
  readonly path: string;
  readonly body: string;
  readonly versionId: string;
  readonly snapshotRef: string;
  readonly author: string;
  readonly summary: string;
}

export interface CreateLocalDocumentResult extends DocumentIdentity {
  readonly currentVersionId: string;
}

export interface RenameLocalDocumentCommand extends DocumentIdentity {
  readonly currentVersionId: string;
  readonly title: string;
  readonly path: string;
}

export interface RenameLocalDocumentResult extends DocumentIdentity {
  readonly currentVersionId: string;
  readonly title: string;
  readonly path: string;
}

export interface SaveDocumentRevisionCommand extends DocumentIdentity {
  readonly body: string;
  readonly expectedVersionId: string;
  readonly nextVersionId: string;
  readonly snapshotRef: string;
  readonly author: string;
  readonly summary: string;
  readonly revision: number;
}

export interface SaveDocumentRevisionResult extends SaveCurrentDocumentResult {
  readonly revision: number;
}

export interface RestoreDocumentVersionCommand extends DocumentIdentity {
  readonly commandName: "restore-document-version";
  readonly targetVersionId: string;
  readonly expectedCurrentVersionId: string;
  readonly restoredVersionId: string;
  readonly restoredSnapshotRef: string;
  readonly author: string;
  readonly summary: string;
}

export interface RestoreDocumentVersionResult extends DocumentIdentity {
  readonly restoredVersionId: string;
  readonly currentVersionId: string;
  readonly finalState: "Completed";
}

export interface LocalDesktopCommandClient {
  openDefaultWorkspace(): Promise<OpenDefaultWorkspaceResult>;
  getWorkspaceHome(query: WorkspaceHomeQuery): Promise<WorkspaceHomeResult>;
  getDocumentNavigator(query: DocumentNavigatorQuery): Promise<DocumentNavigatorResult>;
  createDocument(command: CreateLocalDocumentCommand): Promise<CreateLocalDocumentResult>;
  renameDocument(command: RenameLocalDocumentCommand): Promise<RenameLocalDocumentResult>;
  getCurrentDocument(query: CurrentDocumentQuery): Promise<CurrentDocumentView>;
  saveDocumentRevision(command: SaveDocumentRevisionCommand): Promise<SaveDocumentRevisionResult>;
  saveCurrentDocument(command: SaveCurrentDocumentCommand): Promise<SaveCurrentDocumentResult>;
  listDocumentHistory(query: DocumentHistoryQuery): Promise<DocumentHistoryPage>;
  getDocumentVersion(query: DocumentVersionQuery): Promise<DocumentVersionView>;
  previewDocumentRestore(query: RestorePreviewQuery): Promise<RestorePreviewResult>;
  restoreDocumentVersion(command: RestoreDocumentVersionCommand): Promise<RestoreDocumentVersionResult>;
  searchDocuments(query: SearchDocumentsQuery): Promise<SearchResultsPage>;
  getLinkOverview(query: LinkOverviewQuery): Promise<LinkOverviewView>;
  getKnowledgeGraph(query: KnowledgeGraphQuery): Promise<KnowledgeGraphView>;
  getAssetMetadata(query: ListDocumentAssetsQuery): Promise<DocumentAssetsPage>;
}

export class LocalDesktopCommandClientError extends Error {
  readonly code: LocalDesktopCommandErrorCode;
  readonly retryable: boolean;
  readonly repairRequired: boolean;

  constructor(code: LocalDesktopCommandErrorCode, retryable: boolean, repairRequired = false) {
    super(`local desktop command failed: ${code}`);
    this.name = "LocalDesktopCommandClientError";
    this.code = code;
    this.retryable = retryable;
    this.repairRequired = repairRequired;
  }
}

export const LocalDesktopCommandState = Object.freeze({
  Idle: "Idle",
  Dispatching: "Dispatching",
  Succeeded: "Succeeded",
  Failed: "Failed",
});

export const LocalDesktopCommandEvent = Object.freeze({
  Dispatch: "Dispatch",
  Resolve: "Resolve",
  Reject: "Reject",
});

export type LocalDesktopCommandStateValue =
  (typeof LocalDesktopCommandState)[keyof typeof LocalDesktopCommandState];

export type LocalDesktopCommandEventValue =
  (typeof LocalDesktopCommandEvent)[keyof typeof LocalDesktopCommandEvent];

export interface LocalDesktopCommandTransition {
  readonly state: LocalDesktopCommandStateValue;
  readonly errorCode?: LocalDesktopCommandErrorCode;
  readonly retryable?: boolean;
}

export function transitionLocalDesktopCommandState(
  currentState: LocalDesktopCommandStateValue,
  event: LocalDesktopCommandEventValue,
  detail: {
    readonly errorCode?: LocalDesktopCommandErrorCode;
    readonly retryable?: boolean;
  } = {},
): LocalDesktopCommandTransition {
  if (
    currentState === LocalDesktopCommandState.Idle &&
    event === LocalDesktopCommandEvent.Dispatch
  ) {
    return { state: LocalDesktopCommandState.Dispatching };
  }
  if (
    currentState === LocalDesktopCommandState.Dispatching &&
    event === LocalDesktopCommandEvent.Resolve
  ) {
    return { state: LocalDesktopCommandState.Succeeded };
  }
  if (
    currentState === LocalDesktopCommandState.Dispatching &&
    event === LocalDesktopCommandEvent.Reject
  ) {
    return {
      state: LocalDesktopCommandState.Failed,
      errorCode: detail.errorCode ?? "COMMAND_BRIDGE_FAILED",
      retryable: detail.retryable ?? false,
    };
  }
  return {
    state: LocalDesktopCommandState.Failed,
    errorCode: "COMMAND_INVALID_TRANSITION",
    retryable: false,
  };
}

export function createLocalDesktopCommandClient(
  transport: LocalDesktopCommandTransport,
): LocalDesktopCommandClient {
  return {
    openDefaultWorkspace() {
      return callLocalDesktopCommand<OpenDefaultWorkspaceResult>(
        transport,
        "local_workspace_bootstrap",
        {},
      );
    },

    getWorkspaceHome(query) {
      return callLocalDesktopCommand<WorkspaceHomeResult>(
        transport,
        "local_workspace_home",
        query,
      );
    },

    getDocumentNavigator(query) {
      return callLocalDesktopCommand<DocumentNavigatorResult>(
        transport,
        "local_document_navigator",
        query,
      );
    },

    createDocument(command) {
      return callLocalDesktopCommand<CreateLocalDocumentResult>(
        transport,
        "create_document",
        command,
      );
    },

    renameDocument(command) {
      return callLocalDesktopCommand<RenameLocalDocumentResult>(
        transport,
        "rename_document",
        command,
      );
    },

    getCurrentDocument(query) {
      return callLocalDesktopCommand<CurrentDocumentView>(
        transport,
        "get_current_document",
        query,
      );
    },

    saveDocumentRevision(command) {
      return callLocalDesktopCommand<SaveDocumentRevisionResult>(
        transport,
        "save_document_revision",
        command,
      );
    },

    saveCurrentDocument(command) {
      return callLocalDesktopCommand<SaveCurrentDocumentResult>(
        transport,
        "update_current_document",
        command,
      );
    },

    listDocumentHistory(query) {
      return callLocalDesktopCommand<DocumentHistoryPage>(
        transport,
        "get_document_history",
        query,
      );
    },

    getDocumentVersion(query) {
      return callLocalDesktopCommand<DocumentVersionView>(
        transport,
        "get_document_version",
        query,
      );
    },

    previewDocumentRestore(query) {
      return callLocalDesktopCommand<RestorePreviewResult>(
        transport,
        "preview_document_restore",
        query,
      );
    },

    restoreDocumentVersion(command) {
      return callLocalDesktopCommand<RestoreDocumentVersionResult>(
        transport,
        "restore_document_version",
        command,
      );
    },

    searchDocuments(query) {
      return callLocalDesktopCommand<SearchResultsPage>(
        transport,
        "search_documents",
        query,
      );
    },

    getLinkOverview(query) {
      return callLocalDesktopCommand<LinkOverviewView>(
        transport,
        "get_link_overview",
        query,
      );
    },

    getKnowledgeGraph(query) {
      return callLocalDesktopCommand<KnowledgeGraphView>(
        transport,
        "get_graph_projection",
        query,
      );
    },

    getAssetMetadata(query) {
      return callLocalDesktopCommand<DocumentAssetsPage>(
        transport,
        "list_document_assets",
        query,
      );
    },
  };
}

async function callLocalDesktopCommand<TData>(
  transport: LocalDesktopCommandTransport,
  commandName: LocalDesktopCommandName,
  payload: Record<string, unknown>,
): Promise<TData> {
  const dispatch = transitionLocalDesktopCommandState(
    LocalDesktopCommandState.Idle,
    LocalDesktopCommandEvent.Dispatch,
  );
  if (dispatch.state !== LocalDesktopCommandState.Dispatching) {
    throw new LocalDesktopCommandClientError(
      dispatch.errorCode ?? "COMMAND_INVALID_TRANSITION",
      false,
    );
  }

  const response = await transport<TData>({ commandName, payload });
  if (response.ok) {
    transitionLocalDesktopCommandState(dispatch.state, LocalDesktopCommandEvent.Resolve);
    return response.data;
  }

  transitionLocalDesktopCommandState(dispatch.state, LocalDesktopCommandEvent.Reject, {
    errorCode: response.errorCode,
    retryable: response.retryable,
  });
  throw new LocalDesktopCommandClientError(
    response.errorCode,
    response.retryable,
    response.repairRequired ?? false,
  );
}

export function createClientCapabilities(runtime: ClientRuntimeKind): ClientCapabilities {
  return {
    runtime,
    supportsLocalWorkspace: runtime === "web-local" || runtime === "desktop-local",
    supportsRemoteWorkspace: runtime === "remote",
  };
}

export function createPersonalLocalDesktopCapabilityProfile(): PersonalLocalDesktopCapabilityProfile {
  return {
    productScope: "personal_local_desktop",
    runtime: "desktop-local",
    supportsLocalWorkspace: true,
    supportsRemoteWorkspace: false,
    platforms: ["windows", "macos", "linux"],
    actions: [
      { id: "open-home", label: "Home" },
      { id: "new-document", label: "New document" },
      { id: "quick-search", label: "Search" },
      { id: "open-graph", label: "Graph" },
      { id: "open-assets", label: "Assets" },
      { id: "ask-ai", label: "Ask AI" },
      { id: "create-backup", label: "Backup" },
      { id: "import-markdown", label: "Import" },
      { id: "export-package", label: "Export" },
      { id: "open-settings", label: "Settings" },
    ],
  };
}

const forbiddenPersonalLocalDesktopActions = new Set<string>([
  "server-url",
  "tenant-admin",
  "organization-admin",
  "team-invite",
  "tenant-settings",
  "sso-settings",
  "server-workspace-connect",
  "billing",
  "admin-console",
]);

export function isForbiddenPersonalLocalDesktopAction(actionId: string): actionId is ForbiddenPersonalLocalDesktopActionId {
  return forbiddenPersonalLocalDesktopActions.has(actionId);
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

export function createKnowledgeGraphQuery(
  workspaceId: string,
  documentId: string,
  options: {
    readonly depth?: 1 | 2;
    readonly direction?: "incoming" | "outgoing" | "both";
    readonly includeUnresolved?: boolean;
    readonly includeAssets?: boolean;
    readonly nodeLimit?: number;
    readonly edgeLimit?: number;
  } = {},
): KnowledgeGraphQuery {
  return {
    queryName: "get-knowledge-graph",
    workspaceId,
    documentId,
    depth: options.depth ?? 1,
    direction: options.direction ?? "both",
    includeUnresolved: options.includeUnresolved ?? true,
    includeAssets: options.includeAssets ?? false,
    nodeLimit: options.nodeLimit ?? 120,
    edgeLimit: options.edgeLimit ?? 240,
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

export function createCollaborationRealtimeClient(
  transport: CollaborationRealtimeTransport,
): CollaborationRealtimeClient {
  return {
    joinDocumentRoom(command) {
      return sendRealtimeRequest(transport, {
        eventName: "join-document-room",
        ...command,
      });
    },
    broadcastOperation(command) {
      return sendRealtimeRequest(transport, {
        eventName: "broadcast-operation",
        ...command,
      });
    },
    broadcastPresence(command) {
      return sendRealtimeRequest(transport, {
        eventName: "broadcast-presence",
        workspaceId: command.workspaceId,
        documentId: command.documentId,
        sessionId: command.sessionId,
        actorUserId: command.actorUserId,
        cursorStart: command.cursorStart,
        cursorEnd: command.cursorEnd,
      });
    },
    requestReplay(command) {
      return sendRealtimeRequest(transport, {
        eventName: "request-replay",
        ...command,
      });
    },
  };
}

async function sendRealtimeRequest(
  transport: CollaborationRealtimeTransport,
  request: CollaborationRealtimeRequest,
): Promise<CollaborationRealtimeAcceptedResponse> {
  const response = await transport(request);
  if (response.status === "rejected") {
    throw new CabinetRealtimeClientError(
      response.errorCode,
      `collaboration realtime request rejected: ${request.eventName}`,
    );
  }
  return response;
}

export type HttpMethodName = "DELETE" | "GET" | "POST" | "PUT";

export interface CabinetHttpRequest {
  readonly method: HttpMethodName;
  readonly url: string;
  readonly headers: Readonly<Record<string, string>>;
  readonly body?: string;
}

export interface CabinetHttpResponse {
  readonly status: number;
  readonly body: string;
  readonly headers?: Readonly<Record<string, string>>;
}

export type CabinetHttpTransport = (request: CabinetHttpRequest) => Promise<CabinetHttpResponse>;

export interface SelfHostApiClientConfigInput {
  readonly baseUrl: string;
  readonly sessionToken?: string;
}

export interface SelfHostApiClientConfig {
  readonly baseUrl: string;
  readonly sessionToken?: string;
}

export type AdminSessionStatus = "created" | "active" | "expired" | "revoked";

export interface LoginCommand {
  readonly login: string;
  readonly credential: string;
}

export interface ValidateSessionQuery {
  readonly token: string;
}

export interface AdminSessionView {
  readonly userId: string;
  readonly token?: string;
  readonly sessionStatus: AdminSessionStatus;
}

export type UserStatusView = "active" | "suspended" | "deleted";

export interface AdminUserView {
  readonly userId: string;
  readonly login: string;
  readonly email: string;
  readonly displayName: string;
  readonly status: UserStatusView;
}

export interface UserPageView {
  readonly users: readonly AdminUserView[];
}

export interface ListGroupsQuery {
  readonly workspaceId: string;
}

export interface AdminGroupView {
  readonly workspaceId: string;
  readonly groupId: string;
  readonly name: string;
  readonly memberUserIds: readonly string[];
}

export interface GroupPageView {
  readonly groups: readonly AdminGroupView[];
}

export interface AddGroupMemberCommand {
  readonly workspaceId: string;
  readonly groupId: string;
  readonly userId: string;
}

export interface RemoveGroupMemberCommand {
  readonly workspaceId: string;
  readonly groupId: string;
  readonly userId: string;
}

export type GroupMemberMutationResult = "added" | "already-member" | "removed";

export interface GroupMemberMutationResultView {
  readonly groupId: string;
  readonly userId: string;
  readonly result: GroupMemberMutationResult;
}

export type WorkspaceRole = "owner" | "admin" | "editor" | "reviewer" | "viewer";

export type RoleAssignmentSubjectKind = "user" | "group";

export interface RoleAssignmentSubjectView {
  readonly kind: RoleAssignmentSubjectKind;
  readonly id: string;
}

export interface RoleAssignmentView {
  readonly assignmentId: string;
  readonly workspaceId: string;
  readonly subject: RoleAssignmentSubjectView;
  readonly role: WorkspaceRole;
}

export interface ListRoleAssignmentsQuery {
  readonly workspaceId: string;
}

export interface RoleAssignmentPageView {
  readonly assignments: readonly RoleAssignmentView[];
}

export interface RoleAssignmentCommand {
  readonly workspaceId: string;
  readonly subject: RoleAssignmentSubjectView;
  readonly role: WorkspaceRole;
}

export interface RevokeRoleCommand {
  readonly workspaceId: string;
  readonly assignmentId: string;
}

export interface RevokeRoleResultView {
  readonly assignmentId: string;
  readonly result: "revoked";
}

export type PermissionDecisionResultView = "allowed" | "denied" | "not_found" | "indeterminate";

export interface PermissionDecisionView {
  readonly result: PermissionDecisionResultView;
  readonly reasonCode: string;
}

export interface AccessibleDocumentView extends CurrentDocumentView {
  readonly permissionDecision: PermissionDecisionView;
}

export interface GetAccessibleDocumentQuery extends DocumentIdentity {}

export interface CollaborationSearchQuery {
  readonly workspaceId: string;
  readonly text: string;
  readonly limit: number;
}

export interface SearchAccessibleDocumentsView extends SearchResultsPage {
  readonly permissionFilteredCount: number;
  readonly durationMs: number;
}

export type CollaborationPermission =
  | "read"
  | "write"
  | "review"
  | "publish"
  | "manage"
  | "read_asset_metadata"
  | "read_asset_content";

export type SharingEffect = "allow" | "deny" | "hide";

export interface SharingSubjectView {
  readonly kind: RoleAssignmentSubjectKind;
  readonly id: string;
}

export interface DocumentSharingEntryView {
  readonly subject: SharingSubjectView;
  readonly permission: CollaborationPermission;
  readonly effect: SharingEffect;
}

export interface GetDocumentSharingQuery extends DocumentIdentity {}

export interface DocumentSharingView extends DocumentIdentity {
  readonly entries: readonly DocumentSharingEntryView[];
  readonly effectivePermissions: readonly CollaborationPermission[];
}

export interface UpdateDocumentSharingCommand extends DocumentIdentity {
  readonly subject: SharingSubjectView;
  readonly permission: CollaborationPermission;
  readonly effect: SharingEffect;
}

export type CommentThreadStateView = "open" | "resolved" | "reopened";
export type InlineAnchorStatusView = "valid" | "stale" | "invalid_range" | "document_version_missing";

export interface InlineCommentAnchorView {
  readonly versionId: string;
  readonly startOffset: number;
  readonly endOffset: number;
  readonly status: InlineAnchorStatusView;
}

export interface CommentView {
  readonly commentId: string;
  readonly authorUserId: string;
  readonly body: string;
  readonly createdAt: string;
}

export interface CommentThreadView {
  readonly threadId: string;
  readonly documentId: string;
  readonly state: CommentThreadStateView;
  readonly comments: readonly CommentView[];
  readonly anchor?: InlineCommentAnchorView;
}

export interface ListDocumentCommentsQuery extends DocumentIdentity {}

export interface CommentThreadPageView {
  readonly threads: readonly CommentThreadView[];
}

export interface AddDocumentCommentCommand extends DocumentIdentity {
  readonly threadId: string;
  readonly commentId: string;
  readonly body: string;
}

export interface AddInlineDocumentCommentCommand extends AddDocumentCommentCommand {
  readonly versionId: string;
  readonly startOffset: number;
  readonly endOffset: number;
}

export interface ResolveDocumentCommentCommand extends DocumentIdentity {
  readonly threadId: string;
}

export interface CommentThreadMutationView {
  readonly thread: CommentThreadView;
  readonly anchorStatus?: InlineAnchorStatusView;
}

export type PublishWorkflowStateView =
  | "editing"
  | "review_requested"
  | "changes_requested"
  | "approved"
  | "published"
  | "rejected";

export type ReviewRequestStatusView =
  | "open"
  | "approved"
  | "rejected"
  | "changes_requested"
  | "published";

export interface ReviewRequestView {
  readonly reviewRequestId: string;
  readonly documentId: string;
  readonly requestedBy: string;
  readonly status: ReviewRequestStatusView;
}

export interface ListReviewRequestsQuery {
  readonly workspaceId: string;
  readonly documentId?: string;
}

export interface ReviewRequestPageView {
  readonly requests: readonly ReviewRequestView[];
}

export interface ReviewRequestCommand {
  readonly workspaceId: string;
  readonly documentId: string;
  readonly reviewRequestId: string;
}

export interface ReviewDecisionCommand {
  readonly workspaceId: string;
  readonly reviewRequestId: string;
}

export interface PublishDocumentCommand extends DocumentIdentity {}

export interface ReviewWorkflowActionView {
  readonly documentId: string;
  readonly reviewRequestId?: string;
  readonly previousState: PublishWorkflowStateView;
  readonly nextState: PublishWorkflowStateView;
}

export type DocumentLockStatusView = "unlocked" | "locked" | "expired";

export interface GetDocumentLockQuery extends DocumentIdentity {}

export interface DocumentLockCommand extends DocumentIdentity {
  readonly lockId?: string;
}

export interface DocumentLockView {
  readonly documentId: string;
  readonly status: DocumentLockStatusView;
  readonly lockId?: string;
  readonly ownerUserId?: string;
  readonly expiresAt?: string;
}

export type AuditListScopeView = "workspace" | "actor" | "target";

export interface ListAuditEventsQuery {
  readonly workspaceId: string;
  readonly scope: AuditListScopeView;
  readonly limit: number;
  readonly cursor?: string;
  readonly actorUserId?: string;
  readonly targetType?: string;
  readonly targetId?: string;
}

export interface AuditMetadataEntryView {
  readonly key: string;
  readonly value: string;
}

export interface AuditEventView {
  readonly eventId: string;
  readonly action: string;
  readonly targetType: string;
  readonly targetId: string;
  readonly actorId: string;
  readonly occurredAt: string;
  readonly metadata: readonly AuditMetadataEntryView[];
}

export interface AuditEventPageView {
  readonly events: readonly AuditEventView[];
  readonly nextCursor?: string;
}

export type MobileReadApiVersion = "phase002.mobile.read.v1";

export interface MobileReadApiEndpointContract {
  readonly method: "GET";
  readonly path: string;
  readonly responseName:
    | "MobileCurrentDocumentResponse"
    | "MobileDocumentHistoryResponse"
    | "MobileSearchResponse"
    | "MobileCommentThreadsResponse"
    | "MobileReviewRequestsResponse";
  readonly requiredFields: readonly string[];
}

export interface MobileReadApiContract {
  readonly version: MobileReadApiVersion;
  readonly endpoints: readonly MobileReadApiEndpointContract[];
}

export interface MobileReadApiValidationResult {
  readonly valid: boolean;
  readonly responseName: MobileReadApiEndpointContract["responseName"];
  readonly missingFields: readonly string[];
}

export interface PlatformCapabilityProfile {
  readonly platform: "web" | "desktop" | "windows" | "macos" | "linux" | "ios" | "android";
  readonly supportsLocalWorkspace: boolean;
  readonly supportsRemoteWorkspace: boolean;
  readonly supportsSelfHostAdminUi: boolean;
  readonly supportsCollaborationUi: boolean;
  readonly supportsMobileReadApi: boolean;
  readonly supportsRemoteEdit: boolean;
  readonly supportsOfflineRemoteEdit: boolean;
  readonly knowledgeGraphSupport: PlatformFeatureSupport;
  readonly canvasSupport: PlatformFeatureSupport;
  readonly realtimeCollaborationSupport: PlatformFeatureSupport;
  readonly supportsCanvasFullEdit: boolean;
  readonly aiQuerySupport: PlatformFeatureSupport;
  readonly aiCitationSupport: PlatformFeatureSupport;
  readonly connectorAdminSupport: PlatformFeatureSupport;
}

export interface PlatformCapabilityMatrix {
  readonly web: PlatformCapabilityProfile;
  readonly desktop: PlatformCapabilityProfile;
  readonly windows: PlatformCapabilityProfile;
  readonly macos: PlatformCapabilityProfile;
  readonly linux: PlatformCapabilityProfile;
  readonly ios: PlatformCapabilityProfile;
  readonly android: PlatformCapabilityProfile;
}

export interface CabinetAdminApiClient {
  login(command: LoginCommand): Promise<AdminSessionView>;
  validateSession(query: ValidateSessionQuery): Promise<AdminSessionView>;
  listUsers(): Promise<UserPageView>;
  listGroups(query: ListGroupsQuery): Promise<GroupPageView>;
  addGroupMember(command: AddGroupMemberCommand): Promise<GroupMemberMutationResultView>;
  removeGroupMember(command: RemoveGroupMemberCommand): Promise<GroupMemberMutationResultView>;
  listRoleAssignments(query: ListRoleAssignmentsQuery): Promise<RoleAssignmentPageView>;
  assignWorkspaceRole(command: RoleAssignmentCommand): Promise<RoleAssignmentView>;
  revokeWorkspaceRole(command: RevokeRoleCommand): Promise<RevokeRoleResultView>;
}

export interface CabinetCollaborationApiClient {
  getAccessibleDocument(query: GetAccessibleDocumentQuery): Promise<AccessibleDocumentView>;
  getKnowledgeGraph(query: KnowledgeGraphQuery): Promise<KnowledgeGraphView>;
  createCanvas(command: CreateCanvasCommand): Promise<CanvasCommandView>;
  addCanvasNode(command: AddCanvasNodeCommand): Promise<CanvasCommandView>;
  embedCanvas(command: EmbedCanvasCommand): Promise<CanvasEmbedView>;
  searchAccessibleDocuments(query: CollaborationSearchQuery): Promise<SearchAccessibleDocumentsView>;
  getDocumentSharing(query: GetDocumentSharingQuery): Promise<DocumentSharingView>;
  updateDocumentSharing(command: UpdateDocumentSharingCommand): Promise<DocumentSharingView>;
  listDocumentComments(query: ListDocumentCommentsQuery): Promise<CommentThreadPageView>;
  addDocumentComment(command: AddDocumentCommentCommand): Promise<CommentThreadMutationView>;
  addInlineDocumentComment(command: AddInlineDocumentCommentCommand): Promise<CommentThreadMutationView>;
  resolveDocumentComment(command: ResolveDocumentCommentCommand): Promise<CommentThreadMutationView>;
  reopenDocumentComment(command: ResolveDocumentCommentCommand): Promise<CommentThreadMutationView>;
  listReviewRequests(query: ListReviewRequestsQuery): Promise<ReviewRequestPageView>;
  requestDocumentReview(command: ReviewRequestCommand): Promise<ReviewWorkflowActionView>;
  approveDocumentReview(command: ReviewDecisionCommand): Promise<ReviewWorkflowActionView>;
  rejectDocumentReview(command: ReviewDecisionCommand): Promise<ReviewWorkflowActionView>;
  publishDocument(command: PublishDocumentCommand): Promise<ReviewWorkflowActionView>;
  getDocumentLock(query: GetDocumentLockQuery): Promise<DocumentLockView>;
  lockDocument(command: DocumentLockCommand): Promise<DocumentLockView>;
  unlockDocument(command: DocumentLockCommand): Promise<DocumentLockView>;
  listAuditEvents(query: ListAuditEventsQuery): Promise<AuditEventPageView>;
}

export interface CabinetAiApiClient {
  searchAiRetrieval(query: AiRetrievalQuery): Promise<AiRetrievalResultPage>;
  askKnowledgeBase(command: AskKnowledgeBaseCommand): Promise<AiAnswerJobView>;
  getAiAnswerStatus(query: AiAnswerJobQuery): Promise<AiAnswerJobView>;
  getAiAnswerResult(query: AiAnswerJobQuery): Promise<AiAnswerResultView>;
}

export type CanvasApiClient = Pick<
  CabinetCollaborationApiClient,
  "createCanvas" | "addCanvasNode" | "embedCanvas"
>;

export type CabinetApiErrorCode =
  | "API_ERROR"
  | "INVALID_CLIENT_CONFIG"
  | "NETWORK_FAILURE"
  | "UNAUTHORIZED"
  | "SESSION_EXPIRED"
  | "VALIDATION_ERROR"
  | string;

export class CabinetApiClientError extends Error {
  readonly code: CabinetApiErrorCode;
  readonly status?: number;

  constructor(code: CabinetApiErrorCode, message: string, status?: number) {
    super(message);
    this.name = "CabinetApiClientError";
    this.code = code;
    this.status = status;
  }
}

export function createSelfHostApiClientConfig(
  input: SelfHostApiClientConfigInput,
): SelfHostApiClientConfig {
  const baseUrl = normalizeBaseUrl(input.baseUrl);
  return input.sessionToken
    ? { baseUrl, sessionToken: input.sessionToken }
    : { baseUrl };
}

export function withSelfHostSessionToken(
  config: SelfHostApiClientConfig,
  sessionToken: string,
): SelfHostApiClientConfig {
  if (!sessionToken.trim()) {
    throw new CabinetApiClientError(
      "INVALID_CLIENT_CONFIG",
      "session token must not be empty",
    );
  }
  return {
    ...config,
    sessionToken,
  };
}

export function createMobileReadApiContract(): MobileReadApiContract {
  return {
    version: "phase002.mobile.read.v1",
    endpoints: [
      {
        method: "GET",
        path: "/api/workspaces/{workspaceId}/documents/{documentId}/current",
        responseName: "MobileCurrentDocumentResponse",
        requiredFields: [
          "workspaceId",
          "documentId",
          "title",
          "path",
          "body",
          "versionId",
          "permissionDecision",
        ],
      },
      {
        method: "GET",
        path: "/api/workspaces/{workspaceId}/documents/{documentId}/history",
        responseName: "MobileDocumentHistoryResponse",
        requiredFields: ["workspaceId", "documentId", "entries"],
      },
      {
        method: "GET",
        path: "/api/workspaces/{workspaceId}/search",
        responseName: "MobileSearchResponse",
        requiredFields: [
          "queryName",
          "workspaceId",
          "text",
          "results",
          "permissionFilteredCount",
          "durationMs",
        ],
      },
      {
        method: "GET",
        path: "/api/documents/{documentId}/comments",
        responseName: "MobileCommentThreadsResponse",
        requiredFields: ["threads"],
      },
      {
        method: "GET",
        path: "/api/review-requests",
        responseName: "MobileReviewRequestsResponse",
        requiredFields: ["requests"],
      },
    ],
  };
}

export function validateMobileReadApiResponse(
  contract: MobileReadApiContract,
  responseName: MobileReadApiEndpointContract["responseName"],
  response: unknown,
): MobileReadApiValidationResult {
  const endpoint = contract.endpoints.find((candidate) => candidate.responseName === responseName);
  if (!endpoint) {
    return {
      valid: false,
      responseName,
      missingFields: ["responseContract"],
    };
  }
  const missingFields = endpoint.requiredFields.filter((field) => !hasOwnField(response, field));
  return {
    valid: missingFields.length === 0,
    responseName,
    missingFields,
  };
}

export function createPlatformCapabilityMatrix(): PlatformCapabilityMatrix {
  const desktopBase = {
    supportsLocalWorkspace: true,
    supportsRemoteWorkspace: true,
    supportsSelfHostAdminUi: false,
    supportsCollaborationUi: true,
    supportsMobileReadApi: false,
    supportsRemoteEdit: true,
    supportsOfflineRemoteEdit: false,
    knowledgeGraphSupport: "interactive" as const,
    canvasSupport: "interactive" as const,
    realtimeCollaborationSupport: "interactive" as const,
    supportsCanvasFullEdit: true,
    aiQuerySupport: "interactive" as const,
    aiCitationSupport: "interactive" as const,
    connectorAdminSupport: "view_only" as const,
  };

  return {
    web: {
      platform: "web",
      supportsLocalWorkspace: false,
      supportsRemoteWorkspace: true,
      supportsSelfHostAdminUi: true,
      supportsCollaborationUi: true,
      supportsMobileReadApi: false,
      supportsRemoteEdit: true,
      supportsOfflineRemoteEdit: false,
      knowledgeGraphSupport: "interactive",
      canvasSupport: "interactive",
      realtimeCollaborationSupport: "interactive",
      supportsCanvasFullEdit: true,
      aiQuerySupport: "interactive",
      aiCitationSupport: "interactive",
      connectorAdminSupport: "interactive",
    },
    desktop: {
      platform: "desktop",
      ...desktopBase,
    },
    windows: {
      platform: "windows",
      ...desktopBase,
    },
    macos: {
      platform: "macos",
      ...desktopBase,
    },
    linux: {
      platform: "linux",
      ...desktopBase,
    },
    ios: {
      platform: "ios",
      supportsLocalWorkspace: false,
      supportsRemoteWorkspace: true,
      supportsSelfHostAdminUi: false,
      supportsCollaborationUi: false,
      supportsMobileReadApi: true,
      supportsRemoteEdit: false,
      supportsOfflineRemoteEdit: false,
      knowledgeGraphSupport: "view_only",
      canvasSupport: "view_only",
      realtimeCollaborationSupport: "unsupported",
      supportsCanvasFullEdit: false,
      aiQuerySupport: "interactive",
      aiCitationSupport: "view_only",
      connectorAdminSupport: "unsupported",
    },
    android: {
      platform: "android",
      supportsLocalWorkspace: false,
      supportsRemoteWorkspace: true,
      supportsSelfHostAdminUi: false,
      supportsCollaborationUi: false,
      supportsMobileReadApi: true,
      supportsRemoteEdit: false,
      supportsOfflineRemoteEdit: false,
      knowledgeGraphSupport: "view_only",
      canvasSupport: "view_only",
      realtimeCollaborationSupport: "unsupported",
      supportsCanvasFullEdit: false,
      aiQuerySupport: "interactive",
      aiCitationSupport: "view_only",
      connectorAdminSupport: "unsupported",
    },
  };
}

export function createFetchHttpTransport(fetchImpl = globalThis.fetch): CabinetHttpTransport {
  if (typeof fetchImpl !== "function") {
    throw new CabinetApiClientError(
      "INVALID_CLIENT_CONFIG",
      "fetch transport is not available",
    );
  }

  return async (request) => {
    const response = await fetchImpl(request.url, {
      method: request.method,
      headers: request.headers,
      body: request.body,
    });
    return {
      status: response.status,
      body: await response.text(),
      headers: headersToRecord(response.headers),
    };
  };
}

export function createSelfHostApiClient(
  config: SelfHostApiClientConfig,
  transport: CabinetHttpTransport = createFetchHttpTransport(),
): CabinetAdminApiClient & CabinetCollaborationApiClient & CabinetAiApiClient {
  return {
    login(command) {
      return requestJson<AdminSessionView>(config, transport, "POST", "/api/auth/login", command);
    },
    validateSession(query) {
      return requestJson<AdminSessionView>(
        config,
        transport,
        "POST",
        "/api/auth/session/validate",
        query,
      );
    },
    listUsers() {
      return requestJson<UserPageView>(config, transport, "GET", "/api/users");
    },
    listGroups(query) {
      return requestJson<GroupPageView>(
        config,
        transport,
        "GET",
        `/api/workspaces/${encodePath(query.workspaceId)}/groups`,
      );
    },
    addGroupMember(command) {
      return requestJson<GroupMemberMutationResultView>(
        config,
        transport,
        "POST",
        `/api/workspaces/${encodePath(command.workspaceId)}/groups/${encodePath(command.groupId)}/members`,
        { userId: command.userId },
      );
    },
    removeGroupMember(command) {
      return requestJson<GroupMemberMutationResultView>(
        config,
        transport,
        "DELETE",
        `/api/workspaces/${encodePath(command.workspaceId)}/groups/${encodePath(command.groupId)}/members/${encodePath(command.userId)}`,
      );
    },
    listRoleAssignments(query) {
      return requestJson<RoleAssignmentPageView>(
        config,
        transport,
        "GET",
        `/api/workspaces/${encodePath(query.workspaceId)}/roles`,
      );
    },
    assignWorkspaceRole(command) {
      return requestJson<RoleAssignmentView>(
        config,
        transport,
        "POST",
        `/api/workspaces/${encodePath(command.workspaceId)}/roles`,
        { subject: command.subject, role: command.role },
      );
    },
    revokeWorkspaceRole(command) {
      return requestJson<RevokeRoleResultView>(
        config,
        transport,
        "DELETE",
        `/api/workspaces/${encodePath(command.workspaceId)}/roles/${encodePath(command.assignmentId)}`,
      );
    },
    getAccessibleDocument(query) {
      return requestJson<AccessibleDocumentView>(
        config,
        transport,
        "GET",
        `/api/workspaces/${encodePath(query.workspaceId)}/documents/${encodePath(query.documentId)}/current`,
      );
    },
    getKnowledgeGraph(query) {
      return requestJson<KnowledgeGraphView>(
        config,
        transport,
        "GET",
        `/api/workspaces/${encodePath(query.workspaceId)}/documents/${encodePath(query.documentId)}/graph`,
      );
    },
    createCanvas(command) {
      return requestJson<CanvasCommandView>(
        config,
        transport,
        "POST",
        `/api/workspaces/${encodePath(command.workspaceId)}/canvases`,
        { canvasId: command.canvasId },
      );
    },
    addCanvasNode(command) {
      return requestJson<CanvasCommandView>(
        config,
        transport,
        "POST",
        `/api/workspaces/${encodePath(command.workspaceId)}/canvases/${encodePath(command.canvasId)}/nodes`,
        {
          nodeId: command.nodeId,
          target: command.target,
          x: command.x,
          y: command.y,
        },
      );
    },
    embedCanvas(command) {
      return requestJson<CanvasEmbedView>(
        config,
        transport,
        "POST",
        `/api/workspaces/${encodePath(command.workspaceId)}/documents/${encodePath(command.documentId)}/canvas-embeds`,
        { canvasId: command.canvasId },
      );
    },
    searchAccessibleDocuments(query) {
      return requestJson<SearchAccessibleDocumentsView>(
        config,
        transport,
        "GET",
        `/api/workspaces/${encodePath(query.workspaceId)}/search${queryString({
          text: query.text,
          limit: String(query.limit),
        })}`,
      );
    },
    getDocumentSharing(query) {
      return requestJson<DocumentSharingView>(
        config,
        transport,
        "GET",
        `/api/documents/${encodePath(query.documentId)}/sharing${queryString({
          workspaceId: query.workspaceId,
        })}`,
      );
    },
    updateDocumentSharing(command) {
      return requestJson<DocumentSharingView>(
        config,
        transport,
        "PUT",
        `/api/documents/${encodePath(command.documentId)}/sharing`,
        {
          workspaceId: command.workspaceId,
          subject: command.subject,
          permission: command.permission,
          effect: command.effect,
        },
      );
    },
    listDocumentComments(query) {
      return requestJson<CommentThreadPageView>(
        config,
        transport,
        "GET",
        `/api/documents/${encodePath(query.documentId)}/comments${queryString({
          workspaceId: query.workspaceId,
        })}`,
      );
    },
    addDocumentComment(command) {
      return requestJson<CommentThreadMutationView>(
        config,
        transport,
        "POST",
        `/api/documents/${encodePath(command.documentId)}/comments`,
        {
          workspaceId: command.workspaceId,
          threadId: command.threadId,
          commentId: command.commentId,
          body: command.body,
        },
      );
    },
    addInlineDocumentComment(command) {
      return requestJson<CommentThreadMutationView>(
        config,
        transport,
        "POST",
        `/api/documents/${encodePath(command.documentId)}/inline-comments`,
        {
          workspaceId: command.workspaceId,
          versionId: command.versionId,
          startOffset: command.startOffset,
          endOffset: command.endOffset,
          threadId: command.threadId,
          commentId: command.commentId,
          body: command.body,
        },
      );
    },
    resolveDocumentComment(command) {
      return requestJson<CommentThreadMutationView>(
        config,
        transport,
        "POST",
        `/api/comments/${encodePath(command.threadId)}/resolve`,
        {
          workspaceId: command.workspaceId,
          documentId: command.documentId,
        },
      );
    },
    reopenDocumentComment(command) {
      return requestJson<CommentThreadMutationView>(
        config,
        transport,
        "POST",
        `/api/comments/${encodePath(command.threadId)}/reopen`,
        {
          workspaceId: command.workspaceId,
          documentId: command.documentId,
        },
      );
    },
    listReviewRequests(query) {
      return requestJson<ReviewRequestPageView>(
        config,
        transport,
        "GET",
        `/api/review-requests${queryString({
          workspaceId: query.workspaceId,
          documentId: query.documentId,
        })}`,
      );
    },
    requestDocumentReview(command) {
      return requestJson<ReviewWorkflowActionView>(
        config,
        transport,
        "POST",
        `/api/documents/${encodePath(command.documentId)}/review-requests`,
        {
          workspaceId: command.workspaceId,
          reviewRequestId: command.reviewRequestId,
        },
      );
    },
    approveDocumentReview(command) {
      return requestJson<ReviewWorkflowActionView>(
        config,
        transport,
        "POST",
        `/api/review-requests/${encodePath(command.reviewRequestId)}/approve`,
        { workspaceId: command.workspaceId },
      );
    },
    rejectDocumentReview(command) {
      return requestJson<ReviewWorkflowActionView>(
        config,
        transport,
        "POST",
        `/api/review-requests/${encodePath(command.reviewRequestId)}/reject`,
        { workspaceId: command.workspaceId },
      );
    },
    publishDocument(command) {
      return requestJson<ReviewWorkflowActionView>(
        config,
        transport,
        "POST",
        `/api/documents/${encodePath(command.documentId)}/publish`,
        { workspaceId: command.workspaceId },
      );
    },
    getDocumentLock(query) {
      return requestJson<DocumentLockView>(
        config,
        transport,
        "GET",
        `/api/documents/${encodePath(query.documentId)}/locks/current${queryString({
          workspaceId: query.workspaceId,
        })}`,
      );
    },
    lockDocument(command) {
      return requestJson<DocumentLockView>(
        config,
        transport,
        "POST",
        `/api/documents/${encodePath(command.documentId)}/locks`,
        {
          workspaceId: command.workspaceId,
          lockId: command.lockId,
        },
      );
    },
    unlockDocument(command) {
      return requestJson<DocumentLockView>(
        config,
        transport,
        "DELETE",
        `/api/documents/${encodePath(command.documentId)}/locks/current${queryString({
          workspaceId: command.workspaceId,
        })}`,
      );
    },
    listAuditEvents(query) {
      return requestJson<AuditEventPageView>(
        config,
        transport,
        "GET",
        `/api/audit-events${queryString({
          workspaceId: query.workspaceId,
          scope: query.scope,
          limit: String(query.limit),
          cursor: query.cursor,
          actorUserId: query.actorUserId,
          targetType: query.targetType,
          targetId: query.targetId,
        })}`,
      );
    },
    searchAiRetrieval(query) {
      return requestJson<AiRetrievalResultPage>(
        config,
        transport,
        "GET",
        `/api/workspaces/${encodePath(query.workspaceId)}/ai/retrieval${queryString({
          text: query.text,
          limit: String(query.limit),
        })}`,
      );
    },
    askKnowledgeBase(command) {
      return requestJson<AiAnswerJobView>(
        config,
        transport,
        "POST",
        `/api/workspaces/${encodePath(command.workspaceId)}/ai/answers`,
        {
          question: command.question,
          retrievalLimit: command.retrievalLimit,
        },
      );
    },
    getAiAnswerStatus(query) {
      return requestJson<AiAnswerJobView>(
        config,
        transport,
        "GET",
        `/api/ai/answers/${encodePath(query.jobId)}/status${queryString({
          workspaceId: query.workspaceId,
        })}`,
      );
    },
    getAiAnswerResult(query) {
      return requestJson<AiAnswerResultView>(
        config,
        transport,
        "GET",
        `/api/ai/answers/${encodePath(query.jobId)}/result${queryString({
          workspaceId: query.workspaceId,
        })}`,
      );
    },
  };
}

function normalizeBaseUrl(baseUrl: string): string {
  const trimmed = baseUrl.trim();
  if (!trimmed) {
    throw new CabinetApiClientError(
      "INVALID_CLIENT_CONFIG",
      "self-host API base URL must not be empty",
    );
  }

  try {
    const parsed = new URL(trimmed);
    if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
      throw new Error("unsupported protocol");
    }
  } catch {
    throw new CabinetApiClientError(
      "INVALID_CLIENT_CONFIG",
      "self-host API base URL must be an absolute HTTP URL",
    );
  }

  return trimmed.replace(/\/+$/, "");
}

async function requestJson<T>(
  config: SelfHostApiClientConfig,
  transport: CabinetHttpTransport,
  method: HttpMethodName,
  path: string,
  body?: unknown,
): Promise<T> {
  let response: CabinetHttpResponse;
  try {
    response = await transport({
      method,
      url: `${config.baseUrl}${path}`,
      headers: requestHeaders(config, body !== undefined),
      body: body === undefined ? undefined : JSON.stringify(body),
    });
  } catch (error) {
    if (error instanceof CabinetApiClientError) {
      throw error;
    }
    throw new CabinetApiClientError("NETWORK_FAILURE", "network request failed");
  }

  if (response.status >= 200 && response.status < 300) {
    return parseJsonResponse<T>(response.body, response.status);
  }

  const errorBody = parseUnknownJson(response.body);
  const errorCode = mapApiErrorCode(response.status, errorBody);
  throw new CabinetApiClientError(errorCode, mapApiErrorMessage(errorCode, errorBody), response.status);
}

function requestHeaders(
  config: SelfHostApiClientConfig,
  includesJsonBody: boolean,
): Record<string, string> {
  const headers: Record<string, string> = {
    accept: "application/json",
  };
  if (includesJsonBody) {
    headers["content-type"] = "application/json";
  }
  if (config.sessionToken) {
    headers.authorization = `Bearer ${config.sessionToken}`;
  }
  return headers;
}

function parseJsonResponse<T>(body: string, status: number): T {
  if (!body.trim()) {
    return {} as T;
  }
  try {
    return JSON.parse(body) as T;
  } catch {
    throw new CabinetApiClientError("API_ERROR", "server returned invalid JSON", status);
  }
}

function parseUnknownJson(body: string): unknown {
  if (!body.trim()) {
    return undefined;
  }
  try {
    return JSON.parse(body);
  } catch {
    return undefined;
  }
}

function mapApiErrorCode(status: number, body: unknown): CabinetApiErrorCode {
  if (isRecord(body)) {
    const code = body.errorCode ?? body.code;
    if (typeof code === "string" && code.trim()) {
      return code;
    }
  }
  if (status === 401 || status === 403) {
    return "UNAUTHORIZED";
  }
  if (status === 400 || status === 422) {
    return "VALIDATION_ERROR";
  }
  return "API_ERROR";
}

function mapApiErrorMessage(code: CabinetApiErrorCode, body: unknown): string {
  if (isRecord(body) && typeof body.message === "string" && body.message.trim()) {
    return body.message;
  }
  return code;
}

function headersToRecord(headers: Headers): Record<string, string> {
  const record: Record<string, string> = {};
  headers.forEach((value, key) => {
    record[key.toLowerCase()] = value;
  });
  return record;
}

function encodePath(value: string): string {
  return encodeURIComponent(value);
}

function queryString(params: Readonly<Record<string, string | undefined>>): string {
  const search = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined) {
      search.set(key, value);
    }
  }
  const serialized = search.toString();
  return serialized ? `?${serialized}` : "";
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function hasOwnField(value: unknown, field: string): boolean {
  return isRecord(value) && Object.prototype.hasOwnProperty.call(value, field);
}
