import type {
  ClientCapabilities,
  CurrentDocumentView,
  CurrentDocumentQuery,
  DocumentHistoryEntry,
  DocumentHistoryPage,
  AssetView,
  AttachAssetCommand,
  BacklinkView,
  CanvasView,
  DocumentAssetsPage,
  KnowledgeGraphView,
  LinkOverviewView,
  OrphanDocumentView,
  SearchResultView,
  SearchResultsPage,
  SelectedAssetDraft,
  UnresolvedLinkView,
  AddGroupMemberCommand,
  AccessibleDocumentView,
  AdminGroupView,
  AdminSessionView,
  AdminUserView,
  AiAnswerJobView,
  AiAnswerResultView,
  AiFreshnessStatusView,
  AiPermissionDecisionSummaryView,
  AiProviderSettingsStateView,
  AiProviderSettingsSummaryView,
  BackupArtifactManifestSummaryView,
  AiRetrievalResultPage,
  AiRetrievalSourceKindView,
  AuditEventView,
  CabinetAdminApiClient,
  CabinetApiErrorCode,
  CabinetCollaborationApiClient,
  CollaborationPermission,
  CommentThreadView,
  DocumentLockView,
  DocumentSharingView,
  GroupMemberMutationResultView,
  InlineAnchorStatusView,
  ImportConflictItemView,
  ImportPreviewStateView,
  ImportPreviewSummaryView,
  LoginCommand,
  PermissionDecisionView,
  RemoveGroupMemberCommand,
  RevokeRoleResultView,
  RoleAssignmentSubjectView,
  RoleAssignmentView,
  RestoreStagingIssueView,
  RestoreStagingStateView,
  ReviewRequestView,
  ReviewWorkflowActionView,
  SearchAccessibleDocumentsView,
  SharingEffect,
  SharingSubjectView,
  WorkspaceRole,
  PersonalLocalDesktopAction,
  PersonalLocalDesktopActionId,
  PersonalLocalDesktopCapabilityProfile,
  LocalAiToolDescriptorView,
  LocalAiToolOperationView,
  WorkspaceHomeResult,
  DocumentNavigatorItem,
  DocumentNavigatorQuery,
  DocumentNavigatorResult,
  DocumentNavigatorView,
} from "@sponzey-cabinet/client-core";
import { createCurrentDocumentQuery } from "@sponzey-cabinet/client-core";

export type {
  AiProviderSettingsStateView,
  AiProviderSettingsSummaryView,
  BackupArtifactManifestSummaryView,
  ImportConflictItemView,
  ImportPreviewStateView,
  ImportPreviewSummaryView,
  LocalAiToolDescriptorView,
  LocalAiToolOperationView,
  RestoreStagingIssueView,
  RestoreStagingStateView,
} from "@sponzey-cabinet/client-core";

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

export type PersonalWorkspaceHealthState =
  | "Loading"
  | "Ready"
  | "NeedsRepair"
  | "ReadOnlyRecovery"
  | "Failed";

export type PersonalWorkspaceHealthDisplayState =
  | "loading"
  | "ready"
  | "needs-repair"
  | "read-only-recovery"
  | "failed";

export type PersonalWorkspaceNavigationId =
  | "home"
  | "documents"
  | "search"
  | "graph"
  | "assets"
  | "ai"
  | "backup"
  | "settings";

export type WorkspaceHealthActionId = PersonalLocalDesktopActionId | "repair-local-workspace" | "open-recovery";

export interface PersonalWorkspaceNavigationItem {
  readonly id: PersonalWorkspaceNavigationId;
  readonly label: string;
}

export interface WorkspaceHealthAction {
  readonly id: WorkspaceHealthActionId;
  readonly label: string;
}

export interface WorkspaceHealthActionModel {
  readonly displayState: PersonalWorkspaceHealthDisplayState;
  readonly actions: readonly WorkspaceHealthAction[];
}

export interface PersonalWorkspaceShellModel {
  readonly mode: "personal-workspace-shell";
  readonly productScope: "personal_local_desktop";
  readonly runtime: "desktop-local";
  readonly firstRoute: "home";
  readonly navigationItems: readonly PersonalWorkspaceNavigationItem[];
  readonly commandActions: readonly WorkspaceHealthAction[];
  readonly health: WorkspaceHealthActionModel;
}

export interface PersonalWorkspaceShellInput {
  readonly profile: PersonalLocalDesktopCapabilityProfile;
  readonly healthState: PersonalWorkspaceHealthState;
}

export function createPersonalWorkspaceShellModel(
  input: PersonalWorkspaceShellInput,
): PersonalWorkspaceShellModel {
  const health = createWorkspaceHealthActionModel(input.healthState, input.profile.actions);
  return {
    mode: "personal-workspace-shell",
    productScope: input.profile.productScope,
    runtime: input.profile.runtime,
    firstRoute: "home",
    navigationItems: [
      { id: "home", label: "Home" },
      { id: "documents", label: "Documents" },
      { id: "search", label: "Search" },
      { id: "graph", label: "Graph" },
      { id: "assets", label: "Assets" },
      { id: "ai", label: "AI" },
      { id: "backup", label: "Backup" },
      { id: "settings", label: "Settings" },
    ],
    commandActions: createPersonalWorkspaceCommandPaletteActions(input.profile.actions),
    health,
  };
}

export type PersonalWorkspaceCommandPaletteActionId =
  | "new-document"
  | "quick-search"
  | "open-graph"
  | "ask-ai"
  | "create-backup"
  | "import-markdown"
  | "export-package"
  | "open-settings";

const personalWorkspaceCommandPaletteActionOrder: readonly PersonalWorkspaceCommandPaletteActionId[] = [
  "new-document",
  "quick-search",
  "open-graph",
  "ask-ai",
  "create-backup",
  "import-markdown",
  "export-package",
  "open-settings",
];

export function createPersonalWorkspaceCommandPaletteActions(
  actions: readonly PersonalLocalDesktopAction[] = defaultPersonalWorkspaceActions(),
): readonly PersonalLocalDesktopAction[] {
  return personalWorkspaceCommandPaletteActionOrder
    .map((id) => actions.find((action) => action.id === id))
    .filter((action): action is PersonalLocalDesktopAction => action !== undefined);
}

export type PersonalWorkspaceHomeSectionId =
  | "recent-documents"
  | "favorites"
  | "tags"
  | "recent-changes"
  | "unfinished-items"
  | "quick-search"
  | "ai-entry"
  | "backup-status"
  | "workspace-health";

export type PersonalWorkspaceBackupState = "NeverCreated" | "Fresh" | "Stale" | "Failed";

export interface PersonalWorkspaceHomeSummary {
  readonly recentDocumentCount?: number;
  readonly favoriteCount?: number;
  readonly tagCount?: number;
  readonly recentChangeCount?: number;
  readonly unfinishedItemCount?: number;
  readonly backupState?: PersonalWorkspaceBackupState;
}

export interface PersonalWorkspaceHomeSection {
  readonly id: PersonalWorkspaceHomeSectionId;
  readonly label: string;
  readonly visible: true;
  readonly itemCount?: number;
  readonly emptyState?: string;
  readonly primaryActionId?: WorkspaceHealthActionId;
  readonly status?: PersonalWorkspaceBackupState | PersonalWorkspaceHealthDisplayState;
}

export interface PersonalWorkspaceHomeModel {
  readonly mode: "personal-workspace-home";
  readonly productScope: "personal_local_desktop";
  readonly runtime: "desktop-local";
  readonly firstRoute: "home";
  readonly displayState: "Loading" | "Ready" | "Empty" | "Degraded" | "Failed";
  readonly workspaceId?: string;
  readonly recentDocuments: readonly PersonalWorkspaceHomeDocumentItem[];
  readonly favorites: readonly PersonalWorkspaceHomeDocumentItem[];
  readonly tags: readonly PersonalWorkspaceHomeTagItem[];
  readonly recentChanges: readonly PersonalWorkspaceHomeChangeItem[];
  readonly unfinishedItems: readonly PersonalWorkspaceHomeUnfinishedItem[];
  readonly error?: PersonalWorkspaceHomeError;
  readonly sections: readonly PersonalWorkspaceHomeSection[];
  readonly commandActions: readonly WorkspaceHealthAction[];
  readonly health: WorkspaceHealthActionModel;
  readonly productLogEventNames: readonly ("workspace.home.ready" | "workspace.health.failed")[];
}

export interface PersonalWorkspaceHomeDocumentItem {
  readonly documentId: string;
  readonly title: string;
  readonly path: string;
  readonly actionId: "open-document";
}

export interface PersonalWorkspaceHomeTagItem {
  readonly label: string;
  readonly documentCount: number;
  readonly actionId: "filter-by-tag";
}

export interface PersonalWorkspaceHomeChangeItem {
  readonly documentId: string;
  readonly summary: string;
  readonly actionId: "open-document";
}

export interface PersonalWorkspaceHomeUnfinishedItem {
  readonly documentId: string;
  readonly label: string;
  readonly actionId: "open-document";
}

export interface PersonalWorkspaceHomeError {
  readonly code: string;
  readonly retryable: boolean;
  readonly actionId: "retry-workspace-home";
}

export interface PersonalWorkspaceHomeInput {
  readonly profile: PersonalLocalDesktopCapabilityProfile;
  readonly healthState: PersonalWorkspaceHealthState;
  readonly summary?: PersonalWorkspaceHomeSummary;
}

export function createPersonalWorkspaceHomeModel(
  input: PersonalWorkspaceHomeInput,
): PersonalWorkspaceHomeModel {
  const health = createWorkspaceHealthActionModel(input.healthState, input.profile.actions);
  const summary = input.summary ?? {};
  const recentDocumentCount = summary.recentDocumentCount ?? 0;
  const favoriteCount = summary.favoriteCount ?? 0;
  const tagCount = summary.tagCount ?? 0;
  const recentChangeCount = summary.recentChangeCount ?? 0;
  const unfinishedItemCount = summary.unfinishedItemCount ?? 0;
  const backupState = summary.backupState ?? "NeverCreated";

  return {
    mode: "personal-workspace-home",
    productScope: input.profile.productScope,
    runtime: input.profile.runtime,
    firstRoute: "home",
    displayState:
      input.healthState === "Loading"
        ? "Loading"
        : input.healthState === "Failed"
          ? "Failed"
          : recentDocumentCount + favoriteCount + tagCount + recentChangeCount + unfinishedItemCount === 0
            ? "Empty"
            : "Ready",
    recentDocuments: [],
    favorites: [],
    tags: [],
    recentChanges: [],
    unfinishedItems: [],
    commandActions: createPersonalWorkspaceCommandPaletteActions(input.profile.actions),
    health,
    productLogEventNames:
      input.healthState === "Failed" ? ["workspace.health.failed"] : ["workspace.home.ready"],
    sections: [
      {
        id: "recent-documents",
        label: "Recent documents",
        visible: true,
        itemCount: recentDocumentCount,
        emptyState: recentDocumentCount === 0 ? "NoRecentDocuments" : undefined,
        primaryActionId: "new-document",
      },
      {
        id: "favorites",
        label: "Favorites",
        visible: true,
        itemCount: favoriteCount,
        emptyState: favoriteCount === 0 ? "NoFavorites" : undefined,
      },
      {
        id: "tags",
        label: "Tags",
        visible: true,
        itemCount: tagCount,
        emptyState: tagCount === 0 ? "NoTags" : undefined,
      },
      {
        id: "recent-changes",
        label: "Recent changes",
        visible: true,
        itemCount: recentChangeCount,
        emptyState: recentChangeCount === 0 ? "NoRecentChanges" : undefined,
      },
      {
        id: "unfinished-items",
        label: "Unfinished",
        visible: true,
        itemCount: unfinishedItemCount,
        emptyState: unfinishedItemCount === 0 ? "NoUnfinishedItems" : undefined,
      },
      {
        id: "quick-search",
        label: "Quick search",
        visible: true,
        primaryActionId: "quick-search",
      },
      {
        id: "ai-entry",
        label: "Ask AI",
        visible: true,
        primaryActionId: "ask-ai",
      },
      {
        id: "backup-status",
        label: "Backup status",
        visible: true,
        status: backupState,
        primaryActionId: backupState === "Failed" ? "open-settings" : "create-backup",
      },
      {
        id: "workspace-health",
        label: "Workspace health",
        visible: true,
        status: health.displayState,
        primaryActionId: health.actions[0]?.id,
      },
    ],
  };
}

export function createPersonalWorkspaceHomeModelFromResult(
  profile: PersonalLocalDesktopCapabilityProfile,
  result: WorkspaceHomeResult,
): PersonalWorkspaceHomeModel {
  const healthState: PersonalWorkspaceHealthState =
    result.healthStatus === "ReadOnlyRecovery"
      ? "ReadOnlyRecovery"
      : result.healthStatus === "Degraded"
        ? "NeedsRepair"
        : "Ready";
  const model = createPersonalWorkspaceHomeModel({
    profile,
    healthState,
    summary: {
      recentDocumentCount: result.recentDocuments.length,
      favoriteCount: result.favorites.length,
      tagCount: result.tags.length,
      recentChangeCount: result.recentChanges.length,
      unfinishedItemCount: result.unfinishedItems.length,
      backupState: result.backupStatus,
    },
  });

  return {
    ...model,
    workspaceId: result.workspaceId,
    displayState: result.state,
    recentDocuments: result.recentDocuments.map((item) => ({
      ...item,
      actionId: "open-document" as const,
    })),
    favorites: result.favorites.map((item) => ({
      ...item,
      actionId: "open-document" as const,
    })),
    tags: result.tags.map((item) => ({
      ...item,
      actionId: "filter-by-tag" as const,
    })),
    recentChanges: result.recentChanges.map((item) => ({
      ...item,
      actionId: "open-document" as const,
    })),
    unfinishedItems: result.unfinishedItems.map((item) => ({
      ...item,
      actionId: "open-document" as const,
    })),
  };
}

export function createPersonalWorkspaceHomeFailedModel(
  profile: PersonalLocalDesktopCapabilityProfile,
  errorCode: string,
  retryable: boolean,
): PersonalWorkspaceHomeModel {
  return {
    ...createPersonalWorkspaceHomeModel({ profile, healthState: "Failed" }),
    displayState: "Failed",
    error: {
      code: errorCode,
      retryable,
      actionId: "retry-workspace-home",
    },
  };
}

export const PersonalWorkspaceAppFrameState = Object.freeze({
  Booting: "Booting",
  LoadingWorkspace: "LoadingWorkspace",
  HomeReady: "HomeReady",
  NeedsRepair: "NeedsRepair",
  ReadOnlyRecovery: "ReadOnlyRecovery",
  Failed: "Failed",
});

export type PersonalWorkspaceAppFrameStateValue =
  (typeof PersonalWorkspaceAppFrameState)[keyof typeof PersonalWorkspaceAppFrameState];

export const PersonalWorkspaceAppFrameEvent = Object.freeze({
  StartLoading: "StartLoading",
  WorkspaceLoaded: "WorkspaceLoaded",
  RepairRequired: "RepairRequired",
  ReadOnlyRecoveryOpened: "ReadOnlyRecoveryOpened",
  Fail: "Fail",
  RecoveryCompleted: "RecoveryCompleted",
});

export type PersonalWorkspaceAppFrameEventValue =
  (typeof PersonalWorkspaceAppFrameEvent)[keyof typeof PersonalWorkspaceAppFrameEvent];

export const PersonalWorkspaceAppFrameErrorCode = Object.freeze({
  InvalidTransition: "PERSONAL_WORKSPACE_APP_FRAME_INVALID_TRANSITION",
});

export type PersonalWorkspaceAppFrameErrorCodeValue =
  (typeof PersonalWorkspaceAppFrameErrorCode)[keyof typeof PersonalWorkspaceAppFrameErrorCode];

export interface PersonalWorkspaceAppFrameTransitionResult {
  readonly state: PersonalWorkspaceAppFrameStateValue;
  readonly errorCode?: PersonalWorkspaceAppFrameErrorCodeValue;
}

