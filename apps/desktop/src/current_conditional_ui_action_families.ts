import type { ConditionalUiActionFamily } from "./ui_action_inventory.ts";

export const CURRENT_CONDITIONAL_UI_ACTION_FAMILIES: readonly ConditionalUiActionFamily[] = Object.freeze([
  family("react_workspace_home.ts", "actionId", ["home-open-all-documents", "home-open-graph"]),
  family("react_workspace_shell.ts", 'shortcut.actionId ?? "open-sidebar-document"', ["open-sidebar-document"]),
  family("react_workspace_shell.ts", 'options.searchActionId ?? "workspace-search-input"', [
    "workspace-search-input", "navigator-search-field",
  ]),
  family("react_document_authoring_workbench.ts", 'snapshot?.retryable ? "repair-authoring-graph" : "retry-authoring-graph"', [
    "repair-authoring-graph", "retry-authoring-graph",
  ]),
  family("react_document_authoring_workbench.ts", "action", [
    "format-heading", "format-bold", "format-italic", "format-link", "format-list", "format-checklist", "format-table",
  ]),
  family("react_topology_visual_host.ts", 'layoutPaused ? "graph-resume-layout" : "graph-pause-layout"', [
    "graph-resume-layout", "graph-pause-layout",
  ]),
  family("react_topology_visual_host.ts", 'node.kind === "attachment" ? "open-graph-asset" : "open-graph-document"', [
    "open-graph-asset", "open-graph-document",
  ]),
  family("react_exploration_surfaces.ts", 'snapshot.retryable && snapshot.query.scope !== "global" ? "reindex-graph" : "retry-graph"', [
    "reindex-graph", "retry-graph",
  ]),
  family("react_document_attachment_panel.ts", "callbacks.cancelActionId", [
    "cancel-document-asset-import", "cancel-asset-import",
  ]),
  family("react_document_attachment_panel.ts", "callbacks.repairActionId", [
    "repair-document-asset-import", "repair-asset-import",
  ]),
  family("react_document_attachment_panel.ts", "callbacks.restartActionId", [
    "retry-document-asset-import", "restart-asset-import",
  ]),
]);

function family(source: string, expression: string, actionIds: readonly string[]): ConditionalUiActionFamily {
  return Object.freeze({ source, expression, actionIds: Object.freeze([...actionIds]) });
}
