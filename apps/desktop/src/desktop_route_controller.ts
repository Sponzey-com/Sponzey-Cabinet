export type DesktopRoute =
  | { readonly kind: "Home" }
  | { readonly kind: "Search"; readonly query?: string; readonly scope?: string }
  | { readonly kind: "Document"; readonly documentId?: string }
  | { readonly kind: "Graph"; readonly centerDocumentId?: string; readonly scope: "Global" | "Local" }
  | { readonly kind: "Canvas"; readonly canvasId?: string }
  | { readonly kind: "Assets"; readonly documentId?: string; readonly assetId?: string }
  | { readonly kind: "Backup" };

export interface DesktopSelectionContext {
  readonly workspaceId: string;
  readonly documentId?: string;
  readonly canvasId?: string;
  readonly assetId?: string;
  readonly originRoute: DesktopRoute["kind"];
}

export type DesktopRouteBlocker =
  | { readonly kind: "None" }
  | { readonly kind: "DirtyDocument"; readonly resourceId: string }
  | { readonly kind: "PendingCanvasMutation"; readonly resourceId: string }
  | { readonly kind: "ActiveAssetImport"; readonly resourceId: string };

interface StableRouteState {
  readonly status: "Stable";
  readonly route: DesktopRoute;
  readonly selection: DesktopSelectionContext;
}

interface AwaitingDecisionRouteState {
  readonly status: "AwaitingDecision";
  readonly currentRoute: DesktopRoute;
  readonly currentSelection: DesktopSelectionContext;
  readonly targetRoute: DesktopRoute;
  readonly targetSelection: DesktopSelectionContext;
  readonly blocker: Exclude<DesktopRouteBlocker, { readonly kind: "None" }>;
}

interface AwaitingResolutionRouteState extends Omit<AwaitingDecisionRouteState, "status"> {
  readonly status: "AwaitingResolution";
  readonly operationId: string;
  readonly effect: DesktopRouteEffect;
}

interface FailedRouteState extends Omit<AwaitingResolutionRouteState, "status"> {
  readonly status: "Failed";
  readonly errorCode: string;
}

export type DesktopRouteControllerState =
  | StableRouteState
  | AwaitingDecisionRouteState
  | AwaitingResolutionRouteState
  | FailedRouteState;

export type DesktopRouteEffect =
  | { readonly type: "SaveDocument"; readonly resourceId: string; readonly operationId: string }
  | { readonly type: "FlushCanvas"; readonly resourceId: string; readonly operationId: string }
  | { readonly type: "CancelAssetImport"; readonly resourceId: string; readonly operationId: string };

export function graphQueryScopeForRoute(
  route: Extract<DesktopRoute, { readonly kind: "Graph" }>,
): "global" | "local" {
  return route.scope === "Global" ? "global" : "local";
}

export function createDesktopGraphScopeIntent(
  scope: "local" | "global",
  workspaceId: string,
  centerDocumentId?: string,
): Readonly<{ route: DesktopRoute; selection: DesktopSelectionContext }> {
  const workspace = workspaceId.trim();
  if (!workspace) throw new Error("INVALID_ROUTE_SELECTION");
  const center = centerDocumentId?.trim() || undefined;
  return Object.freeze({
    route: scope === "local"
      ? Object.freeze({ kind: "Graph", scope: "Local", ...(center ? { centerDocumentId: center } : {}) })
      : Object.freeze({ kind: "Graph", scope: "Global" }),
    selection: Object.freeze({
      workspaceId: workspace,
      ...(center ? { documentId: center } : {}),
      originRoute: "Graph",
    }),
  });
}

export type DesktopRouteEvent =
  | {
      readonly type: "TransitionRequested";
      readonly target: DesktopRoute;
      readonly selection: DesktopSelectionContext;
      readonly blocker: DesktopRouteBlocker;
    }
  | { readonly type: "CancelTransition" }
  | { readonly type: "DiscardAndContinue" }
  | { readonly type: "ResolveAndContinue"; readonly operationId: string }
  | { readonly type: "ResolutionCompleted"; readonly operationId: string }
  | { readonly type: "ResolutionFailed"; readonly operationId: string; readonly errorCode: string }
  | { readonly type: "RetryResolution"; readonly operationId: string };

