import type { DynamicUiActionFamily } from "./ui_action_inventory.ts";

export const CURRENT_DYNAMIC_UI_ACTION_FAMILIES: readonly DynamicUiActionFamily[] = Object.freeze([
  family("react_document_navigator.ts", "navigator-view-${view.toLowerCase()}", [
    "navigator-view-tree", "navigator-view-collection", "navigator-view-tag", "navigator-view-recent", "navigator-view-favorite",
  ]),
  family("react_workspace_shell.ts", "navigate-${item.route.toLowerCase()}", [
    "navigate-home", "navigate-document", "navigate-graph", "navigate-canvas", "navigate-assets", "navigate-backup",
  ]),
  family("react_document_authoring_workbench.ts", "select-document-inspector-${tab.id}", [
    "select-document-inspector-links", "select-document-inspector-attachments", "select-document-inspector-history",
  ]),
  family("react_document_authoring_workbench.ts", "authoring-graph-depth-${depth}", [
    "authoring-graph-depth-1", "authoring-graph-depth-2",
  ]),
  family("react_document_authoring_workbench.ts", "authoring-graph-direction-${direction}", [
    "authoring-graph-direction-both", "authoring-graph-direction-incoming", "authoring-graph-direction-outgoing",
  ]),
  family("react_exploration_surfaces.ts", "filter-assets-${id}", [
    "filter-assets-all", "filter-assets-image", "filter-assets-pdf", "filter-assets-document", "filter-assets-other",
  ]),
]);

function family(source: string, expression: string, actionIds: readonly string[]): DynamicUiActionFamily {
  return Object.freeze({ source, expression, actionIds: Object.freeze([...actionIds]) });
}
