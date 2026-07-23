import type { DesktopRoute } from "./desktop_route_controller.ts";

export type GlobalSearchOverlayLifecycleStatus =
  | "Closed"
  | "Open"
  | "Searching"
  | "ResultsReady"
  | "Empty"
  | "Failed";

export type GlobalSearchResultTarget =
  | Readonly<{ kind: "Document"; documentId: string }>
  | Readonly<{ kind: "Asset"; assetId: string }>;

export interface GlobalSearchOverlayLifecycle {
  readonly status: GlobalSearchOverlayLifecycleStatus;
  readonly originRoute?: DesktopRoute;
  readonly query?: string;
  readonly resultCount?: number;
  readonly errorCode?: string;
}

export type GlobalSearchOverlayEvent =
  | Readonly<{ type: "OpenRequested"; originRoute: DesktopRoute; query?: string }>
  | Readonly<{ type: "QuerySubmitted"; query: string }>
  | Readonly<{ type: "SearchSucceeded"; resultCount: number }>
  | Readonly<{ type: "SearchFailed"; errorCode: string }>
  | Readonly<{ type: "CloseRequested" }>
  | Readonly<{ type: "EscapePressed" }>
  | Readonly<{ type: "ResultOpened"; result: GlobalSearchResultTarget }>;

export type GlobalSearchOverlaySideEffect =
  | Readonly<{ type: "FocusSearchInput" }>
  | Readonly<{ type: "RunSearch"; query: string }>
  | Readonly<{ type: "RestoreFocus"; originRoute: DesktopRoute }>
  | Readonly<{ type: "OpenResult"; result: GlobalSearchResultTarget }>;

export interface GlobalSearchOverlayTransitionResult {
  readonly state: GlobalSearchOverlayLifecycle;
  readonly sideEffects: readonly GlobalSearchOverlaySideEffect[];
  readonly errorCode?: "GLOBAL_SEARCH_EMPTY_QUERY" | "GLOBAL_SEARCH_INVALID_TRANSITION";
}

export function createGlobalSearchOverlayLifecycle(): GlobalSearchOverlayLifecycle {
  return Object.freeze({ status: "Closed" });
}

export function transitionGlobalSearchOverlay(
  state: GlobalSearchOverlayLifecycle,
  event: GlobalSearchOverlayEvent,
): GlobalSearchOverlayTransitionResult {
  if (event.type === "OpenRequested") {
    const query = normalizeQuery(event.query);
    if (query) {
      return result(
        { status: "Searching", originRoute: event.originRoute, query },
        [{ type: "RunSearch", query }],
      );
    }
    return result(
      { status: "Open", originRoute: event.originRoute },
      [{ type: "FocusSearchInput" }],
    );
  }

  if (event.type === "QuerySubmitted" && state.status === "Open") {
    const query = normalizeQuery(event.query);
    if (!query) return result(state, [], "GLOBAL_SEARCH_EMPTY_QUERY");
    return result(
      { status: "Searching", originRoute: state.originRoute, query },
      [{ type: "RunSearch", query }],
    );
  }

  if (event.type === "SearchSucceeded" && state.status === "Searching") {
    const resultCount = Math.max(0, event.resultCount);
    return result({
      ...state,
      status: resultCount > 0 ? "ResultsReady" : "Empty",
      resultCount,
      errorCode: undefined,
    }, []);
  }

  if (event.type === "SearchFailed" && state.status === "Searching") {
    return result({
      ...state,
      status: "Failed",
      errorCode: event.errorCode,
    }, []);
  }

  if ((event.type === "CloseRequested" || event.type === "EscapePressed") && state.status !== "Closed") {
    return result(
      createGlobalSearchOverlayLifecycle(),
      state.originRoute ? [{ type: "RestoreFocus", originRoute: state.originRoute }] : [],
    );
  }

  if (event.type === "ResultOpened" && state.status !== "Closed") {
    return result(
      createGlobalSearchOverlayLifecycle(),
      [{ type: "OpenResult", result: event.result }],
    );
  }

  return result(state, [], "GLOBAL_SEARCH_INVALID_TRANSITION");
}

function result(
  state: GlobalSearchOverlayLifecycle,
  sideEffects: readonly GlobalSearchOverlaySideEffect[],
  errorCode?: GlobalSearchOverlayTransitionResult["errorCode"],
): GlobalSearchOverlayTransitionResult {
  return Object.freeze({
    state: Object.freeze(state),
    sideEffects: Object.freeze([...sideEffects]),
    ...(errorCode ? { errorCode } : {}),
  });
}

function normalizeQuery(query: string | undefined): string {
  return query?.trim() ?? "";
}
