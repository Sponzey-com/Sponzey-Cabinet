import React, { useCallback, useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";

import {
  LocalDesktopCommandClientError,
  createLocalDesktopCommandClient,
  createPersonalLocalDesktopCapabilityProfile,
} from "@sponzey-cabinet/client-core";
import {
  createPersonalWorkspaceHomeFailedModel,
  createPersonalWorkspaceHomeModel,
  createDocumentNavigatorFailedModel,
  createDocumentNavigatorLoadingModel,
  transitionDocumentNavigatorModel,
  DocumentSaveCoordinatorState,
  type DocumentNavigatorModel,
  type PersonalWorkspaceHomeModel,
} from "@sponzey-cabinet/ui";
import { applyMarkdownFormattingCommand } from "@sponzey-cabinet/editor";
import type { DocumentDiffQuery, DocumentNavigatorView } from "@sponzey-cabinet/client-core";

import { loadDesktopDocumentNavigator } from "./desktop_navigator_controller.ts";
import {
  cancelDesktopBackupOperation,
  cancelDesktopRestore,
  createDesktopBackupRecoverySnapshot,
  dismissDesktopRestoreConfirmation,
  loadDesktopBackupCatalog,
  pollDesktopBackupOperation,
  pollDesktopRestoreOperation,
  previewDesktopRestore,
  recoverDesktopBackupStartup,
  startDesktopBackupOperation,
  startDesktopRestoreOperation,
  selectDesktopBackupCatalogPackage,
  type DesktopBackupRecoverySnapshot,
} from "./desktop_backup_recovery_controller.ts";
import {
  createDesktopLinkOverviewSnapshot,
  loadDesktopLinkOverview,
  requestDesktopLinkOverviewLoad,
  type DesktopLinkOverviewSnapshot,
} from "./desktop_link_overview_controller.ts";
import {
  applyDesktopAssetDragState,
  applyDesktopAssetDetailResult,
  createDesktopAssetPlacementOptions,
  createDesktopAssetSnapshot,
  beginDesktopAssetImport,
  cancelDesktopAssetImport,
  importDesktopDocumentAssets,
  linkDesktopSelectedAsset,
  loadDesktopAssetDetail,
  loadDesktopAssetPreview,
  openDesktopSelectedAsset,
  loadDesktopDocumentAssets,
  loadDesktopWorkspaceAssets,
  requestDesktopAssetLoad,
  requestDesktopAssetPreview,
  requestDesktopAssetOpen,
  requestDesktopWorkspaceAssetLoad,
  requestDesktopWorkspaceAssetNextPage,
  repairDesktopAttachmentProjection,
  setDesktopAssetMediaFilter,
  setDesktopAssetQuery,
  selectDesktopAsset,
  closeDesktopAssetPreview,
  unlinkDesktopSelectedAsset,
  type DesktopAssetSurfaceSnapshot,
} from "./desktop_asset_controller.ts";
import {
  beginDesktopCanvasDrag,
  beginDesktopCanvasResize,
  cancelDesktopCanvasArrangePreview,
  createDesktopCanvas,
  createDesktopCanvasSnapshot,
  createDesktopCanvasViewportDraft,
  finishDesktopCanvasDrag,
  finishDesktopCanvasResize,
  loadDesktopCanvas,
  requestDesktopCanvasLoad,
  requestDesktopCanvasArrangeApply,
  requestDesktopCanvasArrangePreview,
  requestDesktopCanvasMutation,
  requestDesktopCanvasRecovery,
  runDesktopCanvasRecovery,
  runDesktopCanvasMutation,
  runDesktopCanvasArrangePreview,
  selectDesktopCanvasEdge,
  selectDesktopCanvasNode,
  type DesktopCanvasSurfaceSnapshot,
} from "./desktop_canvas_controller.ts";
import { createDesktopCanvasViewportDebouncer } from "./desktop_canvas_viewport_debouncer.ts";
import { createDesktopCanvasMutationQueue } from "./desktop_canvas_mutation_queue.ts";
import {
  changeCanvasTextEditDraft,
  closeCanvasTextEditDialog,
  createClosedCanvasTextEditDialog,
  openCanvasTextEditDialog,
} from "./canvas_text_edit_dialog.ts";
import {
  createDesktopCanvasCatalogSnapshot,
  loadDesktopCanvasCatalog,
  requestDesktopCanvasCatalogLoad,
  requestDesktopCanvasSelection,
  resolveDesktopCanvasMenuTarget,
  selectDesktopCanvas as selectDesktopCanvasCatalog,
  type DesktopCanvasCatalogSnapshot,
} from "./desktop_canvas_catalog_controller.ts";
import {
  HOME_GRAPH_PROJECTION_LIMIT,
  createDesktopGraphSnapshot,
  loadDesktopKnowledgeGraphWithFreshness,
  loadDesktopGlobalKnowledgeGraph,
  repairDesktopKnowledgeGraph,
  requestDesktopGraphRepair,
  requestDesktopGraphLoad,
  selectDesktopGraphNode,
  type DesktopGraphQueryState,
  type DesktopGraphSurfaceSnapshot,
} from "./desktop_graph_controller.ts";
import {
  createDesktopDocumentAuthoringController,
  type DesktopDocumentAuthoringSnapshot,
} from "./desktop_document_authoring_controller.ts";
import { createDesktopRevisionMetadataGenerator } from "./desktop_revision_metadata_generator.ts";
import {
  createDesktopGraphScopeIntent,
  createDesktopRouteControllerState,
  graphQueryScopeForRoute,
  transitionDesktopRoute,
  type DesktopRoute,
  type DesktopRouteControllerState,
  type DesktopSelectionContext,
} from "./desktop_route_controller.ts";
import { loadDesktopWorkspaceHome } from "./index.ts";
import { createDesktopDocumentNavigatorElement } from "./react_document_navigator.ts";
import { createDesktopDocumentEmptyStateElement } from "./react_document_empty_state.ts";
import {
  createDesktopDocumentAuthoringWorkbenchElement,
  type DesktopDocumentHistoryWorkbenchState,
} from "./react_document_authoring_workbench.ts";
import { createDesktopWorkspaceHomeElement } from "./react_workspace_home.ts";
import { createDesktopBackupRecoveryElement } from "./react_backup_recovery.ts";
import {
  createDesktopAttachmentsElement,
  createDesktopCanvasElement,
  createDesktopKnowledgeGraphElement,
} from "./react_exploration_surfaces.ts";
import { createTauriDesktopTransport } from "./tauri_desktop_transport.ts";
import { getGlobalTauriInvoke } from "./tauri_home_transport.ts";
import { createTauriProjectionTransport } from "./tauri_projection_transport.ts";
import { synchronizeCurrentDocumentProjections } from "./desktop_projection_synchronizer.ts";
import { createTauriGlobalGraphTransport } from "./tauri_global_graph_transport.ts";
import { createTauriGraphPreferenceTransport } from "./tauri_graph_preference_transport.ts";
import {
  createDefaultDesktopGraphPreference,
  graphQueryPatchFromPreference,
  preferenceFromGraphQuery,
} from "./desktop_graph_preference.ts";
import { createDesktopGraphCameraSaveScheduler } from "./desktop_graph_camera_save_scheduler.ts";
import {
  applyDesktopGraphPreferenceLoad,
  applyDesktopGraphPreferenceSave,
  applyDesktopGraphPreferenceSaveFailure,
  createDesktopGraphPreferenceSnapshot,
  requestDesktopGraphPreferenceLoad,
  requestDesktopGraphPreferenceSave,
} from "./desktop_graph_preference_controller.ts";
import { createTauriAssetImportTransport, type DesktopAssetImportSelection } from "./tauri_asset_import_transport.ts";
import { getGlobalTauriEventListen, subscribeTauriAssetDrop } from "./tauri_asset_drop_transport.ts";
import { createTauriCanvasTransport, type DesktopCanvasMutationDraft } from "./tauri_canvas_transport.ts";
import { createTauriCanvasCatalogTransport } from "./tauri_canvas_catalog_transport.ts";
import { createTauriBackupRecoveryTransport } from "./tauri_backup_recovery_transport.ts";
import { createTauriDocumentDiffOperationTransport } from "./tauri_document_diff_operation_transport.ts";
import {
  applyDesktopDocumentDiffOperationCandidate,
  cancelDesktopDocumentDiffOperation,
  createDesktopDocumentDiffOperationSnapshot,
  pollDesktopDocumentDiffOperation,
  retryDesktopDocumentDiffOperation,
  startDesktopDocumentDiffOperation,
  type DesktopDocumentDiffOperationSnapshot,
} from "./desktop_document_diff_operation_controller.ts";
import {
  presentDesktopDocumentDiffOperation,
  type DesktopDocumentDiffPresentationTarget,
} from "./document_diff_operation_presenter.ts";
import { runPackagedUiSmoke } from "./packaged_ui_smoke.ts";
import { isMacDocumentSaveShortcut } from "./desktop_authoring_shortcut.ts";
import { isMacWorkspaceSearchShortcut } from "./desktop_shell_shortcut.ts";
import {
  createDesktopSearchNavigationIntent,
  focusDesktopWorkspaceSearch,
} from "./desktop_search_navigation.ts";
import {
  captureDesktopSearchViewport,
  createDesktopSearchReturnContext,
  restoreDesktopSearchViewport,
  transitionDesktopSearchReturnContext,
  type DesktopSearchReturnContext,
} from "./desktop_search_return_context.ts";
import {
  beginDesktopRouteQuery,
  canApplyDesktopRouteQuery,
  createDesktopRouteQueryLifecycle,
  transitionDesktopRouteQueryLifecycle,
} from "./desktop_route_query_lifecycle.ts";
import {
  createDesktopSearchResultWindow,
  transitionDesktopSearchResultWindow,
} from "./desktop_search_result_window.ts";
import { createDesktopSearchEscapeIntent } from "./desktop_search_escape_intent.ts";
import {
  createGlobalSearchOverlayLifecycle,
  transitionGlobalSearchOverlay,
} from "./global_search_overlay_lifecycle.ts";
import {
  createKoKrHistoryDateFormatter,
  presentDocumentHistory,
} from "./document_history_presenter.ts";
import { focusWorkspaceRouteMain } from "./route_main_focus.ts";
import { resolveDesktopDocumentMenuTarget } from "./desktop_document_menu_target.ts";
import {
  createDocumentInspectorState,
  transitionDocumentInspector,
  type DocumentInspectorState,
} from "./document_inspector_state.ts";
import {
  createDocumentHistoryCompareSelection,
  transitionDocumentHistoryCompareSelection,
} from "./document_history_compare_selection.ts";
import {
  applyDocumentAssetLibraryLoad,
  beginDocumentAssetLibraryLink,
  closeDocumentAssetLibrary,
  completeDocumentAssetLibraryLink,
  createDocumentAssetLibraryState,
  requestDocumentAssetLibraryOpen,
  requestDocumentAssetLibraryMore,
  selectDocumentAssetLibraryItem,
  type DocumentAssetLibraryState,
} from "./document_asset_library_state.ts";
import {
  beginRestoreApply,
  beginRestorePreview,
  cancelRestoreConfirmation,
  completeRestoreApply,
  completeRestorePreview,
  failRestoreApply,
  requestRestoreConfirmation,
  retryRestoreRecovery,
} from "./document_restore_presentation.ts";
import {
  createDocumentRestoreRequestContext,
  isCurrentDocumentRestoreRequest,
  type DocumentRestoreRequestContext,
} from "./document_restore_request_context.ts";

const profile = createPersonalLocalDesktopCapabilityProfile();
const homeQuery = Object.freeze({
  workspaceId: "workspace-1",
  recentDocuments: 20,
  favorites: 20,
  tags: 20,
  recentChanges: 20,
  unfinishedItems: 20,
});
const bootstrapInvoke = getGlobalTauriInvoke();
const desktopClient = bootstrapInvoke
  ? createLocalDesktopCommandClient(createTauriDesktopTransport(bootstrapInvoke))
  : undefined;
const projectionClient = bootstrapInvoke ? createTauriProjectionTransport(bootstrapInvoke) : undefined;
const globalGraphClient = bootstrapInvoke ? createTauriGlobalGraphTransport(bootstrapInvoke) : undefined;
const graphPreferenceClient = bootstrapInvoke ? createTauriGraphPreferenceTransport(bootstrapInvoke) : undefined;
const assetImportClient = bootstrapInvoke ? createTauriAssetImportTransport(bootstrapInvoke) : undefined;
const canvasClient = bootstrapInvoke ? createTauriCanvasTransport(bootstrapInvoke) : undefined;
const canvasCatalogClient = bootstrapInvoke ? createTauriCanvasCatalogTransport(bootstrapInvoke) : undefined;
const backupClient = bootstrapInvoke ? createTauriBackupRecoveryTransport(bootstrapInvoke) : undefined;
const documentDiffOperationClient = bootstrapInvoke
  ? createTauriDocumentDiffOperationTransport(bootstrapInvoke)
  : undefined;
const DOCUMENT_DIFF_OPERATION_POLL_INTERVAL_MS = 100;

function waitForDocumentDiffOperationPoll(): Promise<void> {
  return new Promise((resolve) => {
    globalThis.setTimeout(resolve, DOCUMENT_DIFF_OPERATION_POLL_INTERVAL_MS);
  });
}
const bootstrapUuidSource = globalThis.crypto?.randomUUID
  ? globalThis.crypto.randomUUID.bind(globalThis.crypto)
  : undefined;
const revisionMetadataGenerator = bootstrapUuidSource
  ? createDesktopRevisionMetadataGenerator(bootstrapUuidSource)
  : undefined;
const canvasOperationIdSource = bootstrapUuidSource
  ? () => `canvas-${bootstrapUuidSource()}`
  : undefined;
const authoringController = desktopClient && bootstrapUuidSource
  ? createDesktopDocumentAuthoringController({
      client: desktopClient,
      operationIdSource: () => `document-save-${bootstrapUuidSource()}`,
      author: "local-user",
      summary: "Updated",
      autosaveDelayMs: 800,
    })
  : undefined;
const historyDateFormatter = createKoKrHistoryDateFormatter();

function DesktopApp(): React.ReactElement {
  const [homeModel, setHomeModel] = useState<PersonalWorkspaceHomeModel>(() =>
    createPersonalWorkspaceHomeModel({ profile, healthState: "Loading" }),
  );
  const [routeState, setRouteState] = useState<DesktopRouteControllerState>(() =>
    createDesktopRouteControllerState({ kind: "Home" }, {
      workspaceId: "workspace-1",
      originRoute: "Home",
    }),
  );
  const routeStateRef = useRef(routeState);
  const routeQueryLifecycleRef = useRef(createDesktopRouteQueryLifecycle("Home"));
  const generation = useRef(0);
  const [navigatorModel, setNavigatorModel] = useState<DocumentNavigatorModel>(() =>
    createDocumentNavigatorLoadingModel({
      workspaceId: "workspace-1",
      view: "Tree",
      generation: 0,
    }),
  );
  const [searchReturnContext, setSearchReturnContext] = useState<DesktopSearchReturnContext>(
    () => createDesktopSearchReturnContext(),
  );
  const [searchResultWindow, setSearchResultWindow] = useState(() =>
    createDesktopSearchResultWindow(0, 0),
  );
  const [globalSearchOverlay, setGlobalSearchOverlay] = useState(() =>
    createGlobalSearchOverlayLifecycle(),
  );
  const globalSearchOverlayRef = useRef(globalSearchOverlay);
  const [searchOriginContext, setSearchOriginContext] = useState<Readonly<{
    route: DesktopRoute;
    selection: DesktopSelectionContext;
  }>>(() => ({ route: { kind: "Home" }, selection: { workspaceId: "workspace-1", originRoute: "Home" } }));
  const authoringGeneration = useRef(0);
  const diffGeneration = useRef(0);
  const restoreGeneration = useRef(0);
  const diffOperationSnapshotRef = useRef<DesktopDocumentDiffOperationSnapshot>(
    createDesktopDocumentDiffOperationSnapshot(),
  );
  const diffOperationContextRef = useRef<{
    readonly authoringGeneration: number;
    readonly diffGeneration: number;
    readonly documentId: string;
    readonly target: DesktopDocumentDiffPresentationTarget;
  } | undefined>(undefined);
  const [authoringSnapshot, setAuthoringSnapshot] = useState<DesktopDocumentAuthoringSnapshot>(
    () => authoringController?.snapshot() ?? emptyAuthoringSnapshot(),
  );
  const [historyState, setHistoryState] = useState<DesktopDocumentHistoryWorkbenchState>(() => ({
    status: "Idle",
    entries: [],
  }));
  const isCurrentAuthoringRestoreContext = useCallback((context: DocumentRestoreRequestContext) => (
    isCurrentDocumentRestoreRequest(
      context,
      createDocumentRestoreRequestContext(
        authoringGeneration.current,
        restoreGeneration.current,
        authoringController?.snapshot().documentId ?? "",
      ),
    )
  ), []);
  const beginAuthoringDiffGeneration = useCallback(() => {
    const active = diffOperationSnapshotRef.current;
    if (documentDiffOperationClient && (active.state === "Accepted" || active.state === "Running")) {
      void cancelDesktopDocumentDiffOperation(documentDiffOperationClient, active);
    }
    diffOperationSnapshotRef.current = createDesktopDocumentDiffOperationSnapshot();
    diffOperationContextRef.current = undefined;
    diffGeneration.current += 1;
    return diffGeneration.current;
  }, []);
  const [documentInspectorState, setDocumentInspectorState] = useState<DocumentInspectorState>(() =>
    createDocumentInspectorState(),
  );
  const documentInspectorStateRef = useRef(documentInspectorState);
  useEffect(() => {
    routeStateRef.current = routeState;
  }, [routeState]);
  useEffect(() => {
    documentInspectorStateRef.current = documentInspectorState;
  }, [documentInspectorState]);
  const [linkOverviewSnapshot, setLinkOverviewSnapshot] = useState<DesktopLinkOverviewSnapshot>(
    () => createDesktopLinkOverviewSnapshot("workspace-1", "unloaded"),
  );
  const [plainTextEditorOpen, setPlainTextEditorOpen] = useState(false);
  const [graphSnapshot, setGraphSnapshot] = useState<DesktopGraphSurfaceSnapshot>(() =>
    createDesktopGraphSnapshot("workspace-1"),
  );
  const [graphVisualSearch, setGraphVisualSearch] = useState("");
  const [graphIncludeExternal, setGraphIncludeExternal] = useState(false);
  const graphSnapshotRef = useRef(graphSnapshot);
  const graphPreferenceSnapshotRef = useRef(createDesktopGraphPreferenceSnapshot(homeQuery.workspaceId));
  const graphCameraSaveSchedulerRef = useRef(createDesktopGraphCameraSaveScheduler({
    delayMs: 180,
    schedule: (run, delayMs) => globalThis.setTimeout(run, delayMs),
    cancel: (handle) => globalThis.clearTimeout(handle as number),
  }));
  const [assetSnapshot, setAssetSnapshot] = useState<DesktopAssetSurfaceSnapshot>(() =>
    createDesktopAssetSnapshot("workspace-1"),
  );
  const assetSnapshotRef = useRef(assetSnapshot);
  const [documentAssetLibraryState, setDocumentAssetLibraryState] = useState<DocumentAssetLibraryState>(() =>
    createDocumentAssetLibraryState("workspace-1"),
  );
  const documentAssetLibraryStateRef = useRef(documentAssetLibraryState);
  const [canvasSnapshot, setCanvasSnapshot] = useState<DesktopCanvasSurfaceSnapshot>(() =>
    createDesktopCanvasSnapshot("workspace-1"),
  );
  const [canvasCatalogSnapshot, setCanvasCatalogSnapshot] = useState<DesktopCanvasCatalogSnapshot>(() =>
    createDesktopCanvasCatalogSnapshot("workspace-1"),
  );
  const [canvasArchiveConfirmationOpen, setCanvasArchiveConfirmationOpen] = useState(false);
  const [canvasRenameDialogOpen, setCanvasRenameDialogOpen] = useState(false);
  const [canvasRenameDraft, setCanvasRenameDraft] = useState("");
  const [canvasTextEditDialog, setCanvasTextEditDialog] = useState(createClosedCanvasTextEditDialog);
  const [canvasDocumentPlacementId, setCanvasDocumentPlacementId] = useState<string>();
  const [canvasAssetPlacementId, setCanvasAssetPlacementId] = useState<string>();
  const canvasSnapshotRef = useRef(canvasSnapshot);
  const canvasCatalogSnapshotRef = useRef(canvasCatalogSnapshot);
  const canvasCatalogSignatureRef = useRef<string>();
  const [backupSnapshot, setBackupSnapshot] = useState<DesktopBackupRecoverySnapshot>(() =>
    createDesktopBackupRecoverySnapshot("workspace-1"),
  );
  const backupSnapshotRef = useRef(backupSnapshot);
  const canvasViewportDebouncerRef = useRef<ReturnType<typeof createDesktopCanvasViewportDebouncer> | undefined>(undefined);
  const canvasMutationQueueRef = useRef<ReturnType<typeof createDesktopCanvasMutationQueue> | undefined>(undefined);
  const pendingCanvasViewportRef = useRef<{
    readonly canvasId: string;
    readonly draft: Extract<DesktopCanvasMutationDraft, { readonly kind: "update_viewport" }>;
  } | undefined>(undefined);
  const previousSurface = useRef<string>("Home");
  const activeRoute = visibleRoute(routeState);
  const surface = surfaceForRoute(activeRoute);
  const graphCenterDocumentId = activeRoute.kind === "Graph"
    ? activeRoute.centerDocumentId
      ?? currentSelection(routeState).documentId
      ?? homeModel.recentDocuments[0]?.documentId
    : activeRoute.kind === "Document"
      ? activeRoute.documentId
      : undefined;
  const graphQueryScope = activeRoute.kind === "Graph" ? graphQueryScopeForRoute(activeRoute) : "local";
  const assetDocumentId = activeRoute.kind === "Assets"
    ? activeRoute.documentId
    : activeRoute.kind === "Document"
      ? activeRoute.documentId
      : undefined;
  const requestedAssetId = activeRoute.kind === "Assets" ? activeRoute.assetId : undefined;
  const activeCanvasId = activeRoute.kind === "Canvas" ? activeRoute.canvasId : undefined;
  useEffect(() => {
    const activeDocumentId = activeRoute.kind === "Document" ? activeRoute.documentId : undefined;
    const current = documentAssetLibraryStateRef.current;
    if (current.status === "Closed") return;
    if (activeDocumentId && activeDocumentId === current.documentId) return;
    const closed = closeDocumentAssetLibrary(current);
    documentAssetLibraryStateRef.current = closed;
    setDocumentAssetLibraryState(closed);
  }, [activeRoute.kind, activeRoute.kind === "Document" ? activeRoute.documentId : undefined]);

  const commitGraphSnapshot = useCallback((snapshot: DesktopGraphSurfaceSnapshot) => {
    graphSnapshotRef.current = snapshot;
    setGraphSnapshot(snapshot);
  }, []);

  const commitCanvasSnapshot = useCallback((snapshot: DesktopCanvasSurfaceSnapshot) => {
    if (snapshot.generation < canvasSnapshotRef.current.generation) return;
    canvasSnapshotRef.current = snapshot;
    setCanvasSnapshot(snapshot);
  }, []);

  const commitCanvasCatalogSnapshot = useCallback((snapshot: DesktopCanvasCatalogSnapshot) => {
    if (snapshot.generation < canvasCatalogSnapshotRef.current.generation) return;
    canvasCatalogSnapshotRef.current = snapshot;
    setCanvasCatalogSnapshot(snapshot);
  }, []);

  const commitBackupSnapshot = useCallback((snapshot: DesktopBackupRecoverySnapshot) => {
    if (snapshot.generation < backupSnapshotRef.current.generation) return;
    backupSnapshotRef.current = snapshot;
    setBackupSnapshot(snapshot);
  }, []);

  const runBackupCreate = useCallback(async () => {
    if (!backupClient || !bootstrapUuidSource) {
      commitBackupSnapshot(Object.freeze({
        ...backupSnapshotRef.current,
        state: "Failed",
        errorCode: "COMMAND_BRIDGE_FAILED",
        retryable: false,
      }));
      return;
    }
    commitBackupSnapshot(await startDesktopBackupOperation(
      backupClient,
      backupSnapshotRef.current,
      `backup-${bootstrapUuidSource()}`,
    ));
  }, [commitBackupSnapshot]);

  const runBackupCatalogLoad = useCallback(async (cursor?: string) => {
    if (!backupClient) return;
    const current = backupSnapshotRef.current;
    const next = await loadDesktopBackupCatalog(backupClient, current, { cursor, limit: 20 });
    if (backupSnapshotRef.current.generation !== current.generation) return;
    commitBackupSnapshot(next);
  }, [commitBackupSnapshot]);

  const runBackupPoll = useCallback(async () => {
    if (!backupClient) return;
    const current = backupSnapshotRef.current;
    const next = await pollDesktopBackupOperation(backupClient, current);
    if (backupSnapshotRef.current.generation !== current.generation
      || backupSnapshotRef.current.operationId !== current.operationId) return;
    commitBackupSnapshot(next);
    if (next.state === "Ready") await runBackupCatalogLoad();
  }, [commitBackupSnapshot, runBackupCatalogLoad]);

  const runBackupOperationCancel = useCallback(async () => {
    if (!backupClient) return;
    const current = backupSnapshotRef.current;
    commitBackupSnapshot(await cancelDesktopBackupOperation(backupClient, current));
  }, [commitBackupSnapshot]);

  const runBackupPreview = useCallback(async () => {
    const packageId = backupSnapshotRef.current.packageId;
    if (!backupClient || !packageId) return;
    commitBackupSnapshot(await previewDesktopRestore(
      backupClient,
      backupSnapshotRef.current,
      packageId,
    ));
  }, [commitBackupSnapshot]);

  const runBackupConfirm = useCallback(async () => {
    if (!backupClient || !bootstrapUuidSource) return;
    commitBackupSnapshot(await startDesktopRestoreOperation(
      backupClient,
      backupSnapshotRef.current,
      `restore-${bootstrapUuidSource()}`,
    ));
  }, [commitBackupSnapshot]);

  const runRestorePoll = useCallback(async () => {
    if (!backupClient) return;
    const current = backupSnapshotRef.current;
    const next = await pollDesktopRestoreOperation(backupClient, current);
    if (backupSnapshotRef.current.generation !== current.generation
      || backupSnapshotRef.current.operationId !== current.operationId) return;
    commitBackupSnapshot(next);
  }, [commitBackupSnapshot]);

  const runBackupCancel = useCallback(async () => {
    const current = backupSnapshotRef.current;
    if (current.state === "AwaitingConfirmation") {
      commitBackupSnapshot(dismissDesktopRestoreConfirmation(current));
      return;
    }
    if (!backupClient) return;
    commitBackupSnapshot(await cancelDesktopRestore(backupClient, current));
  }, [commitBackupSnapshot]);

  const runBackupRecovery = useCallback(async () => {
    if (!backupClient) {
      commitBackupSnapshot(Object.freeze({
        ...backupSnapshotRef.current,
        state: "Failed",
        errorCode: "COMMAND_BRIDGE_FAILED",
        retryable: false,
      }));
      return;
    }
    commitBackupSnapshot(await recoverDesktopBackupStartup(backupClient, backupSnapshotRef.current));
  }, [commitBackupSnapshot]);

  const runCanvasLoad = useCallback(async () => {
    if (!canvasClient || !activeCanvasId) return;
    const loading = requestDesktopCanvasLoad(canvasSnapshotRef.current, activeCanvasId);
    commitCanvasSnapshot(loading);
    commitCanvasSnapshot(await loadDesktopCanvas(canvasClient, loading));
  }, [activeCanvasId, commitCanvasSnapshot]);

  const runCanvasRecovery = useCallback(async () => {
    if (!canvasClient || !canvasOperationIdSource) return;
    const recovering = requestDesktopCanvasRecovery(
      canvasSnapshotRef.current,
      canvasOperationIdSource(),
    );
    if (recovering.state !== "Recovering") return;
    commitCanvasSnapshot(recovering);
    const result = await runDesktopCanvasRecovery(canvasClient, recovering);
    if (canvasSnapshotRef.current.generation === recovering.generation) {
      commitCanvasSnapshot(result);
    }
  }, [commitCanvasSnapshot]);

  if (!canvasMutationQueueRef.current && canvasClient && canvasOperationIdSource) {
    canvasMutationQueueRef.current = createDesktopCanvasMutationQueue({
      client: canvasClient,
      operationIdSource: canvasOperationIdSource,
      readSnapshot: () => canvasSnapshotRef.current,
      commitSnapshot: commitCanvasSnapshot,
      capacity: 32,
    });
  }

  const runCanvasMutation = useCallback((draft: DesktopCanvasMutationDraft) => {
    canvasMutationQueueRef.current?.enqueue(draft);
  }, []);

  if (!canvasViewportDebouncerRef.current) {
    canvasViewportDebouncerRef.current = createDesktopCanvasViewportDebouncer({
      schedule(delayMs, callback) { return globalThis.setTimeout(callback, delayMs); },
      cancel(handle) { globalThis.clearTimeout(handle as ReturnType<typeof setTimeout>); },
    }, 250);
  }
  useEffect(() => () => {
    canvasViewportDebouncerRef.current?.dispose();
    canvasMutationQueueRef.current?.dispose();
  }, []);

  const scheduleCanvasViewport = useCallback((patch: Parameters<typeof createDesktopCanvasViewportDraft>[1]) => {
    const current = canvasSnapshotRef.current;
    const pending = pendingCanvasViewportRef.current;
    const base = pending && pending.canvasId === current.canvasId && current.canvas
      ? Object.freeze({
          ...current,
          canvas: Object.freeze({
            ...current.canvas,
            viewport: Object.freeze({
              centerX: pending.draft.centerX,
              centerY: pending.draft.centerY,
              zoomPercent: pending.draft.zoomPercent,
            }),
          }),
        })
      : current;
    const draft = createDesktopCanvasViewportDraft(base, patch);
    if (!draft || draft.kind !== "update_viewport") return;
    const canvasId = current.canvasId;
    if (!canvasId) return;
    pendingCanvasViewportRef.current = Object.freeze({ canvasId, draft });
    canvasViewportDebouncerRef.current?.queue(draft, (queued) => {
      pendingCanvasViewportRef.current = undefined;
      if (canvasSnapshotRef.current.canvasId === canvasId) void runCanvasMutation(queued);
    });
  }, [runCanvasMutation]);

  const runCanvasArrangePreview = useCallback(async () => {
    if (!canvasClient) return;
    const previewing = requestDesktopCanvasArrangePreview(canvasSnapshotRef.current);
    if (previewing === canvasSnapshotRef.current) return;
    commitCanvasSnapshot(previewing);
    commitCanvasSnapshot(await runDesktopCanvasArrangePreview(canvasClient, previewing));
  }, [commitCanvasSnapshot]);

  const applyCanvasArrangePreview = useCallback(async () => {
    if (!canvasClient || !canvasOperationIdSource) return;
    const mutating = requestDesktopCanvasArrangeApply(
      canvasSnapshotRef.current,
      canvasOperationIdSource(),
    );
    if (mutating === canvasSnapshotRef.current) return;
    commitCanvasSnapshot(mutating);
    commitCanvasSnapshot(await runDesktopCanvasMutation(canvasClient, mutating));
  }, [commitCanvasSnapshot]);

  const runGraphQuery = useCallback(async (
    patch: Partial<DesktopGraphQueryState>,
    globalProjectionLimit?: number,
  ) => {
    const loading = requestDesktopGraphLoad(graphSnapshotRef.current, {
      centerDocumentId: graphCenterDocumentId,
      ...patch,
    });
    commitGraphSnapshot(loading);
    const preferenceFieldsChanged = ["depth", "direction", "includeUnresolved", "includeAssets"]
      .some((key) => Object.prototype.hasOwnProperty.call(patch, key));
    if (graphPreferenceClient && preferenceFieldsChanged) {
      const currentPreference = graphPreferenceSnapshotRef.current.preference;
      const candidate = preferenceFromGraphQuery(loading.query, currentPreference.camera, currentPreference.includeExternal);
      const saving = requestDesktopGraphPreferenceSave(graphPreferenceSnapshotRef.current, candidate);
      graphPreferenceSnapshotRef.current = saving;
      if (saving.state === "Saving") {
        void graphPreferenceClient.save(saving.workspaceId, saving.preference)
          .then(() => {
            graphPreferenceSnapshotRef.current = applyDesktopGraphPreferenceSave(
              graphPreferenceSnapshotRef.current, saving.generation, saving.workspaceId,
            );
          })
          .catch(() => {
            graphPreferenceSnapshotRef.current = applyDesktopGraphPreferenceSaveFailure(
              graphPreferenceSnapshotRef.current, saving.generation, saving.workspaceId,
            );
          });
      }
    }
    if (loading.state !== "Loading") return;
    if (loading.query.scope === "global") {
      if (!globalGraphClient) return;
      const result = await loadDesktopGlobalKnowledgeGraph(
        globalGraphClient,
        loading,
        globalProjectionLimit,
      );
      if (graphSnapshotRef.current.generation === result.generation) commitGraphSnapshot(result);
      return;
    }
    if (!desktopClient) {
      const failed = await loadDesktopKnowledgeGraphWithFreshness({
        async getKnowledgeGraph() { throw new Error("COMMAND_BRIDGE_FAILED"); },
      }, { async getFreshness() { throw new Error("COMMAND_BRIDGE_FAILED"); } }, loading);
      if (graphSnapshotRef.current.generation === failed.generation) commitGraphSnapshot(failed);
      return;
    }
    if (!projectionClient) return;
    const result = await loadDesktopKnowledgeGraphWithFreshness(desktopClient, projectionClient, loading);
    if (graphSnapshotRef.current.generation === result.generation) commitGraphSnapshot(result);
  }, [commitGraphSnapshot, graphCenterDocumentId]);

  const queueGraphCameraPreference = useCallback((camera: ReturnType<typeof createDefaultDesktopGraphPreference>["camera"]) => {
    const current = graphPreferenceSnapshotRef.current;
    const candidate = preferenceFromGraphQuery(current.preference, camera, current.preference.includeExternal);
    const saving = requestDesktopGraphPreferenceSave(current, candidate);
    graphPreferenceSnapshotRef.current = saving;
    graphCameraSaveSchedulerRef.current.queue(camera, () => {
      const latest = graphPreferenceSnapshotRef.current;
      if (!graphPreferenceClient || latest.state !== "Saving") return;
      void graphPreferenceClient.save(latest.workspaceId, latest.preference)
        .then(() => {
          graphPreferenceSnapshotRef.current = applyDesktopGraphPreferenceSave(
            graphPreferenceSnapshotRef.current, latest.generation, latest.workspaceId,
          );
        })
        .catch(() => {
          graphPreferenceSnapshotRef.current = applyDesktopGraphPreferenceSaveFailure(
            graphPreferenceSnapshotRef.current, latest.generation, latest.workspaceId,
          );
        });
    });
  }, []);

  const updateGraphIncludeExternal = useCallback((includeExternal: boolean) => {
    setGraphIncludeExternal(includeExternal);
    const current = graphPreferenceSnapshotRef.current;
    const saving = requestDesktopGraphPreferenceSave(current, Object.freeze({ ...current.preference, includeExternal }));
    graphPreferenceSnapshotRef.current = saving;
    if (!graphPreferenceClient || saving.state !== "Saving") return;
    void graphPreferenceClient.save(saving.workspaceId, saving.preference)
      .then(() => {
        graphPreferenceSnapshotRef.current = applyDesktopGraphPreferenceSave(
          graphPreferenceSnapshotRef.current, saving.generation, saving.workspaceId,
        );
      })
      .catch(() => {
        graphPreferenceSnapshotRef.current = applyDesktopGraphPreferenceSaveFailure(
          graphPreferenceSnapshotRef.current, saving.generation, saving.workspaceId,
        );
      });
  }, []);

  const runGraphRepair = useCallback(async () => {
    const repairing = requestDesktopGraphRepair(graphSnapshotRef.current);
    commitGraphSnapshot(repairing);
    if (repairing.state !== "Repairing" || !desktopClient || !projectionClient) return;
    const result = await repairDesktopKnowledgeGraph(projectionClient, desktopClient, repairing);
    if (graphSnapshotRef.current.generation === result.generation) commitGraphSnapshot(result);
  }, [commitGraphSnapshot]);

  const refreshVisibleKnowledgeSurfaces = useCallback((documentId: string) => {
    const visible = visibleRoute(routeState);
    if (visible.kind === "Home") {
      void runGraphQuery({ scope: "global", globalCursor: undefined }, HOME_GRAPH_PROJECTION_LIMIT);
      return;
    }
    if (visible.kind === "Graph") {
      if (graphSnapshotRef.current.query.scope === "global") {
        void runGraphQuery({ scope: "global", globalCursor: undefined }, HOME_GRAPH_PROJECTION_LIMIT);
        return;
      }
      void runGraphQuery({
        scope: "local",
        centerDocumentId: graphSnapshotRef.current.query.centerDocumentId ?? documentId,
      });
      return;
    }
    if (visible.kind === "Document") {
      void runGraphQuery({ scope: "local", centerDocumentId: documentId });
    }
  }, [routeState, runGraphQuery]);

  const synchronizeDocumentKnowledgeSurfaces = useCallback(async (workspaceId: string, documentId: string) => {
    if (!projectionClient) return undefined;
    const result = await synchronizeCurrentDocumentProjections(
      projectionClient,
      workspaceId,
      documentId,
    );
    if (result.state === "Completed") refreshVisibleKnowledgeSurfaces(documentId);
    return result;
  }, [projectionClient, refreshVisibleKnowledgeSurfaces]);

  const commitAssetSnapshot = useCallback((snapshot: DesktopAssetSurfaceSnapshot) => {
    assetSnapshotRef.current = snapshot;
    setAssetSnapshot(snapshot);
  }, []);

  const commitDocumentAssetLibraryState = useCallback((state: DocumentAssetLibraryState) => {
    documentAssetLibraryStateRef.current = state;
    setDocumentAssetLibraryState(state);
  }, []);

  const synchronizeAssetProjection = useCallback(async (snapshot: DesktopAssetSurfaceSnapshot) => {
    if (snapshot.scope !== "Document" || !snapshot.documentId || !projectionClient) return;
    return synchronizeDocumentKnowledgeSurfaces(
      snapshot.workspaceId,
      snapshot.documentId,
    );
  }, [synchronizeDocumentKnowledgeSurfaces]);

  const openDocumentAssetLibrary = useCallback(async (documentId: string | undefined) => {
    const loading = requestDocumentAssetLibraryOpen(documentAssetLibraryStateRef.current, documentId);
    if (loading === documentAssetLibraryStateRef.current) return;
    commitDocumentAssetLibraryState(loading);
    const result = assetImportClient
      ? await loadDesktopWorkspaceAssets(assetImportClient, loading.assets)
      : Object.freeze({ ...loading.assets, state: "Failed" as const, errorCode: "COMMAND_BRIDGE_FAILED", retryable: false });
    const completed = applyDocumentAssetLibraryLoad(loading, loading.generation, result);
    if (documentAssetLibraryStateRef.current.generation === loading.generation) {
      commitDocumentAssetLibraryState(completed);
    }
  }, [commitDocumentAssetLibraryState]);

  const linkDocumentAssetLibrarySelection = useCallback(async () => {
    const linking = beginDocumentAssetLibraryLink(documentAssetLibraryStateRef.current);
    if (linking === documentAssetLibraryStateRef.current) return;
    commitDocumentAssetLibraryState(linking);
    const generation = linking.generation;
    const linked = assetImportClient && desktopClient && bootstrapUuidSource
      ? await linkDesktopSelectedAsset(
          assetImportClient,
          desktopClient,
          linking.assets,
          () => `document-asset-link-${bootstrapUuidSource()}`,
        )
      : Object.freeze({ ...linking.assets, mutationState: "Failed" as const });
    const completion = completeDocumentAssetLibraryLink(linking, generation, linked);
    if (documentAssetLibraryStateRef.current.generation !== generation) return;
    commitDocumentAssetLibraryState(completion.library);
    if (completion.documentAssets) {
      commitAssetSnapshot(completion.documentAssets);
      if (completion.documentAssets.mutationState === "Idle") {
        await synchronizeAssetProjection(completion.documentAssets);
      }
    }
    return completion;
  }, [commitAssetSnapshot, commitDocumentAssetLibraryState, synchronizeAssetProjection]);

  const loadMoreDocumentAssetLibrary = useCallback(async () => {
    const loading = requestDocumentAssetLibraryMore(documentAssetLibraryStateRef.current);
    if (loading === documentAssetLibraryStateRef.current) return;
    commitDocumentAssetLibraryState(loading);
    const result = assetImportClient
      ? await loadDesktopWorkspaceAssets(assetImportClient, loading.assets)
      : Object.freeze({ ...loading.assets, state: "Failed" as const, errorCode: "COMMAND_BRIDGE_FAILED", retryable: false });
    const completed = applyDocumentAssetLibraryLoad(loading, loading.generation, result);
    if (documentAssetLibraryStateRef.current.generation === loading.generation) {
      commitDocumentAssetLibraryState(completed);
    }
  }, [commitDocumentAssetLibraryState]);

  const runAssetQuery = useCallback(async () => {
    const loading = requestDesktopAssetLoad(assetSnapshotRef.current, assetDocumentId);
    commitAssetSnapshot(loading);
    if (loading.state !== "Loading") return;
    const applyRequestedAsset = async (snapshot: DesktopAssetSurfaceSnapshot) => {
      if (!requestedAssetId) return snapshot;
      const selected = selectDesktopAsset(snapshot, requestedAssetId);
      return assetImportClient && selected.detailState === "Loading"
        ? loadDesktopAssetDetail(assetImportClient, selected)
        : selected;
    };
    if (loading.scope === "Workspace") {
      let result = assetImportClient
        ? await loadDesktopWorkspaceAssets(assetImportClient, loading)
        : Object.freeze({ ...loading, state: "Failed" as const, errorCode: "COMMAND_BRIDGE_FAILED", retryable: false });
      result = await applyRequestedAsset(result);
      if (assetSnapshotRef.current.generation === result.generation) commitAssetSnapshot(result);
      return;
    }
    const client = desktopClient ?? {
      async getAssetMetadata() { throw new Error("COMMAND_BRIDGE_FAILED"); },
    };
    let result = await loadDesktopDocumentAssets(client, loading);
    result = await applyRequestedAsset(result);
    if (assetSnapshotRef.current.generation === result.generation) commitAssetSnapshot(result);
  }, [assetDocumentId, commitAssetSnapshot, requestedAssetId]);

  const runAssetWorkspaceLibrary = useCallback(async () => {
    const loading = requestDesktopWorkspaceAssetLoad(assetSnapshotRef.current);
    commitAssetSnapshot(loading);
    const result = assetImportClient
      ? await loadDesktopWorkspaceAssets(assetImportClient, loading)
      : Object.freeze({ ...loading, state: "Failed" as const, errorCode: "COMMAND_BRIDGE_FAILED", retryable: false });
    if (assetSnapshotRef.current.generation === result.generation) commitAssetSnapshot(result);
  }, [commitAssetSnapshot]);

  const runAssetLoadMore = useCallback(async () => {
    const loading = requestDesktopWorkspaceAssetNextPage(assetSnapshotRef.current);
    if (loading === assetSnapshotRef.current) return;
    commitAssetSnapshot(loading);
    const result = assetImportClient
      ? await loadDesktopWorkspaceAssets(assetImportClient, loading)
      : Object.freeze({ ...loading, state: "Failed" as const, errorCode: "COMMAND_BRIDGE_FAILED", retryable: false });
    if (assetSnapshotRef.current.generation === result.generation) commitAssetSnapshot(result);
  }, [commitAssetSnapshot]);

  const runAssetImport = useCallback(async (preparedSelection?: DesktopAssetImportSelection) => {
    const selecting = beginDesktopAssetImport(assetSnapshotRef.current);
    if (selecting === assetSnapshotRef.current) return;
    commitAssetSnapshot(selecting);
    if (!assetImportClient || !desktopClient || !bootstrapUuidSource) {
      const failed = Object.freeze({
        ...selecting,
        importState: "Failed",
        importErrorCode: "COMMAND_BRIDGE_FAILED",
      } as const);
      commitAssetSnapshot(failed);
      return failed;
    }
    const importGeneration = selecting.importGeneration;
    const result = await importDesktopDocumentAssets(
      assetImportClient,
      desktopClient,
      selecting,
      () => `document-attachment-import-${bootstrapUuidSource()}`,
      (progress) => {
        if (assetSnapshotRef.current.importGeneration === importGeneration) commitAssetSnapshot(progress);
      },
      preparedSelection,
    );
    if (assetSnapshotRef.current.importGeneration === importGeneration) commitAssetSnapshot(result);
    if (result.importState === "Completed") await synchronizeAssetProjection(result);
    return result;
  }, [commitAssetSnapshot, synchronizeAssetProjection]);

  useEffect(() => {
    return () => graphCameraSaveSchedulerRef.current.dispose();
  }, []);

  useEffect(() => {
    const listen = getGlobalTauriEventListen();
    if (!listen) return undefined;
    let disposed = false;
    let unsubscribe: (() => void) | undefined;
    const acceptsDocumentAttachmentDrop = () =>
      visibleRoute(routeStateRef.current).kind === "Document"
      && documentInspectorStateRef.current.tab === "attachments"
      && assetSnapshotRef.current.scope === "Document"
      && Boolean(assetSnapshotRef.current.documentId);
    void subscribeTauriAssetDrop(listen, {
      onState(state) {
        if (!acceptsDocumentAttachmentDrop()) return;
        commitAssetSnapshot(applyDesktopAssetDragState(assetSnapshotRef.current, state));
      },
      onSelection(selection) {
        if (!acceptsDocumentAttachmentDrop()) return;
        commitAssetSnapshot(applyDesktopAssetDragState(assetSnapshotRef.current, {
          state: "dropped",
          fileCount: selection.files.length,
        }));
        void runAssetImport(selection);
      },
      onError(errorCode) {
        if (!acceptsDocumentAttachmentDrop()) return;
        commitAssetSnapshot(Object.freeze({
          ...assetSnapshotRef.current,
          dropState: "Idle",
          dropFileCount: 0,
          importState: "Failed",
          importErrorCode: errorCode,
        }));
      },
    }).then((cleanup) => {
      if (disposed) cleanup();
      else unsubscribe = cleanup;
    });
    return () => {
      disposed = true;
      unsubscribe?.();
    };
  }, [commitAssetSnapshot, runAssetImport]);

  const selectAndLoadAssetDetail = useCallback(async (assetId: string) => {
    const selected = selectDesktopAsset(assetSnapshotRef.current, assetId);
    commitAssetSnapshot(selected);
    if (!assetImportClient || selected.detailState !== "Loading") return;
    const detailed = await loadDesktopAssetDetail(assetImportClient, selected);
    const current = assetSnapshotRef.current;
    if (current.selectedAssetId === assetId) {
      commitAssetSnapshot(applyDesktopAssetDetailResult(current, detailed));
    }
  }, [commitAssetSnapshot]);

  const runAssetUnlink = useCallback(async () => {
    const current = assetSnapshotRef.current;
    if (!assetImportClient || !desktopClient || !current.selectedAssetId || current.mutationState === "Unlinking") return;
    commitAssetSnapshot(Object.freeze({ ...current, mutationState: "Unlinking" }));
    const result = await unlinkDesktopSelectedAsset(
      assetImportClient,
      desktopClient,
      current,
      () => bootstrapUuidSource ? `document-attachment-unlink-${bootstrapUuidSource()}` : "",
    );
    if (assetSnapshotRef.current.selectedAssetId === current.selectedAssetId) commitAssetSnapshot(result);
    if (result.mutationState === "Idle") await synchronizeAssetProjection(result);
    return result;
  }, [commitAssetSnapshot, synchronizeAssetProjection]);

  const runAssetLink = useCallback(async () => {
    const current = assetSnapshotRef.current;
    if (!assetImportClient || !desktopClient || !current.selectedAssetId || current.mutationState === "Linking") return;
    commitAssetSnapshot(Object.freeze({ ...current, mutationState: "Linking" }));
    const result = await linkDesktopSelectedAsset(
      assetImportClient,
      desktopClient,
      current,
      () => bootstrapUuidSource ? `document-attachment-link-${bootstrapUuidSource()}` : "",
    );
    if (assetSnapshotRef.current.selectedAssetId === current.selectedAssetId) commitAssetSnapshot(result);
    if (result.mutationState === "Idle") await synchronizeAssetProjection(result);
    return result;
  }, [commitAssetSnapshot, synchronizeAssetProjection]);

  const runAssetImportCancel = useCallback(async () => {
    if (!assetImportClient) return;
    const current = assetSnapshotRef.current;
    const result = await cancelDesktopAssetImport(assetImportClient, current);
    if (assetSnapshotRef.current.importOperationId === current.importOperationId) commitAssetSnapshot(result);
  }, [commitAssetSnapshot]);

  const runAssetProjectionRepair = useCallback(async (operationId: string) => {
    if (!projectionClient || !desktopClient) return;
    const current = assetSnapshotRef.current;
    const result = await repairDesktopAttachmentProjection(
      projectionClient,
      desktopClient,
      current,
      operationId,
      (progress) => {
        const stillCurrent = assetSnapshotRef.current.documentId === current.documentId
          && assetSnapshotRef.current.importOperations?.some((operation) => operation.operationId === operationId);
        if (stillCurrent) commitAssetSnapshot(progress);
      },
    );
    const stillCurrent = assetSnapshotRef.current.documentId === current.documentId
      && assetSnapshotRef.current.importOperations?.some((operation) => operation.operationId === operationId);
    if (stillCurrent) commitAssetSnapshot(result);
    return result;
  }, [commitAssetSnapshot]);

  const runAssetPreview = useCallback(async () => {
    const loading = requestDesktopAssetPreview(assetSnapshotRef.current);
    if (loading === assetSnapshotRef.current) return;
    commitAssetSnapshot(loading);
    if (!assetImportClient) return commitAssetSnapshot(Object.freeze({ ...loading, previewState: "Failed" }));
    const generation = loading.previewGeneration;
    const result = await loadDesktopAssetPreview(assetImportClient, loading);
    if (assetSnapshotRef.current.previewGeneration === generation) commitAssetSnapshot(result);
  }, [commitAssetSnapshot]);

  const closeAssetPreview = useCallback(() => {
    commitAssetSnapshot(closeDesktopAssetPreview(assetSnapshotRef.current));
  }, [commitAssetSnapshot]);

  const runAssetExternalOpen = useCallback(async () => {
    const opening = requestDesktopAssetOpen(assetSnapshotRef.current);
    if (opening === assetSnapshotRef.current) return;
    commitAssetSnapshot(opening);
    if (!assetImportClient) {
      return commitAssetSnapshot(Object.freeze({
        ...opening,
        openState: "OpenFailed" as const,
        openErrorCode: "COMMAND_BRIDGE_FAILED",
      }));
    }
    const generation = opening.openGeneration;
    const result = await openDesktopSelectedAsset(assetImportClient, opening);
    if (assetSnapshotRef.current.openGeneration === generation) commitAssetSnapshot(result);
  }, [commitAssetSnapshot]);

  const requestDesktopRoute = useCallback((
    target: DesktopRoute,
    selection: DesktopSelectionContext,
  ) => {
    const current = visibleRoute(routeState);
    const dirtyDocument = current.kind === "Document" && ![
      DocumentSaveCoordinatorState.NoDocument,
      DocumentSaveCoordinatorState.Clean,
      DocumentSaveCoordinatorState.Saved,
    ].includes(authoringSnapshot.saveState);
    let result = transitionDesktopRoute(routeState, {
      type: "TransitionRequested",
      target,
      selection,
      blocker: dirtyDocument && authoringSnapshot.documentId
        ? { kind: "DirtyDocument", resourceId: authoringSnapshot.documentId }
        : { kind: "None" },
    });
    if (result.state.status === "AwaitingDecision" && authoringController) {
      const close = authoringController.requestClose();
      setAuthoringSnapshot(authoringController.snapshot());
      if (close.canClose) {
        result = transitionDesktopRoute(routeState, {
          type: "TransitionRequested",
          target,
          selection,
          blocker: { kind: "None" },
        });
      }
    }
    setRouteState(result.state);
    if (result.state.status === "Stable") {
      routeQueryLifecycleRef.current = transitionDesktopRouteQueryLifecycle(
        routeQueryLifecycleRef.current,
        { type: "RouteActivated", route: visibleRoute(result.state).kind },
      );
    }
    return result.state.status === "Stable";
  }, [authoringSnapshot.documentId, authoringSnapshot.saveState, routeState]);

  const openCanvas = useCallback((documentId?: string) => {
    const selectedCanvasId = resolveDesktopCanvasMenuTarget({
      catalogState: canvasCatalogSnapshotRef.current.state,
      selectedCanvasId: canvasCatalogSnapshotRef.current.selectedCanvasId,
      entries: canvasCatalogSnapshotRef.current.entries,
      displayedCanvasId: canvasSnapshotRef.current.canvasId,
      displayedLifecycle: canvasSnapshotRef.current.canvas?.lifecycle,
    });
    requestDesktopRoute(
      { kind: "Canvas", ...(selectedCanvasId ? { canvasId: selectedCanvasId } : {}) },
      {
        workspaceId: "workspace-1",
        ...(documentId ? { documentId } : {}),
        ...(selectedCanvasId ? { canvasId: selectedCanvasId } : {}),
        originRoute: visibleRoute(routeStateRef.current).kind,
      },
    );
  }, [requestDesktopRoute]);

  const runCanvasCatalogLoad = useCallback(async () => {
    const loading = requestDesktopCanvasCatalogLoad(canvasCatalogSnapshotRef.current);
    commitCanvasCatalogSnapshot(loading);
    if (!canvasCatalogClient) return;
    const result = await loadDesktopCanvasCatalog(canvasCatalogClient, loading, 20);
    if (canvasCatalogSnapshotRef.current.generation !== loading.generation) return;
    commitCanvasCatalogSnapshot(result);
    const route = visibleRoute(routeStateRef.current);
    if (route.kind !== "Canvas" || route.canvasId || result.state !== "Ready" || !result.selectedCanvasId) return;
    requestDesktopRoute(
      { kind: "Canvas", canvasId: result.selectedCanvasId },
      {
        workspaceId: result.workspaceId,
        canvasId: result.selectedCanvasId,
        ...(currentSelection(routeStateRef.current).documentId
          ? { documentId: currentSelection(routeStateRef.current).documentId }
          : {}),
        originRoute: "Canvas",
      },
    );
  }, [commitCanvasCatalogSnapshot, requestDesktopRoute]);

  const chooseCanvas = useCallback(async (canvasId: string) => {
    if (!canvasCatalogClient) return;
    const entry = canvasCatalogSnapshotRef.current.entries.find(
      (candidate) => candidate.canvasId === canvasId,
    );
    if (entry?.lifecycle === "archived") {
      requestDesktopRoute(
        { kind: "Canvas", canvasId },
        { workspaceId: "workspace-1", canvasId, originRoute: "Canvas" },
      );
      return;
    }
    const selecting = requestDesktopCanvasSelection(canvasCatalogSnapshotRef.current, canvasId);
    if (selecting.state !== "Selecting") {
      if (canvasCatalogSnapshotRef.current.selectedCanvasId === canvasId) {
        requestDesktopRoute(
          { kind: "Canvas", canvasId },
          { workspaceId: "workspace-1", canvasId, originRoute: "Canvas" },
        );
      }
      return;
    }
    commitCanvasCatalogSnapshot(selecting);
    const result = await selectDesktopCanvasCatalog(canvasCatalogClient, selecting);
    if (canvasCatalogSnapshotRef.current.generation !== selecting.generation) return;
    commitCanvasCatalogSnapshot(result);
    if (result.state !== "Ready" || !result.selectedCanvasId) return;
    requestDesktopRoute(
      { kind: "Canvas", canvasId: result.selectedCanvasId },
      { workspaceId: result.workspaceId, canvasId: result.selectedCanvasId, originRoute: "Canvas" },
    );
  }, [commitCanvasCatalogSnapshot, requestDesktopRoute]);

  const createCanvasFromCatalog = useCallback(async () => {
    if (!canvasClient || !canvasCatalogClient || !bootstrapUuidSource) return;
    const canvasId = `canvas-${bootstrapUuidSource()}`;
    const loading = requestDesktopCanvasLoad(canvasSnapshotRef.current, canvasId);
    commitCanvasSnapshot(loading);
    const created = await createDesktopCanvas(canvasClient, loading, "새 캔버스");
    if (canvasSnapshotRef.current.generation !== loading.generation) return;
    commitCanvasSnapshot(created);
    if (created.state !== "Ready") return;
    const catalogLoading = requestDesktopCanvasCatalogLoad(canvasCatalogSnapshotRef.current);
    commitCanvasCatalogSnapshot(catalogLoading);
    const catalog = await loadDesktopCanvasCatalog(canvasCatalogClient, catalogLoading, 20);
    if (canvasCatalogSnapshotRef.current.generation !== catalogLoading.generation) return;
    commitCanvasCatalogSnapshot(catalog);
    const selecting = requestDesktopCanvasSelection(catalog, canvasId);
    const selected = selecting.state === "Selecting"
      ? await selectDesktopCanvasCatalog(canvasCatalogClient, selecting)
      : catalog;
    commitCanvasCatalogSnapshot(selected);
    if (selected.state !== "Ready" || selected.selectedCanvasId !== canvasId) return;
    requestDesktopRoute(
      { kind: "Canvas", canvasId },
      { workspaceId: selected.workspaceId, canvasId, originRoute: "Canvas" },
    );
  }, [commitCanvasCatalogSnapshot, commitCanvasSnapshot, requestDesktopRoute]);

  const changeGraphScope = useCallback((scope: "local" | "global") => {
    const intent = createDesktopGraphScopeIntent(
      scope,
      homeQuery.workspaceId,
      graphCenterDocumentId,
    );
    requestDesktopRoute(intent.route, intent.selection);
  }, [graphCenterDocumentId, requestDesktopRoute]);

  const loadHome = useCallback(async () => {
    const started = beginDesktopRouteQuery(routeQueryLifecycleRef.current, "Home");
    routeQueryLifecycleRef.current = started.state;
    if (!started.ticket) return;
    const ticket = started.ticket;
    setHomeModel(createPersonalWorkspaceHomeModel({ profile, healthState: "Loading" }));
    if (!desktopClient) {
      if (canApplyDesktopRouteQuery(routeQueryLifecycleRef.current, ticket)) {
        setHomeModel(createPersonalWorkspaceHomeFailedModel(profile, "COMMAND_BRIDGE_FAILED", false));
      }
      return;
    }
    const result = await loadDesktopWorkspaceHome(desktopClient, homeQuery);
    if (canApplyDesktopRouteQuery(routeQueryLifecycleRef.current, ticket)) setHomeModel(result);
  }, []);

  const loadNavigator = useCallback((next: DocumentNavigatorModel) => {
    setNavigatorModel(next);
    if (!desktopClient) {
      setNavigatorModel(createDocumentNavigatorFailedModel({
        workspaceId: next.workspaceId,
        view: next.view,
        viewKey: next.viewKey,
        filter: next.filter,
        generation: next.generation,
        errorCode: "COMMAND_BRIDGE_FAILED",
        retryable: false,
      }));
      return;
    }
    void loadDesktopDocumentNavigator(desktopClient, next).then((result) => {
      setNavigatorModel((current) =>
        current.generation === result.generation ? result : current,
      );
    });
  }, []);

  const openGlobalSearchOverlay = useCallback(() => {
    const origin = Object.freeze({
      route: visibleRoute(routeState),
      selection: currentSelection(routeState),
    });
    const opened = transitionGlobalSearchOverlay(globalSearchOverlayRef.current, {
      type: "OpenRequested",
      originRoute: origin.route,
    });
    if (!requestDesktopRoute({ kind: "Search" }, {
      workspaceId: "workspace-1",
      originRoute: origin.route.kind,
    })) return;
    setSearchOriginContext(origin);
    globalSearchOverlayRef.current = opened.state;
    setGlobalSearchOverlay(opened.state);
    generation.current += 1;
    loadNavigator(createDocumentNavigatorLoadingModel({
      workspaceId: "workspace-1",
      view: "Tree",
      generation: generation.current,
    }));
  }, [loadNavigator, requestDesktopRoute, routeState]);

  const openNavigator = useCallback((query?: string) => {
    const origin = Object.freeze({
      route: visibleRoute(routeState),
      selection: currentSelection(routeState),
    });
    const opened = transitionGlobalSearchOverlay(globalSearchOverlayRef.current, {
      type: "OpenRequested",
      originRoute: origin.route,
      query,
    });
    globalSearchOverlayRef.current = opened.state;
    setGlobalSearchOverlay(opened.state);
    const intent = createDesktopSearchNavigationIntent(
      query,
      "workspace-1",
      visibleRoute(routeState).kind,
    );
    if (intent.kind === "NoOp") return;
    if (!requestDesktopRoute(intent.route, intent.selection)) return;
    setSearchOriginContext(origin);
    setSearchReturnContext((current) => transitionDesktopSearchReturnContext(current, {
      type: "SearchStarted",
      query: intent.route.query,
    }));
    generation.current += 1;
    loadNavigator(createDocumentNavigatorLoadingModel({
      workspaceId: "workspace-1",
      view: "Tree",
      filter: intent.route.query,
      generation: generation.current,
    }));
  }, [loadNavigator, requestDesktopRoute, routeState]);

  useEffect(() => {
    const handleSearchShortcut = (event: KeyboardEvent) => {
      if (!isMacWorkspaceSearchShortcut(event)) return;
      event.preventDefault();
      openGlobalSearchOverlay();
      focusDesktopWorkspaceSearch(document);
    };
    window.addEventListener("keydown", handleSearchShortcut);
    return () => window.removeEventListener("keydown", handleSearchShortcut);
  }, [openGlobalSearchOverlay]);

  const selectNavigatorView = useCallback((view: DocumentNavigatorView, viewKey?: string) => {
    generation.current += 1;
    const next = transitionDocumentNavigatorModel(navigatorModel, {
      type: "ViewSelected",
      view,
      viewKey,
      generation: generation.current,
    });
    loadNavigator(next);
  }, [loadNavigator, navigatorModel]);

  const filterNavigator = useCallback((filter: string) => {
    generation.current += 1;
    const next = transitionDocumentNavigatorModel(navigatorModel, {
      type: "FilterChanged",
      filter,
      generation: generation.current,
    });
    loadNavigator(next);
  }, [loadNavigator, navigatorModel]);

  const retryNavigator = useCallback(() => {
    generation.current += 1;
    loadNavigator(transitionDocumentNavigatorModel(navigatorModel, {
      type: "RetryRequested",
      generation: generation.current,
    }));
  }, [loadNavigator, navigatorModel]);

  const closeSearchWithEscape = useCallback((query: string) => {
    const closed = transitionGlobalSearchOverlay(globalSearchOverlayRef.current, { type: "EscapePressed" });
    globalSearchOverlayRef.current = closed.state;
    setGlobalSearchOverlay(closed.state);
    const intent = createDesktopSearchEscapeIntent(query, searchOriginContext.route);
    if (intent.kind === "ClearQuery") {
      setSearchReturnContext(createDesktopSearchReturnContext());
      filterNavigator("");
      return;
    }
    requestDesktopRoute(intent.route, searchOriginContext.selection);
  }, [filterNavigator, requestDesktopRoute, searchOriginContext]);

  useEffect(() => {
    setSearchResultWindow((current) => transitionDesktopSearchResultWindow(current, {
      type: "Reconcile",
      generation: navigatorModel.generation,
      total: navigatorModel.items.length,
    }));
  }, [navigatorModel.generation, navigatorModel.items.length]);

  useEffect(() => {
    if (globalSearchOverlayRef.current.status !== "Searching") return;
    if (navigatorModel.displayState === "Ready" || navigatorModel.displayState === "EmptyResult") {
      const resolved = transitionGlobalSearchOverlay(globalSearchOverlayRef.current, {
        type: "SearchSucceeded",
        resultCount: navigatorModel.items.length,
      });
      globalSearchOverlayRef.current = resolved.state;
      setGlobalSearchOverlay(resolved.state);
      return;
    }
    if (navigatorModel.displayState === "Failed") {
      const failed = transitionGlobalSearchOverlay(globalSearchOverlayRef.current, {
        type: "SearchFailed",
        errorCode: "GLOBAL_SEARCH_QUERY_FAILED",
      });
      globalSearchOverlayRef.current = failed.state;
      setGlobalSearchOverlay(failed.state);
    }
  }, [navigatorModel.displayState, navigatorModel.items.length]);

  const openDocument = useCallback((documentId: string) => {
    const openingFromSearch = visibleRoute(routeState).kind === "Search"
      && searchReturnContext.status === "Results";
    let nextSearchReturnContext = searchReturnContext;
    const selected = transitionGlobalSearchOverlay(globalSearchOverlayRef.current, {
      type: "ResultOpened",
      result: { kind: "Document", documentId },
    });
    if (openingFromSearch && typeof document !== "undefined") {
      const viewport = captureDesktopSearchViewport(document, documentId);
      nextSearchReturnContext = transitionDesktopSearchReturnContext(searchReturnContext, {
        type: "ViewportCaptured",
        query: searchReturnContext.query,
        ...viewport,
      });
      nextSearchReturnContext = transitionDesktopSearchReturnContext(nextSearchReturnContext, {
        type: "ResultOpened",
        query: searchReturnContext.query,
        documentId,
      });
    }
    if (!requestDesktopRoute({ kind: "Document", documentId }, {
      workspaceId: "workspace-1",
      documentId,
      originRoute: visibleRoute(routeState).kind,
    })) return;
    if (openingFromSearch) {
      setSearchReturnContext(nextSearchReturnContext);
      globalSearchOverlayRef.current = selected.state;
      setGlobalSearchOverlay(selected.state);
    }
    beginAuthoringDiffGeneration();
    setHistoryState({ status: "Idle", entries: [] });
    authoringGeneration.current += 1;
    const requestGeneration = authoringGeneration.current;
    if (!authoringController) {
      setAuthoringSnapshot(failedAuthoringSnapshot("COMMAND_BRIDGE_FAILED"));
      return;
    }
    setAuthoringSnapshot(authoringController.snapshot());
    void authoringController
      .open({
        queryName: "get-current-document",
        workspaceId: "workspace-1",
        documentId,
      })
      .then((snapshot) => {
        if (authoringGeneration.current === requestGeneration) {
          setAuthoringSnapshot(snapshot);
        }
      })
      .catch(() => {
        if (authoringGeneration.current === requestGeneration) {
          setAuthoringSnapshot(failedAuthoringSnapshot("COMMAND_BRIDGE_FAILED"));
        }
      });
  }, [beginAuthoringDiffGeneration, requestDesktopRoute, routeState, searchReturnContext]);

  const openSearchAsset = useCallback((assetId: string) => {
    const selected = transitionGlobalSearchOverlay(globalSearchOverlayRef.current, {
      type: "ResultOpened",
      result: { kind: "Asset", assetId },
    });
    if (!requestDesktopRoute({ kind: "Assets", assetId }, {
      workspaceId: "workspace-1",
      assetId,
      originRoute: visibleRoute(routeState).kind,
    })) return;
    globalSearchOverlayRef.current = selected.state;
    setGlobalSearchOverlay(selected.state);
  }, [requestDesktopRoute, routeState]);

  const returnToSearch = useCallback(() => {
    if (searchReturnContext.status !== "DocumentOpen") return;
    const restored = transitionDesktopSearchReturnContext(searchReturnContext, { type: "ReturnRequested" });
    if (restored.status !== "Results") return;
    if (!requestDesktopRoute({ kind: "Search", query: restored.query }, {
      workspaceId: "workspace-1",
      documentId: searchReturnContext.documentId,
      originRoute: "Document",
    })) return;
    setSearchReturnContext(restored);
  }, [requestDesktopRoute, searchReturnContext]);

  const createNewDocument = useCallback(() => {
    beginAuthoringDiffGeneration();
    setHistoryState({ status: "Idle", entries: [] });
    authoringGeneration.current += 1;
    const requestGeneration = authoringGeneration.current;
    if (!desktopClient || !authoringController || !bootstrapUuidSource) {
      setAuthoringSnapshot(failedAuthoringSnapshot("COMMAND_BRIDGE_FAILED"));
      return;
    }
    const documentId = `doc-${bootstrapUuidSource()}`;
    if (!requestDesktopRoute({ kind: "Document", documentId }, {
      workspaceId: "workspace-1",
      documentId,
      originRoute: visibleRoute(routeState).kind,
    })) return;
    const body = "# 제목 없는 문서\n\n";
    setAuthoringSnapshot(authoringController.snapshot());
    void desktopClient
      .createDocument({
        operationId: `document-create-${bootstrapUuidSource()}`,
        workspaceId: "workspace-1",
        documentId,
        body,
        author: "local-user",
        summary: "문서 생성",
      })
      .then(() =>
        authoringController.open({
          queryName: "get-current-document",
          workspaceId: "workspace-1",
          documentId,
        }),
      )
      .then(async (snapshot) => {
        if (authoringGeneration.current === requestGeneration) {
          setAuthoringSnapshot(snapshot);
          await synchronizeDocumentKnowledgeSurfaces("workspace-1", documentId);
          if (authoringGeneration.current === requestGeneration) {
            void runGraphQuery({ scope: "local", centerDocumentId: documentId });
          }
        }
      })
      .catch(() => {
        if (authoringGeneration.current === requestGeneration) {
          setAuthoringSnapshot(failedAuthoringSnapshot("COMMAND_BRIDGE_FAILED"));
        }
      });
  }, [beginAuthoringDiffGeneration, requestDesktopRoute, routeState, runGraphQuery, synchronizeDocumentKnowledgeSurfaces]);

  const resumeDocument = useCallback(() => {
    const target = resolveDesktopDocumentMenuTarget(
      authoringSnapshot.documentId,
      homeModel.recentDocuments.map((document) => document.documentId),
    );
    if (target.kind === "EmptyWorkspace") {
      requestDesktopRoute({ kind: "Document" }, {
        workspaceId: "workspace-1",
        originRoute: visibleRoute(routeState).kind,
      });
      return;
    }
    openDocument(target.documentId);
  }, [authoringSnapshot.documentId, homeModel.recentDocuments, openDocument, requestDesktopRoute, routeState]);

  const runAuthoringSave = useCallback((kind: "manual" | "retry") => {
    if (!authoringController) return Promise.resolve(undefined);
    const pending = kind === "retry"
      ? authoringController.retrySave()
      : authoringController.manualSave();
    void pending.then(async (snapshot) => {
      setAuthoringSnapshot(snapshot);
      if (
        snapshot.workspaceId &&
        snapshot.documentId &&
        snapshot.saveState === DocumentSaveCoordinatorState.Saved
      ) {
        await synchronizeDocumentKnowledgeSurfaces(snapshot.workspaceId, snapshot.documentId);
      }
    });
    setAuthoringSnapshot(authoringController.snapshot());
    return pending;
  }, [synchronizeDocumentKnowledgeSurfaces]);

  useEffect(() => {
    if (surface !== "Authoring") return undefined;
    const handleSaveShortcut = (event: KeyboardEvent) => {
      if (!isMacDocumentSaveShortcut(event)) return;
      event.preventDefault();
      void runAuthoringSave("manual");
    };
    window.addEventListener("keydown", handleSaveShortcut);
    return () => window.removeEventListener("keydown", handleSaveShortcut);
  }, [surface, runAuthoringSave]);

  const loadAuthoringHistory = useCallback(() => {
    const snapshot = authoringController?.snapshot();
    if (!desktopClient || !snapshot?.workspaceId || !snapshot.documentId) return;
    const requestGeneration = authoringGeneration.current;
    const requestDocumentId = snapshot.documentId;
    setHistoryState({ status: "Loading", entries: [] });
    void desktopClient
      .listDocumentHistory({
        queryName: "get-document-history",
        workspaceId: snapshot.workspaceId,
        documentId: snapshot.documentId,
        limit: 50,
      })
      .then((page) => {
        if (
          authoringGeneration.current !== requestGeneration ||
          authoringController?.snapshot().documentId !== requestDocumentId
        ) return;
        const entries = presentDocumentHistory(page.entries, historyDateFormatter);
        setHistoryState({
          status: entries.length === 0 ? "Empty" : "Ready",
          entries,
          nextCursor: page.nextCursor,
        });
      })
      .catch(() => {
        if (
          authoringGeneration.current !== requestGeneration ||
          authoringController?.snapshot().documentId !== requestDocumentId
        ) return;
        setHistoryState({ status: "Failed", entries: [], errorCode: "COMMAND_BRIDGE_FAILED" });
      });
  }, []);

  const loadMoreAuthoringHistory = useCallback(() => {
    const snapshot = authoringController?.snapshot();
    const currentCursor = historyState.nextCursor;
    if (
      !desktopClient ||
      !snapshot?.workspaceId ||
      !snapshot.documentId ||
      !currentCursor ||
      historyState.status === "LoadingMore"
    ) return;
    const requestGeneration = authoringGeneration.current;
    const requestDocumentId = snapshot.documentId;
    setHistoryState((current) => current.nextCursor === currentCursor
      ? { ...current, status: "LoadingMore", loadMoreErrorCode: undefined }
      : current);
    void desktopClient
      .listDocumentHistory({
        queryName: "get-document-history",
        workspaceId: snapshot.workspaceId,
        documentId: snapshot.documentId,
        cursor: currentCursor,
        limit: 50,
      })
      .then((page) => {
        if (
          authoringGeneration.current !== requestGeneration ||
          authoringController?.snapshot().documentId !== requestDocumentId
        ) return;
        setHistoryState((current) => {
          if (current.nextCursor !== currentCursor) return current;
          const entriesByVersion = new Map(current.entries.map((entry) => [entry.versionId, entry]));
          for (const entry of presentDocumentHistory(page.entries, historyDateFormatter)) {
            if (!entriesByVersion.has(entry.versionId)) entriesByVersion.set(entry.versionId, entry);
          }
          const entries = [...entriesByVersion.values()];
          return {
            ...current,
            status: entries.length === 0 ? "Empty" : "Ready",
            entries,
            nextCursor: page.nextCursor,
            loadMoreErrorCode: undefined,
          };
        });
      })
      .catch(() => {
        if (
          authoringGeneration.current !== requestGeneration ||
          authoringController?.snapshot().documentId !== requestDocumentId
        ) return;
        setHistoryState((current) => current.nextCursor === currentCursor
          ? {
              ...current,
              status: current.entries.length === 0 ? "Empty" : "Ready",
              loadMoreErrorCode: "COMMAND_BRIDGE_FAILED",
            }
          : current);
      });
  }, [historyState.nextCursor, historyState.status]);

  const previewAuthoringRestore = useCallback((versionId: string) => {
    const snapshot = authoringController?.snapshot();
    if (!desktopClient || !snapshot?.workspaceId || !snapshot.documentId) return;
    const requestGeneration = authoringGeneration.current;
    const requestRestoreGeneration = ++restoreGeneration.current;
    const requestDocumentId = snapshot.documentId;
    const context = createDocumentRestoreRequestContext(
      requestGeneration,
      requestRestoreGeneration,
      requestDocumentId,
    );
    const targetVersionLabel = historyState.entries.find((entry) => entry.versionId === versionId)
      ?.versionLabel ?? "선택한 버전";
    const previewing = beginRestorePreview(versionId);
    setHistoryState((current) => ({ ...current, status: "Loading", restore: previewing }));
    void desktopClient
      .previewDocumentRestore({
        queryName: "preview-document-restore",
        workspaceId: snapshot.workspaceId,
        documentId: snapshot.documentId,
        targetVersionId: versionId,
      })
      .then((preview) => {
        if (!isCurrentAuthoringRestoreContext(context)) return;
        const restore = completeRestorePreview(previewing, {
          targetVersionId: preview.targetVersionId,
          expectedCurrentVersionId: preview.expectedCurrentVersionId,
          targetVersionLabel,
          changedLineCount: preview.diff.addedCount + preview.diff.removedCount,
          missingAssetLabels: preview.missingAssetLabels,
          canRestore: preview.canRestore,
          diff: preview.diff,
        });
        setHistoryState((current) => ({
          status: restore.status === "BlockedMissingAsset" || restore.status === "BlockedLargeDiff"
            ? "Blocked"
            : "PreviewReady",
          entries: current.entries,
          nextCursor: current.nextCursor,
          restore,
        }));
      })
      .catch(() => {
        if (!isCurrentAuthoringRestoreContext(context)) return;
        setHistoryState((current) => ({
          ...current,
          status: "Failed",
          errorCode: "COMMAND_BRIDGE_FAILED",
        }));
      });
  }, [historyState.entries, isCurrentAuthoringRestoreContext]);

  const isCurrentAuthoringDiffContext = useCallback((context: NonNullable<typeof diffOperationContextRef.current>) => (
    authoringGeneration.current === context.authoringGeneration
      && diffGeneration.current === context.diffGeneration
      && authoringController?.snapshot().documentId === context.documentId
  ), []);

  const publishAuthoringBackgroundDiff = useCallback((
    operation: DesktopDocumentDiffOperationSnapshot,
    context: NonNullable<typeof diffOperationContextRef.current>,
  ) => {
    if (!isCurrentAuthoringDiffContext(context)) return false;
    diffOperationSnapshotRef.current = operation;
    diffOperationContextRef.current = context;
    setHistoryState((current) => current.diff?.targetVersionId === context.target.targetVersionId
      ? { ...current, diff: presentDesktopDocumentDiffOperation(operation, context.target) }
      : current);
    return true;
  }, [isCurrentAuthoringDiffContext]);

  const pollAuthoringBackgroundDiff = useCallback(async (
    initial: DesktopDocumentDiffOperationSnapshot,
    context: NonNullable<typeof diffOperationContextRef.current>,
  ) => {
    if (!documentDiffOperationClient) return;
    let operation = initial;
    while (operation.state === "Accepted" || operation.state === "Running") {
      await waitForDocumentDiffOperationPoll();
      if (!isCurrentAuthoringDiffContext(context)) {
        void cancelDesktopDocumentDiffOperation(documentDiffOperationClient, operation);
        return;
      }
      const candidate = await pollDesktopDocumentDiffOperation(documentDiffOperationClient, operation);
      const active = diffOperationSnapshotRef.current;
      if (active.state !== "Accepted" && active.state !== "Running") return;
      operation = applyDesktopDocumentDiffOperationCandidate(active, candidate);
      if (!publishAuthoringBackgroundDiff(operation, context)) return;
    }
  }, [isCurrentAuthoringDiffContext, publishAuthoringBackgroundDiff]);

  const startAuthoringBackgroundDiff = useCallback(async (
    query: DocumentDiffQuery,
    target: DesktopDocumentDiffPresentationTarget,
    requestGeneration: number,
    requestDiffGeneration: number,
    requestDocumentId: string,
  ) => {
    if (!documentDiffOperationClient) return;
    const context = Object.freeze({
      authoringGeneration: requestGeneration,
      diffGeneration: requestDiffGeneration,
      documentId: requestDocumentId,
      target,
    });
    const operation = await startDesktopDocumentDiffOperation(
      documentDiffOperationClient,
      createDesktopDocumentDiffOperationSnapshot(),
      query,
    );
    if (!publishAuthoringBackgroundDiff(operation, context)) {
      if (operation.state === "Accepted" || operation.state === "Running") {
        void cancelDesktopDocumentDiffOperation(documentDiffOperationClient, operation);
      }
      return;
    }
    await pollAuthoringBackgroundDiff(operation, context);
  }, [pollAuthoringBackgroundDiff, publishAuthoringBackgroundDiff]);

  const cancelAuthoringBackgroundDiff = useCallback(() => {
    if (!documentDiffOperationClient) return;
    const operation = diffOperationSnapshotRef.current;
    const context = diffOperationContextRef.current;
    if (!context || (operation.state !== "Accepted" && operation.state !== "Running")) return;
    void cancelDesktopDocumentDiffOperation(documentDiffOperationClient, operation).then((candidate) => {
      if (!isCurrentAuthoringDiffContext(context)) return;
      const applied = applyDesktopDocumentDiffOperationCandidate(operation, candidate);
      publishAuthoringBackgroundDiff(applied, context);
    });
  }, [isCurrentAuthoringDiffContext, publishAuthoringBackgroundDiff]);

  const retryAuthoringBackgroundDiff = useCallback(() => {
    if (!documentDiffOperationClient) return;
    const operation = diffOperationSnapshotRef.current;
    const context = diffOperationContextRef.current;
    if (!context || !["Cancelled", "Expired", "Failed"].includes(operation.state)) return;
    void retryDesktopDocumentDiffOperation(documentDiffOperationClient, operation).then(async (candidate) => {
      if (!publishAuthoringBackgroundDiff(candidate, context)) return;
      await pollAuthoringBackgroundDiff(candidate, context);
    });
  }, [pollAuthoringBackgroundDiff, publishAuthoringBackgroundDiff]);

  const compareAuthoringVersion = useCallback((versionId: string) => {
    const snapshot = authoringController?.snapshot();
    if (!desktopClient || !snapshot?.workspaceId || !snapshot.documentId) return;
    const requestGeneration = authoringGeneration.current;
    const requestDiffGeneration = beginAuthoringDiffGeneration();
    const requestDocumentId = snapshot.documentId;
    const targetVersionLabel = historyState.entries.find((entry) => entry.versionId === versionId)
      ?.versionLabel ?? "선택한 버전";
    setHistoryState((current) => ({
      ...current,
      diff: {
        status: "Loading",
        targetVersionId: versionId,
        targetVersionLabel,
      },
    }));
    const query = Object.freeze({
        queryName: "compare-current-document-to-version",
        workspaceId: snapshot.workspaceId,
        documentId: snapshot.documentId,
        targetVersionId: versionId,
      } satisfies DocumentDiffQuery);
    void desktopClient
      .compareDocumentVersions({ ...query })
      .then((result) => {
        if (
          authoringGeneration.current !== requestGeneration ||
          diffGeneration.current !== requestDiffGeneration ||
          authoringController?.snapshot().documentId !== requestDocumentId
        ) return;
        if (result.status === "TooLarge" && documentDiffOperationClient) {
          void startAuthoringBackgroundDiff(
            query,
            { targetVersionId: versionId, targetVersionLabel },
            requestGeneration,
            requestDiffGeneration,
            requestDocumentId,
          );
          return;
        }
        setHistoryState((current) => {
          if (current.diff?.targetVersionId !== versionId) return current;
          if (result.status === "TooLarge") {
            return {
              ...current,
              diff: {
                status: "TooLarge",
                targetVersionId: versionId,
                targetVersionLabel,
                limitReason: result.limitReason ?? "bytes",
                attachmentDiff: result.attachmentDiff,
              },
            };
          }
          return {
            ...current,
            diff: {
              status: "Ready",
              targetVersionId: versionId,
              targetVersionLabel,
              addedCount: result.addedCount,
              removedCount: result.removedCount,
              attachmentDiff: result.attachmentDiff,
              titleDelta: result.titleDelta,
              hunks: result.hunks,
            },
          };
        });
      })
      .catch(() => {
        if (
          authoringGeneration.current !== requestGeneration ||
          diffGeneration.current !== requestDiffGeneration ||
          authoringController?.snapshot().documentId !== requestDocumentId
        ) return;
        setHistoryState((current) => current.diff?.targetVersionId === versionId
          ? {
              ...current,
              diff: {
                status: "Failed",
                targetVersionId: versionId,
                targetVersionLabel,
                errorCode: "COMMAND_BRIDGE_FAILED",
              },
            }
          : current);
      });
  }, [beginAuthoringDiffGeneration, historyState.entries, startAuthoringBackgroundDiff]);

  const toggleHistoryCompareSelection = useCallback((versionId: string, versionLabel: string) => {
    setHistoryState((current) => ({
      ...current,
      comparison: transitionDocumentHistoryCompareSelection(
        current.comparison ?? createDocumentHistoryCompareSelection(),
        { type: "Toggle", versionId, versionLabel },
      ),
    }));
  }, []);

  const compareSelectedAuthoringVersions = useCallback(() => {
    const snapshot = authoringController?.snapshot();
    const comparison = historyState.comparison;
    if (
      !desktopClient ||
      !snapshot?.workspaceId ||
      !snapshot.documentId ||
      comparison?.status !== "TwoSelected"
    ) return;
    const [left, right] = comparison.selections;
    const requestGeneration = authoringGeneration.current;
    const requestDiffGeneration = beginAuthoringDiffGeneration();
    const requestDocumentId = snapshot.documentId;
    const targetVersionLabel = `${left.versionLabel}과 ${right.versionLabel}`;
    setHistoryState((current) => ({
      ...current,
      diff: { status: "Loading", targetVersionId: right.versionId, targetVersionLabel },
    }));
    const query = Object.freeze({
      queryName: "compare-document-versions",
      workspaceId: snapshot.workspaceId,
      documentId: snapshot.documentId,
      leftVersionId: left.versionId,
      rightVersionId: right.versionId,
    } satisfies DocumentDiffQuery);
    void desktopClient.compareDocumentVersions({ ...query }).then((result) => {
      if (
        authoringGeneration.current !== requestGeneration ||
        diffGeneration.current !== requestDiffGeneration ||
        authoringController?.snapshot().documentId !== requestDocumentId
      ) return;
      if (result.status === "TooLarge" && documentDiffOperationClient) {
        void startAuthoringBackgroundDiff(
          query,
          { targetVersionId: right.versionId, targetVersionLabel },
          requestGeneration,
          requestDiffGeneration,
          requestDocumentId,
        );
        return;
      }
      setHistoryState((current) => {
        if (current.diff?.targetVersionId !== right.versionId) return current;
        if (result.status === "TooLarge") {
          return {
            ...current,
            diff: {
              status: "TooLarge",
              targetVersionId: right.versionId,
              targetVersionLabel,
              limitReason: result.limitReason ?? "bytes",
              attachmentDiff: result.attachmentDiff,
            },
          };
        }
        return {
          ...current,
          diff: {
            status: "Ready",
            targetVersionId: right.versionId,
            targetVersionLabel,
            addedCount: result.addedCount,
            removedCount: result.removedCount,
            attachmentDiff: result.attachmentDiff,
            titleDelta: result.titleDelta,
            hunks: result.hunks,
          },
        };
      });
    }).catch(() => {
      if (
        authoringGeneration.current !== requestGeneration ||
        diffGeneration.current !== requestDiffGeneration ||
        authoringController?.snapshot().documentId !== requestDocumentId
      ) return;
      setHistoryState((current) => current.diff?.targetVersionId === right.versionId
        ? {
            ...current,
            diff: {
              status: "Failed",
              targetVersionId: right.versionId,
              targetVersionLabel,
              errorCode: "COMMAND_BRIDGE_FAILED",
            },
          }
        : current);
    });
  }, [beginAuthoringDiffGeneration, historyState.comparison, startAuthoringBackgroundDiff]);

  const closeAuthoringDiff = useCallback(() => {
    beginAuthoringDiffGeneration();
    setHistoryState((current) => {
      const { diff: _discarded, ...history } = current;
      return history;
    });
  }, [beginAuthoringDiffGeneration]);

  const applyAuthoringRestore = useCallback(() => {
    const snapshot = authoringController?.snapshot();
    const restore = historyState.restore;
    if (
      !desktopClient ||
      !authoringController ||
      !bootstrapUuidSource ||
      !snapshot?.workspaceId ||
      !snapshot.documentId ||
      !restore ||
      (restore.status !== "Confirming" && restore.status !== "RecoveryRequired")
    ) {
      return;
    }
    const applying = restore.status === "RecoveryRequired"
      ? retryRestoreRecovery(restore)
      : beginRestoreApply(restore, `document-restore-${bootstrapUuidSource()}`);
    if (applying.status !== "Applying") return;
    const requestGeneration = authoringGeneration.current;
    const requestRestoreGeneration = ++restoreGeneration.current;
    const requestDocumentId = snapshot.documentId;
    const context = createDocumentRestoreRequestContext(
      requestGeneration,
      requestRestoreGeneration,
      requestDocumentId,
    );
    const preview = applying;
    let primaryCommitted = false;
    setHistoryState((current) => ({ ...current, status: "Applying", restore: applying }));
    void desktopClient
      .restoreDocumentVersion({
        commandName: "restore-document-version",
        operationId: preview.operationId,
        workspaceId: snapshot.workspaceId,
        documentId: snapshot.documentId,
        targetVersionId: preview.targetVersionId,
        expectedCurrentVersionId: preview.expectedCurrentVersionId,
        author: "local-user",
        summary: "문서 이력 복원",
      })
      .then(async (restored) => {
        primaryCommitted = true;
        if (!isCurrentAuthoringRestoreContext(context)) return undefined;
        const current = await desktopClient.getCurrentDocument({
          queryName: "get-current-document",
          workspaceId: snapshot.workspaceId,
          documentId: snapshot.documentId,
        });
        if (!isCurrentAuthoringRestoreContext(context)) return undefined;
        if (restored.restoredVersionId !== current.versionId) {
          throw new Error("restore readback mismatch");
        }
        const nextSnapshot = await authoringController.open({
          queryName: "get-current-document",
          workspaceId: snapshot.workspaceId,
          documentId: snapshot.documentId,
        });
        if (!isCurrentAuthoringRestoreContext(context)) return undefined;
        if (nextSnapshot.expectedVersionId !== restored.restoredVersionId) {
          throw new Error("restore controller mismatch");
        }
        const history = await desktopClient.listDocumentHistory({
          queryName: "get-document-history",
          workspaceId: snapshot.workspaceId,
          documentId: snapshot.documentId,
          limit: 50,
        });
        if (!isCurrentAuthoringRestoreContext(context)) return undefined;
        await synchronizeDocumentKnowledgeSurfaces(snapshot.workspaceId, snapshot.documentId);
        if (!isCurrentAuthoringRestoreContext(context)) return undefined;
        return {
          nextSnapshot,
          entries: presentDocumentHistory(history.entries, historyDateFormatter),
          nextCursor: history.nextCursor,
        };
      })
      .then((result) => {
        if (!result || !isCurrentAuthoringRestoreContext(context)) return;
        const { nextSnapshot, entries, nextCursor } = result;
        setAuthoringSnapshot(nextSnapshot);
        setHistoryState({
          status: "Applied",
          entries,
          nextCursor,
          restore: completeRestoreApply(applying),
        });
      })
      .catch((error: unknown) => {
        if (!isCurrentAuthoringRestoreContext(context)) return;
        const failure = error instanceof LocalDesktopCommandClientError
          ? { code: error.code, retryable: error.retryable, repairRequired: error.repairRequired }
          : {
              code: primaryCommitted
                ? "DOCUMENT_RESTORE_RECOVERY_REQUIRED"
                : "COMMAND_BRIDGE_FAILED",
              retryable: primaryCommitted,
              repairRequired: primaryCommitted,
            };
        setHistoryState((current) => ({
          ...current,
          status: "Failed",
          restore: failRestoreApply(applying, failure),
        }));
      });
  }, [historyState.restore, isCurrentAuthoringRestoreContext, synchronizeDocumentKnowledgeSurfaces]);

  const requestAuthoringRestoreConfirmation = useCallback(() => {
    setHistoryState((current) => ({
      ...current,
      restore: current.restore
        ? requestRestoreConfirmation(current.restore)
        : current.restore,
    }));
  }, []);

  const cancelAuthoringRestoreConfirmation = useCallback(() => {
    setHistoryState((current) => ({
      ...current,
      restore: current.restore
        ? cancelRestoreConfirmation(current.restore)
        : current.restore,
    }));
  }, []);

  const refreshAuthoringRestorePreview = useCallback(() => {
    const restore = historyState.restore;
    if (restore?.status === "Conflict") previewAuthoringRestore(restore.targetVersionId);
  }, [historyState.restore, previewAuthoringRestore]);

  useEffect(() => {
    if (
      surface !== "Authoring" ||
      authoringSnapshot.saveState !== DocumentSaveCoordinatorState.Dirty ||
      !authoringController
    ) {
      return undefined;
    }
    const requestGeneration = authoringGeneration.current;
    const runAuthoringAutosave = () => {
      void authoringController.autosaveElapsed(800).then(async (snapshot) => {
        if (authoringGeneration.current === requestGeneration) {
          setAuthoringSnapshot(snapshot);
          if (
            snapshot.workspaceId &&
            snapshot.documentId &&
            snapshot.saveState === DocumentSaveCoordinatorState.Saved
          ) {
            await synchronizeDocumentKnowledgeSurfaces(snapshot.workspaceId, snapshot.documentId);
          }
        }
      });
      setAuthoringSnapshot(authoringController.snapshot());
    };
    const timer = setTimeout(runAuthoringAutosave, 800);
    return () => clearTimeout(timer);
  }, [surface, authoringSnapshot.revision, authoringSnapshot.saveState, authoringController, synchronizeDocumentKnowledgeSurfaces]);

  useEffect(() => {
    const initial = graphPreferenceSnapshotRef.current;
    const loading = requestDesktopGraphPreferenceLoad(initial);
    graphPreferenceSnapshotRef.current = loading;
    if (!graphPreferenceClient) {
      const defaulted = applyDesktopGraphPreferenceLoad(
        loading, loading.generation, loading.workspaceId, createDefaultDesktopGraphPreference(),
      );
      graphPreferenceSnapshotRef.current = defaulted;
      setGraphIncludeExternal(defaulted.preference.includeExternal);
      commitGraphSnapshot(Object.freeze({
        ...graphSnapshotRef.current,
        query: Object.freeze({ ...graphSnapshotRef.current.query, ...graphQueryPatchFromPreference(defaulted.preference) }),
      }));
      return;
    }
    let active = true;
    void graphPreferenceClient.load(homeQuery.workspaceId)
      .then((preference) => {
        if (!active) return;
        const loaded = applyDesktopGraphPreferenceLoad(
          graphPreferenceSnapshotRef.current, loading.generation, loading.workspaceId, preference,
        );
        graphPreferenceSnapshotRef.current = loaded;
        setGraphIncludeExternal(loaded.preference.includeExternal);
        commitGraphSnapshot(Object.freeze({
          ...graphSnapshotRef.current,
          query: Object.freeze({ ...graphSnapshotRef.current.query, ...graphQueryPatchFromPreference(loaded.preference) }),
        }));
      })
      .catch(() => {
        if (!active) return;
        const defaulted = applyDesktopGraphPreferenceLoad(
          graphPreferenceSnapshotRef.current, loading.generation, loading.workspaceId, { invalid: true },
        );
        graphPreferenceSnapshotRef.current = defaulted;
        setGraphIncludeExternal(defaulted.preference.includeExternal);
        commitGraphSnapshot(Object.freeze({
          ...graphSnapshotRef.current,
          query: Object.freeze({ ...graphSnapshotRef.current.query, ...graphQueryPatchFromPreference(defaulted.preference) }),
        }));
      });
    return () => { active = false; };
  }, [commitGraphSnapshot]);

  useEffect(() => {
    void loadHome();
  }, [loadHome]);

  useEffect(() => {
    if (routeQueryLifecycleRef.current.activeRoute === activeRoute.kind) return;
    routeQueryLifecycleRef.current = transitionDesktopRouteQueryLifecycle(
      routeQueryLifecycleRef.current,
      { type: "RouteActivated", route: activeRoute.kind },
    );
  }, [activeRoute.kind]);

  useEffect(() => {
    if (surface === "Home" && previousSurface.current !== "Home") void loadHome();
    previousSurface.current = surface;
  }, [surface, loadHome]);

  useEffect(() => {
    if (surface === "Home") void runGraphQuery(
      { scope: "global", globalCursor: undefined },
      HOME_GRAPH_PROJECTION_LIMIT,
    );
    if (surface === "Graph") void runGraphQuery({ scope: graphQueryScope });
    if (surface === "Authoring") void runGraphQuery({ scope: "local" });
  }, [surface, graphCenterDocumentId, graphQueryScope]);

  useEffect(() => {
    if (surface === "Assets" || surface === "Authoring") void runAssetQuery();
  }, [surface, assetDocumentId, requestedAssetId]);

  useEffect(() => {
    if (surface === "Canvas") {
      void runCanvasCatalogLoad();
      if (activeCanvasId) void runCanvasLoad();
    }
    if (surface === "Backup") {
      void (async () => {
        await runBackupRecovery();
        await runBackupCatalogLoad();
      })();
    }
  }, [surface, activeCanvasId]);

  useEffect(() => {
    const canvas = canvasSnapshot.canvas;
    if (surface !== "Canvas" || canvasSnapshot.state !== "Ready" || !canvas) return;
    const signature = `${canvas.canvasId}\u0000${canvas.title}\u0000${canvas.lifecycle}`;
    if (canvasCatalogSignatureRef.current === signature) return;
    canvasCatalogSignatureRef.current = signature;
    const catalogEntry = canvasCatalogSnapshotRef.current.entries.find(
      (entry) => entry.canvasId === canvas.canvasId,
    );
    if (catalogEntry?.title === canvas.title && catalogEntry.lifecycle === canvas.lifecycle) return;
    void runCanvasCatalogLoad();
  }, [surface, canvasSnapshot.state, canvasSnapshot.canvas?.canvasId, canvasSnapshot.canvas?.title, canvasSnapshot.canvas?.lifecycle]);

  useEffect(() => {
    if (routeState.status !== "Stable" || typeof document === "undefined") return;
    globalThis.queueMicrotask(() => focusWorkspaceRouteMain(document));
  }, [routeState.status, activeRoute.kind]);

  useEffect(() => {
    if (surface !== "Navigator" || searchReturnContext.status !== "Results" || typeof document === "undefined") return;
    globalThis.queueMicrotask(() => restoreDesktopSearchViewport(document, searchReturnContext));
  }, [surface, searchReturnContext]);

  useEffect(() => {
    if (activeRoute.kind === "Document" || activeRoute.kind === "Search" || searchReturnContext.status === "Inactive") return;
    setSearchReturnContext(createDesktopSearchReturnContext());
  }, [activeRoute.kind, searchReturnContext.status]);

  useEffect(() => {
    if (surface !== "Backup" || backupSnapshot.state !== "Creating") return undefined;
    const timer = globalThis.setTimeout(() => { void runBackupPoll(); }, 150);
    return () => globalThis.clearTimeout(timer);
  }, [surface, backupSnapshot.state, backupSnapshot.generation, runBackupPoll]);

  useEffect(() => {
    if (surface !== "Backup" || backupSnapshot.state !== "Applying") return undefined;
    const timer = globalThis.setTimeout(() => { void runRestorePoll(); }, 150);
    return () => globalThis.clearTimeout(timer);
  }, [surface, backupSnapshot.state, backupSnapshot.generation, runRestorePoll]);

  useEffect(() => {
    const workspaceId = authoringSnapshot.workspaceId;
    const documentId = authoringSnapshot.documentId;
    if (surface !== "Authoring" || !desktopClient || !workspaceId || !documentId) return undefined;
    let active = true;
    const loading = requestDesktopLinkOverviewLoad(
      createDesktopLinkOverviewSnapshot(workspaceId, documentId),
    );
    setLinkOverviewSnapshot(loading);
    void loadDesktopLinkOverview(desktopClient, loading).then((snapshot) => {
      if (active) setLinkOverviewSnapshot(snapshot);
    });
    return () => {
      active = false;
    };
  }, [surface, authoringSnapshot.workspaceId, authoringSnapshot.documentId]);

  useEffect(() => {
    setDocumentInspectorState(createDocumentInspectorState());
  }, [authoringSnapshot.documentId]);

  const sidebarDocumentShortcuts = homeModel.recentDocuments.slice(0, 5).map((document) => ({
    label: document.title,
    actionId: "open-sidebar-document",
    onOpen: () => openDocument(document.documentId),
  }));

  if (surface === "Home") {
    return createDesktopWorkspaceHomeElement(homeModel, {
        documentShortcuts: sidebarDocumentShortcuts,
        onRetry: loadHome,
        onCreateDocument: createNewDocument,
        onOpenSearchOverlay: openGlobalSearchOverlay,
        onOpenNavigator: openNavigator,
        onResumeDocument: resumeDocument,
        onOpenGraph: () => requestDesktopRoute({ kind: "Graph", scope: "Global" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind }),
        onOpenCanvas: () => openCanvas(),
        onOpenAssets: () => requestDesktopRoute({ kind: "Assets" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind }),
        onOpenBackup: () => requestDesktopRoute({ kind: "Backup" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind }),
        onOpenDocument: openDocument,
        knowledgeGraph: graphSnapshot,
        onRetryKnowledgeGraph: () => {
          void runGraphQuery(
            { scope: "global", globalCursor: undefined },
            HOME_GRAPH_PROJECTION_LIMIT,
          );
        },
      });
  }
  if (surface === "Navigator") {
    return createDesktopDocumentNavigatorElement(navigatorModel, {
        onCreateDocument: createNewDocument,
        onHome: () => requestDesktopRoute({ kind: "Home" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind }),
        onDocument: resumeDocument,
        onView: selectNavigatorView,
        onSearchOpen: openGlobalSearchOverlay,
        onFilter: filterNavigator,
        onSearchEscape: closeSearchWithEscape,
        onRetry: retryNavigator,
        onOpenDocument: openDocument,
        onOpenAsset: openSearchAsset,
        onPreviousResults: () => setSearchResultWindow((current) => transitionDesktopSearchResultWindow(current, { type: "Previous" })),
        onNextResults: () => setSearchResultWindow((current) => transitionDesktopSearchResultWindow(current, { type: "Next" })),
        onGraph: () => requestDesktopRoute({ kind: "Graph", scope: "Global" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind }),
        onCanvas: () => openCanvas(),
        onAssets: () => requestDesktopRoute({ kind: "Assets" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind }),
        onBackup: () => requestDesktopRoute({ kind: "Backup" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind }),
      }, {
        resultWindow: searchResultWindow,
        documentShortcuts: sidebarDocumentShortcuts,
      });
  }
  const explorationCallbacks = {
    documentShortcuts: sidebarDocumentShortcuts,
    onCreateDocument: createNewDocument,
    onHome: () => requestDesktopRoute({ kind: "Home" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind }),
    onSearchOpen: openGlobalSearchOverlay,
    onSearch: openNavigator,
    onDocument: resumeDocument,
    onGraph: () => requestDesktopRoute({ kind: "Graph", scope: "Global" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind }),
    onCanvas: () => openCanvas(),
    onAssets: () => requestDesktopRoute({ kind: "Assets" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind }),
    onBackup: () => requestDesktopRoute({ kind: "Backup" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind }),
    onOpenDocument: openDocument,
    onOpenAsset: (assetId: string) => requestDesktopRoute({ kind: "Assets", assetId }, { workspaceId: "workspace-1", assetId, originRoute: activeRoute.kind }),
  };
  if (surface === "DocumentEmpty") {
    return createDesktopDocumentEmptyStateElement({
      onCreateDocument: createNewDocument,
      onHome: explorationCallbacks.onHome,
      onSearchOpen: explorationCallbacks.onSearchOpen,
      onSearch: explorationCallbacks.onSearch,
      onGraph: explorationCallbacks.onGraph,
      onCanvas: explorationCallbacks.onCanvas,
      onAssets: explorationCallbacks.onAssets,
      onBackup: explorationCallbacks.onBackup,
    }, { documentShortcuts: sidebarDocumentShortcuts });
  }
  if (surface === "Graph") {
    return createDesktopKnowledgeGraphElement(homeModel, graphSnapshot, {
      ...explorationCallbacks,
      onGraphQuery: (patch) => { void runGraphQuery(patch); },
      onGraphScopeChange: changeGraphScope,
      onGraphNodeSelect: (nodeId) => commitGraphSnapshot(selectDesktopGraphNode(graphSnapshotRef.current, nodeId)),
      onGraphRetry: () => { void runGraphQuery({}); },
      onGraphReindex: () => { void runGraphRepair(); },
      graphVisualSearch,
      onGraphVisualSearch: setGraphVisualSearch,
      graphCameraPreference: graphPreferenceSnapshotRef.current.preference.camera,
      onGraphCameraPreferenceChanged: queueGraphCameraPreference,
      graphIncludeExternal,
      onGraphIncludeExternalChange: updateGraphIncludeExternal,
    });
  }
  if (surface === "Canvas") {
    return createDesktopCanvasElement(homeModel, canvasSnapshot, {
      ...explorationCallbacks,
      canvasCatalog: canvasCatalogSnapshot,
      displayedCanvasId: activeCanvasId,
      onCanvasCatalogRetry: () => { void runCanvasCatalogLoad(); },
      onCanvasSelect: (canvasId) => { void chooseCanvas(canvasId); },
      onCanvasCreate: () => { void createCanvasFromCatalog(); },
      onCanvasRetry: () => { void runCanvasLoad(); },
      onCanvasRecover: () => { void runCanvasRecovery(); },
      onCanvasAddNote: () => {
        const canvas = canvasSnapshotRef.current.canvas;
        if (!canvas) return;
        const sequence = canvas.revision + canvas.nodes.length + 1;
        void runCanvasMutation({
          kind: "add_text_node",
          nodeId: `memo-${sequence}`,
          text: `새 메모 ${canvas.nodes.filter((node) => node.targetKind === "text").length + 1}`,
          x: 80 + (canvas.nodes.length % 4) * 360,
          y: 80 + Math.floor(canvas.nodes.length / 4) * 240,
          width: 320,
          height: 180,
        });
      },
      onCanvasAutoArrange: () => { void runCanvasArrangePreview(); },
      onCanvasApplyArrange: () => { void applyCanvasArrangePreview(); },
      onCanvasCancelArrange: () => commitCanvasSnapshot(cancelDesktopCanvasArrangePreview(canvasSnapshotRef.current)),
      onCanvasZoom: (zoomPercent) => {
        scheduleCanvasViewport({ zoomPercent });
      },
      onCanvasPan: (deltaX, deltaY) => scheduleCanvasViewport({ deltaX, deltaY }),
      onCanvasRemoveNode: (nodeId) => { void runCanvasMutation({ kind: "remove_node", nodeId }); },
      onCanvasAddDocument: (documentId) => {
        const canvas = canvasSnapshotRef.current.canvas;
        if (!canvas || !documentId) return;
        const sequence = canvas.revision + canvas.nodes.length + 1;
        void runCanvasMutation({ kind: "add_document_node", nodeId: `document-${sequence}`, documentId, x: 80, y: 80, width: 320, height: 180 });
      },
      onCanvasAddAsset: (assetId) => {
        const canvas = canvasSnapshotRef.current.canvas;
        if (!canvas || !assetId) return;
        const sequence = canvas.revision + canvas.nodes.length + 1;
        void runCanvasMutation({ kind: "add_asset_node", nodeId: `asset-${sequence}`, assetId, x: 440, y: 80, width: 320, height: 180 });
      },
      onCanvasConnect: () => {
        const canvas = canvasSnapshotRef.current.canvas;
        const [sourceNodeId, targetNodeId] = canvasSnapshotRef.current.selectedNodeIds;
        if (!canvas || !sourceNodeId || !targetNodeId) return;
        void runCanvasMutation({ kind: "connect_edge", edgeId: `edge-${canvas.revision + 1}`, sourceNodeId, targetNodeId });
      },
      onCanvasRemoveEdge: () => {
        const edgeId = canvasSnapshotRef.current.selectedEdgeId;
        if (edgeId) void runCanvasMutation({ kind: "remove_edge", edgeId });
      },
      canvasArchiveConfirmationOpen,
      canvasRenameDialogOpen,
      canvasRenameDraft,
      onCanvasArchiveRequest: () => setCanvasArchiveConfirmationOpen(true),
      onCanvasArchiveCancel: () => setCanvasArchiveConfirmationOpen(false),
      onCanvasRenameRequest: () => {
        const title = canvasSnapshotRef.current.canvas?.title;
        if (!title) return;
        setCanvasRenameDraft(title);
        setCanvasRenameDialogOpen(true);
      },
      onCanvasRenameDraftChange: setCanvasRenameDraft,
      onCanvasRenameCancel: () => setCanvasRenameDialogOpen(false),
      onCanvasRename: (title) => {
        const normalizedTitle = title.trim();
        if (!normalizedTitle || normalizedTitle === canvasSnapshotRef.current.canvas?.title) return;
        setCanvasRenameDialogOpen(false);
        void runCanvasMutation({ kind: "rename", title: normalizedTitle });
      },
      canvasTextEditDialog,
      onCanvasTextEditRequest: (nodeId, text) => setCanvasTextEditDialog((current) =>
        openCanvasTextEditDialog(
          current,
          nodeId,
          text,
          canvasSnapshotRef.current.state === "Ready"
            && canvasSnapshotRef.current.canvas?.lifecycle !== "archived",
        )),
      onCanvasTextEditDraftChange: (text) => setCanvasTextEditDialog((current) =>
        changeCanvasTextEditDraft(current, text)),
      onCanvasTextEditCancel: () => setCanvasTextEditDialog((current) => closeCanvasTextEditDialog(current)),
      onCanvasTextEdit: (nodeId, text) => {
        setCanvasTextEditDialog((current) => closeCanvasTextEditDialog(current));
        void runCanvasMutation({ kind: "update_text_card", nodeId, text });
      },
      onCanvasArchive: () => {
        setCanvasArchiveConfirmationOpen(false);
        void runCanvasMutation({ kind: "archive" });
      },
      onCanvasNodeSelect: (nodeId) => commitCanvasSnapshot(selectDesktopCanvasNode(canvasSnapshotRef.current, nodeId)),
      onCanvasEdgeSelect: (edgeId) => commitCanvasSnapshot(selectDesktopCanvasEdge(canvasSnapshotRef.current, edgeId)),
      onCanvasDragStart: (nodeId, clientX, clientY) => commitCanvasSnapshot(beginDesktopCanvasDrag(canvasSnapshotRef.current, nodeId, clientX, clientY)),
      onCanvasDragEnd: (nodeId, clientX, clientY) => {
        const result = finishDesktopCanvasDrag(canvasSnapshotRef.current, nodeId, clientX, clientY);
        commitCanvasSnapshot(result.snapshot);
        if (result.draft) void runCanvasMutation(result.draft);
      },
      onCanvasResizeStart: (nodeId, clientX, clientY) => commitCanvasSnapshot(beginDesktopCanvasResize(canvasSnapshotRef.current, nodeId, clientX, clientY)),
      onCanvasResizeEnd: (nodeId, clientX, clientY) => {
        const result = finishDesktopCanvasResize(canvasSnapshotRef.current, nodeId, clientX, clientY);
        commitCanvasSnapshot(result.snapshot);
        if (result.draft) void runCanvasMutation(result.draft);
      },
      canPlaceDocument: Boolean(currentSelection(routeState).documentId ?? homeModel.recentDocuments[0]?.documentId),
      canPlaceAsset: Boolean(assetSnapshot.selectedAssetId),
      documentPlacementOptions: homeModel.recentDocuments.map((document) => ({
        identity: document.documentId,
        label: document.title,
      })),
      assetPlacementOptions: createDesktopAssetPlacementOptions(assetSnapshot),
      selectedDocumentPlacementId: canvasDocumentPlacementId,
      selectedAssetPlacementId: canvasAssetPlacementId,
      onDocumentPlacementSelect: setCanvasDocumentPlacementId,
      onAssetPlacementSelect: setCanvasAssetPlacementId,
    });
  }
  if (surface === "Assets") {
    return createDesktopAttachmentsElement(homeModel, assetSnapshot, {
      ...explorationCallbacks,
      onAssetSelect: (assetId) => { void selectAndLoadAssetDetail(assetId); },
      onAssetRetry: () => { void runAssetQuery(); },
      onAssetImport: () => { void runAssetImport(); },
      onAssetWorkspace: () => { void runAssetWorkspaceLibrary(); },
      onAssetLink: () => { void runAssetLink(); },
      onAssetUnlink: () => { void runAssetUnlink(); },
      onAssetCancel: () => { void runAssetImportCancel(); },
      onAssetPreview: () => { void runAssetPreview(); },
      onAssetPreviewClose: closeAssetPreview,
      onAssetOpen: () => { void runAssetExternalOpen(); },
      onAssetQueryChange: (query) => commitAssetSnapshot(setDesktopAssetQuery(assetSnapshotRef.current, query)),
      onAssetMediaFilterChange: (filter) => commitAssetSnapshot(setDesktopAssetMediaFilter(assetSnapshotRef.current, filter)),
      onAssetLoadMore: () => { void runAssetLoadMore(); },
      onAssetRepair: (operationId) => { void runAssetProjectionRepair(operationId); },
    });
  }
  if (surface === "Backup") {
    return createDesktopBackupRecoveryElement(backupSnapshot, {
      onCreateDocument: createNewDocument,
      onHome: () => requestDesktopRoute({ kind: "Home" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind }),
      onSearchOpen: openGlobalSearchOverlay,
      onSearch: openNavigator,
      onDocument: resumeDocument,
      onGraph: () => requestDesktopRoute({ kind: "Graph", scope: "Global" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind }),
      onCanvas: () => openCanvas(),
      onAssets: () => requestDesktopRoute({ kind: "Assets" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind }),
      onCreateBackup: () => { void runBackupCreate(); },
      onCancelBackup: () => { void runBackupOperationCancel(); },
      onPreviewRestore: () => { void runBackupPreview(); },
      onConfirmRestore: () => { void runBackupConfirm(); },
      onCancelRestore: () => { void runBackupCancel(); },
      onRecover: () => { void runBackupRecovery(); },
      onReloadCatalog: () => { void runBackupCatalogLoad(); },
      onLoadMoreCatalog: () => { void runBackupCatalogLoad(backupSnapshotRef.current.catalogNextCursor); },
      onSelectCatalogPackage: (packageId) => commitBackupSnapshot(selectDesktopBackupCatalogPackage(backupSnapshotRef.current, packageId)),
    }, { documentShortcuts: sidebarDocumentShortcuts });
  }
  return createDesktopDocumentAuthoringWorkbenchElement(
    authoringSnapshot,
    {
      onHome() {
        requestDesktopRoute({ kind: "Home" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind });
      },
      onReturnToSearch: searchReturnContext.status === "DocumentOpen" ? returnToSearch : undefined,
      onOpenPlainTextEditor() {
        setPlainTextEditorOpen(true);
      },
      onClosePlainTextEditor() {
        setPlainTextEditorOpen(false);
      },
      onBodyChange(body) {
        if (authoringController) {
          setAuthoringSnapshot(authoringController.changeContent(body));
        }
      },
      onFormatCommand(command) {
        if (!authoringController) return;
        setAuthoringSnapshot(authoringController.changeContent(
          applyMarkdownFormattingCommand(authoringController.snapshot().body ?? "", command),
        ));
      },
      onSave() {
        runAuthoringSave("manual");
      },
      onRetry() {
        if (!authoringController) return;
        if (authoringController.snapshot().saveState === DocumentSaveCoordinatorState.CloseBlocked) {
          const restored = authoringController.cancelClose();
          setAuthoringSnapshot(restored);
          const operationId = `route-save-${restored.revision}`;
          setRouteState((current) => transitionDesktopRoute(current, {
            type: "ResolveAndContinue",
            operationId,
          }).state);
          void runAuthoringSave(
            restored.saveState === DocumentSaveCoordinatorState.SaveFailed ? "retry" : "manual",
          ).then((saved) => {
            if (!saved) return;
            setRouteState((current) => transitionDesktopRoute(current, saved.saveState === DocumentSaveCoordinatorState.Saved
              ? { type: "ResolutionCompleted", operationId }
              : { type: "ResolutionFailed", operationId, errorCode: saved.errorCode ?? "DOCUMENT_SAVE_FAILED" }).state);
          });
          return;
        }
        runAuthoringSave("retry");
      },
      onDiscard() {
        if (!authoringController) return;
        if (authoringController.snapshot().saveState !== DocumentSaveCoordinatorState.CloseBlocked) {
          authoringController.requestClose();
        }
        setAuthoringSnapshot(authoringController.discard());
        setRouteState((current) => transitionDesktopRoute(current, { type: "DiscardAndContinue" }).state);
      },
      onCancel() {
        if (authoringController) {
          setAuthoringSnapshot(authoringController.cancelClose());
        }
        setRouteState((current) => transitionDesktopRoute(current, { type: "CancelTransition" }).state);
      },
      onLoadHistory: loadAuthoringHistory,
      onLoadMoreHistory: loadMoreAuthoringHistory,
      onToggleHistoryCompareSelection: toggleHistoryCompareSelection,
      onCompareSelectedVersions: compareSelectedAuthoringVersions,
      onCreateDocument: createNewDocument,
      onCompareVersion: compareAuthoringVersion,
      onCloseDiff: closeAuthoringDiff,
      onCancelBackgroundDiff: cancelAuthoringBackgroundDiff,
      onRetryBackgroundDiff: retryAuthoringBackgroundDiff,
      onPreviewRestore: previewAuthoringRestore,
      onRequestRestoreConfirmation: requestAuthoringRestoreConfirmation,
      onCancelRestoreConfirmation: cancelAuthoringRestoreConfirmation,
      onApplyRestore: applyAuthoringRestore,
      onRefreshRestorePreview: refreshAuthoringRestorePreview,
      onContinueRestoreRecovery: applyAuthoringRestore,
      onSearchOpen: openGlobalSearchOverlay,
      onSearch: openNavigator,
      onGraph: () => requestDesktopRoute({ kind: "Graph", scope: "Local", centerDocumentId: authoringSnapshot.documentId }, { workspaceId: "workspace-1", documentId: authoringSnapshot.documentId, originRoute: activeRoute.kind }),
      onLocalGraphNodeSelect: (nodeId) => commitGraphSnapshot(selectDesktopGraphNode(graphSnapshotRef.current, nodeId)),
      onLocalGraphQuery: (patch) => { void runGraphQuery(patch); },
      onLocalGraphVisualSearch: setGraphVisualSearch,
      onLocalGraphCameraPreferenceChanged: queueGraphCameraPreference,
      onLocalGraphIncludeExternalChange: updateGraphIncludeExternal,
      onOpenLocalGraphAsset: (assetId) => requestDesktopRoute({ kind: "Assets", documentId: authoringSnapshot.documentId, assetId }, { workspaceId: "workspace-1", documentId: authoringSnapshot.documentId, assetId, originRoute: activeRoute.kind }),
      onLocalGraphRetry: () => { void runGraphQuery({ scope: "local" }); },
      onLocalGraphRepair: () => { void runGraphRepair(); },
      onCanvas: () => openCanvas(authoringSnapshot.documentId),
      onAssets: () => requestDesktopRoute({ kind: "Assets", documentId: authoringSnapshot.documentId }, { workspaceId: "workspace-1", documentId: authoringSnapshot.documentId, originRoute: activeRoute.kind }),
      onBackup: () => requestDesktopRoute({ kind: "Backup" }, { workspaceId: "workspace-1", documentId: authoringSnapshot.documentId, originRoute: activeRoute.kind }),
      onOpenLinkedDocument: openDocument,
      onInspectorTab: (tab) => {
        setDocumentInspectorState((current) => transitionDocumentInspector(current, { type: "SelectTab", tab }));
        if (tab === "attachments") void runAssetQuery();
      },
      onAssetImport: () => {
        void (async () => {
          const result = await runAssetImport();
          if (result?.importState === "Completed") await loadAuthoringHistory();
        })();
      },
      onAssetRetry: () => { void runAssetQuery(); },
      onAssetCancel: () => { void runAssetImportCancel(); },
      onAssetRepair: (operationId) => {
        void (async () => {
          const result = await runAssetProjectionRepair(operationId);
          if (result?.importState === "Completed") await loadAuthoringHistory();
        })();
      },
      onAssetSelect: (assetId) => { void selectAndLoadAssetDetail(assetId); },
      onAssetPreview: () => { void runAssetPreview(); },
      onAssetPreviewClose: closeAssetPreview,
      onAssetOpen: () => { void runAssetExternalOpen(); },
      onAssetUnlinkRequest: () => {
        const selected = assetSnapshotRef.current.page?.assets.find(
          (asset) => asset.assetId === assetSnapshotRef.current.selectedAssetId,
        );
        if (!selected) return;
        setDocumentInspectorState((current) => transitionDocumentInspector(current, {
          type: "RequestUnlink",
          fileName: selected.fileName,
        }));
      },
      onAssetUnlinkCancel: () => {
        setDocumentInspectorState((current) => transitionDocumentInspector(current, { type: "CancelUnlink" }));
      },
      onAssetUnlinkConfirm: () => {
        if (!["Confirming", "Failed"].includes(documentInspectorState.unlink.status)) return;
        setDocumentInspectorState((current) => transitionDocumentInspector(current, { type: "ConfirmUnlink" }));
        void (async () => {
          const result = await runAssetUnlink();
          if (result?.mutationState === "Idle") {
            await loadAuthoringHistory();
            setDocumentInspectorState((current) => transitionDocumentInspector(current, { type: "UnlinkSucceeded" }));
            return;
          }
          setDocumentInspectorState((current) => transitionDocumentInspector(current, { type: "UnlinkFailed" }));
        })();
      },
      onOpenLibrary: () => { void openDocumentAssetLibrary(authoringSnapshot.documentId); },
      onAssetLibraryClose: () => commitDocumentAssetLibraryState(closeDocumentAssetLibrary(documentAssetLibraryStateRef.current)),
      onAssetLibraryRetry: () => { void openDocumentAssetLibrary(documentAssetLibraryStateRef.current.documentId); },
      onAssetLibrarySelect: (assetId) => commitDocumentAssetLibraryState(selectDocumentAssetLibraryItem(documentAssetLibraryStateRef.current, assetId)),
      onAssetLibraryLoadMore: () => { void loadMoreDocumentAssetLibrary(); },
      onAssetLibraryLink: () => {
        void linkDocumentAssetLibrarySelection().then((completion) => {
          if (completion?.documentAssets) void loadAuthoringHistory();
        });
      },
    },
    {
      documentShortcuts: sidebarDocumentShortcuts,
      plainTextEditorOpen,
      history: historyState,
      links: linkOverviewSnapshot,
      assets: assetSnapshot,
      assetLibrary: documentAssetLibraryState,
      inspector: documentInspectorState,
      graph: graphSnapshot,
      graphVisualSearch,
      graphCameraPreference: graphPreferenceSnapshotRef.current.preference.camera,
      graphIncludeExternal,
    },
  );
}

function visibleRoute(state: DesktopRouteControllerState): DesktopRoute {
  return state.status === "Stable" ? state.route : state.currentRoute;
}

function currentSelection(state: DesktopRouteControllerState): DesktopSelectionContext {
  return state.status === "Stable" ? state.selection : state.currentSelection;
}

function surfaceForRoute(route: DesktopRoute): "Home" | "Navigator" | "Authoring" | "DocumentEmpty" | "Graph" | "Canvas" | "Assets" | "Backup" {
  switch (route.kind) {
    case "Home": return "Home";
    case "Search": return "Navigator";
    case "Document": return route.documentId ? "Authoring" : "DocumentEmpty";
    case "Graph": return "Graph";
    case "Canvas": return "Canvas";
    case "Assets": return "Assets";
    case "Backup": return "Backup";
  }
}

function emptyAuthoringSnapshot(): DesktopDocumentAuthoringSnapshot {
  return {
    revision: 0,
    persistedRevision: 0,
    saveState: DocumentSaveCoordinatorState.NoDocument,
  };
}

function failedAuthoringSnapshot(errorCode: string): DesktopDocumentAuthoringSnapshot {
  return {
    ...emptyAuthoringSnapshot(),
    saveState: DocumentSaveCoordinatorState.ReadOnlyRecovery,
    errorCode,
    retryable: false,
  };
}

const rootElement = document.querySelector("#app");
if (rootElement) {
  createRoot(rootElement).render(React.createElement(DesktopApp));
  if (bootstrapInvoke) {
    globalThis.setTimeout(() => {
      void runPackagedUiSmoke({ invoke: bootstrapInvoke, document });
    }, 0);
  }
}
