export interface RouteMainFocusTarget {
  focus(): void;
}

export interface RouteMainFocusRoot {
  querySelector(selector: string): RouteMainFocusTarget | null;
}

export function focusWorkspaceRouteMain(root: RouteMainFocusRoot): boolean {
  const target = root.querySelector("[data-workspace-route-main]");
  if (!target || typeof target.focus !== "function") return false;
  target.focus();
  return true;
}