export function transitionPersonalWorkspaceAppFrameState(
  state: PersonalWorkspaceAppFrameStateValue,
  event: PersonalWorkspaceAppFrameEventValue,
): PersonalWorkspaceAppFrameTransitionResult {
  if (state === PersonalWorkspaceAppFrameState.Booting && event === PersonalWorkspaceAppFrameEvent.StartLoading) {
    return { state: PersonalWorkspaceAppFrameState.LoadingWorkspace };
  }
  if (
    state === PersonalWorkspaceAppFrameState.LoadingWorkspace &&
    event === PersonalWorkspaceAppFrameEvent.WorkspaceLoaded
  ) {
    return { state: PersonalWorkspaceAppFrameState.HomeReady };
  }
  if (
    [PersonalWorkspaceAppFrameState.LoadingWorkspace, PersonalWorkspaceAppFrameState.HomeReady].includes(state) &&
    event === PersonalWorkspaceAppFrameEvent.RepairRequired
  ) {
    return { state: PersonalWorkspaceAppFrameState.NeedsRepair };
  }
  if (
    [PersonalWorkspaceAppFrameState.LoadingWorkspace, PersonalWorkspaceAppFrameState.NeedsRepair].includes(state) &&
    event === PersonalWorkspaceAppFrameEvent.ReadOnlyRecoveryOpened
  ) {
    return { state: PersonalWorkspaceAppFrameState.ReadOnlyRecovery };
  }
  if (
    state === PersonalWorkspaceAppFrameState.Failed &&
    event === PersonalWorkspaceAppFrameEvent.RecoveryCompleted
  ) {
    return { state: PersonalWorkspaceAppFrameState.HomeReady };
  }
  if (event === PersonalWorkspaceAppFrameEvent.Fail && state !== PersonalWorkspaceAppFrameState.Failed) {
    return { state: PersonalWorkspaceAppFrameState.Failed };
  }
  return {
    state,
    errorCode: PersonalWorkspaceAppFrameErrorCode.InvalidTransition,
  };
}

export function createWorkspaceHealthActionModel(
  state: PersonalWorkspaceHealthState,
  baseActions: readonly PersonalLocalDesktopAction[] = defaultPersonalWorkspaceActions(),
): WorkspaceHealthActionModel {
  if (state === "Loading") {
    return {
      displayState: "loading",
      actions: [],
    };
  }
  if (state === "NeedsRepair") {
    return {
      displayState: "needs-repair",
      actions: [
        { id: "repair-local-workspace", label: "Repair workspace" },
        { id: "open-recovery", label: "Open recovery" },
        ...selectActions(baseActions, ["create-backup", "export-package", "open-settings"]),
      ],
    };
  }
  if (state === "ReadOnlyRecovery") {
    return {
      displayState: "read-only-recovery",
      actions: [
        ...selectActions(baseActions, ["create-backup", "export-package"]),
        { id: "open-recovery", label: "Open recovery" },
      ],
    };
  }
  if (state === "Failed") {
    return {
      displayState: "failed",
      actions: [{ id: "open-recovery", label: "Open recovery" }],
    };
  }
  return {
    displayState: "ready",
    actions: baseActions,
  };
}

function selectActions(
  actions: readonly PersonalLocalDesktopAction[],
  ids: readonly PersonalLocalDesktopActionId[],
): readonly PersonalLocalDesktopAction[] {
  return ids
    .map((id) => actions.find((action) => action.id === id))
    .filter((action): action is PersonalLocalDesktopAction => action !== undefined);
}

