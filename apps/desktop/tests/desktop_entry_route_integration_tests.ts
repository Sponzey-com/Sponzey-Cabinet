import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const source = await readFile(new URL("../src/desktop_entry.ts", import.meta.url), "utf8");

test("desktop entry uses route controller as the surface source of truth", () => {
  assert.match(source, /createDesktopRouteControllerState/);
  assert.match(source, /transitionDesktopRoute/);
  assert.match(source, /requestDesktopRoute/);
  assert.doesNotMatch(source, /const \[surface, setSurface\]/);
});

test("all primary route kinds are mapped through the shared request", () => {
  for (const kind of ["Home", "Document", "Graph", "Assets", "Backup"]) {
    assert.match(source, new RegExp(`requestDesktopRoute\\(\\{ kind: "${kind}"`));
  }
  assert.match(source, /const openCanvas = useCallback/);
  assert.match(source, /\{ kind: "Canvas", \.\.\.\(selectedCanvasId/);
  assert.match(source, /onCanvas: \(\) => openCanvas\(\)/);
  assert.match(source, /createDesktopSearchNavigationIntent/);
  assert.match(source, /requestDesktopRoute\(intent\.route, intent\.selection\)/);
});

test("Graph mode controls transition the route instead of mutating query scope alone", () => {
  assert.match(source, /createDesktopGraphScopeIntent/);
  assert.match(source, /onGraphScopeChange:\s*changeGraphScope/);
  assert.match(source, /requestDesktopRoute\(intent\.route, intent\.selection\)/);
});

test("backup route uses the typed transport, controller, and recovery surface", () => {
  assert.match(source, /createTauriBackupRecoveryTransport\(bootstrapInvoke\)/);
  assert.match(source, /createDesktopBackupRecoverySnapshot/);
  assert.match(source, /recoverDesktopBackupStartup/);
  assert.match(source, /startDesktopBackupOperation/);
  assert.match(source, /pollDesktopBackupOperation/);
  assert.match(source, /cancelDesktopBackupOperation/);
  assert.match(source, /startDesktopRestoreOperation/);
  assert.match(source, /pollDesktopRestoreOperation/);
  assert.doesNotMatch(source, /createDesktopBackup\(/);
  assert.doesNotMatch(source, /confirmDesktopRestore\(/);
  assert.match(source, /createDesktopBackupRecoveryElement/);
  assert.match(source, /dismissDesktopRestoreConfirmation/);
});

test("dirty authoring resolution preserves target until save completion", () => {
  assert.match(source, /kind: "DirtyDocument"/);
  assert.match(source, /type: "ResolveAndContinue"/);
  assert.match(source, /type: "ResolutionCompleted"/);
  assert.match(source, /type: "ResolutionFailed"/);
  assert.match(source, /type: "DiscardAndContinue"/);
  assert.match(source, /type: "CancelTransition"/);
});

test("blocked navigation does not start target loading before route commit", () => {
  assert.match(source, /if \(!requestDesktopRoute\(intent\.route, intent\.selection\)/);
  assert.match(source, /if \(!requestDesktopRoute\(\{ kind: "Document"/);
});

test("global search forwards its query into the navigator loading model", () => {
  assert.match(source, /const openNavigator = useCallback\(\(query\?: string\)/);
  assert.match(source, /filter: intent\.route\.query/);
  assert.match(source, /if \(intent\.kind === "NoOp"\) return/);
  assert.match(source, /focusDesktopWorkspaceSearch\(document\)/);
});

test("route guard treats clean loaded documents as navigable without dirty resolution", () => {
  assert.match(source, /DocumentSaveCoordinatorState\.Clean/);
  assert.match(source, /DocumentSaveCoordinatorState\.NoDocument,[\s\S]{0,120}DocumentSaveCoordinatorState\.Clean,[\s\S]{0,120}DocumentSaveCoordinatorState\.Saved/);
});

test("search result document navigation preserves and restores result context", () => {
  for (const token of [
    "createDesktopSearchReturnContext",
    "captureDesktopSearchViewport(document, documentId)",
    'type: "ResultOpened"',
    'type: "ReturnRequested"',
    "restoreDesktopSearchViewport(document, searchReturnContext)",
    "onReturnToSearch:",
  ]) assert.match(source, new RegExp(token.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")));
});

test("returning home refreshes durable recent documents before reopen", () => {
  assert.match(source, /previousSurface/);
  assert.match(source, /surface === "Home"/);
  assert.match(source, /void loadHome\(\)/);
});

test("Home query completion is guarded by the active route epoch", () => {
  for (const token of [
    "createDesktopRouteQueryLifecycle",
    "transitionDesktopRouteQueryLifecycle",
    "beginDesktopRouteQuery",
    "canApplyDesktopRouteQuery",
    "routeQueryLifecycleRef",
  ]) assert.match(source, new RegExp(token));
});

test("Search result window is reconciled and wired through explicit actions", () => {
  for (const token of [
    "createDesktopSearchResultWindow",
    "transitionDesktopSearchResultWindow",
    "resultWindow:",
    "onPreviousResults:",
    "onNextResults:",
  ]) assert.match(source, new RegExp(token));
});

test("global search asset results open the Assets route with explicit selection", () => {
  for (const token of [
    "const openSearchAsset = useCallback((assetId: string)",
    'type: "ResultOpened"',
    'result: { kind: "Asset", assetId }',
    'requestDesktopRoute({ kind: "Assets", assetId }',
    "workspaceId: \"workspace-1\",",
    "assetId,",
    "onOpenAsset: openSearchAsset",
  ]) assert.match(source, new RegExp(token.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")));
});

test("Search Escape clears query then restores the explicit origin route", () => {
  for (const token of [
    "createDesktopSearchEscapeIntent",
    "searchOriginContext",
    "onSearchEscape:",
    'intent.kind === "ClearQuery"',
    "requestDesktopRoute(intent.route, searchOriginContext.selection)",
  ]) assert.match(source, new RegExp(token.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")));
});

test("global search route actions are driven by the explicit overlay lifecycle", () => {
  for (const token of [
    "createGlobalSearchOverlayLifecycle",
    "transitionGlobalSearchOverlay",
    "globalSearchOverlayRef",
    'type: "OpenRequested"',
    'type: "SearchSucceeded"',
    'type: "EscapePressed"',
    'type: "ResultOpened"',
  ]) assert.match(source, new RegExp(token.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")));

  assert.match(source, /setGlobalSearchOverlay\(opened\.state\)/);
  assert.match(source, /setGlobalSearchOverlay\(closed\.state\)/);
  assert.match(source, /setGlobalSearchOverlay\(selected\.state\)/);
  assert.doesNotMatch(source, /let isSearchOpen/);
  assert.doesNotMatch(source, /const \[isSearchOpen/);
});

test("Command K and search focus open the global overlay before text search runs", () => {
  assert.match(source, /const openGlobalSearchOverlay = useCallback\(\(\)/);
  assert.match(source, /requestDesktopRoute\(\{ kind: "Search" \}/);
  assert.match(source, /onSearchOpen: openGlobalSearchOverlay/);
  assert.match(source, /openGlobalSearchOverlay\(\);\s*\n\s*focusDesktopWorkspaceSearch\(document\)/);
  assert.match(source, /createDocumentNavigatorLoadingModel\(\{\s*workspaceId: "workspace-1",\s*view: "Tree",\s*generation: generation\.current,/s);
});
test("desktop entry wires global shell actions into every routed surface", async () => {
  const source = await readFile(new URL("../src/desktop_entry.ts", import.meta.url), "utf8");
  for (const token of [
    "onCreateDocument: createNewDocument",
    "onBackup:",
    "onDocument: resumeDocument",
    "onSearch: openNavigator",
  ]) assert.match(source, new RegExp(token));
  assert.match(source, /onResumeDocument: resumeDocument/);
  assert.match(source, /resolveDesktopDocumentMenuTarget/);
  assert.match(source, /target\.kind === "EmptyWorkspace"/);
  assert.match(source, /requestDesktopRoute\(\{ kind: "Document" \}/);
  assert.doesNotMatch(source, /if \(documentId\)[\s\S]{0,180}createNewDocument\(\)/);
  assert.doesNotMatch(source, /onDocument: openNavigator/);
});

test("desktop entry owns one route-invariant sidebar document list", () => {
  assert.match(source, /const sidebarDocumentShortcuts = homeModel\.recentDocuments\.slice\(0, 5\)\.map/);
  assert.ok((source.match(/documentShortcuts: sidebarDocumentShortcuts/g) ?? []).length >= 6);
});