export interface DesktopRouteTransitionResult {
  readonly state: DesktopRouteControllerState;
  readonly effect?: DesktopRouteEffect;
}

export function createDesktopRouteControllerState(
  route: DesktopRoute,
  selection: DesktopSelectionContext,
): DesktopRouteControllerState {
  validateRouteSelection(route, selection);
  return Object.freeze({ status: "Stable", route, selection });
}

export function transitionDesktopRoute(
  state: DesktopRouteControllerState,
  event: DesktopRouteEvent,
): DesktopRouteTransitionResult {
  if (state.status === "Stable" && event.type === "TransitionRequested") {
    validateRouteSelection(event.target, event.selection);
    if (event.blocker.kind === "None") {
      return { state: createDesktopRouteControllerState(event.target, event.selection) };
    }
    return {
      state: Object.freeze({
        status: "AwaitingDecision",
        currentRoute: state.route,
        currentSelection: state.selection,
        targetRoute: event.target,
        targetSelection: event.selection,
        blocker: event.blocker,
      }),
    };
  }

  if (state.status === "AwaitingDecision") {
    if (event.type === "CancelTransition") return { state: stableCurrent(state) };
    if (event.type === "DiscardAndContinue") {
      return { state: createDesktopRouteControllerState(state.targetRoute, state.targetSelection) };
    }
    if (event.type === "ResolveAndContinue") {
      const effect = resolutionEffect(state.blocker, event.operationId);
      return {
        state: Object.freeze({ ...state, status: "AwaitingResolution", operationId: event.operationId, effect }),
        effect,
      };
    }
  }

  if (state.status === "AwaitingResolution") {
    if ((event.type === "ResolutionCompleted" || event.type === "ResolutionFailed") && event.operationId !== state.operationId) {
      return { state };
    }
    if (event.type === "ResolutionCompleted") {
      return { state: createDesktopRouteControllerState(state.targetRoute, state.targetSelection) };
    }
    if (event.type === "ResolutionFailed") {
      return { state: Object.freeze({ ...state, status: "Failed", errorCode: event.errorCode }) };
    }
    if (event.type === "CancelTransition") return { state: stableCurrent(state) };
  }

  if (state.status === "Failed") {
    if (event.type === "CancelTransition") return { state: stableCurrent(state) };
    if (event.type === "RetryResolution") {
      const effect = resolutionEffect(state.blocker, event.operationId);
      return {
        state: Object.freeze({ ...state, status: "AwaitingResolution", operationId: event.operationId, effect }),
        effect,
      };
    }
  }

  return { state };
}

function validateRouteSelection(route: DesktopRoute, selection: DesktopSelectionContext): void {
  if (!selection.workspaceId.trim()) throw new Error("INVALID_ROUTE_SELECTION");
  if (route.kind === "Document" && route.documentId !== selection.documentId) throw new Error("INVALID_ROUTE_SELECTION");
  if (route.kind === "Graph" && route.centerDocumentId && route.centerDocumentId !== selection.documentId) throw new Error("INVALID_ROUTE_SELECTION");
  if (route.kind === "Canvas" && route.canvasId !== selection.canvasId) throw new Error("INVALID_ROUTE_SELECTION");
  if (route.kind === "Assets" && route.documentId && route.documentId !== selection.documentId) throw new Error("INVALID_ROUTE_SELECTION");
  if (route.kind === "Assets" && route.assetId && route.assetId !== selection.assetId) throw new Error("INVALID_ROUTE_SELECTION");
}

function resolutionEffect(
  blocker: Exclude<DesktopRouteBlocker, { readonly kind: "None" }>,
  operationId: string,
): DesktopRouteEffect {
  if (!operationId.trim()) throw new Error("INVALID_ROUTE_OPERATION_ID");
  if (blocker.kind === "DirtyDocument") return { type: "SaveDocument", resourceId: blocker.resourceId, operationId };
  if (blocker.kind === "PendingCanvasMutation") return { type: "FlushCanvas", resourceId: blocker.resourceId, operationId };
  return { type: "CancelAssetImport", resourceId: blocker.resourceId, operationId };
}

function stableCurrent(
  state: AwaitingDecisionRouteState | AwaitingResolutionRouteState | FailedRouteState,
): DesktopRouteControllerState {
  return createDesktopRouteControllerState(state.currentRoute, state.currentSelection);
}