function defaultPersonalWorkspaceActions(): readonly PersonalLocalDesktopAction[] {
  return [
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
  ];
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

export type MarkdownPreviewState =
  | "NotRendered"
  | "Rendering"
  | "Rendered"
  | "PartiallyRendered"
  | "RenderFailed";

export type MarkdownTableAlignment = "left" | "center" | "right" | "default";

export interface MarkdownPreviewInput {
  readonly documentId: string;
  readonly versionId: string;
  readonly source: string;
  readonly resolvedWikilinkTargets?: readonly string[];
  readonly availableAssetIds?: readonly string[];
}

export interface MarkdownSourceRange {
  readonly start: number;
  readonly end: number;
}

export interface MarkdownPreviewInlineAction {
  readonly kind: "open-wikilink" | "open-asset-reference";
  readonly target?: string;
  readonly assetId?: string;
  readonly label: string;
  readonly sourceRange: MarkdownSourceRange;
  readonly resolutionState?: "resolved" | "unresolved";
  readonly assetState?: "available" | "missing";
}

export interface MarkdownHeadingPreviewBlock {
  readonly kind: "heading";
  readonly level: number;
  readonly text: string;
  readonly anchor: string;
}

export interface MarkdownParagraphPreviewBlock {
  readonly kind: "paragraph";
  readonly text: string;
  readonly inlineActions: readonly MarkdownPreviewInlineAction[];
}

export interface MarkdownTablePreviewBlock {
  readonly kind: "table";
  readonly headers: readonly string[];
  readonly alignments: readonly MarkdownTableAlignment[];
  readonly rows: readonly (readonly string[])[];
}

export interface MarkdownChecklistPreviewBlock {
  readonly kind: "checklist";
  readonly items: readonly {
    readonly checked: boolean;
    readonly text: string;
  }[];
}

export interface MarkdownCodePreviewBlock {
  readonly kind: "code";
  readonly language?: string;
  readonly lineCount: number;
}

export interface MarkdownBlockquotePreviewBlock {
  readonly kind: "blockquote";
  readonly text: string;
}

export interface MarkdownCalloutPreviewBlock {
  readonly kind: "callout";
  readonly calloutType: string;
  readonly title: string;
  readonly text: string;
}

export type MarkdownPreviewBlock =
  | MarkdownHeadingPreviewBlock
  | MarkdownParagraphPreviewBlock
  | MarkdownTablePreviewBlock
  | MarkdownChecklistPreviewBlock
  | MarkdownCodePreviewBlock
  | MarkdownBlockquotePreviewBlock
  | MarkdownCalloutPreviewBlock;

export interface MarkdownPreviewModel {
  readonly mode: "markdown-preview";
  readonly sourceMode: "markdown-source";
  readonly documentId: string;
  readonly sourceVersionId: string;
  readonly state: MarkdownPreviewState;
  readonly blocks: readonly MarkdownPreviewBlock[];
}

export interface DocumentReadQuerySeparationSummary {
  readonly currentReadQueryName: "get-current-document";
  readonly historyReadQueryName: "get-document-history";
}

export interface DocumentReadingWorkspaceModel {
  readonly mode: "document-reading-workspace";
  readonly current: CurrentDocumentViewModel;
  readonly preview: MarkdownPreviewModel;
  readonly history: HistoryPanelViewModel;
  readonly querySeparation: DocumentReadQuerySeparationSummary;
}

export type DocumentEditorViewMode = "source" | "preview" | "split";

export const DocumentEditorViewModeEvent = Object.freeze({
  ShowSource: "ShowSource",
  ShowPreview: "ShowPreview",
  ShowSplit: "ShowSplit",
});

export type DocumentEditorViewModeEventValue =
  (typeof DocumentEditorViewModeEvent)[keyof typeof DocumentEditorViewModeEvent];

export interface DocumentEditorViewModeTransition {
  readonly mode: DocumentEditorViewMode;
}

export interface DocumentAuthoringWorkspaceModel {
  readonly mode: "document-authoring-workspace";
  readonly viewMode: DocumentEditorViewMode;
  readonly availableModes: readonly DocumentEditorViewMode[];
  readonly current: CurrentDocumentViewModel;
  readonly preview: MarkdownPreviewModel;
  readonly history: HistoryPanelViewModel;
  readonly querySeparation: DocumentReadQuerySeparationSummary;
}

export const DocumentEditorState = Object.freeze({
  Loading: "Loading",
  ReadyClean: "ReadyClean",
  ReadyDirty: "ReadyDirty",
  Saving: "Saving",
  Saved: "Saved",
  SaveFailed: "SaveFailed",
});

export type DocumentEditorStateValue =
  (typeof DocumentEditorState)[keyof typeof DocumentEditorState];

export const DocumentEditorEvent = Object.freeze({
  DocumentLoaded: "DocumentLoaded",
  ContentChanged: "ContentChanged",
  SaveRequested: "SaveRequested",
  SaveSucceeded: "SaveSucceeded",
  SaveFailed: "SaveFailed",
  ReloadRequested: "ReloadRequested",
});

export type DocumentEditorEventValue =
  (typeof DocumentEditorEvent)[keyof typeof DocumentEditorEvent];

export const DocumentEditorErrorCode = Object.freeze({
  InvalidTransition: "DOCUMENT_EDITOR_INVALID_TRANSITION",
  SaveFailed: "DOCUMENT_SAVE_FAILED",
});

export type DocumentEditorErrorCodeValue =
  (typeof DocumentEditorErrorCode)[keyof typeof DocumentEditorErrorCode];

export interface DocumentEditorSnapshot {
  readonly state: DocumentEditorStateValue;
  readonly currentVersionId?: string;
  readonly dirtyContentRef?: string;
  readonly savedVersionId?: string;
  readonly errorCode?: string;
}

export interface DocumentEditorTransitionEvent {
  readonly type: DocumentEditorEventValue;
  readonly currentVersionId?: string;
  readonly dirtyContentRef?: string;
  readonly savedVersionId?: string;
  readonly errorCode?: string;
}

export const RestoreFlowState = Object.freeze({
  Idle: "Idle",
  Previewing: "Previewing",
  PreviewReady: "PreviewReady",
  Applying: "Applying",
  Completed: "Completed",
  Failed: "Failed",
});

export type RestoreFlowStateValue = (typeof RestoreFlowState)[keyof typeof RestoreFlowState];

export const RestoreFlowEvent = Object.freeze({
  PreviewRequested: "PreviewRequested",
  PreviewLoaded: "PreviewLoaded",
  ApplyRequested: "ApplyRequested",
  ApplySucceeded: "ApplySucceeded",
  Fail: "Fail",
  Reset: "Reset",
});

export type RestoreFlowEventValue = (typeof RestoreFlowEvent)[keyof typeof RestoreFlowEvent];

export const RestoreFlowErrorCode = Object.freeze({
  InvalidTransition: "RESTORE_INVALID_TRANSITION",
  ConfirmationRequired: "RESTORE_CONFIRMATION_REQUIRED",
  RestoreNotAllowed: "RESTORE_NOT_ALLOWED",
});

export type RestoreFlowErrorCodeValue =
  (typeof RestoreFlowErrorCode)[keyof typeof RestoreFlowErrorCode];

export interface RestoreFlowTransitionResult {
  readonly state: RestoreFlowStateValue;
  readonly errorCode?: RestoreFlowErrorCodeValue;
}

export interface RestorePreviewRequest {
  readonly queryName: "preview-document-restore";
  readonly workspaceId: string;
  readonly documentId: string;
  readonly targetVersionId: string;
}

export type RestoreDiffLineKind = "unchanged" | "removed" | "added";

export interface RestoreDiffLineInput {
  readonly kind: RestoreDiffLineKind;
  readonly text: string;
}

export interface RestoreDiffLineViewModel {
  readonly kind: RestoreDiffLineKind;
  readonly text: string;
}

export interface RestorePreviewModelInput {
  readonly workspaceId: string;
  readonly documentId: string;
  readonly targetVersionId: string;
  readonly canRestore: boolean;
  readonly lines: readonly RestoreDiffLineInput[];
}

export interface RestorePreviewModel {
  readonly mode: "restore-preview";
  readonly state: RestoreFlowStateValue;
  readonly workspaceId: string;
  readonly documentId: string;
  readonly targetVersionId: string;
  readonly canRestore: boolean;
  readonly lines: readonly RestoreDiffLineViewModel[];
  readonly productLogEvent: "document.restore.previewed";
}

export interface RestoreConfirmationInput {
  readonly confirmed: boolean;
  readonly expectedCurrentVersionId: string;
  readonly restoredVersionId: string;
  readonly restoredSnapshotRef: string;
  readonly author: string;
  readonly summary: string;
}

export interface RestoreApplyCommand {
  readonly commandName: "restore-document-version";
  readonly workspaceId: string;
  readonly documentId: string;
  readonly targetVersionId: string;
  readonly expectedCurrentVersionId: string;
  readonly restoredVersionId: string;
  readonly restoredSnapshotRef: string;
  readonly author: string;
  readonly summary: string;
}

export interface RestoreApplyCommandResult {
  readonly status: "created" | "not-created";
  readonly command?: RestoreApplyCommand;
  readonly errorCode?: RestoreFlowErrorCodeValue;
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

export function createMarkdownPreviewModel(input: MarkdownPreviewInput): MarkdownPreviewModel {
  try {
    return {
      mode: "markdown-preview",
      sourceMode: "markdown-source",
      documentId: input.documentId,
      sourceVersionId: input.versionId,
      state: "Rendered",
      blocks: parseMarkdownPreviewBlocks(input),
    };
  } catch {
    return {
      mode: "markdown-preview",
      sourceMode: "markdown-source",
      documentId: input.documentId,
      sourceVersionId: input.versionId,
      state: "RenderFailed",
      blocks: [],
    };
  }
}

export function createDocumentReadingWorkspaceModel(
  current: CurrentDocumentView,
  history: DocumentHistoryPage,
): DocumentReadingWorkspaceModel {
  return {
    mode: "document-reading-workspace",
    current: createCurrentDocumentViewModel(current),
    preview: createMarkdownPreviewModel({
      documentId: current.documentId,
      versionId: current.versionId,
      source: current.body,
    }),
    history: createHistoryPanelViewModel(history),
    querySeparation: {
      currentReadQueryName: "get-current-document",
      historyReadQueryName: "get-document-history",
    },
  };
}

export function transitionDocumentEditorViewMode(
  current: DocumentEditorViewMode,
  event: DocumentEditorViewModeEventValue,
): DocumentEditorViewModeTransition {
  if (event === DocumentEditorViewModeEvent.ShowSource) {
    return { mode: "source" };
  }
  if (event === DocumentEditorViewModeEvent.ShowPreview) {
    return { mode: "preview" };
  }
  if (event === DocumentEditorViewModeEvent.ShowSplit) {
    return { mode: "split" };
  }
  return { mode: current };
}

export function transitionDocumentEditorState(
  current: DocumentEditorStateValue | DocumentEditorSnapshot,
  event: DocumentEditorTransitionEvent,
): DocumentEditorSnapshot {
  const snapshot = typeof current === "string" ? { state: current } : current;

  if (
    event.type === DocumentEditorEvent.DocumentLoaded &&
    [DocumentEditorState.Loading, DocumentEditorState.Saved, DocumentEditorState.ReadyClean].includes(
      snapshot.state,
    )
  ) {
    return {
      state: DocumentEditorState.ReadyClean,
      currentVersionId: event.currentVersionId ?? snapshot.currentVersionId,
    };
  }

  if (
    event.type === DocumentEditorEvent.ContentChanged &&
    event.dirtyContentRef &&
    [
      DocumentEditorState.ReadyClean,
      DocumentEditorState.ReadyDirty,
      DocumentEditorState.Saved,
      DocumentEditorState.SaveFailed,
    ].includes(snapshot.state)
  ) {
    return {
      state: DocumentEditorState.ReadyDirty,
      currentVersionId: snapshot.currentVersionId,
      dirtyContentRef: event.dirtyContentRef,
    };
  }

  if (
    event.type === DocumentEditorEvent.SaveRequested &&
    [DocumentEditorState.ReadyDirty, DocumentEditorState.SaveFailed].includes(snapshot.state)
  ) {
    return {
      state: DocumentEditorState.Saving,
      currentVersionId: snapshot.currentVersionId,
      dirtyContentRef: snapshot.dirtyContentRef,
    };
  }

  if (
    event.type === DocumentEditorEvent.SaveSucceeded &&
    snapshot.state === DocumentEditorState.Saving &&
    event.savedVersionId
  ) {
    return {
      state: DocumentEditorState.Saved,
      currentVersionId: event.savedVersionId,
      savedVersionId: event.savedVersionId,
    };
  }

  if (
    event.type === DocumentEditorEvent.SaveFailed &&
    snapshot.state === DocumentEditorState.Saving
  ) {
    return {
      state: DocumentEditorState.SaveFailed,
      currentVersionId: snapshot.currentVersionId,
      dirtyContentRef: snapshot.dirtyContentRef,
      errorCode: event.errorCode ?? DocumentEditorErrorCode.SaveFailed,
    };
  }

  if (
    event.type === DocumentEditorEvent.ReloadRequested &&
    snapshot.state !== DocumentEditorState.Saving
  ) {
    return { state: DocumentEditorState.Loading };
  }

  return {
    ...snapshot,
    errorCode: DocumentEditorErrorCode.InvalidTransition,
  };
}

export function createDocumentAuthoringWorkspaceModel(
  current: CurrentDocumentView,
  history: DocumentHistoryPage,
  options: { readonly viewMode?: DocumentEditorViewMode } = {},
): DocumentAuthoringWorkspaceModel {
  return {
    mode: "document-authoring-workspace",
    viewMode: options.viewMode ?? "split",
    availableModes: ["source", "preview", "split"],
    current: createCurrentDocumentViewModel(current),
    preview: createMarkdownPreviewModel({
      documentId: current.documentId,
      versionId: current.versionId,
      source: current.body,
    }),
    history: createHistoryPanelViewModel(history),
    querySeparation: {
      currentReadQueryName: "get-current-document",
      historyReadQueryName: "get-document-history",
    },
  };
}

export const DocumentAutosaveState = Object.freeze({
  Idle: "Idle",
  DirtyQueued: "DirtyQueued",
  Saving: "Saving",
  Saved: "Saved",
  SaveFailed: "SaveFailed",
  PausedReadOnly: "PausedReadOnly",
});

export type DocumentAutosaveStateValue =
  (typeof DocumentAutosaveState)[keyof typeof DocumentAutosaveState];

export const DocumentAutosaveEvent = Object.freeze({
  ContentChanged: "ContentChanged",
  DebounceElapsed: "DebounceElapsed",
  SaveSucceeded: "SaveSucceeded",
  SaveFailed: "SaveFailed",
  RetryRequested: "RetryRequested",
  ReadOnlyEntered: "ReadOnlyEntered",
  EditResumed: "EditResumed",
});

export type DocumentAutosaveEventValue =
  (typeof DocumentAutosaveEvent)[keyof typeof DocumentAutosaveEvent];

export const DocumentAutosaveErrorCode = Object.freeze({
  InvalidTransition: "DOCUMENT_AUTOSAVE_INVALID_TRANSITION",
});

export interface DocumentAutosaveSnapshot {
  readonly state: DocumentAutosaveStateValue;
  readonly dirtyContentRef?: string;
  readonly savedVersionId?: string;
  readonly errorCode?: string;
}

export interface DocumentAutosaveTransitionEvent {
  readonly type: DocumentAutosaveEventValue;
  readonly dirtyContentRef?: string;
  readonly savedVersionId?: string;
  readonly errorCode?: string;
}

export function transitionDocumentAutosaveState(
  current: DocumentAutosaveStateValue | DocumentAutosaveSnapshot,
  event: DocumentAutosaveTransitionEvent,
): DocumentAutosaveSnapshot {
  const snapshot = typeof current === "string" ? { state: current } : current;
  if (event.type === DocumentAutosaveEvent.ContentChanged && event.dirtyContentRef) {
    return {
      state: DocumentAutosaveState.DirtyQueued,
      dirtyContentRef: event.dirtyContentRef,
    };
  }
  if (
    snapshot.state === DocumentAutosaveState.DirtyQueued &&
    event.type === DocumentAutosaveEvent.DebounceElapsed
  ) {
    return {
      state: DocumentAutosaveState.Saving,
      dirtyContentRef: snapshot.dirtyContentRef,
    };
  }
  if (
    snapshot.state === DocumentAutosaveState.Saving &&
    event.type === DocumentAutosaveEvent.SaveSucceeded &&
    event.savedVersionId
  ) {
    return {
      state: DocumentAutosaveState.Saved,
      savedVersionId: event.savedVersionId,
    };
  }
  if (
    snapshot.state === DocumentAutosaveState.Saving &&
    event.type === DocumentAutosaveEvent.SaveFailed
  ) {
    return {
      state: DocumentAutosaveState.SaveFailed,
      dirtyContentRef: snapshot.dirtyContentRef,
      errorCode: event.errorCode ?? "DOCUMENT_AUTOSAVE_SAVE_FAILED",
    };
  }
  if (
    snapshot.state === DocumentAutosaveState.SaveFailed &&
    event.type === DocumentAutosaveEvent.RetryRequested
  ) {
    return {
      state: DocumentAutosaveState.Saving,
      dirtyContentRef: snapshot.dirtyContentRef,
    };
  }
  if (
    [DocumentAutosaveState.DirtyQueued, DocumentAutosaveState.Saving, DocumentAutosaveState.SaveFailed].includes(
      snapshot.state,
    ) &&
    event.type === DocumentAutosaveEvent.ReadOnlyEntered
  ) {
    return {
      state: DocumentAutosaveState.PausedReadOnly,
      dirtyContentRef: snapshot.dirtyContentRef,
    };
  }
  if (
    snapshot.state === DocumentAutosaveState.PausedReadOnly &&
    event.type === DocumentAutosaveEvent.EditResumed
  ) {
    return {
      state: DocumentAutosaveState.DirtyQueued,
      dirtyContentRef: snapshot.dirtyContentRef,
    };
  }
  return {
    ...snapshot,
    errorCode: DocumentAutosaveErrorCode.InvalidTransition,
  };
}

export function transitionRestoreFlowState(
  currentState: RestoreFlowStateValue,
  event: RestoreFlowEventValue,
): RestoreFlowTransitionResult {
  if (currentState === RestoreFlowState.Idle && event === RestoreFlowEvent.PreviewRequested) {
    return { state: RestoreFlowState.Previewing };
  }
  if (currentState === RestoreFlowState.Previewing && event === RestoreFlowEvent.PreviewLoaded) {
    return { state: RestoreFlowState.PreviewReady };
  }
  if (currentState === RestoreFlowState.PreviewReady && event === RestoreFlowEvent.ApplyRequested) {
    return { state: RestoreFlowState.Applying };
  }
  if (currentState === RestoreFlowState.Applying && event === RestoreFlowEvent.ApplySucceeded) {
    return { state: RestoreFlowState.Completed };
  }
  if (event === RestoreFlowEvent.Fail) {
    return { state: RestoreFlowState.Failed };
  }
  if (event === RestoreFlowEvent.Reset) {
    return { state: RestoreFlowState.Idle };
  }
  return {
    state: RestoreFlowState.Failed,
    errorCode: RestoreFlowErrorCode.InvalidTransition,
  };
}

export function createRestorePreviewRequestFromHistoryEntry(
  workspaceId: string,
  documentId: string,
  entry: HistoryEntryViewModel,
): RestorePreviewRequest {
  return {
    queryName: "preview-document-restore",
    workspaceId,
    documentId,
    targetVersionId: entry.versionId,
  };
}

export function createRestorePreviewModel(input: RestorePreviewModelInput): RestorePreviewModel {
  return {
    mode: "restore-preview",
    state: RestoreFlowState.PreviewReady,
    workspaceId: input.workspaceId,
    documentId: input.documentId,
    targetVersionId: input.targetVersionId,
    canRestore: input.canRestore,
    lines: input.lines.map((line) => ({
      kind: line.kind,
      text: line.text,
    })),
    productLogEvent: "document.restore.previewed",
  };
}

export function createRestoreApplyCommand(
  preview: RestorePreviewModel,
  confirmation: RestoreConfirmationInput,
): RestoreApplyCommandResult {
  if (!confirmation.confirmed) {
    return {
      status: "not-created",
      errorCode: RestoreFlowErrorCode.ConfirmationRequired,
    };
  }
  if (!preview.canRestore) {
    return {
      status: "not-created",
      errorCode: RestoreFlowErrorCode.RestoreNotAllowed,
    };
  }
  if (!confirmation.expectedCurrentVersionId.trim()) {
    return {
      status: "not-created",
      errorCode: RestoreFlowErrorCode.InvalidTransition,
    };
  }
  return {
    status: "created",
    command: {
      commandName: "restore-document-version",
      workspaceId: preview.workspaceId,
      documentId: preview.documentId,
      targetVersionId: preview.targetVersionId,
      expectedCurrentVersionId: confirmation.expectedCurrentVersionId,
      restoredVersionId: confirmation.restoredVersionId,
      restoredSnapshotRef: confirmation.restoredSnapshotRef,
      author: confirmation.author,
      summary: confirmation.summary,
    },
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

function parseMarkdownPreviewBlocks(input: MarkdownPreviewInput): readonly MarkdownPreviewBlock[] {
  const source = input.source;
  const lines = source.split(/\r?\n/);
  const blocks: MarkdownPreviewBlock[] = [];
  const resolvedWikilinkTargets = new Set(input.resolvedWikilinkTargets ?? []);
  const availableAssetIds = new Set(input.availableAssetIds ?? []);
  let lineIndex = 0;
  let offset = 0;

  while (lineIndex < lines.length) {
    const line = lines[lineIndex] ?? "";
    const lineOffset = offset;
    if (line.trim() === "") {
      offset += line.length + 1;
      lineIndex += 1;
      continue;
    }

    if (line.startsWith("```")) {
      const language = sanitizeMarkdownPreviewText(line.slice(3).trim()) || undefined;
      const codeStart = lineIndex + 1;
      lineIndex += 1;
      offset += line.length + 1;
      while (lineIndex < lines.length && !(lines[lineIndex] ?? "").startsWith("```")) {
        offset += (lines[lineIndex] ?? "").length + 1;
        lineIndex += 1;
      }
      const lineCount = Math.max(0, lineIndex - codeStart);
      if (lineIndex < lines.length) {
        offset += (lines[lineIndex] ?? "").length + 1;
        lineIndex += 1;
      }
      blocks.push({ kind: "code", language, lineCount });
      continue;
    }

    const headingMatch = /^(#{1,6})\s+(.+)$/.exec(line);
    if (headingMatch) {
      blocks.push({
        kind: "heading",
        level: headingMatch[1].length,
        text: sanitizeMarkdownPreviewText(headingMatch[2]),
        anchor: createMarkdownPreviewAnchor(headingMatch[2]),
      });
      offset += line.length + 1;
      lineIndex += 1;
      continue;
    }

    if (isTableHeader(lines, lineIndex)) {
      const tableStart = lineIndex;
      const headers = splitMarkdownTableRow(lines[tableStart] ?? "");
      const alignments = splitMarkdownTableRow(lines[tableStart + 1] ?? "").map(parseTableAlignment);
      lineIndex += 2;
      offset += (lines[tableStart] ?? "").length + 1;
      offset += (lines[tableStart + 1] ?? "").length + 1;
      const rows: string[][] = [];
      while (lineIndex < lines.length && isMarkdownTableRow(lines[lineIndex] ?? "")) {
        rows.push(splitMarkdownTableRow(lines[lineIndex] ?? ""));
        offset += (lines[lineIndex] ?? "").length + 1;
        lineIndex += 1;
      }
      blocks.push({ kind: "table", headers, alignments, rows });
      continue;
    }

    if (isChecklistLine(line)) {
      const items = [];
      while (lineIndex < lines.length && isChecklistLine(lines[lineIndex] ?? "")) {
        const item = parseChecklistLine(lines[lineIndex] ?? "");
        if (item) {
          items.push(item);
        }
        offset += (lines[lineIndex] ?? "").length + 1;
        lineIndex += 1;
      }
      blocks.push({ kind: "checklist", items });
      continue;
    }

    if (line.trim().startsWith(">")) {
      const quoteLines: string[] = [];
      while (lineIndex < lines.length && (lines[lineIndex] ?? "").trim().startsWith(">")) {
        quoteLines.push((lines[lineIndex] ?? "").replace(/^\s*>\s?/, ""));
        offset += (lines[lineIndex] ?? "").length + 1;
        lineIndex += 1;
      }
      const calloutMatch = /^\[!(\w+)]\s*(.*)$/.exec(quoteLines[0] ?? "");
      if (calloutMatch) {
        blocks.push({
          kind: "callout",
          calloutType: sanitizeMarkdownPreviewText(calloutMatch[1].toLowerCase()),
          title: sanitizeMarkdownPreviewText(calloutMatch[2] || calloutMatch[1]),
          text: sanitizeMarkdownPreviewText(quoteLines.slice(1).join("\n")),
        });
      } else {
        blocks.push({
          kind: "blockquote",
          text: sanitizeMarkdownPreviewText(quoteLines.join("\n")),
        });
      }
      continue;
    }

    blocks.push({
      kind: "paragraph",
      text: sanitizeMarkdownPreviewText(line),
      inlineActions: parseMarkdownInlineActions(line, lineOffset, {
        resolvedWikilinkTargets,
        availableAssetIds,
      }),
    });
    offset += line.length + 1;
    lineIndex += 1;
  }

  return blocks.filter((block) => {
    if (block.kind === "paragraph") {
      return block.text.length > 0 || block.inlineActions.length > 0;
    }
    return true;
  });
}

function isMarkdownTableRow(line: string): boolean {
  return line.trim().startsWith("|") && line.trim().endsWith("|");
}

function isTableHeader(lines: readonly string[], index: number): boolean {
  return isMarkdownTableRow(lines[index] ?? "") && isMarkdownTableAlignmentRow(lines[index + 1] ?? "");
}

function isMarkdownTableAlignmentRow(line: string): boolean {
  if (!isMarkdownTableRow(line)) {
    return false;
  }
  return splitMarkdownTableRow(line).every((cell) => /^:?-{3,}:?$/.test(cell.trim()));
}

function splitMarkdownTableRow(line: string): string[] {
  return line
    .trim()
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|")
    .map((cell) => sanitizeMarkdownPreviewText(cell.trim()));
}

function parseTableAlignment(value: string): MarkdownTableAlignment {
  const trimmed = value.trim();
  if (trimmed.startsWith(":") && trimmed.endsWith(":")) {
    return "center";
  }
  if (trimmed.startsWith(":")) {
    return "left";
  }
  if (trimmed.endsWith(":")) {
    return "right";
  }
  return "default";
}

function isChecklistLine(line: string): boolean {
  return /^-\s+\[[ xX]\]\s+/.test(line.trim());
}

function parseChecklistLine(line: string): { readonly checked: boolean; readonly text: string } | undefined {
  const match = /^-\s+\[([ xX])\]\s+(.+)$/.exec(line.trim());
  if (!match) {
    return undefined;
  }
  return {
    checked: match[1].toLowerCase() === "x",
    text: sanitizeMarkdownPreviewText(match[2]),
  };
}

function parseMarkdownInlineActions(
  line: string,
  lineOffset: number,
  context: {
    readonly resolvedWikilinkTargets: ReadonlySet<string>;
    readonly availableAssetIds: ReadonlySet<string>;
  },
): readonly MarkdownPreviewInlineAction[] {
  const actions: MarkdownPreviewInlineAction[] = [];
  for (const match of line.matchAll(/!\[\[asset:([^|\]]+)\|([^\]]+)]]/g)) {
    actions.push({
      kind: "open-asset-reference",
      assetId: match[1].trim(),
      label: sanitizeMarkdownPreviewText(match[2].trim()),
      assetState: context.availableAssetIds.has(match[1].trim()) ? "available" : "missing",
      sourceRange: {
        start: lineOffset + (match.index ?? 0),
        end: lineOffset + (match.index ?? 0) + match[0].length,
      },
    });
  }
  for (const match of line.matchAll(/(?<!!)\[\[([^|\]]+)(?:\|([^\]]+))?]]/g)) {
    const target = match[1].trim();
    actions.push({
      kind: "open-wikilink",
      target,
      label: sanitizeMarkdownPreviewText((match[2] ?? target).trim()),
      resolutionState: context.resolvedWikilinkTargets.has(target) ? "resolved" : "unresolved",
      sourceRange: {
        start: lineOffset + (match.index ?? 0),
        end: lineOffset + (match.index ?? 0) + match[0].length,
      },
    });
  }
  return actions.sort((left, right) => left.sourceRange.start - right.sourceRange.start);
}

function createMarkdownPreviewAnchor(text: string): string {
  return sanitizeMarkdownPreviewText(text)
    .toLowerCase()
    .replace(/[^\p{Letter}\p{Number}\s-]/gu, "")
    .trim()
    .replace(/\s+/g, "-");
}

function sanitizeMarkdownPreviewText(value: string): string {
  return value
    .replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, "")
    .replace(/<[^>]*\bon\w+\s*=\s*[^>]*>/gi, "")
    .replace(/<[^>]+>/g, "")
    .replace(/provider_api_key_fixture/g, "")
    .replace(/sessionToken/g, "")
    .trim();
}

export interface SearchResultItemViewModel {
  readonly workspaceId: string;
  readonly documentId: string;
  readonly title: string;
  readonly path: string;
  readonly snippet: string;
  readonly score: number;
  readonly actions: readonly SearchResultActionViewModel[];
}

export interface SearchResultActionViewModel {
  readonly id: "open-document" | "ask-ai";
  readonly label: string;
}

export interface SearchPanelViewModel {
  readonly mode: "search";
  readonly queryName: "search-documents";
  readonly workspaceId: string;
  readonly text: string;
  readonly results: readonly SearchResultItemViewModel[];
}

export type LocalSearchState = "Idle" | "Searching" | "ResultsReady" | "NoResults" | "Failed";

export type DiscoveryWorkflowState =
  | "Idle"
  | "Searching"
  | "ResultsReady"
  | "NoResults"
  | "IndexStale"
  | "Repairing"
  | "RepairSucceeded"
  | "RepairFailed";

export type IndexFreshnessState =
  | "Fresh"
  | "Stale"
  | "RebuildQueued"
  | "Rebuilding"
  | "RebuildFailed";

export interface IndexFreshnessAction {
  readonly id: "rebuild-index";
  readonly label: string;
}

export interface IndexFreshnessActionModel {
  readonly state: IndexFreshnessState;
  readonly actions: readonly IndexFreshnessAction[];
}

