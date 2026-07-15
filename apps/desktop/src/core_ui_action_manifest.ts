import { defineUiActionContract, type UiActionBoundary, type UiActionContract, type UiActionDurability } from "./ui_action_contract.ts";

const routeActions = [
  "navigate-home", "navigate-search", "navigate-document", "navigate-graph", "navigate-canvas",
  "navigate-assets", "navigate-backup", "open-sidebar-document", "open-recent-document",
  "home-open-all-documents", "home-open-graph", "open-navigator-document", "authoring-home",
  "open-authoring-graph", "open-linked-authoring-document",
] as const;
const viewActions = [
  "navigator-search-field", "navigator-view-tree", "navigator-view-collection", "navigator-view-tag",
  "navigator-view-recent", "navigator-view-favorite", "authoring-mode-source",
  "edit-document-body", "authoring-mode-split", "authoring-mode-preview", "cancel-backup", "cancel-backup-restore",
  "cancel-authoring-recovery",
] as const;
const commandActions = [
  "new-document", "retry-workspace-home", "retry-navigator", "save-document", "load-history",
  "preview-restore", "apply-restore", "create-backup", "retry-backup-recovery",
  "preview-backup-restore", "confirm-backup-restore", "retry-authoring-save",
  "discard-authoring-changes",
] as const;

export const CORE_UI_ACTION_MANIFEST: readonly UiActionContract[] = Object.freeze([
  ...routeActions.map((actionId) => connected(actionId, "shell/home/search/document", "route", "none")),
  ...viewActions.map((actionId) => connected(actionId, "search/document/backup", "view_state", "none")),
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
    interactionTest: "apps/desktop/tests/core_route_action_contract_tests.ts",
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
  if (["new-document", "save-document", "apply-restore", "create-backup", "confirm-backup-restore"].includes(actionId)) return "reopen";
  if (["load-history", "preview-restore", "preview-backup-restore"].includes(actionId)) return "readback";
  return "none";
}
