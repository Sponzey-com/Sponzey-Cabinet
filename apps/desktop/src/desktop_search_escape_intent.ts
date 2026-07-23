import type { DesktopRoute } from "./desktop_route_controller.ts";

export type DesktopSearchEscapeIntent =
  | Readonly<{ kind: "ClearQuery" }>
  | Readonly<{ kind: "ReturnToOrigin"; route: DesktopRoute }>;

export function createDesktopSearchEscapeIntent(
  query: string,
  originRoute: DesktopRoute | undefined,
): DesktopSearchEscapeIntent {
  if (query.trim()) return Object.freeze({ kind: "ClearQuery" });
  const route = !originRoute || originRoute.kind === "Search"
    ? Object.freeze({ kind: "Home" as const })
    : Object.freeze({ ...originRoute });
  return Object.freeze({ kind: "ReturnToOrigin", route });
}