export interface DiscoveryQueryPolicyInput {
  readonly debounceMs?: number;
  readonly cancelPrevious?: boolean;
  readonly pageSize?: number;
  readonly resultLimit?: number;
  readonly graphDepthLimit?: number;
}

export interface DiscoveryQueryPolicy {
  readonly debounceMs: number;
  readonly cancelPrevious: true;
  readonly pageSize: number;
  readonly resultLimit: number;
  readonly graphDepthLimit: number;
  readonly fullWorkspaceScan: false;
}

export interface LocalDiscoverySearchPanelModel {
  readonly mode: "local-search";
  readonly queryName: "search-documents";
  readonly workspaceId: string;
  readonly queryHash: string;
  readonly resultCount: number;
  readonly state: LocalSearchState;
  readonly filters: readonly LocalDiscoverySearchFilterViewModel[];
  readonly recentSearches: readonly LocalDiscoveryRecentSearchViewModel[];
  readonly results: readonly SearchResultItemViewModel[];
}

export type LocalDiscoverySearchFilterKind = "tag" | "status" | "asset";

export interface LocalDiscoverySearchFilterViewModel {
  readonly kind: LocalDiscoverySearchFilterKind;
  readonly label: string;
  readonly active: boolean;
}

export interface LocalDiscoveryRecentSearchViewModel {
  readonly queryHash: string;
}

export interface LocalDiscoveryLinkPanelModel extends LinkPanelViewModel {
  readonly backlinkCount: number;
  readonly unresolvedCount: number;
  readonly orphanCount: number;
}

export interface LocalDiscoveryPanelInput {
  readonly search: SearchResultsPage;
  readonly links: LinkOverviewView;
  readonly assets: DocumentAssetsPage;
  readonly indexFreshness: IndexFreshnessState;
  readonly filters?: readonly LocalDiscoverySearchFilterViewModel[];
  readonly recentSearches?: readonly string[];
  readonly queryPolicy?: DiscoveryQueryPolicyInput;
}

export interface LocalDiscoveryPanelModel {
  readonly mode: "local-discovery";
  readonly workflowState: DiscoveryWorkflowState;
  readonly queryPolicy: DiscoveryQueryPolicy;
  readonly search: LocalDiscoverySearchPanelModel;
  readonly links: LocalDiscoveryLinkPanelModel;
  readonly assets: AssetPanelViewModel;
  readonly index: IndexFreshnessActionModel;
}

export type GraphPanelState = "Ready" | "ReindexQueued" | "Reindexing" | "Degraded" | "Failed";

export interface GraphPanelAction {
  readonly id: "rebuild-index";
  readonly label: string;
}

export interface GraphPanelOptions {
  readonly depthLimit: number;
  readonly pageSize: number;
}

export interface GraphNodeItemViewModel {
  readonly id: string;
  readonly kind: string;
}

export interface GraphEdgeItemViewModel {
  readonly id: string;
  readonly sourceId: string;
  readonly targetId: string;
  readonly kind: string;
}

export interface GraphPanelViewModel {
  readonly mode: "graph";
  readonly loadMode: "neighborhood";
  readonly fullWorkspaceScan: false;
  readonly centerDocumentId: string;
  readonly state: GraphPanelState;
  readonly depthLimit: number;
  readonly pageSize: number;
  readonly nodeCount: number;
  readonly edgeCount: number;
  readonly nodes: readonly GraphNodeItemViewModel[];
  readonly edges: readonly GraphEdgeItemViewModel[];
  readonly actions: readonly GraphPanelAction[];
  readonly performance?: {
    readonly targetMs: number;
    readonly observedMs: number;
  };
}

