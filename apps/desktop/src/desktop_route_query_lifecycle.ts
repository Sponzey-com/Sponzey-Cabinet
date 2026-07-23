import type { DesktopRoute } from "./desktop_route_controller.ts";

export type DesktopRouteKind = DesktopRoute["kind"];

export interface DesktopRouteQueryLifecycle {
  readonly activeRoute: DesktopRouteKind;
  readonly epoch: number;
}

export interface DesktopRouteQueryTicket {
  readonly route: DesktopRouteKind;
  readonly epoch: number;
}

export type DesktopRouteQueryLifecycleEvent = Readonly<{
  type: "RouteActivated";
  route: DesktopRouteKind;
}>;

const routeKinds: readonly DesktopRouteKind[] = [
  "Home", "Search", "Document", "Graph", "Canvas", "Assets", "Backup",
];

export function createDesktopRouteQueryLifecycle(
  activeRoute: DesktopRouteKind,
): DesktopRouteQueryLifecycle {
  requireRoute(activeRoute);
  return Object.freeze({ activeRoute, epoch: 0 });
}

export function transitionDesktopRouteQueryLifecycle(
  state: DesktopRouteQueryLifecycle,
  event: DesktopRouteQueryLifecycleEvent,
): DesktopRouteQueryLifecycle {
  requireState(state);
  requireRoute(event.route);
  return Object.freeze({ activeRoute: event.route, epoch: nextEpoch(state.epoch) });
}

export function beginDesktopRouteQuery(
  state: DesktopRouteQueryLifecycle,
  route: DesktopRouteKind,
): Readonly<{ state: DesktopRouteQueryLifecycle; ticket?: DesktopRouteQueryTicket }> {
  requireState(state);
  requireRoute(route);
  if (state.activeRoute !== route) return Object.freeze({ state });
  const next = Object.freeze({ ...state, epoch: nextEpoch(state.epoch) });
  return Object.freeze({
    state: next,
    ticket: Object.freeze({ route, epoch: next.epoch }),
  });
}

export function canApplyDesktopRouteQuery(
  state: DesktopRouteQueryLifecycle,
  ticket: DesktopRouteQueryTicket,
): boolean {
  if (!isState(state) || !isRoute(ticket.route) || !validEpoch(ticket.epoch)) return false;
  return state.activeRoute === ticket.route && state.epoch === ticket.epoch;
}

function nextEpoch(epoch: number): number {
  if (!validEpoch(epoch) || epoch === Number.MAX_SAFE_INTEGER) {
    throw new Error("INVALID_ROUTE_QUERY_LIFECYCLE");
  }
  return epoch + 1;
}

function requireState(state: DesktopRouteQueryLifecycle): void {
  if (!isState(state)) throw new Error("INVALID_ROUTE_QUERY_LIFECYCLE");
}

function isState(state: DesktopRouteQueryLifecycle): boolean {
  return isRoute(state.activeRoute) && validEpoch(state.epoch);
}

function requireRoute(route: DesktopRouteKind): void {
  if (!isRoute(route)) throw new Error("INVALID_ROUTE_QUERY_LIFECYCLE");
}

function isRoute(route: DesktopRouteKind): boolean {
  return routeKinds.includes(route);
}

function validEpoch(epoch: number): boolean {
  return Number.isSafeInteger(epoch) && epoch >= 0;
}
