import { defineUiActionContract, type UiActionContract } from "./ui_action_contract.ts";

export type ExplorationActionState = "connected";
export type ExplorationActionBoundary = "route" | "view_state" | "native_command";

export interface ExplorationUiActionManifestEntry {
  readonly actionId: string;
  readonly surface: "shell" | "graph" | "canvas" | "assets";
  readonly selector: string;
  readonly state: ExplorationActionState;
  readonly controller: string;
  readonly boundary: ExplorationActionBoundary;
  readonly target: string;
  readonly interactionEvidence: string;
  readonly unavailableReason?: string;
}

const interactionEvidence = "apps/desktop/tests/desktop_react_exploration_surfaces_tests.ts";

export const EXPLORATION_UI_ACTION_MANIFEST: readonly ExplorationUiActionManifestEntry[] = Object.freeze([
  connected("navigate-home", "shell", "route", "DesktopRoute.Home", "onHome"),
  connected("navigate-search", "shell", "route", "DesktopRoute.Search", "onSearch"),
  connected("navigate-graph", "shell", "route", "DesktopRoute.Graph", "onGraph"),
  connected("navigate-canvas", "shell", "route", "DesktopRoute.Canvas", "onCanvas"),
  connected("navigate-assets", "shell", "route", "DesktopRoute.Assets", "onAssets"),
  connected("open-sidebar-document", "shell", "route", "DesktopRoute.Document", "onOpenDocument"),
  connected("open-graph-document", "graph", "route", "DesktopRoute.Document", "onOpenDocument"),
  connected("graph-scope-local", "graph", "native_command", "get_desktop_knowledge_graph", "onGraphQuery"),
  connected("graph-scope-global", "graph", "native_command", "get_desktop_global_knowledge_graph", "onGraphQuery"),
  connected("graph-toggle-depth", "graph", "native_command", "get_desktop_knowledge_graph", "onGraphQuery"),
  connected("graph-toggle-direction", "graph", "native_command", "get_desktop_knowledge_graph", "onGraphQuery"),
  connected("graph-toggle-unresolved", "graph", "native_command", "get_desktop_knowledge_graph", "onGraphQuery"),
  connected("graph-toggle-assets", "graph", "native_command", "get_desktop_knowledge_graph", "onGraphQuery"),
  connected("select-graph-node", "graph", "view_state", "GraphController.selection", "onGraphNodeSelect"),
  connected("reindex-graph", "graph", "native_command", "start_desktop_projection_repair", "onGraphReindex"),
  connected("retry-graph", "graph", "native_command", "get_desktop_knowledge_graph", "onGraphRetry"),
  connected("load-next-graph-page", "graph", "native_command", "get_desktop_global_knowledge_graph", "onGraphQuery"),
  connected("graph-zoom-in", "graph", "view_state", "GraphCamera.zoom", "setCameraZoom"),
  connected("graph-zoom-out", "graph", "view_state", "GraphCamera.zoom", "setCameraZoom"),
  connected("graph-fit-view", "graph", "view_state", "GraphCamera.fit", "setCameraZoom"),
  connected("filter-graph-nodes", "graph", "view_state", "GraphSurface.filter", "setQuery"),
  connected("create-canvas", "canvas", "native_command", "execute_desktop_canvas", "onCanvasCreate"),
  connected("select-canvas-document-target", "canvas", "view_state", "CanvasPlacement.document", "onDocumentPlacementSelect"),
  connected("add-canvas-document", "canvas", "native_command", "execute_desktop_canvas", "onCanvasAddDocument"),
  connected("add-canvas-note", "canvas", "native_command", "execute_desktop_canvas", "onCanvasAddNote"),
  connected("select-canvas-asset-target", "canvas", "view_state", "CanvasPlacement.asset", "onAssetPlacementSelect"),
  connected("add-canvas-asset", "canvas", "native_command", "execute_desktop_canvas", "onCanvasAddAsset"),
  connected("connect-canvas-nodes", "canvas", "native_command", "execute_desktop_canvas", "onCanvasConnect"),
  connected("remove-canvas-edge", "canvas", "native_command", "execute_desktop_canvas", "onCanvasRemoveEdge"),
  connected("remove-canvas-node", "canvas", "native_command", "execute_desktop_canvas", "onCanvasRemoveNode"),
  connected("rename-canvas", "canvas", "view_state", "CanvasRenameDialog", "onCanvasRenameRequest"),
  connected("edit-canvas-title", "canvas", "view_state", "CanvasRenameDialog.draft", "onCanvasRenameDraftChange"),
  connected("cancel-canvas-rename", "canvas", "view_state", "CanvasRenameDialog", "onCanvasRenameCancel"),
  connected("confirm-canvas-rename", "canvas", "native_command", "execute_desktop_canvas", "onCanvasRename"),
  connected("archive-canvas", "canvas", "view_state", "CanvasArchiveConfirmation", "setArchiveConfirmation"),
  connected("cancel-canvas-archive", "canvas", "view_state", "CanvasArchiveConfirmation", "setArchiveConfirmation"),
  connected("confirm-canvas-archive", "canvas", "native_command", "execute_desktop_canvas", "onCanvasArchive"),
  connected("select-canvas-node", "canvas", "view_state", "CanvasController.selection", "onCanvasNodeSelect"),
  connected("select-canvas-edge", "canvas", "view_state", "CanvasController.selection", "onCanvasEdgeSelect"),
  connected("open-canvas-document", "canvas", "route", "DesktopRoute.Document", "onOpenDocument"),
  connected("open-canvas-asset", "canvas", "route", "DesktopRoute.Assets", "onOpenAsset"),
  connected("resize-canvas-node", "canvas", "native_command", "execute_desktop_canvas", "onCanvasResizeStart/onCanvasResizeEnd"),
  connected("auto-arrange-canvas", "canvas", "native_command", "execute_desktop_canvas", "onCanvasAutoArrange"),
  connected("apply-canvas-arrange", "canvas", "native_command", "execute_desktop_canvas", "onCanvasApplyArrange"),
  connected("cancel-canvas-arrange", "canvas", "view_state", "CanvasController.arrangePreview", "onCanvasCancelArrange"),
  connected("pan-canvas-left", "canvas", "native_command", "execute_desktop_canvas", "onCanvasPan"),
  connected("pan-canvas-up", "canvas", "native_command", "execute_desktop_canvas", "onCanvasPan"),
  connected("pan-canvas-down", "canvas", "native_command", "execute_desktop_canvas", "onCanvasPan"),
  connected("pan-canvas-right", "canvas", "native_command", "execute_desktop_canvas", "onCanvasPan"),
  connected("zoom-canvas-in", "canvas", "native_command", "execute_desktop_canvas", "onCanvasZoom"),
  connected("zoom-canvas-out", "canvas", "native_command", "execute_desktop_canvas", "onCanvasZoom"),
  connected("retry-canvas", "canvas", "native_command", "execute_desktop_canvas", "onCanvasRetry"),
  connected("recover-canvas", "canvas", "native_command", "execute_desktop_canvas", "onCanvasRecover"),
  connected("open-asset-library", "assets", "route", "DesktopRoute.Assets", "onAssetWorkspace"),
  connected("import-asset", "assets", "native_command", "import_desktop_asset", "onAssetImport"),
  connected("cancel-asset-import", "assets", "native_command", "cancel_desktop_asset_import", "onAssetCancel"),
  connected("select-asset", "assets", "native_command", "get_desktop_asset_detail", "onAssetSelect"),
  connected("open-asset-preview", "assets", "native_command", "get_desktop_asset_preview", "onAssetPreview"),
  connected("retry-asset-preview", "assets", "native_command", "get_desktop_asset_preview", "onAssetPreview"),
  connected("close-asset-preview", "assets", "view_state", "AssetSurface.preview", "onAssetPreviewClose"),
  connected("link-asset", "assets", "native_command", "link_desktop_asset", "onAssetLink"),
  connected("unlink-asset", "assets", "native_command", "unlink_desktop_asset", "onAssetUnlink"),
  connected("open-linked-document", "assets", "route", "DesktopRoute.Document", "onOpenDocument"),
  connected("retry-assets", "assets", "native_command", "get_desktop_document_assets", "onAssetRetry"),
  connected("filter-assets-all", "assets", "view_state", "AssetSurface.filter", "setFilter"),
  connected("filter-assets-image", "assets", "view_state", "AssetSurface.filter", "setFilter"),
  connected("filter-assets-pdf", "assets", "view_state", "AssetSurface.filter", "setFilter"),
  connected("filter-assets-document", "assets", "view_state", "AssetSurface.filter", "setFilter"),
  connected("filter-assets-other", "assets", "view_state", "AssetSurface.filter", "setFilter"),
  connected("search-assets", "assets", "view_state", "AssetSurface.query", "setQuery"),
]);