export interface CanvasViewport {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

export interface CanvasViewportPanelOptions {
  readonly viewport: CanvasViewport;
  readonly pageSize: number;
}

export type CanvasViewportLoadState =
  | "Idle"
  | "LoadingViewport"
  | "ViewportReady"
  | "LoadingMore"
  | "Failed";

export interface CanvasNodeItemViewModel {
  readonly id: string;
  readonly targetKind: string;
  readonly x: number;
  readonly y: number;
}

export interface CanvasEdgeItemViewModel {
  readonly id: string;
  readonly sourceId: string;
  readonly targetId: string;
}

export interface CanvasViewportPanelViewModel {
  readonly mode: "canvas";
  readonly canvasId: string;
  readonly state: string;
  readonly loadState: CanvasViewportLoadState;
  readonly viewport: CanvasViewport;
  readonly pageSize: number;
  readonly viewOnly: boolean;
  readonly viewportNodeCount: number;
  readonly visibleNodes: readonly CanvasNodeItemViewModel[];
  readonly visibleEdges: readonly CanvasEdgeItemViewModel[];
  readonly actions: readonly {
    readonly id: "open-canvas";
    readonly label: string;
  }[];
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

export function createLocalDiscoveryPanelModel(input: LocalDiscoveryPanelInput): LocalDiscoveryPanelModel {
  const links = createLinkPanelViewModel(input.links);
  return {
    mode: "local-discovery",
    workflowState: createDiscoveryWorkflowState(input.search, input.indexFreshness),
    queryPolicy: createDiscoveryQueryPolicy(input.queryPolicy),
    search: createLocalDiscoverySearchPanelModel(input.search, input),
    links: {
      ...links,
      backlinkCount: links.backlinks.length,
      unresolvedCount: links.unresolvedLinks.length,
      orphanCount: links.orphanDocuments.length,
    },
    assets: createAssetPanelViewModel(input.assets),
    index: createIndexFreshnessActionModel(input.indexFreshness),
  };
}

export function createDiscoveryQueryPolicy(input: DiscoveryQueryPolicyInput = {}): DiscoveryQueryPolicy {
  return {
    debounceMs: clampInteger(input.debounceMs ?? 180, 120, 500),
    cancelPrevious: true,
    pageSize: clampInteger(input.pageSize ?? 25, 1, 100),
    resultLimit: clampInteger(input.resultLimit ?? 25, 1, 100),
    graphDepthLimit: clampInteger(input.graphDepthLimit ?? 2, 1, 3),
    fullWorkspaceScan: false,
  };
}

export function createIndexFreshnessActionModel(state: IndexFreshnessState): IndexFreshnessActionModel {
  if (state === "Stale" || state === "RebuildFailed") {
    return {
      state,
      actions: [{ id: "rebuild-index", label: "Rebuild index" }],
    };
  }
  return {
    state,
    actions: [],
  };
}

function createDiscoveryWorkflowState(
  page: SearchResultsPage,
  indexFreshness: IndexFreshnessState,
): DiscoveryWorkflowState {
  if (indexFreshness === "Stale") {
    return "IndexStale";
  }
  if (indexFreshness === "RebuildQueued" || indexFreshness === "Rebuilding") {
    return "Repairing";
  }
  if (indexFreshness === "RebuildFailed") {
    return "RepairFailed";
  }
  return page.results.length > 0 ? "ResultsReady" : "NoResults";
}

export function createGraphPanelViewModel(
  graph: KnowledgeGraphView,
  options: GraphPanelOptions,
): GraphPanelViewModel {
  const state = mapGraphPanelState(graph.status);
  return {
    mode: "graph",
    loadMode: "neighborhood",
    fullWorkspaceScan: false,
    centerDocumentId: graph.centerDocumentId,
    state,
    depthLimit: options.depthLimit,
    pageSize: options.pageSize,
    nodeCount: graph.nodes.length,
    edgeCount: graph.edges.length,
    nodes: graph.nodes.map((node) => ({ id: node.id, kind: node.kind })),
    edges: graph.edges.map((edge) => ({
      id: edge.id,
      sourceId: edge.sourceId,
      targetId: edge.targetId,
      kind: edge.kind,
    })),
    actions: state === "Degraded" || state === "ReindexQueued"
      ? [{ id: "rebuild-index", label: "Rebuild index" }]
      : [],
    performance: graph.performance,
  };
}

export function createCanvasViewportPanelModel(
  canvas: CanvasView,
  options: CanvasViewportPanelOptions,
): CanvasViewportPanelViewModel {
  const visibleNodes = canvas.nodes
    .filter((node) => isNodeInViewport(node.x, node.y, options.viewport))
    .slice(0, options.pageSize)
    .map((node) => ({
      id: node.id,
      targetKind: node.targetKind,
      x: node.x,
      y: node.y,
    }));
  const visibleNodeIds = new Set(visibleNodes.map((node) => node.id));
  const visibleEdges = canvas.edges
    .filter((edge) => visibleNodeIds.has(edge.sourceId) && visibleNodeIds.has(edge.targetId))
    .map((edge) => ({
      id: edge.id,
      sourceId: edge.sourceId,
      targetId: edge.targetId,
    }));
  const viewOnly = canvas.state === "archived";

  return {
    mode: "canvas",
    canvasId: canvas.canvasId,
    state: canvas.state,
    loadState: "ViewportReady",
    viewport: options.viewport,
    pageSize: options.pageSize,
    viewOnly,
    viewportNodeCount: visibleNodes.length,
    visibleNodes,
    visibleEdges,
    actions: viewOnly ? [] : [{ id: "open-canvas", label: "Open canvas" }],
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
    snippet: sanitizeMarkdownPreviewText(result.snippet),
    score: result.score,
    actions: [
      { id: "open-document", label: "Open document" },
      { id: "ask-ai", label: "Ask AI" },
    ],
  };
}

function createLocalDiscoverySearchPanelModel(
  page: SearchResultsPage,
  input: Pick<LocalDiscoveryPanelInput, "filters" | "recentSearches">,
): LocalDiscoverySearchPanelModel {
  const results = page.results.map(createSearchResultItemViewModel);
  return {
    mode: "local-search",
    queryName: "search-documents",
    workspaceId: page.workspaceId,
    queryHash: createUiTextHash(page.text),
    resultCount: results.length,
    state: results.length > 0 ? "ResultsReady" : "NoResults",
    filters: input.filters ?? [],
    recentSearches: (input.recentSearches ?? []).map((query) => ({
      queryHash: createUiTextHash(query),
    })),
    results,
  };
}

function clampInteger(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) {
    return min;
  }
  return Math.max(min, Math.min(max, Math.trunc(value)));
}

function mapGraphPanelState(status: KnowledgeGraphView["status"]): GraphPanelState {
  if (status === "clean") {
    return "Ready";
  }
  if (status === "reindex_requested") {
    return "ReindexQueued";
  }
  if (status === "reindexing") {
    return "Reindexing";
  }
  if (status === "degraded") {
    return "Degraded";
  }
  return "Failed";
}

function isNodeInViewport(x: number, y: number, viewport: CanvasViewport): boolean {
  return (
    x >= viewport.x &&
    y >= viewport.y &&
    x <= viewport.x + viewport.width &&
    y <= viewport.y + viewport.height
  );
}

export type ConnectorKindView = "Slack" | "Teams" | "Jira";
export type ConnectorScopeView = "read" | "write";
export type ConnectorInstallationStateView =
  | "AuthorizationRequested"
  | "Installed"
  | "AuthorizationFailed"
  | "SyncQueued"
  | "Syncing"
  | "Synced"
  | "RetryScheduled"
  | "Failed"
  | "Disabled";

export interface ConnectorDefinitionView {
  readonly connectorId: string;
  readonly kind: ConnectorKindView;
  readonly displayName: string;
  readonly scopes: readonly ConnectorScopeView[];
}

export interface ConnectorInstallationView {
  readonly installationId: string;
  readonly workspaceId: string;
  readonly connectorId: string;
  readonly state: ConnectorInstallationStateView;
  readonly scopes: readonly ConnectorScopeView[];
}

export interface ConnectorAdminViewInput {
  readonly definitions: readonly ConnectorDefinitionView[];
  readonly installations: readonly ConnectorInstallationView[];
}

export interface ConnectorAdminCardViewModel {
  readonly connectorId: string;
  readonly kind: ConnectorKindView;
  readonly displayName: string;
  readonly scopeLabel: "Read only" | "Read and write";
  readonly supportsWrite: boolean;
  readonly installationId?: string;
  readonly installationState?: ConnectorInstallationStateView;
}

export interface ConnectorAdminViewModel {
  readonly mode: "connector-admin";
  readonly cards: readonly ConnectorAdminCardViewModel[];
}

export function createConnectorAdminViewModel(input: ConnectorAdminViewInput): ConnectorAdminViewModel {
  const installationsByConnector = new Map(
    input.installations.map((installation) => [installation.connectorId, installation]),
  );

  return {
    mode: "connector-admin",
    cards: input.definitions.map((definition) => {
      const installation = installationsByConnector.get(definition.connectorId);
      const supportsWrite = definition.scopes.includes("write");
      return {
        connectorId: definition.connectorId,
        kind: definition.kind,
        displayName: definition.displayName,
        scopeLabel: supportsWrite ? "Read and write" : "Read only",
        supportsWrite,
        installationId: installation?.installationId,
        installationState: installation?.state,
      };
    }),
  };
}

export type AiQueryDisplayState =
  | "idle"
  | "provider-disabled"
  | "retrieval-ready"
  | "waiting-for-result"
  | "completed"
  | "refused"
  | "failed"
  | "invalid-result";

export interface AiCitationCardViewModel {
  readonly sourceId: string;
  readonly sourceKind: AiRetrievalSourceKindView;
  readonly sourceTitle?: string;
  readonly citationReference: string;
  readonly headingAnchor?: string;
  readonly blockReference?: string;
  readonly freshness: AiFreshnessStatusView;
  readonly permissionDecision?: AiPermissionDecisionSummaryView;
}

export interface AiQueryPanelInput {
  readonly workspaceId: string;
  readonly question: string;
  readonly retrieval?: AiRetrievalResultPage;
  readonly status?: AiAnswerJobView;
  readonly result?: AiAnswerResultView;
  readonly providerSettings?: AiProviderSettingsSummaryView;
}

export interface AiQueryPanelViewModel {
  readonly mode: "ai-query";
  readonly workspaceId: string;
  readonly questionHash: string;
  readonly displayState: AiQueryDisplayState;
  readonly providerState?: AiProviderSettingsStateView;
  readonly providerBlocksLocalWorkspace: false;
  readonly providerActions: readonly AiProviderSettingsAction[];
  readonly canDisplayAnswer: boolean;
  readonly citationCards: readonly AiCitationCardViewModel[];
  readonly answerReference?: string;
  readonly refusalCode?: string;
  readonly freshnessStatus?: AiFreshnessStatusView;
}

export function createAiQueryPanelViewModel(input: AiQueryPanelInput): AiQueryPanelViewModel {
  const citationCards = input.result
    ? input.result.citations.map((citation) => ({
        sourceId: citation.sourceId,
        sourceKind: citation.sourceKind,
        sourceTitle: citation.sourceTitle,
        citationReference: citation.citationReference,
        headingAnchor: citation.headingAnchor,
        blockReference: citation.blockReference,
        freshness: citation.freshness,
        permissionDecision: citation.permissionDecision,
      }))
    : (input.retrieval?.candidates.map((candidate) => ({
        sourceId: candidate.sourceId,
        sourceKind: candidate.sourceKind,
        sourceTitle: candidate.sourceTitle,
        citationReference: candidate.citationReference,
        headingAnchor: candidate.headingAnchor,
        blockReference: candidate.blockReference,
        freshness: candidate.freshness,
        permissionDecision: candidate.permissionDecision,
      })) ?? []);

  const providerSettings = input.providerSettings
    ? createAiProviderSettingsViewModel(input.providerSettings)
    : undefined;
  const displayState = resolveAiQueryDisplayState(input, citationCards.length);
  return {
    mode: "ai-query",
    workspaceId: input.workspaceId,
    questionHash: createUiTextHash(input.question),
    displayState,
    providerState: providerSettings?.state,
    providerBlocksLocalWorkspace: false,
    providerActions: providerSettings?.actions ?? [],
    canDisplayAnswer: displayState === "completed",
    citationCards,
    answerReference: displayState === "completed" ? input.result?.answerReference : undefined,
    refusalCode: displayState === "refused" ? input.result?.refusalCode : undefined,
    freshnessStatus: input.result?.freshnessStatus ?? input.status?.freshnessStatus,
  };
}

function resolveAiQueryDisplayState(
  input: AiQueryPanelInput,
  citationCount: number,
): AiQueryDisplayState {
  if (!input.result && !input.status && !input.retrieval && input.providerSettings?.state === "Disabled") {
    return "provider-disabled";
  }
  if (input.result) {
    if (input.result.state === "Completed") {
      return input.result.answerReference && citationCount > 0 ? "completed" : "invalid-result";
    }
    if (input.result.state === "Refused") {
      return "refused";
    }
    return "failed";
  }
  if (input.status) {
    if (input.status.state === "Failed") {
      return "failed";
    }
    return "waiting-for-result";
  }
  if (input.retrieval) {
    return "retrieval-ready";
  }
  return "idle";
}

export type AiCitationSourceState =
  | "NoCitation"
  | "CitationReady"
  | "SourceUnavailable"
  | "SourceStale"
  | "SourceAccessDenied";

export type AiCitationSourceEvent =
  | "OpenCurrentRequested"
  | "OpenVersionRequested"
  | "SourceMissing"
  | "SourceStaleDetected"
  | "AccessDenied";

export interface AiCitationSourceTransitionResult {
  readonly state: AiCitationSourceState;
  readonly errorCode?: string;
  readonly warningCode?: string;
}

export type AiCitationSourceTarget =
  | { readonly kind: "current" }
  | { readonly kind: "version"; readonly versionId: string };

export type AiCitationSourceOpenCommand =
  | {
      readonly type: "open-current-document";
      readonly workspaceId: string;
      readonly documentId: string;
      readonly citationReference: string;
      readonly anchor?: string;
    }
  | {
      readonly type: "open-document-version";
      readonly workspaceId: string;
      readonly documentId: string;
      readonly versionId: string;
      readonly citationReference: string;
      readonly anchor?: string;
    };

export interface AiCitationSourceOpenInput {
  readonly workspaceId: string;
  readonly citation?: AiCitationCardViewModel;
  readonly target: AiCitationSourceTarget;
  readonly sourceState: AiCitationSourceState;
}

export interface AiCitationSourceOpenAction {
  readonly mode: "ai-citation-source-open";
  readonly state: AiCitationSourceState;
  readonly canOpen: boolean;
  readonly command?: AiCitationSourceOpenCommand;
  readonly errorCode?: string;
  readonly warningCode?: string;
}

export function transitionAiCitationSourceState(
  state: AiCitationSourceState,
  event: AiCitationSourceEvent,
): AiCitationSourceTransitionResult {
  if (
    (state === "SourceAccessDenied" || state === "SourceUnavailable") &&
    (event === "OpenCurrentRequested" || event === "OpenVersionRequested")
  ) {
    return { state, errorCode: "AI_CITATION_SOURCE_INVALID_TRANSITION" };
  }

  switch (event) {
    case "OpenCurrentRequested":
    case "OpenVersionRequested":
      return { state: "CitationReady" };
    case "SourceMissing":
      return { state: "SourceUnavailable", errorCode: "AI_CITATION_SOURCE_UNAVAILABLE" };
    case "SourceStaleDetected":
      return { state: "SourceStale", warningCode: "AI_CITATION_SOURCE_STALE" };
    case "AccessDenied":
      return {
        state: "SourceAccessDenied",
        errorCode: "AI_CITATION_SOURCE_ACCESS_DENIED",
      };
  }
}

export function createAiCitationSourceOpenAction(
  input: AiCitationSourceOpenInput,
): AiCitationSourceOpenAction {
  if (!input.citation || input.sourceState === "NoCitation") {
    return {
      mode: "ai-citation-source-open",
      state: "NoCitation",
      canOpen: false,
      errorCode: "AI_CITATION_REQUIRED",
    };
  }
  if (input.sourceState === "SourceUnavailable") {
    return {
      mode: "ai-citation-source-open",
      state: "SourceUnavailable",
      canOpen: false,
      errorCode: "AI_CITATION_SOURCE_UNAVAILABLE",
    };
  }
  if (input.sourceState === "SourceAccessDenied") {
    return {
      mode: "ai-citation-source-open",
      state: "SourceAccessDenied",
      canOpen: false,
      errorCode: "AI_CITATION_SOURCE_ACCESS_DENIED",
    };
  }

  const command =
    input.target.kind === "version"
      ? {
          type: "open-document-version" as const,
          workspaceId: input.workspaceId,
          documentId: input.citation.sourceId,
          versionId: input.target.versionId,
          citationReference: input.citation.citationReference,
          anchor: input.citation.headingAnchor,
        }
      : {
          type: "open-current-document" as const,
          workspaceId: input.workspaceId,
          documentId: input.citation.sourceId,
          citationReference: input.citation.citationReference,
          anchor: input.citation.headingAnchor,
        };

  return {
    mode: "ai-citation-source-open",
    state: input.sourceState,
    canOpen: true,
    command,
    warningCode: input.sourceState === "SourceStale" ? "AI_CITATION_SOURCE_STALE" : undefined,
  };
}

export type LocalAiToolScopeState =
  | "Hidden"
  | "VisibleReadOnly"
  | "DisabledByCapability"
  | "Failed";

export interface LocalAiToolScopeViewModel {
  readonly mode: "local-ai-tool-scope";
  readonly state: LocalAiToolScopeState;
  readonly tools: readonly LocalAiToolDescriptorView[];
  readonly hiddenToolIds: readonly string[];
}

export interface LocalAiToolScopeInput {
  readonly profile: PersonalLocalDesktopCapabilityProfile;
  readonly tools: readonly LocalAiToolDescriptorView[];
}

const allowedLocalAiToolOperations = new Set<LocalAiToolOperationView>([
  "read-document",
  "search-documents",
  "open-citation",
  "ask-ai",
  "read-asset-metadata",
  "read-graph",
]);

export function createLocalAiToolScopeViewModel(input: LocalAiToolScopeInput): LocalAiToolScopeViewModel {
  if (!input.profile.supportsLocalWorkspace || input.profile.supportsRemoteWorkspace) {
    return {
      mode: "local-ai-tool-scope",
      state: "DisabledByCapability",
      tools: [],
      hiddenToolIds: input.tools.map((tool) => tool.id),
    };
  }

  const tools = input.tools.filter((tool) => allowedLocalAiToolOperations.has(tool.operation));
  const hiddenToolIds = input.tools
    .filter((tool) => !allowedLocalAiToolOperations.has(tool.operation))
    .map((tool) => tool.id);

  return {
    mode: "local-ai-tool-scope",
    state: tools.length === 0 ? "Hidden" : "VisibleReadOnly",
    tools,
    hiddenToolIds,
  };
}

export type AiProviderCredentialState = "not-configured" | "handle-present";

export interface AiProviderSettingsAction {
  readonly id: "open-optional-provider-settings" | "validate-provider-settings";
  readonly label: string;
}

export interface AiProviderSettingsViewModel {
  readonly mode: "ai-provider-settings";
  readonly state: AiProviderSettingsStateView;
  readonly providerName?: string;
  readonly modelName?: string;
  readonly credentialState: AiProviderCredentialState;
  readonly blocksLocalWorkspace: false;
  readonly actions: readonly AiProviderSettingsAction[];
}

export function createAiProviderSettingsViewModel(
  input: AiProviderSettingsSummaryView,
): AiProviderSettingsViewModel {
  const actions: AiProviderSettingsAction[] =
    input.state === "Disabled"
      ? []
      : [
          {
            id: "open-optional-provider-settings",
            label: "Provider settings",
          },
        ];
  if (input.state === "Configured" || input.state === "Invalid") {
    actions.push({
      id: "validate-provider-settings",
      label: "Validate provider",
    });
  }

  return {
    mode: "ai-provider-settings",
    state: input.state,
    providerName: sanitizeOptionalAiSettingsLabel(input.providerName),
    modelName: sanitizeOptionalAiSettingsLabel(input.modelName),
    credentialState: input.credentialHandlePresent ? "handle-present" : "not-configured",
    blocksLocalWorkspace: false,
    actions,
  };
}

function sanitizeOptionalAiSettingsLabel(value: string | undefined): string | undefined {
  if (!value) {
    return undefined;
  }
  if (/(api[_-]?key|token|secret|credential|endpoint|password)/i.test(value)) {
    return undefined;
  }
  return value;
}

export type DataOwnershipSettingsSectionId =
  | "storage"
  | "backup-export"
  | "import"
  | "restore"
  | "ai-provider"
  | "field-debug"
  | "workspace-health";

export type DataOwnershipBackupState = "NeverCreated" | "Fresh" | "Stale" | "Failed";
export type DataOwnershipImportState = ImportPreviewStateView;
export type DataOwnershipRestoreState = RestoreStagingStateView;
export type DataOwnershipWorkspaceHealthState = "Healthy" | "Degraded" | "RepairAvailable" | "Failed";

export interface DataOwnershipSettingsInput {
  readonly workspaceId: string;
  readonly storageLabel: string;
  readonly backupState: DataOwnershipBackupState;
  readonly importState: DataOwnershipImportState;
  readonly restoreState: DataOwnershipRestoreState;
  readonly aiProviderState: AiProviderSettingsStateView;
  readonly fieldDebugState: FieldDebugSettingsState;
  readonly workspaceHealthState: DataOwnershipWorkspaceHealthState;
}

export interface DataOwnershipSettingsSectionViewModel {
  readonly id: DataOwnershipSettingsSectionId;
  readonly status: string;
  readonly actionId: string;
}

export interface DataOwnershipSettingsModel {
  readonly mode: "data-ownership-settings";
  readonly productScope: "personal_local_desktop";
  readonly workspaceId: string;
  readonly storageLabel: string;
  readonly sections: readonly DataOwnershipSettingsSectionViewModel[];
  readonly forbiddenSectionIds: readonly string[];
}

export type FieldDebugSettingsState =
  | "Disabled"
  | "ActivationRequested"
  | "Active"
  | "Expired"
  | "Rejected";

export interface FieldDebugSettingsInput {
  readonly state: FieldDebugSettingsState;
  readonly scope?: string;
  readonly expiryMinutes?: number;
  readonly reason?: string;
  readonly maskingPolicyAccepted: boolean;
}

export interface FieldDebugSettingsModel {
  readonly mode: "field-debug-settings";
  readonly state: FieldDebugSettingsState;
  readonly canActivate: boolean;
  readonly scopeHash?: string;
  readonly expiryMinutes?: number;
  readonly reasonProvided: boolean;
  readonly maskingPolicyAccepted: boolean;
  readonly requiredFixes: readonly ("scope" | "expiry" | "reason" | "masking-policy")[];
}

export function createDataOwnershipSettingsModel(
  input: DataOwnershipSettingsInput,
): DataOwnershipSettingsModel {
  return {
    mode: "data-ownership-settings",
    productScope: "personal_local_desktop",
    workspaceId: input.workspaceId,
    storageLabel: sanitizeSettingsStorageLabel(input.storageLabel),
    sections: [
      { id: "storage", status: "ready", actionId: "view-storage-summary" },
      { id: "backup-export", status: input.backupState, actionId: "open-backup-export" },
      { id: "import", status: input.importState, actionId: "open-import-preview" },
      { id: "restore", status: input.restoreState, actionId: "open-restore-staging" },
      { id: "ai-provider", status: input.aiProviderState, actionId: "open-ai-provider-settings" },
      { id: "field-debug", status: input.fieldDebugState, actionId: "open-field-debug-settings" },
      { id: "workspace-health", status: input.workspaceHealthState, actionId: "open-workspace-health" },
    ],
    forbiddenSectionIds: [],
  };
}

export function createFieldDebugSettingsModel(input: FieldDebugSettingsInput): FieldDebugSettingsModel {
  const requiredFixes: ("scope" | "expiry" | "reason" | "masking-policy")[] = [];
  const scope = input.scope?.trim() ?? "";
  const reason = input.reason?.trim() ?? "";
  if (!scope || /(document_body|raw|token|secret|credential|password|api[_-]?key)/i.test(scope)) {
    requiredFixes.push("scope");
  }
  if (!input.expiryMinutes || input.expiryMinutes < 1 || input.expiryMinutes > 60) {
    requiredFixes.push("expiry");
  }
  if (!reason) {
    requiredFixes.push("reason");
  }
  if (!input.maskingPolicyAccepted) {
    requiredFixes.push("masking-policy");
  }
  return {
    mode: "field-debug-settings",
    state: requiredFixes.length === 0 ? input.state : "Rejected",
    canActivate: input.state === "ActivationRequested" && requiredFixes.length === 0,
    scopeHash: scope ? createUiTextHash(scope) : undefined,
    expiryMinutes: input.expiryMinutes,
    reasonProvided: reason.length > 0,
    maskingPolicyAccepted: input.maskingPolicyAccepted,
    requiredFixes,
  };
}

export type RecoveryActionPanelState =
  | "Healthy"
  | "Degraded"
  | "RepairAvailable"
  | "Repairing"
  | "RepairSucceeded"
  | "RepairFailed"
  | "ReadOnlyRecovery";

export type RecoveryActionId =
  | "repair-workspace"
  | "retry-repair"
  | "view-progress"
  | "open-runbook"
  | "export-safe-copy"
  | "open-backup-settings";

export interface RecoveryActionPanelInput {
  readonly workspaceId: string;
  readonly state: RecoveryActionPanelState;
  readonly issueCode?: string;
}

export interface RecoveryActionViewModel {
  readonly id: RecoveryActionId;
  readonly label: string;
  readonly requiresConfirmation: boolean;
}

export interface RecoveryActionPanelModel {
  readonly mode: "recovery-action-panel";
  readonly workspaceId: string;
  readonly state: RecoveryActionPanelState;
  readonly issueCode?: string;
  readonly readOnly: boolean;
  readonly actions: readonly RecoveryActionViewModel[];
  readonly productLogEvent?: string;
}

export function createRecoveryActionPanelModel(input: RecoveryActionPanelInput): RecoveryActionPanelModel {
  return {
    mode: "recovery-action-panel",
    workspaceId: input.workspaceId,
    state: input.state,
    issueCode: input.issueCode ? sanitizeStableCode(input.issueCode) : undefined,
    readOnly: input.state === "ReadOnlyRecovery",
    actions: recoveryActions(input.state),
    productLogEvent: recoveryProductLogEvent(input.state),
  };
}

function recoveryActions(state: RecoveryActionPanelState): readonly RecoveryActionViewModel[] {
  if (state === "RepairAvailable" || state === "Degraded") {
    return [
      { id: "repair-workspace", label: "Repair workspace", requiresConfirmation: true },
      { id: "export-safe-copy", label: "Export safe copy", requiresConfirmation: false },
    ];
  }
  if (state === "Repairing") {
    return [{ id: "view-progress", label: "View progress", requiresConfirmation: false }];
  }
  if (state === "RepairFailed") {
    return [
      { id: "retry-repair", label: "Retry repair", requiresConfirmation: true },
      { id: "open-runbook", label: "Open recovery guide", requiresConfirmation: false },
      { id: "export-safe-copy", label: "Export safe copy", requiresConfirmation: false },
    ];
  }
  if (state === "ReadOnlyRecovery") {
    return [
      { id: "export-safe-copy", label: "Export safe copy", requiresConfirmation: false },
      { id: "open-backup-settings", label: "Open backup settings", requiresConfirmation: false },
    ];
  }
  return [];
}

function recoveryProductLogEvent(state: RecoveryActionPanelState): string | undefined {
  if (state === "RepairSucceeded") {
    return "workspace.repair.completed";
  }
  if (state === "RepairFailed") {
    return "workspace.repair.failed";
  }
  return undefined;
}

function sanitizeSettingsStorageLabel(value: string): string {
  if (/([A-Za-z]:\\|\/Users\/|\/home\/|\/var\/|\/private\/)/.test(value)) {
    return "platform app data";
  }
  return sanitizeStableCode(value.toLowerCase());
}

export interface BackupArtifactActionViewModel {
  readonly id: "inspect-backup-manifest" | "restore-from-backup" | "export-package";
  readonly label: string;
}

export type BackupSettingsLocationState = "PlatformDefault" | "CustomSelected" | "Unavailable";
export type BackupSettingsLatestState = "NeverCreated" | "Fresh" | "Stale" | "Failed";

export interface BackupSettingsInput {
  readonly locationState: BackupSettingsLocationState;
  readonly defaultLocationLabel: string;
  readonly latestBackupState: BackupSettingsLatestState;
  readonly lastBackupAtIso?: string;
}

export interface BackupSettingsActionViewModel {
  readonly id: "create-backup" | "choose-backup-location";
  readonly label: string;
}

export interface BackupSettingsViewModel {
  readonly mode: "backup-settings";
  readonly locationState: BackupSettingsLocationState;
  readonly defaultLocationLabel: string;
  readonly latestBackupState: BackupSettingsLatestState;
  readonly lastBackupAtIso?: string;
  readonly blocksLocalStartup: false;
  readonly actions: readonly BackupSettingsActionViewModel[];
}

export function createBackupSettingsViewModel(input: BackupSettingsInput): BackupSettingsViewModel {
  return {
    mode: "backup-settings",
    locationState: input.locationState,
    defaultLocationLabel: sanitizeBackupCategory(input.defaultLocationLabel),
    latestBackupState: input.latestBackupState,
    lastBackupAtIso: input.lastBackupAtIso,
    blocksLocalStartup: false,
    actions: [
      { id: "create-backup", label: "Create backup" },
      { id: "choose-backup-location", label: "Choose backup location" },
    ],
  };
}

export interface BackupArtifactManifestViewModel {
  readonly mode: "backup-artifact-manifest";
  readonly artifactId: string;
  readonly operation: BackupArtifactManifestSummaryView["operation"];
  readonly documentCount: number;
  readonly assetCount: number;
  readonly versionCount: number;
  readonly byteSizeBucket: string;
  readonly createdAtIso: string;
  readonly sealed: boolean;
  readonly excludedSecretCategories: readonly string[];
  readonly actions: readonly BackupArtifactActionViewModel[];
}

export function createBackupArtifactManifestViewModel(
  input: BackupArtifactManifestSummaryView,
): BackupArtifactManifestViewModel {
  const actions: BackupArtifactActionViewModel[] = [
    { id: "inspect-backup-manifest", label: "Inspect manifest" },
  ];
  if (input.operation === "backup") {
    actions.push({ id: "restore-from-backup", label: "Restore" });
  } else {
    actions.push({ id: "export-package", label: "Export package" });
  }

  return {
    mode: "backup-artifact-manifest",
    artifactId: input.artifactId,
    operation: input.operation,
    documentCount: input.documentCount,
    assetCount: input.assetCount,
    versionCount: input.versionCount,
    byteSizeBucket: sanitizeBackupBucket(input.byteSizeBucket),
    createdAtIso: input.createdAtIso,
    sealed: input.sealed,
    excludedSecretCategories: input.excludedSecretCategories.map(sanitizeBackupCategory),
    actions,
  };
}

export type RestoreStagingEvent =
  | "ValidateRequested"
  | "ValidationPassed"
  | "ValidationFailed"
  | "ApplyRequested"
  | "ApplySucceeded"
  | "Fail"
  | "Reset";

export interface RestoreStagingTransitionResult {
  readonly state: RestoreStagingStateView;
  readonly errorCode?: string;
}

export interface RestoreStagingValidationInput {
  readonly stagingId: string;
  readonly manifest: BackupArtifactManifestSummaryView;
  readonly state: RestoreStagingStateView;
  readonly issues: readonly RestoreStagingIssueView[];
}

export interface RestoreStagingActionViewModel {
  readonly id: "validate-restore-staging" | "apply-restore-staging" | "open-restored-workspace";
  readonly label: string;
}

export interface RestoreStagingValidationViewModel {
  readonly mode: "restore-staging-validation";
  readonly stagingId: string;
  readonly state: RestoreStagingStateView;
  readonly manifest: BackupArtifactManifestViewModel;
  readonly issues: readonly RestoreStagingIssueView[];
  readonly canApply: boolean;
  readonly requiresConfirmation: boolean;
  readonly currentWorkspaceMutationAllowed: boolean;
  readonly actions: readonly RestoreStagingActionViewModel[];
}

export function transitionRestoreStagingState(
  state: RestoreStagingStateView,
  event: RestoreStagingEvent,
): RestoreStagingTransitionResult {
  if (state === "Staging" && event === "ValidateRequested") {
    return { state: "Validating" };
  }
  if (state === "Validating" && event === "ValidationPassed") {
    return { state: "ReadyToApply" };
  }
  if (state === "Validating" && event === "ValidationFailed") {
    return { state: "Failed" };
  }
  if (state === "ReadyToApply" && event === "ApplyRequested") {
    return { state: "Applying" };
  }
  if (state === "Applying" && event === "ApplySucceeded") {
    return { state: "Completed" };
  }
  if (event === "Fail") {
    return { state: "Failed" };
  }
  if (event === "Reset") {
    return { state: "Staging" };
  }
  return {
    state: "Failed",
    errorCode: "RESTORE_STAGING_INVALID_TRANSITION",
  };
}

export function createRestoreStagingValidationModel(
  input: RestoreStagingValidationInput,
): RestoreStagingValidationViewModel {
  return {
    mode: "restore-staging-validation",
    stagingId: input.stagingId,
    state: input.state,
    manifest: createBackupArtifactManifestViewModel(input.manifest),
    issues: input.issues.map((issue) => ({
      code: sanitizeStableCode(issue.code),
      severity: issue.severity,
    })),
    canApply: input.state === "ReadyToApply" && input.issues.every((issue) => issue.severity !== "error"),
    requiresConfirmation: input.state === "ReadyToApply",
    currentWorkspaceMutationAllowed: false,
    actions: restoreStagingActions(input.state),
  };
}

function restoreStagingActions(state: RestoreStagingStateView): readonly RestoreStagingActionViewModel[] {
  if (state === "Staging" || state === "Failed") {
    return [{ id: "validate-restore-staging", label: "Validate restore" }];
  }
  if (state === "ReadyToApply") {
    return [{ id: "apply-restore-staging", label: "Apply restore" }];
  }
  if (state === "Completed") {
    return [{ id: "open-restored-workspace", label: "Open workspace" }];
  }
  return [];
}

function sanitizeBackupBucket(value: string): string {
  return sanitizeStableCode(value.toLowerCase());
}

function sanitizeBackupCategory(value: string): string {
  return sanitizeStableCode(value.toLowerCase());
}

function sanitizeStableCode(value: string): string {
  return value.replace(/[^a-zA-Z0-9_.:-]/g, "-");
}

export type ImportPreviewEvent =
  | "ScanRequested"
  | "ScanCompleted"
  | "ApplyRequested"
  | "ApplySucceeded"
  | "Fail"
  | "Reset";

export interface ImportPreviewTransitionResult {
  readonly state: ImportPreviewStateView;
  readonly errorCode?: string;
}

export interface ImportPreviewInput {
  readonly state: ImportPreviewStateView;
  readonly summary: ImportPreviewSummaryView;
  readonly conflicts: readonly ImportConflictItemView[];
}

export interface ImportPreviewActionViewModel {
  readonly id: "scan-import-source" | "rescan-import-source" | "apply-import-preview";
  readonly label: string;
}

export interface ImportPreviewViewModel {
  readonly mode: "import-preview";
  readonly state: ImportPreviewStateView;
  readonly sourceKind: ImportPreviewSummaryView["sourceKind"];
  readonly sourceHash: string;
  readonly scannedDocumentCount: number;
  readonly assetReferenceCount: number;
  readonly linkCount: number;
  readonly unsupportedItemCount: number;
  readonly estimatedByteSizeBucket: string;
  readonly conflicts: readonly ImportConflictItemView[];
  readonly conflictResolutionPolicies: readonly ImportConflictResolutionPolicyViewModel[];
  readonly canApply: boolean;
  readonly actions: readonly ImportPreviewActionViewModel[];
}

export type ImportConflictResolutionOption = "overwrite" | "rename" | "skip";

export interface ImportConflictResolutionPolicyViewModel {
  readonly code: string;
  readonly options: readonly ImportConflictResolutionOption[];
}

export function transitionImportPreviewState(
  state: ImportPreviewStateView,
  event: ImportPreviewEvent,
): ImportPreviewTransitionResult {
  if (state === "Selected" && event === "ScanRequested") {
    return { state: "Scanning" };
  }
  if (state === "Scanning" && event === "ScanCompleted") {
    return { state: "PreviewReady" };
  }
  if (state === "PreviewReady" && event === "ApplyRequested") {
    return { state: "Applying" };
  }
  if (state === "Applying" && event === "ApplySucceeded") {
    return { state: "Completed" };
  }
  if (event === "Fail") {
    return { state: "Failed" };
  }
  if (event === "Reset") {
    return { state: "Selected" };
  }
  return {
    state: "Failed",
    errorCode: "IMPORT_PREVIEW_INVALID_TRANSITION",
  };
}

export function createImportPreviewViewModel(input: ImportPreviewInput): ImportPreviewViewModel {
  const conflicts = input.conflicts.map((conflict) => ({
    code: sanitizeStableCode(conflict.code),
    severity: conflict.severity,
    count: conflict.count,
  }));
  const canApply =
    input.state === "PreviewReady" &&
    conflicts.every((conflict) => conflict.severity !== "blocking");
  return {
    mode: "import-preview",
    state: input.state,
    sourceKind: input.summary.sourceKind,
    sourceHash: createUiTextHash(input.summary.sourceHash),
    scannedDocumentCount: input.summary.scannedDocumentCount,
    assetReferenceCount: input.summary.assetReferenceCount,
    linkCount: input.summary.linkCount,
    unsupportedItemCount: input.summary.unsupportedItemCount ?? 0,
    estimatedByteSizeBucket: sanitizeBackupBucket(input.summary.estimatedByteSizeBucket),
    conflicts,
    conflictResolutionPolicies: conflicts.map(createImportConflictResolutionPolicy),
    canApply,
    actions: importPreviewActions(input.state, canApply),
  };
}

function createImportConflictResolutionPolicy(
  conflict: ImportConflictItemView,
): ImportConflictResolutionPolicyViewModel {
  if (conflict.severity === "blocking") {
    return { code: conflict.code, options: ["rename", "skip"] };
  }
  return { code: conflict.code, options: ["overwrite", "rename", "skip"] };
}

function importPreviewActions(
  state: ImportPreviewStateView,
  canApply: boolean,
): readonly ImportPreviewActionViewModel[] {
  if (state === "Selected") {
    return [{ id: "scan-import-source", label: "Scan source" }];
  }
  if (canApply) {
    return [{ id: "apply-import-preview", label: "Apply import" }];
  }
  if (state === "PreviewReady" || state === "Failed") {
    return [{ id: "rescan-import-source", label: "Rescan source" }];
  }
  return [];
}

function createUiTextHash(value: string): string {
  let hash = 0;
  for (const char of value) {
    hash = (hash * 31 + char.charCodeAt(0)) >>> 0;
  }
  return `uihash:${hash.toString(16)}`;
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
  readonly referencedDocumentCount: number;
  readonly previewState: "ready" | "unavailable" | "pending";
  readonly ocrState: "not-indexed" | "indexed" | "unavailable";
  readonly indexState: "Fresh" | "Stale" | "Rebuilding" | "RebuildFailed";
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
    referencedDocumentCount: asset.referencedDocumentCount ?? 0,
    previewState: asset.previewState ?? (asset.status === "missing" ? "unavailable" : "ready"),
    ocrState: asset.ocrState ?? "not-indexed",
    indexState: asset.indexState ?? "Fresh",
  };
}

