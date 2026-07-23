import type { DesktopRoute, DesktopSelectionContext } from "./desktop_route_controller.ts";

export type DesktopSearchNavigationIntent =
  | Readonly<{ kind: "NoOp"; reason: "EmptyQuery" }>
  | Readonly<{
      kind: "Navigate";
      route: Extract<DesktopRoute, { readonly kind: "Search" }>;
      selection: DesktopSelectionContext;
    }>;

export function createDesktopSearchNavigationIntent(
  rawQuery: string | undefined,
  workspaceId: string,
  originRoute: DesktopRoute["kind"],
): DesktopSearchNavigationIntent {
  if (!workspaceId.trim()) throw new Error("INVALID_SEARCH_NAVIGATION_CONTEXT");
  const query = rawQuery?.trim() ?? "";
  if (!query) return Object.freeze({ kind: "NoOp", reason: "EmptyQuery" });
  return Object.freeze({
    kind: "Navigate",
    route: Object.freeze({ kind: "Search", query }),
    selection: Object.freeze({ workspaceId, originRoute }),
  });
}

interface SearchFocusable {
  readonly disabled?: boolean;
  focus(): void;
}

interface SearchFocusRoot {
  querySelector(selector: string): SearchFocusable | null;
}

export function focusDesktopWorkspaceSearch(root: SearchFocusRoot): boolean {
  const input = root.querySelector('[data-action="workspace-search-input"]');
  if (!input || input.disabled) return false;
  input.focus();
  return true;
}
