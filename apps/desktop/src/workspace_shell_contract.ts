import type { KoKrMessageKey, MessageCatalog } from "./ko_kr_catalog.ts";

export type WorkspaceShellRouteKind = "Home" | "Search" | "Document" | "Graph" | "Canvas" | "Assets" | "Backup";
export type WorkspaceShellLayoutVariant = "standard" | "focused" | "immersive";

export interface WorkspaceShellInput {
  readonly route: WorkspaceShellRouteKind;
  readonly availableActions: readonly WorkspaceShellRouteKind[];
  readonly variant?: WorkspaceShellLayoutVariant;
  readonly messages: MessageCatalog;
}

export interface WorkspaceShellNavigationItem {
  readonly route: WorkspaceShellRouteKind;
  readonly label: string;
  readonly active: boolean;
  readonly enabled: boolean;
}

export interface WorkspaceShellModel {
  readonly route: WorkspaceShellRouteKind;
  readonly pageTitle: string;
  readonly pageContext: string;
  readonly variant: WorkspaceShellLayoutVariant;
  readonly navigation: readonly WorkspaceShellNavigationItem[];
}

export const WORKSPACE_SHELL_LAYOUT = Object.freeze({
  sidebarWidthPx: 244,
  topbarHeightPx: 50,
  inspectorWidthPx: 315,
  contentGapPx: 24,
});

const routeDefinitions: readonly Readonly<{
  route: WorkspaceShellRouteKind;
  labelKey: KoKrMessageKey;
  contextKey: KoKrMessageKey;
}>[] = Object.freeze([
  Object.freeze({ route: "Home", labelKey: "route.home", contextKey: "routeContext.home" }),
  Object.freeze({ route: "Search", labelKey: "route.search", contextKey: "routeContext.search" }),
  Object.freeze({ route: "Document", labelKey: "route.document", contextKey: "routeContext.document" }),
  Object.freeze({ route: "Graph", labelKey: "route.graph", contextKey: "routeContext.graph" }),
  Object.freeze({ route: "Canvas", labelKey: "route.canvas", contextKey: "routeContext.canvas" }),
  Object.freeze({ route: "Assets", labelKey: "route.assets", contextKey: "routeContext.assets" }),
  Object.freeze({ route: "Backup", labelKey: "route.backup", contextKey: "routeContext.backup" }),
]);

export class WorkspaceShellContractError extends Error {
  readonly code: "SHELL_ROUTE_UNKNOWN" | "SHELL_ACTION_MISSING" | "SHELL_ACTION_DUPLICATE";

  constructor(code: WorkspaceShellContractError["code"]) {
    super(code);
    this.name = "WorkspaceShellContractError";
    this.code = code;
  }
}

export function createWorkspaceShellModel(input: WorkspaceShellInput): WorkspaceShellModel {
  const activeDefinition = routeDefinitions.find((definition) => definition.route === input.route);
  if (!activeDefinition) throw new WorkspaceShellContractError("SHELL_ROUTE_UNKNOWN");
  const available = new Set(input.availableActions);
  if (available.size !== input.availableActions.length) throw new WorkspaceShellContractError("SHELL_ACTION_DUPLICATE");
  if (routeDefinitions.some((definition) => !available.has(definition.route))) throw new WorkspaceShellContractError("SHELL_ACTION_MISSING");
  const navigation = Object.freeze(routeDefinitions.map((definition) => Object.freeze({
    route: definition.route,
    label: input.messages.message(definition.labelKey),
    active: definition.route === input.route,
    enabled: definition.route !== input.route && available.has(definition.route),
  })));
  return Object.freeze({
    route: input.route,
    pageTitle: input.messages.message(activeDefinition.labelKey),
    pageContext: input.messages.message(activeDefinition.contextKey),
    variant: input.variant ?? defaultVariant(input.route),
    navigation,
  });
}

function defaultVariant(route: WorkspaceShellRouteKind): WorkspaceShellLayoutVariant {
  if (route === "Document") return "focused";
  if (route === "Graph" || route === "Canvas") return "immersive";
  return "standard";
}
