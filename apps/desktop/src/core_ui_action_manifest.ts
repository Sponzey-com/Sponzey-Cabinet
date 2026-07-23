import { defineUiActionContract, type UiActionBoundary, type UiActionContract, type UiActionDurability } from "./ui_action_contract.ts";

const routeActions = [
  "navigate-home", "submit-workspace-search", "navigate-document", "navigate-graph", "navigate-canvas",
  "navigate-assets", "navigate-backup", "open-sidebar-document", "open-recent-document",
  "home-open-all-documents", "home-open-graph", "open-navigator-document", "authoring-home",
  "return-search-results", "open-search-asset",
  "open-home-graph-document",
  "open-authoring-graph", "open-linked-authoring-document",
  "open-document-asset-library",
  "open-authoring-graph-document", "open-authoring-graph-asset",
] as const;
const viewActions = [
  "workspace-search-input", "navigator-search-field", "navigator-view-tree", "navigator-view-collection", "navigator-view-tag",
  "navigator-view-recent", "navigator-view-favorite",
  "previous-search-results", "next-search-results", "close-global-search",
  "edit-document-body", "cancel-backup", "cancel-backup-restore",
  "cancel-authoring-recovery",
  "close-document-asset-preview", "select-document-inspector-links",
  "select-document-inspector-attachments", "select-document-inspector-history",
  "unlink-document-asset", "cancel-document-asset-unlink", "select-history-version",
  "previous-history-window", "next-history-window",
  "close-document-asset-library", "search-document-asset-library",
  "select-existing-document-asset",
  "review-restore", "cancel-restore-confirmation", "close-document-diff",
  "select-backup-catalog",
  "previous-diff-hunks", "next-diff-hunks",
  "search-authoring-graph",
  "authoring-graph-depth-1", "authoring-graph-depth-2",
  "authoring-graph-direction-both", "authoring-graph-direction-incoming", "authoring-graph-direction-outgoing",
  "authoring-graph-toggle-unresolved", "authoring-graph-toggle-assets", "authoring-graph-toggle-external", "recenter-authoring-graph",
] as const;
const queryActions = [
  "retry-home-knowledge-map",
  "retry-backup-catalog", "load-more-backup-catalog",
  "retry-document-asset-library", "load-more-document-asset-library",
  "retry-authoring-graph", "repair-authoring-graph",
] as const;
const commandActions = [
  "new-document", "retry-workspace-home", "retry-navigator", "save-document", "load-history", "load-more-history", "compare-selected-versions",
  "preview-restore", "apply-restore", "create-backup", "retry-backup-recovery",
  "retry-backup-restore-preview",
  "preview-backup-restore", "confirm-backup-restore", "retry-authoring-save",
  "format-heading", "format-bold", "format-italic", "format-link", "format-list", "format-checklist", "format-table",
  "discard-authoring-changes",
  "import-document-asset", "retry-document-asset-import", "cancel-document-asset-import",
  "retry-document-assets", "select-document-asset", "preview-document-asset",
  "retry-document-asset-preview", "confirm-document-asset-unlink",
  "open-document-asset-externally",
  "link-existing-document-asset", "repair-document-asset-import",
  "cancel-background-document-diff", "retry-background-document-diff",
  "compare-current-version", "refresh-restore-preview", "continue-restore-recovery",
  "confirm-restore", "create-document-empty-state",
] as const;

export const CORE_UI_ACTION_MANIFEST: readonly UiActionContract[] = Object.freeze([
  ...routeActions.map((actionId) => connected(actionId, "shell/home/search/document", "route", "none")),
  ...viewActions.map((actionId) => connected(actionId, "search/document/backup", "view_state", "none")),
  ...queryActions.map((actionId) => connected(actionId, actionId === "retry-home-knowledge-map" ? "home" : "document", "query", "readback")),
  ...commandActions.map((actionId) => connected(actionId, "home/search/document/backup", "native_command", mutationDurability(actionId))),
  hidden("open-settings"),
  hidden("toggle-theme"),
  hidden("open-ai"),
]);

function connected(
  actionId: string,
  surface: string,
  boundary: UiActionBoundary,
  durability: UiActionDurability,
): UiActionContract {
  return defineUiActionContract({
    actionId,
    surface,
    availability: "connected",
    visibleCondition: "route_and_state_specific",
    enabledCondition: "controller_callback_and_valid_state",
    disabledReasonKey: "ACTION_STATE_UNAVAILABLE",
    input: "typed_action_payload",
    boundary,
    target: actionId,
    progressState: boundary === "native_command" ? "controller_state" : "none",
    successReadback: boundary === "route" ? "active route and heading" : "visible state or command readback",
    failureMapping: boundary === "native_command" ? "UserFacingError" : "none",
    recoveryAction: boundary === "native_command" ? "mapped_by_error" : "none",
    durability,
    interactionTest: "apps/desktop/tests/ui_action_inventory_tests.ts",
    packagedTest: "apps/desktop/src/packaged_ui_smoke.ts",
  });
}

function hidden(actionId: string): UiActionContract {
  return defineUiActionContract({
    ...connected(actionId, "shell", "view_state", "none"),
    availability: "hidden_out_of_scope",
    visibleCondition: "never_in_phase013",
    enabledCondition: "never",
    target: "out_of_scope",
    hiddenReasonKey: "PHASE013_FEATURE_OUT_OF_SCOPE",
  });
}

function mutationDurability(actionId: string): UiActionDurability {
  if (["new-document", "create-document-empty-state", "save-document", "apply-restore", "confirm-restore", "continue-restore-recovery", "create-backup", "confirm-backup-restore", "import-document-asset", "confirm-document-asset-unlink", "link-existing-document-asset", "repair-document-asset-import"].includes(actionId)) return "reopen";
  if (["load-history", "load-more-history", "compare-selected-versions", "compare-current-version", "preview-restore", "refresh-restore-preview", "preview-backup-restore", "retry-backup-restore-preview", "retry-document-assets", "select-document-asset", "preview-document-asset", "retry-document-asset-preview", "open-document-asset-externally", "retry-background-document-diff"].includes(actionId)) return "readback";
  return "none";
}