export const EXPLORATION_UI_ACTION_CONTRACTS: readonly UiActionContract[] = Object.freeze(
  EXPLORATION_UI_ACTION_MANIFEST
    .filter((entry) => entry.surface !== "shell")
    .map((entry) => defineUiActionContract({
      actionId: entry.actionId,
      surface: entry.surface,
      availability: "connected",
      visibleCondition: "route_and_state_specific",
      enabledCondition: "controller_callback_and_valid_state",
      disabledReasonKey: "ACTION_STATE_UNAVAILABLE",
      input: "typed_action_payload",
      boundary: entry.boundary,
      target: entry.target,
      progressState: entry.boundary === "native_command" ? "controller_state" : "none",
      successReadback: entry.boundary === "route" ? "active route and heading" : "visible state or command readback",
      failureMapping: entry.boundary === "native_command" ? "UserFacingError" : "none",
      recoveryAction: entry.actionId.startsWith("retry-") || entry.actionId.startsWith("recover-") ? entry.actionId : "mapped_by_error",
      durability: entry.boundary === "native_command" ? "readback" : "none",
      interactionTest: entry.interactionEvidence,
      packagedTest: "apps/desktop/src/packaged_ui_smoke.ts",
    })),
);

function connected(
  actionId: string,
  surface: ExplorationUiActionManifestEntry["surface"],
  boundary: ExplorationActionBoundary,
  target: string,
  controller: string,
): ExplorationUiActionManifestEntry {
  return Object.freeze({ actionId, surface, selector: `[data-action="${actionId}"]`, state: "connected", controller, boundary, target, interactionEvidence });
}
