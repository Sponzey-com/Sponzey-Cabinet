import assert from "node:assert/strict";
import test from "node:test";

import {
  createDesktopRouteControllerState,
  graphQueryScopeForRoute,
  transitionDesktopRoute,
  type DesktopRouteControllerState,
} from "../src/desktop_route_controller.ts";

const home = { kind: "Home" } as const;
const documentRoute = { kind: "Document", documentId: "doc-1" } as const;
const graphRoute = { kind: "Graph", centerDocumentId: "doc-1", scope: "Local" } as const;

test("Graph route scope maps explicitly to the graph query boundary", () => {
  assert.equal(graphQueryScopeForRoute({ kind: "Graph", scope: "Global" }), "global");
  assert.equal(graphQueryScopeForRoute(graphRoute), "local");
});

test("moves immediately when no operation blocks navigation", () => {
  const initial = createDesktopRouteControllerState(home, { workspaceId: "workspace-1", originRoute: "Home" });
  const result = transitionDesktopRoute(initial, { type: "TransitionRequested", target: documentRoute, selection: { workspaceId: "workspace-1", documentId: "doc-1", originRoute: "Home" }, blocker: { kind: "None" } });
  assert.equal(result.state.status, "Stable");
  assert.deepEqual(result.state.route, documentRoute);
  assert.equal(result.effect, undefined);
});

test("routes to the backup surface with workspace identity only", () => {
  const initial = createDesktopRouteControllerState(home, { workspaceId: "workspace-1", originRoute: "Home" });
  const result = transitionDesktopRoute(initial, {
    type: "TransitionRequested",
    target: { kind: "Backup" },
    selection: { workspaceId: "workspace-1", originRoute: "Home" },
    blocker: { kind: "None" },
  });
  assert.equal(result.state.status, "Stable");
  if (result.state.status === "Stable") assert.deepEqual(result.state.route, { kind: "Backup" });
});

test("keeps current route while dirty document awaits a decision", () => {
  const initial = stableDocument();
  const result = transitionDesktopRoute(initial, { type: "TransitionRequested", target: graphRoute, selection: { workspaceId: "workspace-1", documentId: "doc-1", originRoute: "Document" }, blocker: { kind: "DirtyDocument", resourceId: "doc-1" } });
  assert.equal(result.state.status, "AwaitingDecision");
  assert.deepEqual(result.state.currentRoute, documentRoute);
  assert.deepEqual(result.state.targetRoute, graphRoute);
});

test("requests resolution side effect and accepts only matching completion", () => {
  let state: DesktopRouteControllerState = transitionDesktopRoute(stableDocument(), { type: "TransitionRequested", target: graphRoute, selection: { workspaceId: "workspace-1", documentId: "doc-1", originRoute: "Document" }, blocker: { kind: "DirtyDocument", resourceId: "doc-1" } }).state;
  const resolving = transitionDesktopRoute(state, { type: "ResolveAndContinue", operationId: "save-1" });
  assert.equal(resolving.state.status, "AwaitingResolution");
  assert.deepEqual(resolving.effect, { type: "SaveDocument", resourceId: "doc-1", operationId: "save-1" });
  const stale = transitionDesktopRoute(resolving.state, { type: "ResolutionCompleted", operationId: "old-save" });
  assert.deepEqual(stale.state, resolving.state);
  const completed = transitionDesktopRoute(stale.state, { type: "ResolutionCompleted", operationId: "save-1" });
  assert.equal(completed.state.status, "Stable");
  assert.deepEqual(completed.state.route, graphRoute);
});

test("cancel and failed resolution preserve the current route", () => {
  const awaiting = transitionDesktopRoute(stableDocument(), { type: "TransitionRequested", target: graphRoute, selection: { workspaceId: "workspace-1", documentId: "doc-1", originRoute: "Document" }, blocker: { kind: "ActiveAssetImport", resourceId: "asset-op-1" } }).state;
  const cancelled = transitionDesktopRoute(awaiting, { type: "CancelTransition" });
  assert.equal(cancelled.state.status, "Stable");
  assert.deepEqual(cancelled.state.route, documentRoute);
  const resolving = transitionDesktopRoute(awaiting, { type: "ResolveAndContinue", operationId: "cancel-import-1" });
  const failed = transitionDesktopRoute(resolving.state, { type: "ResolutionFailed", operationId: "cancel-import-1", errorCode: "ASSET_CANCEL_FAILED" });
  assert.equal(failed.state.status, "Failed");
  assert.deepEqual(failed.state.currentRoute, documentRoute);
  assert.equal(failed.state.errorCode, "ASSET_CANCEL_FAILED");
});

test("rejects route and selection identity mismatch", () => {
  assert.throws(() => createDesktopRouteControllerState(documentRoute, { workspaceId: "workspace-1", originRoute: "Home" }), /INVALID_ROUTE_SELECTION/);
});

test("moves from an asset selection to a document without retaining the asset as target identity", () => {
  const assetRoute = { kind: "Assets", assetId: "asset-1" } as const;
  const initial = createDesktopRouteControllerState(assetRoute, {
    workspaceId: "workspace-1",
    assetId: "asset-1",
    originRoute: "Canvas",
  });
  const result = transitionDesktopRoute(initial, {
    type: "TransitionRequested",
    target: { kind: "Document", documentId: "doc-7" },
    selection: { workspaceId: "workspace-1", documentId: "doc-7", originRoute: "Assets" },
    blocker: { kind: "None" },
  });

  assert.equal(result.state.status, "Stable");
  if (result.state.status !== "Stable") return;
  assert.deepEqual(result.state.route, { kind: "Document", documentId: "doc-7" });
  assert.deepEqual(result.state.selection, {
    workspaceId: "workspace-1",
    documentId: "doc-7",
    originRoute: "Assets",
  });
  assert.equal(result.state.selection.assetId, undefined);
});

function stableDocument(): DesktopRouteControllerState {
  return createDesktopRouteControllerState(documentRoute, { workspaceId: "workspace-1", documentId: "doc-1", originRoute: "Home" });
}