export type AdminDisplayState =
  | "Unauthenticated"
  | "Authenticating"
  | "Authenticated"
  | "Error";

export interface SelfHostAdminConfigViewModel {
  readonly serverBaseUrl: string;
  readonly workspaceId: string;
}

export interface AdminSessionViewModel {
  readonly userId: string;
  readonly sessionStatus: AdminSessionView["sessionStatus"];
}

export interface AdminUserRowViewModel {
  readonly userId: string;
  readonly login: string;
  readonly email: string;
  readonly displayName: string;
  readonly status: AdminUserView["status"];
}

export interface AdminGroupRowViewModel {
  readonly workspaceId: string;
  readonly groupId: string;
  readonly name: string;
  readonly memberUserIds: readonly string[];
}

export interface AdminRoleAssignmentRowViewModel {
  readonly assignmentId: string;
  readonly workspaceId: string;
  readonly subject: RoleAssignmentSubjectView;
  readonly role: WorkspaceRole;
}

export interface AdminErrorViewModel {
  readonly code: CabinetApiErrorCode;
  readonly message: string;
  readonly retryable: boolean;
}

export interface SelfHostAdminViewModel {
  readonly displayState: AdminDisplayState;
  readonly serverBaseUrl: string;
  readonly workspaceId: string;
  readonly session?: AdminSessionViewModel;
  readonly users: readonly AdminUserRowViewModel[];
  readonly groups: readonly AdminGroupRowViewModel[];
  readonly roleAssignments: readonly AdminRoleAssignmentRowViewModel[];
  readonly error?: AdminErrorViewModel;
  readonly lastMembershipResult?: GroupMemberMutationResultView;
  readonly lastRoleAssignment?: AdminRoleAssignmentRowViewModel;
  readonly lastRoleRevocation?: RevokeRoleResultView;
}

export interface AdminLoginFormModel extends LoginCommand {
  readonly credential: string;
}

export interface AdminDevelopmentLogger {
  writeDevelopment(eventName: string, metadata?: Readonly<Record<string, string | number | boolean>>): void;
}

export type AdminDisplayEvent =
  | { readonly type: "login-submit" }
  | { readonly type: "logout" }
  | { readonly type: "api-success"; readonly session?: AdminSessionView }
  | { readonly type: "api-failure"; readonly error: unknown };

export function createInitialAdminViewModel(
  config: SelfHostAdminConfigViewModel,
): SelfHostAdminViewModel {
  return {
    displayState: "Unauthenticated",
    serverBaseUrl: config.serverBaseUrl,
    workspaceId: config.workspaceId,
    users: [],
    groups: [],
    roleAssignments: [],
  };
}

export function createAdminLoginFormModel(login: string, credential: string): AdminLoginFormModel {
  return {
    login,
    credential,
  };
}

export function transitionAdminDisplayState(
  state: SelfHostAdminViewModel,
  event: AdminDisplayEvent,
): SelfHostAdminViewModel {
  switch (event.type) {
    case "login-submit":
      return {
        ...state,
        displayState: "Authenticating",
        error: undefined,
      };
    case "logout":
      return {
        ...state,
        displayState: "Unauthenticated",
        session: undefined,
        error: undefined,
        users: [],
        groups: [],
        roleAssignments: [],
      };
    case "api-success":
      return {
        ...state,
        displayState: "Authenticated",
        session: event.session ? createAdminSessionViewModel(event.session) : state.session,
        error: undefined,
      };
    case "api-failure":
      return {
        ...state,
        displayState: "Error",
        error: mapApiClientErrorToAdminMessage(event.error),
      };
  }
}

export async function loginToSelfHostAdmin(
  state: SelfHostAdminViewModel,
  form: AdminLoginFormModel,
  client: CabinetAdminApiClient,
  developmentLogger?: AdminDevelopmentLogger,
): Promise<SelfHostAdminViewModel> {
  developmentLogger?.writeDevelopment("admin.login.submit");
  try {
    const session = await client.login(form);
    developmentLogger?.writeDevelopment("admin.login.success", {
      sessionStatus: session.sessionStatus,
    });
    return transitionAdminDisplayState(state, {
      type: "api-success",
      session,
    });
  } catch (error) {
    developmentLogger?.writeDevelopment("admin.login.failure");
    return transitionAdminDisplayState(state, {
      type: "api-failure",
      error,
    });
  }
}

export async function loadAdminWorkspaceViewModel(
  state: SelfHostAdminViewModel,
  client: CabinetAdminApiClient,
  developmentLogger?: AdminDevelopmentLogger,
): Promise<SelfHostAdminViewModel> {
  if (!state.session) {
    return transitionAdminDisplayState(state, {
      type: "api-failure",
      error: { code: "UNAUTHORIZED" },
    });
  }

  developmentLogger?.writeDevelopment("admin.workspace.load");
  try {
    const [userPage, groupPage, rolePage] = await Promise.all([
      client.listUsers(),
      client.listGroups({ workspaceId: state.workspaceId }),
      client.listRoleAssignments({ workspaceId: state.workspaceId }),
    ]);
    return {
      ...state,
      displayState: "Authenticated",
      users: userPage.users.map(createAdminUserRowViewModel),
      groups: groupPage.groups.map(createAdminGroupRowViewModel),
      roleAssignments: rolePage.assignments.map(createAdminRoleAssignmentRowViewModel),
      error: undefined,
    };
  } catch (error) {
    return transitionAdminDisplayState(state, {
      type: "api-failure",
      error,
    });
  }
}

export async function addAdminGroupMember(
  state: SelfHostAdminViewModel,
  groupId: string,
  userId: string,
  client: CabinetAdminApiClient,
  developmentLogger?: AdminDevelopmentLogger,
): Promise<SelfHostAdminViewModel> {
  const command: AddGroupMemberCommand = {
    workspaceId: state.workspaceId,
    groupId,
    userId,
  };
  developmentLogger?.writeDevelopment("admin.group.member.add");
  try {
    const result = await client.addGroupMember(command);
    return {
      ...state,
      displayState: "Authenticated",
      groups: applyMembershipResult(state.groups, result),
      lastMembershipResult: result,
      error: undefined,
    };
  } catch (error) {
    return transitionAdminDisplayState(state, { type: "api-failure", error });
  }
}

