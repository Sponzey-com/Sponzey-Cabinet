export type DesktopSearchReturnContext =
  | Readonly<{ status: "Inactive" }>
  | Readonly<{ status: "Results"; query: string; selectedIndex: number; scrollOffset: number }>
  | Readonly<{
      status: "DocumentOpen";
      query: string;
      selectedIndex: number;
      scrollOffset: number;
      documentId: string;
    }>;

export type DesktopSearchReturnEvent =
  | Readonly<{ type: "SearchStarted"; query: string }>
  | Readonly<{ type: "ViewportCaptured"; query: string; selectedIndex: number; scrollOffset: number }>
  | Readonly<{ type: "ResultOpened"; query: string; documentId: string }>
  | Readonly<{ type: "ReturnRequested" }>
  | Readonly<{ type: "Cleared" }>;

export function createDesktopSearchReturnContext(): DesktopSearchReturnContext {
  return Object.freeze({ status: "Inactive" });
}

export function transitionDesktopSearchReturnContext(
  state: DesktopSearchReturnContext,
  event: DesktopSearchReturnEvent,
): DesktopSearchReturnContext {
  if (event.type === "Cleared") return createDesktopSearchReturnContext();
  if (event.type === "SearchStarted") {
    const query = event.query.trim();
    if (!query) throw new Error("INVALID_SEARCH_RETURN_CONTEXT");
    return Object.freeze({ status: "Results", query, selectedIndex: 0, scrollOffset: 0 });
  }
  if (event.type === "ViewportCaptured") {
    requireViewport(event.selectedIndex, event.scrollOffset);
    if (state.status !== "Results" || state.query !== event.query) return state;
    return Object.freeze({ ...state, selectedIndex: event.selectedIndex, scrollOffset: event.scrollOffset });
  }
  if (event.type === "ResultOpened") {
    if (!event.documentId.trim()) throw new Error("INVALID_SEARCH_RETURN_CONTEXT");
    if (state.status !== "Results" || state.query !== event.query) return state;
    return Object.freeze({ ...state, status: "DocumentOpen", documentId: event.documentId });
  }
  if (event.type === "ReturnRequested" && state.status === "DocumentOpen") {
    return Object.freeze({
      status: "Results",
      query: state.query,
      selectedIndex: state.selectedIndex,
      scrollOffset: state.scrollOffset,
    });
  }
  return state;
}

interface SearchResultElement {
  readonly dataset: Readonly<{ documentId?: string }>;
  focus(): void;
}

interface SearchScrollElement {
  scrollTop: number;
}

interface SearchViewportRoot {
  querySelector(selector: string): SearchScrollElement | null;
  querySelectorAll(selector: string): ArrayLike<SearchResultElement>;
}

export function captureDesktopSearchViewport(
  root: SearchViewportRoot,
  documentId: string,
): Readonly<{ selectedIndex: number; scrollOffset: number }> {
  const main = root.querySelector('[data-workspace-route-main="true"]');
  const buttons = [...root.querySelectorAll('[data-action="open-navigator-document"]')];
  const selectedIndex = buttons.findIndex((button) => button.dataset.documentId === documentId);
  const scrollOffset = main?.scrollTop ?? 0;
  requireViewport(selectedIndex, scrollOffset);
  return Object.freeze({ selectedIndex, scrollOffset });
}

export function restoreDesktopSearchViewport(
  root: SearchViewportRoot,
  context: DesktopSearchReturnContext,
): boolean {
  if (context.status !== "Results") return false;
  requireViewport(context.selectedIndex, context.scrollOffset);
  const main = root.querySelector('[data-workspace-route-main="true"]');
  const button = [...root.querySelectorAll('[data-action="open-navigator-document"]')][context.selectedIndex];
  if (!main || !button) return false;
  main.scrollTop = context.scrollOffset;
  button.focus();
  return true;
}

function requireViewport(selectedIndex: number, scrollOffset: number): void {
  if (!Number.isSafeInteger(selectedIndex) || selectedIndex < 0 || selectedIndex > 100_000
    || !Number.isFinite(scrollOffset) || scrollOffset < 0 || scrollOffset > 10_000_000) {
    throw new Error("INVALID_SEARCH_RETURN_CONTEXT");
  }
}
