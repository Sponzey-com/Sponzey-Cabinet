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
  type DocumentEditorViewMode,
  type DocumentNavigatorModel,
  type PersonalWorkspaceHomeModel,
} from "@sponzey-cabinet/ui";
import type { DocumentDiffQuery, DocumentNavigatorView } from "@sponzey-cabinet/client-core";

import { loadDesktopDocumentNavigator } from "./desktop_navigator_controller.ts";
import {
  cancelDesktopBackupOperation,
  cancelDesktopRestore,
  createDesktopBackupRecoverySnapshot,
  dismissDesktopRestoreConfirmation,
  pollDesktopBackupOperation,
  pollDesktopRestoreOperation,
  previewDesktopRestore,
  recoverDesktopBackupStartup,
  startDesktopBackupOperation,
  startDesktopRestoreOperation,
  type DesktopBackupRecoverySnapshot,
} from "./desktop_backup_recovery_controller.ts";
import {
  createDesktopLinkOverviewSnapshot,
  loadDesktopLinkOverview,
  requestDesktopLinkOverviewLoad,
  type DesktopLinkOverviewSnapshot,
} from "./desktop_link_overview_controller.ts";
import {
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
import { createTauriGlobalGraphTransport } from "./tauri_global_graph_transport.ts";
import { createTauriAssetImportTransport } from "./tauri_asset_import_transport.ts";
import { createTauriCanvasTransport, type DesktopCanvasMutationDraft } from "./tauri_canvas_transport.ts";
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
  beginRestoreApply,
  beginRestorePreview,
  cancelRestoreConfirmation,
  completeRestoreApply,
  completeRestorePreview,
  failRestoreApply,
  requestRestoreConfirmation,
  retryRestoreRecovery,
} from "./document_restore_presentation.ts";

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
const assetImportClient = bootstrapInvoke ? createTauriAssetImportTransport(bootstrapInvoke) : undefined;
const canvasClient = bootstrapInvoke ? createTauriCanvasTransport(bootstrapInvoke) : undefined;
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
  const generation = useRef(0);
  const [navigatorModel, setNavigatorModel] = useState<DocumentNavigatorModel>(() =>
    createDocumentNavigatorLoadingModel({
      workspaceId: "workspace-1",
      view: "Tree",
      generation: 0,
    }),
  );
  const authoringGeneration = useRef(0);
  const diffGeneration = useRef(0);
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
  const [linkOverviewSnapshot, setLinkOverviewSnapshot] = useState<DesktopLinkOverviewSnapshot>(
    () => createDesktopLinkOverviewSnapshot("workspace-1", "unloaded"),
  );
  const [editorViewMode, setEditorViewMode] = useState<DocumentEditorViewMode>("split");
  const [graphSnapshot, setGraphSnapshot] = useState<DesktopGraphSurfaceSnapshot>(() =>
    createDesktopGraphSnapshot("workspace-1"),
  );
  const graphSnapshotRef = useRef(graphSnapshot);
  const [assetSnapshot, setAssetSnapshot] = useState<DesktopAssetSurfaceSnapshot>(() =>
    createDesktopAssetSnapshot("workspace-1"),
  );
  const assetSnapshotRef = useRef(assetSnapshot);
  const [canvasSnapshot, setCanvasSnapshot] = useState<DesktopCanvasSurfaceSnapshot>(() =>
    createDesktopCanvasSnapshot("workspace-1"),
  );
  const [canvasArchiveConfirmationOpen, setCanvasArchiveConfirmationOpen] = useState(false);
  const [canvasRenameDialogOpen, setCanvasRenameDialogOpen] = useState(false);
  const [canvasRenameDraft, setCanvasRenameDraft] = useState("");
  const [canvasDocumentPlacementId, setCanvasDocumentPlacementId] = useState<string>();
  const [canvasAssetPlacementId, setCanvasAssetPlacementId] = useState<string>();
  const canvasSnapshotRef = useRef(canvasSnapshot);
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
    : undefined;
  const graphQueryScope = activeRoute.kind === "Graph" ? graphQueryScopeForRoute(activeRoute) : "local";
  const assetDocumentId = activeRoute.kind === "Assets"
    ? activeRoute.documentId
    : activeRoute.kind === "Document"
      ? activeRoute.documentId
      : undefined;
  const requestedAssetId = activeRoute.kind === "Assets" ? activeRoute.assetId : undefined;
  const activeCanvasId = activeRoute.kind === "Canvas" ? activeRoute.canvasId : undefined;

  const commitGraphSnapshot = useCallback((snapshot: DesktopGraphSurfaceSnapshot) => {
    graphSnapshotRef.current = snapshot;
    setGraphSnapshot(snapshot);
  }, []);

  const commitCanvasSnapshot = useCallback((snapshot: DesktopCanvasSurfaceSnapshot) => {
    if (snapshot.generation < canvasSnapshotRef.current.generation) return;
    canvasSnapshotRef.current = snapshot;
    setCanvasSnapshot(snapshot);
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

  const runBackupPoll = useCallback(async () => {
    if (!backupClient) return;
    const current = backupSnapshotRef.current;
    const next = await pollDesktopBackupOperation(backupClient, current);
    if (backupSnapshotRef.current.generation !== current.generation
      || backupSnapshotRef.current.operationId !== current.operationId) return;
    commitBackupSnapshot(next);
  }, [commitBackupSnapshot]);

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

  const runGraphQuery = useCallback(async (patch: Partial<DesktopGraphQueryState>) => {
    const loading = requestDesktopGraphLoad(graphSnapshotRef.current, {
      centerDocumentId: graphCenterDocumentId,
      ...patch,
    });
    commitGraphSnapshot(loading);
    if (loading.state !== "Loading") return;
    if (loading.query.scope === "global") {
      if (!globalGraphClient) return;
      const result = await loadDesktopGlobalKnowledgeGraph(globalGraphClient, loading);
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

  const runGraphRepair = useCallback(async () => {
    const repairing = requestDesktopGraphRepair(graphSnapshotRef.current);
    commitGraphSnapshot(repairing);
    if (repairing.state !== "Repairing" || !desktopClient || !projectionClient) return;
    const result = await repairDesktopKnowledgeGraph(projectionClient, desktopClient, repairing);
    if (graphSnapshotRef.current.generation === result.generation) commitGraphSnapshot(result);
  }, [commitGraphSnapshot]);

  const commitAssetSnapshot = useCallback((snapshot: DesktopAssetSurfaceSnapshot) => {
    assetSnapshotRef.current = snapshot;
    setAssetSnapshot(snapshot);
  }, []);

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

  const runAssetImport = useCallback(async () => {
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
    );
    if (assetSnapshotRef.current.importGeneration === importGeneration) commitAssetSnapshot(result);
    return result;
  }, [commitAssetSnapshot]);

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
    return result;
  }, [commitAssetSnapshot]);

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
  }, [commitAssetSnapshot]);

  const runAssetImportCancel = useCallback(async () => {
    if (!assetImportClient) return;
    const current = assetSnapshotRef.current;
    const result = await cancelDesktopAssetImport(assetImportClient, current);
    if (assetSnapshotRef.current.importOperationId === current.importOperationId) commitAssetSnapshot(result);
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
    return result.state.status === "Stable";
  }, [authoringSnapshot.documentId, authoringSnapshot.saveState, routeState]);

  const loadHome = useCallback(async () => {
    setHomeModel(createPersonalWorkspaceHomeModel({ profile, healthState: "Loading" }));
    if (!desktopClient) {
      setHomeModel(createPersonalWorkspaceHomeFailedModel(profile, "COMMAND_BRIDGE_FAILED", false));
      return;
    }
    setHomeModel(await loadDesktopWorkspaceHome(desktopClient, homeQuery));
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

  const openNavigator = useCallback(() => {
    if (!requestDesktopRoute({ kind: "Search" }, {
      workspaceId: "workspace-1",
      originRoute: visibleRoute(routeState).kind,
    })) return;
    generation.current += 1;
    loadNavigator(createDocumentNavigatorLoadingModel({
      workspaceId: "workspace-1",
      view: "Tree",
      generation: generation.current,
    }));
  }, [loadNavigator, requestDesktopRoute, routeState]);

  useEffect(() => {
    const handleSearchShortcut = (event: KeyboardEvent) => {
      if (!isMacWorkspaceSearchShortcut(event)) return;
      event.preventDefault();
      openNavigator();
    };
    window.addEventListener("keydown", handleSearchShortcut);
    return () => window.removeEventListener("keydown", handleSearchShortcut);
  }, [openNavigator]);

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

  const openDocument = useCallback((documentId: string) => {
    if (!requestDesktopRoute({ kind: "Document", documentId }, {
      workspaceId: "workspace-1",
      documentId,
      originRoute: visibleRoute(routeState).kind,
    })) return;
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
  }, [beginAuthoringDiffGeneration, requestDesktopRoute, routeState]);

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
  }, [beginAuthoringDiffGeneration, requestDesktopRoute, routeState]);

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
    void pending.then(setAuthoringSnapshot);
    setAuthoringSnapshot(authoringController.snapshot());
    return pending;
  }, []);

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
        setHistoryState((current) => ({
          ...current,
          status: "Failed",
          errorCode: "COMMAND_BRIDGE_FAILED",
        }));
      });
  }, [historyState.entries]);

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
        const current = await desktopClient.getCurrentDocument({
          queryName: "get-current-document",
          workspaceId: snapshot.workspaceId,
          documentId: snapshot.documentId,
        });
        if (restored.restoredVersionId !== current.versionId) {
          throw new Error("restore readback mismatch");
        }
        const nextSnapshot = await authoringController.open({
          queryName: "get-current-document",
          workspaceId: snapshot.workspaceId,
          documentId: snapshot.documentId,
        });
        if (nextSnapshot.expectedVersionId !== restored.restoredVersionId) {
          throw new Error("restore controller mismatch");
        }
        const history = await desktopClient.listDocumentHistory({
          queryName: "get-document-history",
          workspaceId: snapshot.workspaceId,
          documentId: snapshot.documentId,
          limit: 50,
        });
        return {
          nextSnapshot,
          entries: presentDocumentHistory(history.entries, historyDateFormatter),
          nextCursor: history.nextCursor,
        };
      })
      .then(({ nextSnapshot, entries, nextCursor }) => {
        setAuthoringSnapshot(nextSnapshot);
        setHistoryState({
          status: "Applied",
          entries,
          nextCursor,
          restore: completeRestoreApply(applying),
        });
      })
      .catch((error: unknown) => {
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
  }, [historyState.restore]);

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
      void authoringController.autosaveElapsed(800).then((snapshot) => {
        if (authoringGeneration.current === requestGeneration) {
          setAuthoringSnapshot(snapshot);
        }
      });
      setAuthoringSnapshot(authoringController.snapshot());
    };
    const timer = setTimeout(runAuthoringAutosave, 800);
    return () => clearTimeout(timer);
  }, [surface, authoringSnapshot.revision, authoringSnapshot.saveState]);

  useEffect(() => {
    let active = true;
    if (!desktopClient) {
      setHomeModel(createPersonalWorkspaceHomeFailedModel(profile, "COMMAND_BRIDGE_FAILED", false));
      return () => {
        active = false;
      };
    }
    void loadDesktopWorkspaceHome(desktopClient, homeQuery).then((next) => {
      if (active) setHomeModel(next);
    });
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    if (surface === "Home" && previousSurface.current !== "Home") void loadHome();
    previousSurface.current = surface;
  }, [surface, loadHome]);

  useEffect(() => {
    if (surface === "Graph") void runGraphQuery({ scope: graphQueryScope });
  }, [surface, graphCenterDocumentId, graphQueryScope]);

  useEffect(() => {
    if (surface === "Assets" || surface === "Authoring") void runAssetQuery();
  }, [surface, assetDocumentId, requestedAssetId]);

  useEffect(() => {
    if (surface === "Canvas") void runCanvasLoad();
    if (surface === "Backup") void runBackupRecovery();
  }, [surface, activeCanvasId]);

  useEffect(() => {
    if (routeState.status !== "Stable" || typeof document === "undefined") return;
    globalThis.queueMicrotask(() => focusWorkspaceRouteMain(document));
  }, [routeState.status, activeRoute.kind]);

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

  if (surface === "Home") {
    return createDesktopWorkspaceHomeElement(homeModel, {
        onRetry: loadHome,
        onCreateDocument: createNewDocument,
        onOpenNavigator: openNavigator,
        onResumeDocument: resumeDocument,
        onOpenGraph: () => requestDesktopRoute({ kind: "Graph", scope: "Global" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind }),
        onOpenCanvas: () => requestDesktopRoute({ kind: "Canvas", canvasId: "default-canvas" }, { workspaceId: "workspace-1", canvasId: "default-canvas", originRoute: activeRoute.kind }),
        onOpenAssets: () => requestDesktopRoute({ kind: "Assets" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind }),
        onOpenBackup: () => requestDesktopRoute({ kind: "Backup" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind }),
        onOpenDocument: openDocument,
      });
  }
  if (surface === "Navigator") {
    return createDesktopDocumentNavigatorElement(navigatorModel, {
        onCreateDocument: createNewDocument,
        onHome: () => requestDesktopRoute({ kind: "Home" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind }),
        onDocument: resumeDocument,
        onView: selectNavigatorView,
        onFilter: filterNavigator,
        onRetry: retryNavigator,
        onOpenDocument: openDocument,
        onGraph: () => requestDesktopRoute({ kind: "Graph", scope: "Global" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind }),
        onCanvas: () => requestDesktopRoute({ kind: "Canvas", canvasId: "default-canvas" }, { workspaceId: "workspace-1", canvasId: "default-canvas", originRoute: activeRoute.kind }),
        onAssets: () => requestDesktopRoute({ kind: "Assets" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind }),
        onBackup: () => requestDesktopRoute({ kind: "Backup" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind }),
      });
  }
  const explorationCallbacks = {
    onCreateDocument: createNewDocument,
    onHome: () => requestDesktopRoute({ kind: "Home" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind }),
    onSearch: openNavigator,
    onDocument: resumeDocument,
    onGraph: () => requestDesktopRoute({ kind: "Graph", scope: "Global" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind }),
    onCanvas: () => requestDesktopRoute({ kind: "Canvas", canvasId: "default-canvas" }, { workspaceId: "workspace-1", canvasId: "default-canvas", originRoute: activeRoute.kind }),
    onAssets: () => requestDesktopRoute({ kind: "Assets" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind }),
    onBackup: () => requestDesktopRoute({ kind: "Backup" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind }),
    onOpenDocument: openDocument,
    onOpenAsset: (assetId: string) => requestDesktopRoute({ kind: "Assets", assetId }, { workspaceId: "workspace-1", assetId, originRoute: activeRoute.kind }),
  };
  if (surface === "DocumentEmpty") {
    return createDesktopDocumentEmptyStateElement({
      onCreateDocument: createNewDocument,
      onHome: explorationCallbacks.onHome,
      onSearch: explorationCallbacks.onSearch,
      onGraph: explorationCallbacks.onGraph,
      onCanvas: explorationCallbacks.onCanvas,
      onAssets: explorationCallbacks.onAssets,
      onBackup: explorationCallbacks.onBackup,
    });
  }
  if (surface === "Graph") {
    return createDesktopKnowledgeGraphElement(homeModel, graphSnapshot, {
      ...explorationCallbacks,
      onGraphQuery: (patch) => { void runGraphQuery(patch); },
      onGraphNodeSelect: (nodeId) => commitGraphSnapshot(selectDesktopGraphNode(graphSnapshotRef.current, nodeId)),
      onGraphRetry: () => { void runGraphQuery({}); },
      onGraphReindex: () => { void runGraphRepair(); },
    });
  }
  if (surface === "Canvas") {
    return createDesktopCanvasElement(homeModel, canvasSnapshot, {
      ...explorationCallbacks,
      onCanvasCreate: () => {
        if (!canvasClient) return;
        void createDesktopCanvas(canvasClient, canvasSnapshotRef.current, "Cabinet 제품 지도")
          .then(commitCanvasSnapshot);
      },
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
    });
  }
  if (surface === "Backup") {
    return createDesktopBackupRecoveryElement(backupSnapshot, {
      onCreateDocument: createNewDocument,
      onHome: () => requestDesktopRoute({ kind: "Home" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind }),
      onSearch: openNavigator,
      onDocument: resumeDocument,
      onGraph: () => requestDesktopRoute({ kind: "Graph", scope: "Global" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind }),
      onCanvas: () => requestDesktopRoute({ kind: "Canvas", canvasId: "default-canvas" }, { workspaceId: "workspace-1", canvasId: "default-canvas", originRoute: activeRoute.kind }),
      onAssets: () => requestDesktopRoute({ kind: "Assets" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind }),
      onCreateBackup: () => { void runBackupCreate(); },
      onCancelBackup: () => { void runBackupOperationCancel(); },
      onPreviewRestore: () => { void runBackupPreview(); },
      onConfirmRestore: () => { void runBackupConfirm(); },
      onCancelRestore: () => { void runBackupCancel(); },
      onRecover: () => { void runBackupRecovery(); },
    });
  }
  return createDesktopDocumentAuthoringWorkbenchElement(
    authoringSnapshot,
    {
      onHome() {
        requestDesktopRoute({ kind: "Home" }, { workspaceId: "workspace-1", originRoute: activeRoute.kind });
      },
      onMode: setEditorViewMode,
      onBodyChange(body) {
        if (authoringController) {
          setAuthoringSnapshot(authoringController.changeContent(body));
        }
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
      onSearch: openNavigator,
      onGraph: () => requestDesktopRoute({ kind: "Graph", scope: "Global" }, { workspaceId: "workspace-1", documentId: authoringSnapshot.documentId, originRoute: activeRoute.kind }),
      onCanvas: () => requestDesktopRoute({ kind: "Canvas", canvasId: "default-canvas" }, { workspaceId: "workspace-1", documentId: authoringSnapshot.documentId, canvasId: "default-canvas", originRoute: activeRoute.kind }),
      onAssets: () => requestDesktopRoute({ kind: "Assets", documentId: authoringSnapshot.documentId }, { workspaceId: "workspace-1", documentId: authoringSnapshot.documentId, originRoute: activeRoute.kind }),
      onBackup: () => requestDesktopRoute({ kind: "Backup" }, { workspaceId: "workspace-1", documentId: authoringSnapshot.documentId, originRoute: activeRoute.kind }),
      onOpenLinkedDocument: openDocument,
      onInspectorTab: (tab) => {
        setDocumentInspectorState((current) => transitionDocumentInspector(current, { type: "SelectTab", tab }));
      },
      onAssetImport: () => {
        void (async () => {
          const result = await runAssetImport();
          if (result?.importState === "Completed") await loadAuthoringHistory();
        })();
      },
      onAssetRetry: () => { void runAssetQuery(); },
      onAssetCancel: () => { void runAssetImportCancel(); },
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
      onOpenLibrary: () => requestDesktopRoute({ kind: "Assets", documentId: authoringSnapshot.documentId }, { workspaceId: "workspace-1", documentId: authoringSnapshot.documentId, originRoute: activeRoute.kind }),
    },
    {
      viewMode: editorViewMode,
      history: historyState,
      links: linkOverviewSnapshot,
      assets: assetSnapshot,
      inspector: documentInspectorState,
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