export async function removeAdminGroupMember(
  state: SelfHostAdminViewModel,
  groupId: string,
  userId: string,
  client: CabinetAdminApiClient,
  developmentLogger?: AdminDevelopmentLogger,
): Promise<SelfHostAdminViewModel> {
  const command: RemoveGroupMemberCommand = {
    workspaceId: state.workspaceId,
    groupId,
    userId,
  };
  developmentLogger?.writeDevelopment("admin.group.member.remove");
  try {
    const result = await client.removeGroupMember(command);
    return {
      ...state,
      displayState: "Authenticated",
      groups: applyMembershipResult(state.groups, result),
      lastMembershipResult: result,
      error: undefined,
    };
  } catch (error) {
    return transitionAdminDisplayState(state, { type: "api-failure", error });
  }
}

export async function assignAdminWorkspaceRole(
  state: SelfHostAdminViewModel,
  subject: RoleAssignmentSubjectView,
  role: WorkspaceRole,
  client: CabinetAdminApiClient,
  developmentLogger?: AdminDevelopmentLogger,
): Promise<SelfHostAdminViewModel> {
  developmentLogger?.writeDevelopment("admin.role.assign");
  try {
    const assignment = createAdminRoleAssignmentRowViewModel(
      await client.assignWorkspaceRole({
        workspaceId: state.workspaceId,
        subject,
        role,
      }),
    );
    return {
      ...state,
      displayState: "Authenticated",
      roleAssignments: upsertRoleAssignment(state.roleAssignments, assignment),
      lastRoleAssignment: assignment,
      error: undefined,
    };
  } catch (error) {
    return transitionAdminDisplayState(state, { type: "api-failure", error });
  }
}

export async function revokeAdminWorkspaceRole(
  state: SelfHostAdminViewModel,
  assignmentId: string,
  client: CabinetAdminApiClient,
  developmentLogger?: AdminDevelopmentLogger,
): Promise<SelfHostAdminViewModel> {
  developmentLogger?.writeDevelopment("admin.role.revoke");
  try {
    const result = await client.revokeWorkspaceRole({
      workspaceId: state.workspaceId,
      assignmentId,
    });
    return {
      ...state,
      displayState: "Authenticated",
      roleAssignments: state.roleAssignments.filter(
        (assignment) => assignment.assignmentId !== result.assignmentId,
      ),
      lastRoleRevocation: result,
      error: undefined,
    };
  } catch (error) {
    return transitionAdminDisplayState(state, { type: "api-failure", error });
  }
}

export function mapApiClientErrorToAdminMessage(error: unknown): AdminErrorViewModel {
  const code = errorCodeOf(error);
  switch (code) {
    case "UNAUTHORIZED":
      return {
        code,
        message: "Sign in again to continue.",
        retryable: false,
      };
    case "SESSION_EXPIRED":
      return {
        code,
        message: "The session expired. Sign in again.",
        retryable: false,
      };
    case "NETWORK_FAILURE":
      return {
        code,
        message: "The server is unreachable. Check the self-host server address.",
        retryable: true,
      };
    case "VALIDATION_ERROR":
      return {
        code,
        message: "Review the submitted values and try again.",
        retryable: false,
      };
    default:
      return {
        code,
        message: "The admin request failed.",
        retryable: false,
      };
  }
}

function createAdminSessionViewModel(session: AdminSessionView): AdminSessionViewModel {
  return {
    userId: session.userId,
    sessionStatus: session.sessionStatus,
  };
}

function createAdminUserRowViewModel(user: AdminUserView): AdminUserRowViewModel {
  return {
    userId: user.userId,
    login: user.login,
    email: user.email,
    displayName: user.displayName,
    status: user.status,
  };
}

function createAdminGroupRowViewModel(group: AdminGroupView): AdminGroupRowViewModel {
  return {
    workspaceId: group.workspaceId,
    groupId: group.groupId,
    name: group.name,
    memberUserIds: [...group.memberUserIds],
  };
}

function createAdminRoleAssignmentRowViewModel(
  assignment: RoleAssignmentView,
): AdminRoleAssignmentRowViewModel {
  return {
    assignmentId: assignment.assignmentId,
    workspaceId: assignment.workspaceId,
    subject: assignment.subject,
    role: assignment.role,
  };
}

function applyMembershipResult(
  groups: readonly AdminGroupRowViewModel[],
  result: GroupMemberMutationResultView,
): readonly AdminGroupRowViewModel[] {
  return groups.map((group) => {
    if (group.groupId !== result.groupId) {
      return group;
    }
    if (result.result === "removed") {
      return {
        ...group,
        memberUserIds: group.memberUserIds.filter((memberUserId) => memberUserId !== result.userId),
      };
    }
    if (group.memberUserIds.includes(result.userId)) {
      return group;
    }
    return {
      ...group,
      memberUserIds: [...group.memberUserIds, result.userId],
    };
  });
}

function upsertRoleAssignment(
  assignments: readonly AdminRoleAssignmentRowViewModel[],
  nextAssignment: AdminRoleAssignmentRowViewModel,
): readonly AdminRoleAssignmentRowViewModel[] {
  const existingIndex = assignments.findIndex(
    (assignment) => assignment.assignmentId === nextAssignment.assignmentId,
  );
  if (existingIndex < 0) {
    return [...assignments, nextAssignment];
  }
  return assignments.map((assignment, index) =>
    index === existingIndex ? nextAssignment : assignment,
  );
}

function errorCodeOf(error: unknown): CabinetApiErrorCode {
  if (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    typeof error.code === "string"
  ) {
    return error.code;
  }
  return "API_ERROR";
}

export type CollaborationDisplayState = "Idle" | "Loading" | "Loaded" | "Submitting" | "Error";

export interface CollaborationDocumentConfig {
  readonly workspaceId: string;
  readonly documentId: string;
}

export interface CollaborationCurrentDocumentViewModel {
  readonly workspaceId: string;
  readonly documentId: string;
  readonly title: string;
  readonly path: string;
  readonly versionId: string;
  readonly permissionDecision: PermissionDecisionView;
}

export interface CollaborationSharingEntryViewModel {
  readonly subject: SharingSubjectView;
  readonly permission: CollaborationPermission;
  readonly effect: SharingEffect;
}

export interface CollaborationSharingPanelViewModel {
  readonly workspaceId: string;
  readonly documentId: string;
  readonly entries: readonly CollaborationSharingEntryViewModel[];
  readonly effectivePermissions: readonly CollaborationPermission[];
}

export interface CollaborationSearchPanelViewModel {
  readonly text: string;
  readonly results: readonly SearchResultItemViewModel[];
  readonly permissionFilteredCount: number;
  readonly durationMs: number;
}

export interface CollaborationErrorViewModel extends AdminErrorViewModel {}

export interface CollaborationViewModel {
  readonly displayState: CollaborationDisplayState;
  readonly workspaceId: string;
  readonly documentId: string;
  readonly currentDocument?: CollaborationCurrentDocumentViewModel;
  readonly sharing?: CollaborationSharingPanelViewModel;
  readonly search?: CollaborationSearchPanelViewModel;
  readonly commentThreads: readonly CommentThreadView[];
  readonly reviewRequests: readonly ReviewRequestView[];
  readonly lock?: DocumentLockView;
  readonly auditEvents: readonly AuditEventView[];
  readonly error?: CollaborationErrorViewModel;
  readonly lastMembershipResult?: never;
  readonly lastInlineAnchorStatus?: InlineAnchorStatusView;
  readonly lastReviewAction?: ReviewWorkflowActionView;
}

export interface CollaborationDevelopmentLogger {
  writeDevelopment(eventName: string, metadata?: Readonly<Record<string, string | number | boolean>>): void;
}

export interface EditorInlineAnchorDraft {
  readonly versionId: string;
  readonly startOffset: number;
  readonly endOffset: number;
}

export interface EditorInlineAnchorAdapter {
  getInlineAnchor(): EditorInlineAnchorDraft;
}

export interface CollaborationSharingUpdateDraft {
  readonly subject: SharingSubjectView;
  readonly permission: CollaborationPermission;
  readonly effect: SharingEffect;
}

export function createInitialCollaborationViewModel(
  config: CollaborationDocumentConfig,
): CollaborationViewModel {
  return {
    displayState: "Idle",
    workspaceId: config.workspaceId,
    documentId: config.documentId,
    commentThreads: [],
    reviewRequests: [],
    auditEvents: [],
  };
}

export async function loadCollaborationDocumentViewModel(
  state: CollaborationViewModel,
  client: CabinetCollaborationApiClient,
  developmentLogger?: CollaborationDevelopmentLogger,
): Promise<CollaborationViewModel> {
  developmentLogger?.writeDevelopment("collaboration.load");
  try {
    const [document, sharing, comments, reviews, lock, audit] = await Promise.all([
      client.getAccessibleDocument({
        workspaceId: state.workspaceId,
        documentId: state.documentId,
      }),
      client.getDocumentSharing({
        workspaceId: state.workspaceId,
        documentId: state.documentId,
      }),
      client.listDocumentComments({
        workspaceId: state.workspaceId,
        documentId: state.documentId,
      }),
      client.listReviewRequests({
        workspaceId: state.workspaceId,
        documentId: state.documentId,
      }),
      client.getDocumentLock({
        workspaceId: state.workspaceId,
        documentId: state.documentId,
      }),
      client.listAuditEvents({
        workspaceId: state.workspaceId,
        scope: "workspace",
        limit: 50,
      }),
    ]);
    return {
      ...state,
      displayState: "Loaded",
      currentDocument: createCollaborationCurrentDocumentViewModel(document),
      sharing: createCollaborationSharingPanelViewModel(sharing),
      commentThreads: comments.threads,
      reviewRequests: reviews.requests,
      lock,
      auditEvents: audit.events,
      error: undefined,
    };
  } catch (error) {
    return collaborationErrorState(state, error);
  }
}

export async function searchCollaborationDocuments(
  state: CollaborationViewModel,
  text: string,
  client: CabinetCollaborationApiClient,
  developmentLogger?: CollaborationDevelopmentLogger,
): Promise<CollaborationViewModel> {
  developmentLogger?.writeDevelopment("collaboration.search");
  try {
    const page = await client.searchAccessibleDocuments({
      workspaceId: state.workspaceId,
      text,
      limit: 20,
    });
    return {
      ...state,
      displayState: "Loaded",
      search: createCollaborationSearchPanelViewModel(page),
      error: undefined,
    };
  } catch (error) {
    return collaborationErrorState(state, error);
  }
}

export async function updateCollaborationSharing(
  state: CollaborationViewModel,
  draft: CollaborationSharingUpdateDraft,
  client: CabinetCollaborationApiClient,
  developmentLogger?: CollaborationDevelopmentLogger,
): Promise<CollaborationViewModel> {
  developmentLogger?.writeDevelopment("collaboration.sharing.update");
  try {
    const sharing = await client.updateDocumentSharing({
      workspaceId: state.workspaceId,
      documentId: state.documentId,
      subject: draft.subject,
      permission: draft.permission,
      effect: draft.effect,
    });
    return {
      ...state,
      displayState: "Loaded",
      sharing: createCollaborationSharingPanelViewModel(sharing),
      error: undefined,
    };
  } catch (error) {
    return collaborationErrorState(state, error);
  }
}

export async function addCollaborationComment(
  state: CollaborationViewModel,
  body: string,
  client: CabinetCollaborationApiClient,
  developmentLogger?: CollaborationDevelopmentLogger,
): Promise<CollaborationViewModel> {
  developmentLogger?.writeDevelopment("collaboration.comment.add");
  try {
    const threadId = state.commentThreads[0]?.threadId ?? "thread-1";
    const mutation = await client.addDocumentComment({
      workspaceId: state.workspaceId,
      documentId: state.documentId,
      threadId,
      commentId: nextCommentId(state.commentThreads),
      body,
    });
    return applyCommentMutation(state, mutation.thread);
  } catch (error) {
    return collaborationErrorState(state, error);
  }
}

export async function addCollaborationInlineComment(
  state: CollaborationViewModel,
  body: string,
  anchorAdapter: EditorInlineAnchorAdapter,
  client: CabinetCollaborationApiClient,
  developmentLogger?: CollaborationDevelopmentLogger,
): Promise<CollaborationViewModel> {
  developmentLogger?.writeDevelopment("collaboration.inline_comment.add");
  const anchor = anchorAdapter.getInlineAnchor();
  try {
    const threadId = state.commentThreads[0]?.threadId ?? "thread-1";
    const mutation = await client.addInlineDocumentComment({
      workspaceId: state.workspaceId,
      documentId: state.documentId,
      threadId,
      commentId: nextCommentId(state.commentThreads),
      body,
      versionId: anchor.versionId,
      startOffset: anchor.startOffset,
      endOffset: anchor.endOffset,
    });
    return {
      ...applyCommentMutation(state, mutation.thread),
      lastInlineAnchorStatus: mutation.anchorStatus,
    };
  } catch (error) {
    return collaborationErrorState(state, error);
  }
}

export async function resolveCollaborationComment(
  state: CollaborationViewModel,
  threadId: string,
  client: CabinetCollaborationApiClient,
  developmentLogger?: CollaborationDevelopmentLogger,
): Promise<CollaborationViewModel> {
  developmentLogger?.writeDevelopment("collaboration.comment.resolve");
  try {
    const mutation = await client.resolveDocumentComment({
      workspaceId: state.workspaceId,
      documentId: state.documentId,
      threadId,
    });
    return applyCommentMutation(state, mutation.thread);
  } catch (error) {
    return collaborationErrorState(state, error);
  }
}

export async function reopenCollaborationComment(
  state: CollaborationViewModel,
  threadId: string,
  client: CabinetCollaborationApiClient,
  developmentLogger?: CollaborationDevelopmentLogger,
): Promise<CollaborationViewModel> {
  developmentLogger?.writeDevelopment("collaboration.comment.reopen");
  try {
    const mutation = await client.reopenDocumentComment({
      workspaceId: state.workspaceId,
      documentId: state.documentId,
      threadId,
    });
    return applyCommentMutation(state, mutation.thread);
  } catch (error) {
    return collaborationErrorState(state, error);
  }
}

export async function requestCollaborationReview(
  state: CollaborationViewModel,
  reviewRequestId: string,
  client: CabinetCollaborationApiClient,
  developmentLogger?: CollaborationDevelopmentLogger,
): Promise<CollaborationViewModel> {
  developmentLogger?.writeDevelopment("collaboration.review.request");
  try {
    const action = await client.requestDocumentReview({
      workspaceId: state.workspaceId,
      documentId: state.documentId,
      reviewRequestId,
    });
    return {
      ...state,
      displayState: "Loaded",
      lastReviewAction: action,
      error: undefined,
    };
  } catch (error) {
    return collaborationErrorState(state, error);
  }
}

export async function publishCollaborationDocument(
  state: CollaborationViewModel,
  client: CabinetCollaborationApiClient,
  developmentLogger?: CollaborationDevelopmentLogger,
): Promise<CollaborationViewModel> {
  developmentLogger?.writeDevelopment("collaboration.publish");
  try {
    const action = await client.publishDocument({
      workspaceId: state.workspaceId,
      documentId: state.documentId,
    });
    return {
      ...state,
      displayState: "Loaded",
      lastReviewAction: action,
      error: undefined,
    };
  } catch (error) {
    return collaborationErrorState(state, error);
  }
}

function createCollaborationCurrentDocumentViewModel(
  document: AccessibleDocumentView,
): CollaborationCurrentDocumentViewModel {
  return {
    workspaceId: document.workspaceId,
    documentId: document.documentId,
    title: document.title,
    path: document.path,
    versionId: document.versionId,
    permissionDecision: document.permissionDecision,
  };
}

function createCollaborationSharingPanelViewModel(
  sharing: DocumentSharingView,
): CollaborationSharingPanelViewModel {
  return {
    workspaceId: sharing.workspaceId,
    documentId: sharing.documentId,
    entries: sharing.entries.map((entry) => ({ ...entry })),
    effectivePermissions: [...sharing.effectivePermissions],
  };
}

function createCollaborationSearchPanelViewModel(
  page: SearchAccessibleDocumentsView,
): CollaborationSearchPanelViewModel {
  return {
    text: page.text,
    results: page.results.map(createSearchResultItemViewModel),
    permissionFilteredCount: page.permissionFilteredCount,
    durationMs: page.durationMs,
  };
}

function applyCommentMutation(
  state: CollaborationViewModel,
  thread: CommentThreadView,
): CollaborationViewModel {
  return {
    ...state,
    displayState: "Loaded",
    commentThreads: upsertCommentThread(state.commentThreads, thread),
    error: undefined,
  };
}

function upsertCommentThread(
  threads: readonly CommentThreadView[],
  nextThread: CommentThreadView,
): readonly CommentThreadView[] {
  const existing = threads.findIndex((thread) => thread.threadId === nextThread.threadId);
  if (existing < 0) {
    return [...threads, nextThread];
  }
  return threads.map((thread, index) => (index === existing ? nextThread : thread));
}

function nextCommentId(threads: readonly CommentThreadView[]): string {
  const commentCount = threads.reduce((count, thread) => count + thread.comments.length, 0);
  return `comment-${commentCount + 1}`;
}

function collaborationErrorState(
  state: CollaborationViewModel,
  error: unknown,
): CollaborationViewModel {
  return {
    ...state,
    displayState: "Error",
    error: mapApiClientErrorToAdminMessage(error),
  };
}

export type DocumentNavigatorDisplayState =
  | "Closed"
  | "Loading"
  | "Ready"
  | "Filtering"
  | "EmptyResult"
  | "Degraded"
  | "Failed";

export interface DocumentNavigatorModel {
  readonly workspaceId: string;
  readonly view: DocumentNavigatorView;
  readonly viewKey?: string;
  readonly filter?: string;
  readonly generation: number;
  readonly displayState: DocumentNavigatorDisplayState;
  readonly items: readonly DocumentNavigatorItem[];
  readonly nextCursor?: string | null;
  readonly error?: {
    readonly code: string;
    readonly retryable: boolean;
  };
}

export interface DocumentNavigatorLoadingInput {
  readonly workspaceId: string;
  readonly view: DocumentNavigatorView;
  readonly viewKey?: string;
  readonly filter?: string;
  readonly generation: number;
}

export function createDocumentNavigatorLoadingModel(
  input: DocumentNavigatorLoadingInput,
): DocumentNavigatorModel {
  return {
    workspaceId: input.workspaceId,
    view: input.view,
    viewKey: normalizeOptionalText(input.viewKey),
    filter: normalizeOptionalText(input.filter),
    generation: input.generation,
    displayState: "Loading",
    items: [],
  };
}

export function createDocumentNavigatorFailedModel(
  input: DocumentNavigatorLoadingInput & {
    readonly errorCode: string;
    readonly retryable: boolean;
  },
): DocumentNavigatorModel {
  return {
    ...createDocumentNavigatorLoadingModel(input),
    displayState: "Failed",
    error: { code: input.errorCode, retryable: input.retryable },
  };
}

export function createDocumentNavigatorQuery(
  input: Omit<DocumentNavigatorQuery, "viewKey" | "filter"> & {
    readonly viewKey?: string;
    readonly filter?: string;
  },
): DocumentNavigatorQuery | undefined {
  const workspaceId = input.workspaceId.trim();
  const viewKey = normalizeOptionalText(input.viewKey);
  const filter = normalizeOptionalText(input.filter);
  const keyedView = input.view === "Collection" || input.view === "Tag";
  if (
    workspaceId.length === 0 ||
    !Number.isInteger(input.limit) ||
    input.limit < 1 ||
    input.limit > 100 ||
    (keyedView && !viewKey) ||
    (!keyedView && viewKey)
  ) {
    return undefined;
  }
  return {
    workspaceId,
    view: input.view,
    ...(viewKey ? { viewKey } : {}),
    ...(filter ? { filter } : {}),
    limit: input.limit,
    ...(input.cursor ? { cursor: input.cursor } : {}),
  };
}

export function applyDocumentNavigatorResult(
  model: DocumentNavigatorModel,
  generation: number,
  result: DocumentNavigatorResult,
): DocumentNavigatorModel {
  if (generation !== model.generation) return model;
  if (result.workspaceId !== model.workspaceId || result.view !== model.view) {
    return navigatorInvalidTransition(model);
  }
  return {
    ...model,
    displayState: result.state,
    items: result.items.map((item) => ({
      ...item,
      collections: [...item.collections],
      tags: [...item.tags],
    })),
    nextCursor: result.nextCursor,
    error: undefined,
  };
}

export type DocumentNavigatorModelEvent =
  | { readonly type: "OpenRequested"; readonly generation: number }
  | {
      readonly type: "ViewSelected";
      readonly view: DocumentNavigatorView;
      readonly viewKey?: string;
      readonly generation: number;
    }
  | { readonly type: "FilterChanged"; readonly filter: string; readonly generation: number }
  | { readonly type: "RetryRequested"; readonly generation: number }
  | { readonly type: "CloseRequested" };

export function transitionDocumentNavigatorModel(
  model: DocumentNavigatorModel,
  event: DocumentNavigatorModelEvent,
): DocumentNavigatorModel {
  if (event.type === "CloseRequested" && model.displayState !== "Closed") {
    return { ...model, displayState: "Closed", items: [], error: undefined };
  }
  if (event.type === "OpenRequested" && model.displayState === "Closed") {
    return { ...model, displayState: "Loading", generation: event.generation, error: undefined };
  }
  if (
    event.type === "ViewSelected" &&
    model.displayState !== "Closed" &&
    model.displayState !== "Failed"
  ) {
    return {
      ...model,
      view: event.view,
      viewKey: normalizeOptionalText(event.viewKey),
      filter: undefined,
      generation: event.generation,
      displayState: "Loading",
      items: [],
      error: undefined,
    };
  }
  if (
    event.type === "FilterChanged" &&
    model.displayState !== "Closed" &&
    model.displayState !== "Failed"
  ) {
    return {
      ...model,
      filter: normalizeOptionalText(event.filter),
      generation: event.generation,
      displayState: "Filtering",
      items: [],
      error: undefined,
    };
  }
  if (
    event.type === "RetryRequested" &&
    model.displayState === "Failed" &&
    model.error?.retryable
  ) {
    return {
      ...model,
      generation: event.generation,
      displayState: model.filter ? "Filtering" : "Loading",
      items: [],
      error: undefined,
    };
  }
  return navigatorInvalidTransition(model);
}

function navigatorInvalidTransition(model: DocumentNavigatorModel): DocumentNavigatorModel {
  return {
    ...model,
    displayState: "Failed",
    items: [],
    error: {
      code: "DOCUMENT_NAVIGATOR_INVALID_TRANSITION",
      retryable: false,
    },
  };
}

function normalizeOptionalText(value: string | undefined): string | undefined {
  const normalized = value?.trim().toLowerCase();
  return normalized ? normalized : undefined;
}

export const DocumentSaveCoordinatorState = Object.freeze({
  NoDocument: "NoDocument",
  Loading: "Loading",
  Clean: "Clean",
  Dirty: "Dirty",
  SaveQueued: "SaveQueued",
  Saving: "Saving",
  Saved: "Saved",
  SaveFailed: "SaveFailed",
  CloseBlocked: "CloseBlocked",
  ReadOnlyRecovery: "ReadOnlyRecovery",
});

export type DocumentSaveCoordinatorStateValue =
  (typeof DocumentSaveCoordinatorState)[keyof typeof DocumentSaveCoordinatorState];

export const DocumentSaveCoordinatorEvent = Object.freeze({
  DocumentOpened: "DocumentOpened",
  ContentChanged: "ContentChanged",
  AutosaveElapsed: "AutosaveElapsed",
  SaveRequested: "SaveRequested",
  SaveStarted: "SaveStarted",
  SaveSucceeded: "SaveSucceeded",
  SaveFailed: "SaveFailed",
  RetryRequested: "RetryRequested",
  CloseRequested: "CloseRequested",
  CloseCancelled: "CloseCancelled",
  DiscardConfirmed: "DiscardConfirmed",
  ReadOnlyEntered: "ReadOnlyEntered",
  EditResumed: "EditResumed",
});

export interface DocumentSaveCoordinatorSnapshot {
  readonly state: DocumentSaveCoordinatorStateValue;
  readonly autosaveDelayMs: number;
  readonly currentRevision: number;
  readonly persistedRevision: number;
  readonly dirtyContentRef?: string;
  readonly inFlightRevision?: number;
  readonly expectedVersionId?: string;
  readonly errorCode?: string;
  readonly closeReturnState?: DocumentSaveCoordinatorStateValue;
}

export type DocumentSaveCoordinatorTransitionEvent =
  | { readonly type: "DocumentOpened"; readonly revision: number; readonly versionId: string }
  | { readonly type: "ContentChanged"; readonly revision: number; readonly contentRef: string }
  | { readonly type: "AutosaveElapsed"; readonly elapsedMs: number }
  | { readonly type: "SaveRequested" }
  | { readonly type: "SaveStarted"; readonly revision: number }
  | { readonly type: "SaveSucceeded"; readonly revision: number; readonly savedVersionId: string }
  | { readonly type: "SaveFailed"; readonly revision: number; readonly errorCode: string }
  | { readonly type: "RetryRequested" }
  | { readonly type: "CloseRequested" }
  | { readonly type: "CloseCancelled" }
  | { readonly type: "DiscardConfirmed" }
  | { readonly type: "ReadOnlyEntered" }
  | { readonly type: "EditResumed" };

export interface DocumentSaveSideEffectRequest {
  readonly type: "StartSave";
  readonly revision: number;
  readonly contentRef: string;
  readonly expectedVersionId?: string;
}

export interface DocumentSaveCoordinatorTransitionResult {
  readonly snapshot: DocumentSaveCoordinatorSnapshot;
  readonly sideEffect?: DocumentSaveSideEffectRequest;
  readonly recoveryChoices?: readonly ["RetrySave", "Discard", "Cancel"];
  readonly ignored?: boolean;
  readonly errorCode?: string;
}

export function createDocumentSaveCoordinator(
  policy: { readonly autosaveDelayMs: number } = { autosaveDelayMs: 800 },
): DocumentSaveCoordinatorSnapshot {
  return {
    state: DocumentSaveCoordinatorState.NoDocument,
    autosaveDelayMs: policy.autosaveDelayMs,
    currentRevision: 0,
    persistedRevision: 0,
  };
}

export function transitionDocumentSaveCoordinator(
  snapshot: DocumentSaveCoordinatorSnapshot,
  event: DocumentSaveCoordinatorTransitionEvent,
): DocumentSaveCoordinatorTransitionResult {
  if (
    event.type === DocumentSaveCoordinatorEvent.DocumentOpened &&
    [DocumentSaveCoordinatorState.NoDocument, DocumentSaveCoordinatorState.Loading].includes(snapshot.state)
  ) {
    return {
      snapshot: {
        ...snapshot,
        state: DocumentSaveCoordinatorState.Clean,
        currentRevision: event.revision,
        persistedRevision: event.revision,
        expectedVersionId: event.versionId,
        dirtyContentRef: undefined,
        errorCode: undefined,
      },
    };
  }
  if (
    event.type === DocumentSaveCoordinatorEvent.ContentChanged &&
    event.revision > snapshot.currentRevision &&
    ![DocumentSaveCoordinatorState.NoDocument, DocumentSaveCoordinatorState.Loading,
      DocumentSaveCoordinatorState.CloseBlocked, DocumentSaveCoordinatorState.ReadOnlyRecovery].includes(snapshot.state)
  ) {
    return {
      snapshot: {
        ...snapshot,
        state: snapshot.state === DocumentSaveCoordinatorState.Saving
          ? DocumentSaveCoordinatorState.Saving
          : DocumentSaveCoordinatorState.Dirty,
        currentRevision: event.revision,
        dirtyContentRef: event.contentRef,
        errorCode: undefined,
      },
    };
  }
  if (
    event.type === DocumentSaveCoordinatorEvent.AutosaveElapsed &&
    snapshot.state === DocumentSaveCoordinatorState.Dirty
  ) {
    return event.elapsedMs >= snapshot.autosaveDelayMs
      ? queueSave(snapshot)
      : { snapshot };
  }
  if (
    event.type === DocumentSaveCoordinatorEvent.SaveRequested &&
    [DocumentSaveCoordinatorState.Dirty, DocumentSaveCoordinatorState.SaveFailed].includes(snapshot.state)
  ) {
    return queueSave(snapshot);
  }
  if (
    event.type === DocumentSaveCoordinatorEvent.SaveRequested &&
    snapshot.state === DocumentSaveCoordinatorState.Saving
  ) {
    return { snapshot };
  }
  if (
    event.type === DocumentSaveCoordinatorEvent.SaveStarted &&
    snapshot.state === DocumentSaveCoordinatorState.SaveQueued &&
    event.revision === snapshot.currentRevision
  ) {
    return {
      snapshot: {
        ...snapshot,
        state: DocumentSaveCoordinatorState.Saving,
        inFlightRevision: event.revision,
      },
    };
  }
  if (
    (event.type === DocumentSaveCoordinatorEvent.SaveSucceeded ||
      event.type === DocumentSaveCoordinatorEvent.SaveFailed) &&
    snapshot.state === DocumentSaveCoordinatorState.Saving &&
    event.revision !== snapshot.inFlightRevision
  ) {
    return { snapshot, ignored: true };
  }
  if (
    event.type === DocumentSaveCoordinatorEvent.SaveSucceeded &&
    snapshot.state === DocumentSaveCoordinatorState.Saving &&
    event.revision === snapshot.inFlightRevision
  ) {
    const persisted = {
      ...snapshot,
      persistedRevision: event.revision,
      expectedVersionId: event.savedVersionId,
      inFlightRevision: undefined,
      errorCode: undefined,
    };
    if (snapshot.currentRevision > event.revision) {
      return queueSave({ ...persisted, state: DocumentSaveCoordinatorState.Dirty });
    }
    return {
      snapshot: {
        ...persisted,
        state: DocumentSaveCoordinatorState.Saved,
        dirtyContentRef: undefined,
      },
    };
  }
  if (
    event.type === DocumentSaveCoordinatorEvent.SaveFailed &&
    snapshot.state === DocumentSaveCoordinatorState.Saving &&
    event.revision === snapshot.inFlightRevision
  ) {
    return {
      snapshot: {
        ...snapshot,
        state: DocumentSaveCoordinatorState.SaveFailed,
        inFlightRevision: undefined,
        errorCode: event.errorCode,
      },
    };
  }
  if (
    event.type === DocumentSaveCoordinatorEvent.RetryRequested &&
    snapshot.state === DocumentSaveCoordinatorState.SaveFailed
  ) {
    return queueSave(snapshot);
  }
  if (
    event.type === DocumentSaveCoordinatorEvent.CloseRequested &&
    [DocumentSaveCoordinatorState.Dirty, DocumentSaveCoordinatorState.SaveQueued,
      DocumentSaveCoordinatorState.Saving, DocumentSaveCoordinatorState.SaveFailed,
      DocumentSaveCoordinatorState.ReadOnlyRecovery].includes(snapshot.state)
  ) {
    return {
      snapshot: {
        ...snapshot,
        state: DocumentSaveCoordinatorState.CloseBlocked,
        closeReturnState: snapshot.state,
      },
      recoveryChoices: ["RetrySave", "Discard", "Cancel"],
    };
  }
  if (
    event.type === DocumentSaveCoordinatorEvent.CloseCancelled &&
    snapshot.state === DocumentSaveCoordinatorState.CloseBlocked
  ) {
    return {
      snapshot: {
        ...snapshot,
        state: snapshot.closeReturnState ?? DocumentSaveCoordinatorState.Dirty,
        closeReturnState: undefined,
      },
    };
  }
  if (
    event.type === DocumentSaveCoordinatorEvent.DiscardConfirmed &&
    snapshot.state === DocumentSaveCoordinatorState.CloseBlocked
  ) {
    return { snapshot: createDocumentSaveCoordinator({ autosaveDelayMs: snapshot.autosaveDelayMs }) };
  }
  if (
    event.type === DocumentSaveCoordinatorEvent.ReadOnlyEntered &&
    [DocumentSaveCoordinatorState.Dirty, DocumentSaveCoordinatorState.SaveQueued,
      DocumentSaveCoordinatorState.Saving, DocumentSaveCoordinatorState.SaveFailed].includes(snapshot.state)
  ) {
    return {
      snapshot: {
        ...snapshot,
        state: DocumentSaveCoordinatorState.ReadOnlyRecovery,
        inFlightRevision: undefined,
      },
    };
  }
  if (
    event.type === DocumentSaveCoordinatorEvent.EditResumed &&
    snapshot.state === DocumentSaveCoordinatorState.ReadOnlyRecovery
  ) {
    return { snapshot: { ...snapshot, state: DocumentSaveCoordinatorState.Dirty } };
  }
  return { snapshot, errorCode: "DOCUMENT_SAVE_INVALID_TRANSITION" };
}

function queueSave(
  snapshot: DocumentSaveCoordinatorSnapshot,
): DocumentSaveCoordinatorTransitionResult {
  if (!snapshot.dirtyContentRef || snapshot.currentRevision <= snapshot.persistedRevision) {
    return { snapshot, errorCode: "DOCUMENT_SAVE_INVALID_TRANSITION" };
  }
  return {
    snapshot: {
      ...snapshot,
      state: DocumentSaveCoordinatorState.SaveQueued,
      errorCode: undefined,
    },
    sideEffect: {
      type: "StartSave",
      revision: snapshot.currentRevision,
      contentRef: snapshot.dirtyContentRef,
      expectedVersionId: snapshot.expectedVersionId,
    },
  };
}
