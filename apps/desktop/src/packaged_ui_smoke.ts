import type { TauriInvoke } from "./tauri_home_transport.ts";
import { requestCodeMirrorDocumentReplacement } from "./codemirror_document_editor.ts";

export const PackagedUiSmokeState = Object.freeze({
  Disabled: "Disabled",
  Booting: "Booting",
  HomeReady: "HomeReady",
  DocumentSaved: "DocumentSaved",
  DocumentReopened: "DocumentReopened",
  GraphActionsVerified: "GraphActionsVerified",
  CanvasMutationsVerified: "CanvasMutationsVerified",
  AssetActionsVerified: "AssetActionsVerified",
  CrossSurfaceVerified: "CrossSurfaceVerified",
  BackupRestoreVerified: "BackupRestoreVerified",
  CanvasLifecycleVerified: "CanvasLifecycleVerified",
  CanvasRecoveryVerified: "CanvasRecoveryVerified",
  RoutesMeasured: "RoutesMeasured",
  NativeReadsMeasured: "NativeReadsMeasured",
  Reporting: "Reporting",
  Passed: "Passed",
  Failed: "Failed",
} as const);

export type PackagedUiSmokeStateValue = typeof PackagedUiSmokeState[keyof typeof PackagedUiSmokeState];
type PackagedUiSmokeEvent = "home_ready" | "document_saved" | "document_reopened" | "graph_actions_verified" | "canvas_mutations_verified" | "asset_actions_verified" | "cross_surface_verified" | "backup_restore_verified" | "canvas_lifecycle_verified" | "canvas_recovery_verified" | "route_ready" | "samples_ready" | "report_ready" | "reported" | "failed";

export function transitionPackagedUiSmoke(
  state: PackagedUiSmokeStateValue,
  event: PackagedUiSmokeEvent,
): PackagedUiSmokeStateValue {
  if (event === "failed") return PackagedUiSmokeState.Failed;
  const transitions = new Map<string, PackagedUiSmokeStateValue>([
    [`${PackagedUiSmokeState.Booting}:home_ready`, PackagedUiSmokeState.HomeReady],
    [`${PackagedUiSmokeState.HomeReady}:document_saved`, PackagedUiSmokeState.DocumentSaved],
    [`${PackagedUiSmokeState.DocumentSaved}:document_reopened`, PackagedUiSmokeState.DocumentReopened],
    [`${PackagedUiSmokeState.DocumentReopened}:graph_actions_verified`, PackagedUiSmokeState.GraphActionsVerified],
    [`${PackagedUiSmokeState.GraphActionsVerified}:canvas_mutations_verified`, PackagedUiSmokeState.CanvasMutationsVerified],
    [`${PackagedUiSmokeState.CanvasMutationsVerified}:asset_actions_verified`, PackagedUiSmokeState.AssetActionsVerified],
    [`${PackagedUiSmokeState.AssetActionsVerified}:cross_surface_verified`, PackagedUiSmokeState.CrossSurfaceVerified],
    [`${PackagedUiSmokeState.CrossSurfaceVerified}:backup_restore_verified`, PackagedUiSmokeState.BackupRestoreVerified],
    [`${PackagedUiSmokeState.BackupRestoreVerified}:canvas_lifecycle_verified`, PackagedUiSmokeState.CanvasLifecycleVerified],
    [`${PackagedUiSmokeState.CanvasLifecycleVerified}:canvas_recovery_verified`, PackagedUiSmokeState.CanvasRecoveryVerified],
    [`${PackagedUiSmokeState.CanvasRecoveryVerified}:route_ready`, PackagedUiSmokeState.RoutesMeasured],
    [`${PackagedUiSmokeState.RoutesMeasured}:samples_ready`, PackagedUiSmokeState.NativeReadsMeasured],
    [`${PackagedUiSmokeState.NativeReadsMeasured}:report_ready`, PackagedUiSmokeState.Reporting],
    [`${PackagedUiSmokeState.Reporting}:reported`, PackagedUiSmokeState.Passed],
  ]);
  return transitions.get(`${state}:${event}`) ?? PackagedUiSmokeState.Failed;
}

export function nearestRankP95(samples: readonly number[]): number {
  if (samples.length === 0) return 0;
  const sorted = [...samples].sort((left, right) => left - right);
  return Math.ceil(sorted[Math.ceil(sorted.length * 0.95) - 1] ?? 0);
}

interface SmokeDocument {
  querySelector(selector: string): Element | null;
}

interface SmokeOptions {
  readonly invoke: TauriInvoke;
  readonly document: SmokeDocument;
  readonly now?: () => number;
  readonly delay?: (milliseconds: number) => Promise<void>;
  readonly warmupCount?: number;
  readonly sampleCount?: number;
  readonly markerTimeoutMs?: number;
}

const terminalStates = Object.freeze({
  graph: new Set(["Ready", "Empty", "Stale"]),
  canvas: new Set(["Ready", "Conflict", "RecoveryRequired", "Failed"]),
  assets: new Set(["Ready", "Empty"]),
});

export async function runPackagedUiSmoke(options: SmokeOptions): Promise<PackagedUiSmokeStateValue> {
  const mode = await options.invoke("get_packaged_ui_smoke_mode");
  if (!isEnabledMode(mode)) return PackagedUiSmokeState.Disabled;

  const now = options.now ?? (() => performance.now());
  const delay = options.delay ?? ((milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds)));
  const timeout = options.markerTimeoutMs ?? 5_000;
  const warmups = options.warmupCount ?? 30;
  const sampleCount = options.sampleCount ?? 200;
  let state: PackagedUiSmokeStateValue = PackagedUiSmokeState.Booting;
  let failureStage: "home" | "documentCreate" | "documentEdit" | "documentSave" | "documentReopen" | "graphOpen" | "graphScope" | "graphDepth" | "graphDirection" | "graphUnresolved" | "graphAssets" | "graphZoomIn" | "graphZoomOut" | "graphFitView" | "graphNode" | "graphDocumentRoute" | "canvas" | "canvasOpen" | "canvasCreate" | "canvasNote" | "canvasPan" | "canvasZoom" | "canvasArrange" | "canvasDocument" | "canvasEdge" | "canvasDrag" | "canvasResize" | "canvasReopen" | "canvasRename" | "canvasArchive" | "canvasArchiveReopen" | "canvasRecovery" | "assets" | "assetOpen" | "assetImport" | "assetDetail" | "assetPreview" | "assetUnlink" | "assetLibrary" | "assetDetachedDetail" | "assetRelink" | "assetFilters" | "canvasAsset" | "canvasAssetRoute" | "assetDocumentRoute" | "backupOpen" | "backupCreate" | "restorePreview" | "restoreConfirm" | "restoreReopen" | "measurement" = "home";
  const ready = { home: false, graph: false, canvas: false, assets: false };
  const generations = { graph: -1, canvas: -1, assets: -1 };
  let actionCount = 0;
  let durableReadbackCount = 0;

  try {
    await waitForHome(options.document, delay, timeout);
    ready.home = true;
    state = transitionPackagedUiSmoke(state, "home_ready");

    failureStage = "documentCreate";
    const documentResult = await runDocumentWorkflow(options.document, delay, timeout, (stage) => {
      failureStage = stage;
    });
    actionCount += documentResult.actionCount;
    durableReadbackCount += documentResult.durableReadbackCount;
    state = transitionPackagedUiSmoke(state, "document_saved");
    state = transitionPackagedUiSmoke(state, "document_reopened");

    failureStage = "graphOpen";
    const graphResult = await runGraphActionWorkflow(options.document, generations, documentResult.documentId, delay, timeout, (stage) => {
      failureStage = stage;
    });
    actionCount += graphResult.actionCount;
    durableReadbackCount += graphResult.durableReadbackCount;
    ready.graph = true;
    state = transitionPackagedUiSmoke(state, "graph_actions_verified");

    failureStage = "canvas";
    const canvasResult = await runCanvasActionWorkflow(options.document, generations, documentResult.documentId, delay, timeout, (stage) => {
      failureStage = stage;
    });
    actionCount += canvasResult.actionCount;
    durableReadbackCount += canvasResult.durableReadbackCount;
    ready.canvas = true;
    state = transitionPackagedUiSmoke(state, "canvas_mutations_verified");

    failureStage = "assets";
    const assetResult = await runAssetActionWorkflow(options.document, generations, documentResult.documentId, delay, timeout, (stage) => {
      failureStage = stage;
    });
    actionCount += assetResult.actionCount;
    durableReadbackCount += assetResult.durableReadbackCount;
    ready.assets = true;
    state = transitionPackagedUiSmoke(state, "asset_actions_verified");

    const crossSurfaceResult = await runCrossSurfaceWorkflow(
      options.document,
      generations,
      documentResult.documentId,
      assetResult.assetId,
      delay,
      timeout,
      (stage) => { failureStage = stage; },
    );
    actionCount += crossSurfaceResult.actionCount;
    durableReadbackCount += crossSurfaceResult.durableReadbackCount;
    state = transitionPackagedUiSmoke(state, "cross_surface_verified");

    const backupResult = await runBackupRestoreWorkflow(
      options.document,
      generations,
      documentResult.documentId,
      assetResult.assetId,
      crossSurfaceResult.canvasRevision,
      delay,
      timeout,
      (stage) => { failureStage = stage; },
    );
    actionCount += backupResult.actionCount;
    durableReadbackCount += backupResult.durableReadbackCount;
    state = transitionPackagedUiSmoke(state, "backup_restore_verified");

    const lifecycleResult = await runCanvasLifecycleWorkflow(
      options.document,
      generations,
      assetResult.assetId,
      delay,
      timeout,
      (stage) => { failureStage = stage; },
    );
    actionCount += lifecycleResult.actionCount;
    durableReadbackCount += lifecycleResult.durableReadbackCount;
    state = transitionPackagedUiSmoke(state, "canvas_lifecycle_verified");

    failureStage = "canvasRecovery";
    const recoveryResult = await runCanvasRecoveryWorkflow(
      options.invoke,
      options.document,
      generations,
      assetResult.assetId,
      delay,
      timeout,
    );
    actionCount += recoveryResult.actionCount;
    durableReadbackCount += recoveryResult.durableReadbackCount;
    state = transitionPackagedUiSmoke(state, "canvas_recovery_verified");

    await navigateHome(options.document, delay, timeout);
    actionCount += 1;

    for (const surface of ["graph", "canvas", "assets"] as const) {
      failureStage = surface;
      await navigateAndWait(options.document, surface, generations, delay, timeout);
      if (surface === "canvas") {
        await ensureCanvasReady(options.document, delay, timeout);
        generations.canvas = currentGeneration(options.document, "canvas");
      }
      ready[surface] = true;
      await navigateHome(options.document, delay, timeout);
      actionCount += 2;
    }
    state = transitionPackagedUiSmoke(state, "route_ready");

    const samples: number[] = [];
    failureStage = "measurement";
    const routes = ["graph", "canvas", "assets"] as const;
    for (let index = 0; index < warmups + sampleCount; index += 1) {
      const route = routes[index % routes.length];
      const started = now();
      await navigateAndWait(options.document, route, generations, delay, timeout);
      const elapsed = now() - started;
      if (index >= warmups) samples.push(elapsed);
      await navigateHome(options.document, delay, timeout);
    }
    state = transitionPackagedUiSmoke(state, "samples_ready");
    state = transitionPackagedUiSmoke(state, "report_ready");
    await options.invoke("complete_packaged_ui_smoke", {
      report: {
        homeReady: ready.home,
        graphReady: ready.graph,
        canvasReady: ready.canvas,
        assetsReady: ready.assets,
        sampleCount: samples.length,
        p95Ms: nearestRankP95(samples),
        errorCount: 0,
        failureStage: null,
        actionCount,
        durableReadbackCount,
      },
    });
    return transitionPackagedUiSmoke(state, "reported");
  } catch {
    state = transitionPackagedUiSmoke(state, "failed");
    await options.invoke("complete_packaged_ui_smoke", {
      report: {
        homeReady: ready.home,
        graphReady: ready.graph,
        canvasReady: ready.canvas,
        assetsReady: ready.assets,
        sampleCount: 0,
        p95Ms: 0,
        errorCount: 1,
        failureStage,
        actionCount,
        durableReadbackCount,
      },
    }).catch(() => undefined);
    return state;
  }
}

async function runDocumentWorkflow(
  document: SmokeDocument,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
  setFailureStage: (stage: "documentCreate" | "documentEdit" | "documentSave" | "documentReopen") => void,
): Promise<{ readonly documentId: string; readonly actionCount: number; readonly durableReadbackCount: number }> {
  setFailureStage("documentCreate");
  click(document, '[data-action="new-document"]');
  await waitUntil(document, () => {
    const root = document.querySelector("[data-cabinet-authoring-state]");
    return Boolean(root?.getAttribute("data-document-id") && ["Clean", "Saved"].includes(root?.getAttribute("data-cabinet-authoring-state") ?? ""));
  }, delay, timeout);
  const root = document.querySelector("[data-cabinet-authoring-state]");
  const documentId = root?.getAttribute("data-document-id") ?? "";
  setFailureStage("documentEdit");
  const editorHost = document.querySelector('[data-codemirror-host="mounted"]');
  if (!(editorHost instanceof HTMLElement)) throw new Error("PACKAGED_UI_EDITOR_MISSING");
  requestCodeMirrorDocumentReplacement(editorHost, "# Packaged Workflow\n\nDurable readback marker.\n");
  await waitUntil(document, () => document.querySelector("[data-cabinet-authoring-state]")?.getAttribute("data-cabinet-authoring-state") === "Dirty", delay, timeout);
  setFailureStage("documentSave");
  click(document, '[data-action="save-document"]');
  await waitUntil(document, () => {
    const current = document.querySelector("[data-cabinet-authoring-state]");
    return current?.getAttribute("data-cabinet-authoring-state") === "Saved"
      && current.getAttribute("data-document-revision") === current.getAttribute("data-persisted-revision");
  }, delay, timeout);
  setFailureStage("documentReopen");
  await navigateHome(document, delay, timeout);
  await reopenDocument(document, documentId, delay, timeout);
  return { documentId, actionCount: 5, durableReadbackCount: 2 };
}

async function runGraphActionWorkflow(
  document: SmokeDocument,
  generations: Record<"graph" | "canvas" | "assets", number>,
  documentId: string,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
  setFailureStage: (stage: "graphOpen" | "graphScope" | "graphDepth" | "graphDirection" | "graphUnresolved" | "graphAssets" | "graphZoomIn" | "graphZoomOut" | "graphFitView" | "graphNode" | "graphDocumentRoute") => void,
): Promise<{ readonly actionCount: number; readonly durableReadbackCount: number; readonly canvasRevision: number }> {
  setFailureStage("graphOpen");
  await navigateAndWait(document, "graph", generations, delay, timeout);
  let actionCount = 1;
  for (const [action, stage] of [
    ["graph-scope-local", "graphScope"],
    ["graph-toggle-depth", "graphDepth"],
    ["graph-toggle-direction", "graphDirection"],
    ["graph-toggle-unresolved", "graphUnresolved"],
    ["graph-toggle-assets", "graphAssets"],
  ] as const) {
    setFailureStage(stage);
    actionCount += await clickAndWaitGeneration(document, "graph", action, generations, delay, timeout);
  }
  setFailureStage("graphZoomIn");
  click(document, '[data-action="graph-zoom-in"]');
  await waitUntil(document, () => document.querySelector("[data-graph-camera-zoom]")?.getAttribute("data-graph-camera-zoom") === "125", delay, timeout);
  setFailureStage("graphZoomOut");
  click(document, '[data-action="graph-zoom-out"]');
  await waitUntil(document, () => document.querySelector("[data-graph-camera-zoom]")?.getAttribute("data-graph-camera-zoom") === "100", delay, timeout);
  click(document, '[data-action="graph-zoom-in"]');
  await waitUntil(document, () => document.querySelector("[data-graph-camera-zoom]")?.getAttribute("data-graph-camera-zoom") === "125", delay, timeout);
  setFailureStage("graphFitView");
  click(document, '[data-action="graph-fit-view"]');
  await waitUntil(document, () => document.querySelector("[data-graph-camera-zoom]")?.getAttribute("data-graph-camera-zoom") === "100", delay, timeout);
  setFailureStage("graphNode");
  click(document, `[data-action="select-graph-node"][data-graph-node-id="${documentId}"]`);
  await waitForSelector(document, '[data-action="open-graph-document"]', delay, timeout);
  setFailureStage("graphDocumentRoute");
  click(document, '[data-action="open-graph-document"]');
  await waitForAuthoringDocument(document, documentId, delay, timeout);
  return { actionCount: actionCount + 6, durableReadbackCount: 3 };
}

async function runCanvasActionWorkflow(
  document: SmokeDocument,
  generations: Record<"graph" | "canvas" | "assets", number>,
  documentId: string,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
  setFailureStage: (stage: "canvasOpen" | "canvasCreate" | "canvasNote" | "canvasPan" | "canvasZoom" | "canvasArrange" | "canvasDocument" | "canvasEdge" | "canvasDrag" | "canvasResize" | "canvasReopen") => void,
): Promise<{ readonly actionCount: number; readonly durableReadbackCount: number }> {
  setFailureStage("canvasOpen");
  await navigateHome(document, delay, timeout);
  await reopenDocument(document, documentId, delay, timeout);
  await navigateAndWait(document, "canvas", generations, delay, timeout);
  const neededCreate = document.querySelector('[data-action="create-canvas"]') instanceof HTMLElement;
  setFailureStage("canvasCreate");
  await ensureCanvasReady(document, delay, timeout);
  generations.canvas = currentGeneration(document, "canvas");
  let revision = currentCanvasRevision(document);
  for (const [action, stage] of [
    ["add-canvas-note", "canvasNote"],
    ["pan-canvas-left", "canvasPan"],
    ["zoom-canvas-in", "canvasZoom"],
  ] as const) {
    setFailureStage(stage);
    click(document, `[data-action="${action}"]`);
    revision = await waitForCanvasRevision(document, revision, delay, timeout);
  }
  setFailureStage("canvasArrange");
  click(document, '[data-action="auto-arrange-canvas"]');
  await waitUntil(document, () => document.querySelector('[data-exploration-surface="canvas"]')?.getAttribute("data-exploration-state") === "ArrangePreview", delay, timeout);
  click(document, '[data-action="apply-canvas-arrange"]');
  revision = await waitForCanvasRevision(document, revision, delay, timeout);

  setFailureStage("canvasDocument");
  click(document, '[data-action="add-canvas-document"]');
  revision = await waitForCanvasRevision(document, revision, delay, timeout);
  const documentNode = requireElement(document, '[data-action="select-canvas-node"][data-canvas-target-kind="document"]');
  const noteNode = requireElement(document, '[data-action="select-canvas-node"][data-canvas-target-kind="text"]');

  setFailureStage("canvasEdge");
  (noteNode as HTMLElement).click();
  await waitUntil(document, () => document.querySelector('[data-canvas-target-kind="text"]')?.getAttribute("aria-pressed") === "true", delay, timeout);
  (requireElement(document, `[data-canvas-node-id="${documentNode.getAttribute("data-canvas-node-id") ?? ""}"]`) as HTMLElement).click();
  await waitUntil(document, () => document.querySelector('[data-canvas-target-kind="text"]')?.getAttribute("aria-pressed") === "true"
    && document.querySelector('[data-canvas-target-kind="document"]')?.getAttribute("aria-pressed") === "true", delay, timeout);
  const connect = requireButton(document, '[data-action="connect-canvas-nodes"]');
  if (connect.disabled) throw new Error("PACKAGED_UI_CANVAS_CONNECT_DISABLED");
  connect.click();
  revision = await waitForCanvasRevision(document, revision, delay, timeout);
  const edge = requireElement(document, '[data-action="select-canvas-edge"]');
  (edge as SVGElement).dispatchEvent(new MouseEvent("click", { bubbles: true }));
  await waitUntil(document, () => document.querySelector('[data-action="select-canvas-edge"]')?.getAttribute("class")?.includes("selected") === true, delay, timeout);
  const removeEdge = requireButton(document, '[data-action="remove-canvas-edge"]');
  if (removeEdge.disabled) throw new Error("PACKAGED_UI_CANVAS_EDGE_REMOVE_DISABLED");
  removeEdge.click();
  revision = await waitForCanvasRevision(document, revision, delay, timeout);
  if (document.querySelector('[data-action="select-canvas-edge"]') !== null) {
    throw new Error("PACKAGED_UI_CANVAS_EDGE_READBACK_STALE");
  }

  setFailureStage("canvasDrag");
  const documentNodeId = documentNode.getAttribute("data-canvas-node-id") ?? "";
  const currentDocumentNode = requireElement(document, `[data-canvas-node-id="${documentNodeId}"]`);
  (currentDocumentNode as HTMLElement).click();
  const beforeDrag = canvasNodeGeometry(currentDocumentNode);
  currentDocumentNode.dispatchEvent(new DragEvent("dragstart", { bubbles: true, clientX: 100, clientY: 100 }));
  currentDocumentNode.dispatchEvent(new DragEvent("dragend", { bubbles: true, clientX: 180, clientY: 150 }));
  revision = await waitForCanvasRevision(document, revision, delay, timeout);
  const draggedNode = requireElement(document, `[data-canvas-node-id="${documentNodeId}"]`);
  const afterDrag = canvasNodeGeometry(draggedNode);
  if (afterDrag.x === beforeDrag.x && afterDrag.y === beforeDrag.y) {
    throw new Error("PACKAGED_UI_CANVAS_DRAG_UNCHANGED");
  }

  setFailureStage("canvasResize");
  const resize = requireElement(document, '[data-action="resize-canvas-node"]');
  resize.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, key: "ArrowRight" }));
  revision = await waitForCanvasRevision(document, revision, delay, timeout);
  const resizedGeometry = canvasNodeGeometry(requireElement(document, `[data-canvas-node-id="${documentNodeId}"]`));
  if (resizedGeometry.width <= afterDrag.width) throw new Error("PACKAGED_UI_CANVAS_RESIZE_UNCHANGED");

  setFailureStage("canvasReopen");
  await navigateHome(document, delay, timeout);
  await reopenDocument(document, documentId, delay, timeout);
  await navigateAndWait(document, "canvas", generations, delay, timeout);
  if (currentCanvasRevision(document) < revision) throw new Error("PACKAGED_UI_CANVAS_READBACK_STALE");
  const reopenedGeometry = canvasNodeGeometry(requireElement(document, `[data-canvas-node-id="${documentNodeId}"]`));
  if (JSON.stringify(reopenedGeometry) !== JSON.stringify(resizedGeometry)) {
    throw new Error("PACKAGED_UI_CANVAS_GEOMETRY_READBACK_MISMATCH");
  }
  return { actionCount: neededCreate ? 20 : 19, durableReadbackCount: 7 };
}

function requireElement(document: SmokeDocument, selector: string): Element {
  const element = document.querySelector(selector);
  if (!element) throw new Error("PACKAGED_UI_REQUIRED_ELEMENT_MISSING");
  return element;
}

function requireButton(document: SmokeDocument, selector: string): HTMLButtonElement {
  const element = requireElement(document, selector);
  if (!(element instanceof HTMLButtonElement)) throw new Error("PACKAGED_UI_REQUIRED_BUTTON_MISSING");
  return element;
}

function canvasNodeGeometry(node: Element): { readonly x: number; readonly y: number; readonly width: number; readonly height: number } {
  return Object.freeze({
    x: Number(node.getAttribute("data-canvas-node-x")),
    y: Number(node.getAttribute("data-canvas-node-y")),
    width: Number(node.getAttribute("data-canvas-node-width")),
    height: Number(node.getAttribute("data-canvas-node-height")),
  });
}

async function runAssetActionWorkflow(
  document: SmokeDocument,
  generations: Record<"graph" | "canvas" | "assets", number>,
  documentId: string,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
  setFailureStage: (stage: "assetOpen" | "assetImport" | "assetDetail" | "assetPreview" | "assetUnlink" | "assetLibrary" | "assetDetachedDetail" | "assetRelink" | "assetFilters") => void,
): Promise<{ readonly assetId: string; readonly actionCount: number; readonly durableReadbackCount: number }> {
  setFailureStage("assetOpen");
  await navigateHome(document, delay, timeout);
  await reopenDocument(document, documentId, delay, timeout);
  click(document, '[data-action="navigate-assets"]');
  await waitForNewGeneration(document, "assets", generations, delay, timeout);
  const importButton = document.querySelector('[data-action="import-asset"]');
  if (!(importButton instanceof HTMLButtonElement) || importButton.disabled) {
    throw new Error("PACKAGED_UI_ASSET_IMPORT_DISABLED");
  }
  setFailureStage("assetImport");
  importButton.click();
  await waitUntil(document, () => {
    const root = document.querySelector("[data-asset-import-state]");
    return root?.getAttribute("data-asset-import-state") === "Completed"
      && root.getAttribute("data-asset-scope") === "Document"
      && document.querySelector('[data-action="select-asset"]') !== null;
  }, delay, timeout);
  generations.assets = currentGeneration(document, "assets");

  setFailureStage("assetDetail");
  click(document, '[data-action="select-asset"]');
  await waitForAssetDetail(document, 1, delay, timeout);
  const assetId = document.querySelector("[data-selected-asset-id]")?.getAttribute("data-selected-asset-id") ?? "";
  if (!assetId) throw new Error("PACKAGED_UI_ASSET_ID_MISSING");
  setFailureStage("assetPreview");
  click(document, '[data-action="open-asset-preview"]');
  await waitUntil(document, () => {
    const root = document.querySelector("[data-asset-preview-state]");
    return root?.getAttribute("data-asset-preview-state") === "Ready"
      && document.querySelector('[data-asset-preview-presentation="text"]') !== null;
  }, delay, timeout);
  click(document, '[data-action="close-asset-preview"]');
  await waitUntil(document, () => document.querySelector("[data-asset-preview-state]")?.getAttribute("data-asset-preview-state") === "Idle", delay, timeout);
  setFailureStage("assetUnlink");
  click(document, '[data-action="unlink-asset"]');
  await waitUntil(document, () => {
    const root = document.querySelector("[data-asset-mutation-state]");
    return root?.getAttribute("data-asset-mutation-state") === "Idle"
      && root.getAttribute("data-asset-scope") === "Document"
      && document.querySelector('[data-action="select-asset"]') === null;
  }, delay, timeout);
  generations.assets = currentGeneration(document, "assets");

  setFailureStage("assetLibrary");
  click(document, '[data-action="open-asset-library"]');
  await waitForNewGeneration(document, "assets", generations, delay, timeout);
  setFailureStage("assetDetachedDetail");
  click(document, '[data-action="select-asset"]');
  await waitForAssetDetail(document, undefined, delay, timeout);
  setFailureStage("assetRelink");
  click(document, '[data-action="link-asset"]');
  await waitUntil(document, () => {
    const root = document.querySelector("[data-asset-mutation-state]");
    return root?.getAttribute("data-asset-mutation-state") === "Idle"
      && root.getAttribute("data-asset-scope") === "Document"
      && document.querySelector('[data-action="select-asset"]') !== null;
  }, delay, timeout);
  click(document, '[data-action="select-asset"]');
  await waitForAssetDetail(document, 1, delay, timeout);

  setFailureStage("assetFilters");
  for (const filter of ["all", "image", "pdf", "document", "other"]) {
    const selector = `[data-action="filter-assets-${filter}"]`;
    click(document, selector);
    await waitUntil(document, () => document.querySelector(selector)?.getAttribute("class")?.includes("active") === true, delay, timeout);
  }
  return { assetId, actionCount: 17, durableReadbackCount: 5 };
}

async function runCrossSurfaceWorkflow(
  document: SmokeDocument,
  generations: Record<"graph" | "canvas" | "assets", number>,
  documentId: string,
  assetId: string,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
  setFailureStage: (stage: "canvasAsset" | "canvasAssetRoute" | "assetDocumentRoute") => void,
): Promise<{ readonly actionCount: number; readonly durableReadbackCount: number; readonly canvasRevision: number }> {
  setFailureStage("canvasAsset");
  await navigateHome(document, delay, timeout);
  await reopenDocument(document, documentId, delay, timeout);
  await navigateAndWait(document, "canvas", generations, delay, timeout);
  await ensureCanvasReady(document, delay, timeout);
  generations.canvas = currentGeneration(document, "canvas");
  let revision = currentCanvasRevision(document);
  const addAsset = requireButton(document, '[data-action="add-canvas-asset"]');
  if (addAsset.disabled) throw new Error("PACKAGED_UI_CANVAS_ASSET_DISABLED");
  addAsset.click();
  revision = await waitForCanvasRevision(document, revision, delay, timeout);
  const assetSelector = `[data-canvas-target-kind="attachment"][data-canvas-target-id="${assetId}"]`;
  await waitForSelector(document, assetSelector, delay, timeout);

  await navigateHome(document, delay, timeout);
  await reopenDocument(document, documentId, delay, timeout);
  await navigateAndWait(document, "canvas", generations, delay, timeout);
  if (currentCanvasRevision(document) < revision) throw new Error("PACKAGED_UI_CANVAS_ASSET_READBACK_STALE");
  await waitForSelector(document, assetSelector, delay, timeout);

  setFailureStage("canvasAssetRoute");
  click(document, `${assetSelector} [data-action="open-canvas-asset"]`);
  await waitUntil(document, () => {
    const root = document.querySelector('[data-exploration-surface="assets"]');
    const detail = document.querySelector("[data-asset-detail-state]");
    return root !== null
      && detail?.getAttribute("data-selected-asset-id") === assetId
      && detail.getAttribute("data-asset-detail-state") === "Ready";
  }, delay, timeout);

  setFailureStage("assetDocumentRoute");
  click(document, `[data-action="open-linked-document"][data-linked-document-id="${documentId}"]`);
  await waitForAuthoringDocument(document, documentId, delay, timeout);
  return { actionCount: 9, durableReadbackCount: 4, canvasRevision: revision };
}

async function runBackupRestoreWorkflow(
  document: SmokeDocument,
  generations: Record<"graph" | "canvas" | "assets", number>,
  documentId: string,
  assetId: string,
  canvasRevision: number,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
  setFailureStage: (stage: "backupOpen" | "backupCreate" | "restorePreview" | "restoreConfirm" | "restoreReopen") => void,
): Promise<{ readonly actionCount: number; readonly durableReadbackCount: number }> {
  setFailureStage("backupOpen");
  await navigateHome(document, delay, timeout);
  click(document, '[data-action="navigate-backup"]');
  await waitUntil(document, () => document.querySelector("[data-backup-state]") !== null, delay, timeout);

  setFailureStage("backupCreate");
  click(document, '[data-action="create-backup"]');
  await waitUntil(document, () => {
    const root = document.querySelector("[data-backup-state]");
    const classes = new Set((root?.getAttribute("data-backup-manifest-classes") ?? "").split(","));
    return root?.getAttribute("data-backup-state") === "Ready"
      && Number(root.getAttribute("data-backup-manifest-entry-count")) > 0
      && ["current_documents", "canvas_records", "asset_metadata", "asset_objects", "asset_associations"]
        .every((dataClass) => classes.has(dataClass));
  }, delay, timeout);

  setFailureStage("restorePreview");
  click(document, '[data-action="preview-backup-restore"]');
  await waitUntil(document, () => document.querySelector("[data-backup-state]")?.getAttribute("data-backup-state") === "AwaitingConfirmation", delay, timeout);

  setFailureStage("restoreConfirm");
  click(document, '[data-action="confirm-backup-restore"]');
  await waitUntil(document, () => {
    const root = document.querySelector("[data-backup-state]");
    return root?.getAttribute("data-backup-state") === "Completed"
      && root.getAttribute("data-restore-operation-state") === "Completed";
  }, delay, timeout);

  setFailureStage("restoreReopen");
  await navigateHome(document, delay, timeout);
  await reopenDocument(document, documentId, delay, timeout);
  await navigateAndWait(document, "canvas", generations, delay, timeout);
  await ensureCanvasReady(document, delay, timeout);
  if (currentCanvasRevision(document) < canvasRevision) throw new Error("PACKAGED_UI_RESTORE_CANVAS_STALE");
  await waitForSelector(document, `[data-canvas-target-kind="attachment"][data-canvas-target-id="${assetId}"]`, delay, timeout);

  await navigateHome(document, delay, timeout);
  await reopenDocument(document, documentId, delay, timeout);
  click(document, '[data-action="navigate-assets"]');
  await waitForNewGeneration(document, "assets", generations, delay, timeout);
  click(document, `[data-action="select-asset"][data-asset-id="${assetId}"]`);
  await waitForAssetDetail(document, 1, delay, timeout);
  await waitForSelector(document, `[data-action="open-linked-document"][data-linked-document-id="${documentId}"]`, delay, timeout);
  return { actionCount: 11, durableReadbackCount: 7 };
}

async function runCanvasLifecycleWorkflow(
  document: SmokeDocument,
  generations: Record<"graph" | "canvas" | "assets", number>,
  assetId: string,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
  setFailureStage: (stage: "canvasRename" | "canvasArchive" | "canvasArchiveReopen") => void,
): Promise<{ readonly actionCount: number; readonly durableReadbackCount: number }> {
  await navigateHome(document, delay, timeout);
  await navigateAndWait(document, "canvas", generations, delay, timeout);
  await ensureCanvasReady(document, delay, timeout);
  let revision = currentCanvasRevision(document);

  setFailureStage("canvasRename");
  click(document, '[data-action="rename-canvas"]');
  await waitForSelector(document, '[aria-label="새 캔버스 이름"]', delay, timeout);
  typeTextInput(document, '[aria-label="새 캔버스 이름"]', "지식 캔버스");
  click(document, '[data-action="confirm-canvas-rename"]');
  revision = await waitForCanvasRevision(document, revision, delay, timeout);
  await waitUntil(document, () => document.querySelector("[data-canvas-title]")?.getAttribute("data-canvas-title") === "지식 캔버스", delay, timeout);

  setFailureStage("canvasArchive");
  click(document, '[data-action="archive-canvas"]');
  await waitForSelector(document, '[data-action="confirm-canvas-archive"]', delay, timeout);
  click(document, '[data-action="confirm-canvas-archive"]');
  revision = await waitForCanvasRevision(document, revision, delay, timeout);
  await waitUntil(document, () => {
    const root = document.querySelector("[data-canvas-lifecycle]");
    const rename = document.querySelector('[data-action="rename-canvas"]');
    const archive = document.querySelector('[data-action="archive-canvas"]');
    return root?.getAttribute("data-canvas-lifecycle") === "archived"
      && rename?.hasAttribute("disabled") === true
      && archive?.hasAttribute("disabled") === true;
  }, delay, timeout);

  setFailureStage("canvasArchiveReopen");
  await navigateHome(document, delay, timeout);
  await navigateAndWait(document, "canvas", generations, delay, timeout);
  await waitUntil(document, () => {
    const root = document.querySelector("[data-canvas-lifecycle]");
    return root?.getAttribute("data-canvas-lifecycle") === "archived"
      && root.getAttribute("data-canvas-title") === "지식 캔버스"
      && currentCanvasRevision(document) >= revision
      && document.querySelector(`[data-canvas-target-kind="attachment"][data-canvas-target-id="${assetId}"]`) !== null;
  }, delay, timeout);
  return { actionCount: 7, durableReadbackCount: 3 };
}

async function runCanvasRecoveryWorkflow(
  invoke: TauriInvoke,
  document: SmokeDocument,
  generations: Record<"graph" | "canvas" | "assets", number>,
  assetId: string,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
): Promise<{ readonly actionCount: number; readonly durableReadbackCount: number }> {
  await invoke("corrupt_packaged_ui_smoke_canvas");
  await navigateHome(document, delay, timeout);
  await navigateAndWait(document, "canvas", generations, delay, timeout);
  await waitUntil(document, () => {
    const root = document.querySelector('[data-exploration-surface="canvas"]');
    return root?.getAttribute("data-exploration-state") === "RecoveryRequired"
      && document.querySelector('[data-action="recover-canvas"]') !== null;
  }, delay, timeout);
  click(document, '[data-action="recover-canvas"]');
  await waitUntil(document, () => {
    const root = document.querySelector('[data-exploration-surface="canvas"]');
    const canvas = document.querySelector("[data-canvas-lifecycle]");
    return root?.getAttribute("data-exploration-state") === "Ready"
      && canvas?.getAttribute("data-canvas-lifecycle") === "archived"
      && canvas.getAttribute("data-canvas-title") === "지식 캔버스"
      && document.querySelector(`[data-canvas-target-kind="attachment"][data-canvas-target-id="${assetId}"]`) !== null;
  }, delay, timeout);
  return { actionCount: 3, durableReadbackCount: 2 };
}

async function waitForAuthoringDocument(
  document: SmokeDocument,
  documentId: string,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
): Promise<void> {
  await waitUntil(document, () => {
    const root = document.querySelector("[data-cabinet-authoring-state]");
    return root?.getAttribute("data-document-id") === documentId
      && ["Clean", "Saved"].includes(root.getAttribute("data-cabinet-authoring-state") ?? "");
  }, delay, timeout);
}

async function waitForAssetDetail(
  document: SmokeDocument,
  referenceCount: number | undefined,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
): Promise<void> {
  await waitUntil(document, () => {
    const root = document.querySelector("[data-asset-detail-state]");
    return root?.getAttribute("data-asset-detail-state") === "Ready"
      && (referenceCount === undefined
        || Number(root.getAttribute("data-asset-reference-count")) === referenceCount);
  }, delay, timeout);
}

async function reopenDocument(document: SmokeDocument, documentId: string, delay: (milliseconds: number) => Promise<void>, timeout: number): Promise<void> {
  const selector = `[data-action="open-recent-document"][data-document-id="${documentId}"]`;
  await waitForSelector(document, selector, delay, timeout);
  click(document, selector);
  await waitUntil(document, () => {
    const root = document.querySelector("[data-cabinet-authoring-state]");
    return root?.getAttribute("data-document-id") === documentId
      && ["Clean", "Saved"].includes(root.getAttribute("data-cabinet-authoring-state") ?? "");
  }, delay, timeout);
}

export async function waitForSelector(
  document: SmokeDocument,
  selector: string,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
): Promise<void> {
  await waitUntil(document, () => document.querySelector(selector) !== null, delay, timeout);
}

async function clickAndWaitGeneration(
  document: SmokeDocument,
  surface: "graph" | "canvas" | "assets",
  action: string,
  generations: Record<"graph" | "canvas" | "assets", number>,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
): Promise<number> {
  click(document, `[data-action="${action}"]`);
  await waitForNewGeneration(document, surface, generations, delay, timeout);
  if (surface !== "graph") return 1;
  const root = document.querySelector('[data-exploration-surface="graph"]');
  const state = root?.getAttribute("data-exploration-state");
  const recovery = state === "Failed"
    ? document.querySelector('[data-action="reindex-graph"]') ?? document.querySelector('[data-action="retry-graph"]')
    : state === "Stale" ? document.querySelector('[data-action="reindex-graph"]') : null;
  if (state === "Failed" && !(recovery instanceof HTMLElement)) {
    throw new Error("PACKAGED_UI_GRAPH_RECOVERY_MISSING");
  }
  if (!(recovery instanceof HTMLElement)) return 1;
  const previous = generations.graph;
  recovery.click();
  await waitUntil(document, () => {
    const next = document.querySelector('[data-exploration-surface="graph"]');
    const nextState = next?.getAttribute("data-exploration-state");
    const generation = Number(next?.getAttribute("data-exploration-generation"));
    return Boolean(nextState && terminalStates.graph.has(nextState) && generation > previous);
  }, delay, timeout);
  generations.graph = currentGeneration(document, "graph");
  return 2;
}

async function waitForNewGeneration(
  document: SmokeDocument,
  surface: "graph" | "canvas" | "assets",
  generations: Record<"graph" | "canvas" | "assets", number>,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
): Promise<void> {
  await waitUntil(document, () => {
    const root = document.querySelector(`[data-exploration-surface="${surface}"]`);
    const state = root?.getAttribute("data-exploration-state");
    const generation = Number(root?.getAttribute("data-exploration-generation"));
    return Boolean(state && (terminalStates[surface].has(state) || state === "Failed") && generation > generations[surface]);
  }, delay, timeout);
  generations[surface] = currentGeneration(document, surface);
  if (document.querySelector(`[data-exploration-surface="${surface}"]`)?.getAttribute("data-exploration-state") === "Failed" && surface !== "graph") {
    throw new Error("PACKAGED_UI_ROUTE_FAILED");
  }
}

function currentCanvasRevision(document: SmokeDocument): number {
  return Number(document.querySelector("[data-canvas-revision]")?.getAttribute("data-canvas-revision"));
}

async function waitForCanvasRevision(document: SmokeDocument, previous: number, delay: (milliseconds: number) => Promise<void>, timeout: number): Promise<number> {
  await waitUntil(document, () => currentCanvasRevision(document) > previous
    && document.querySelector('[data-exploration-surface="canvas"]')?.getAttribute("data-exploration-state") === "Ready", delay, timeout);
  return currentCanvasRevision(document);
}

function isEnabledMode(value: unknown): value is { readonly enabled: true } {
  return typeof value === "object" && value !== null && (value as { enabled?: unknown }).enabled === true;
}

async function navigateAndWait(
  document: SmokeDocument,
  surface: "graph" | "canvas" | "assets",
  generations: Record<"graph" | "canvas" | "assets", number>,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
): Promise<void> {
  click(document, `[data-action="navigate-${surface}"]`);
  await waitUntil(document, () => {
    const root = document.querySelector(`[data-exploration-surface="${surface}"]`);
    const state = root?.getAttribute("data-exploration-state");
    const generation = Number(root?.getAttribute("data-exploration-generation"));
    return Boolean(state && terminalStates[surface].has(state) && generation > generations[surface]);
  }, delay, timeout);
  generations[surface] = currentGeneration(document, surface);
  const root = document.querySelector(`[data-exploration-surface="${surface}"]`);
  if (root?.getAttribute("data-exploration-state") === "Failed" && surface !== "canvas") {
    throw new Error("PACKAGED_UI_ROUTE_FAILED");
  }
}

function currentGeneration(document: SmokeDocument, surface: "graph" | "canvas" | "assets"): number {
  return Number(document.querySelector(`[data-exploration-surface="${surface}"]`)?.getAttribute("data-exploration-generation"));
}

async function ensureCanvasReady(
  document: SmokeDocument,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
): Promise<void> {
  const root = document.querySelector('[data-exploration-surface="canvas"]');
  if (root?.getAttribute("data-exploration-state") === "Ready") return;
  const create = document.querySelector('[data-action="create-canvas"]');
  if (!(create instanceof HTMLElement)) throw new Error("PACKAGED_UI_CANVAS_FAILED");
  create.click();
  await waitUntil(document, () =>
    document.querySelector('[data-exploration-surface="canvas"]')?.getAttribute("data-exploration-state") === "Ready",
  delay, timeout);
}

async function navigateHome(
  document: SmokeDocument,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
): Promise<void> {
  click(document, '[data-action="navigate-home"]');
  await waitForHome(document, delay, timeout);
}

async function waitForHome(document: SmokeDocument, delay: (milliseconds: number) => Promise<void>, timeout: number): Promise<void> {
  await waitUntil(document, () => {
    const state = document.querySelector("[data-cabinet-home-state]")?.getAttribute("data-cabinet-home-state");
    return state === "Ready" || state === "Empty" || state === "Degraded";
  }, delay, timeout);
}

function click(document: SmokeDocument, selector: string): void {
  const target = document.querySelector(selector);
  if (!(target instanceof HTMLElement) || target.hasAttribute("disabled")) throw new Error("PACKAGED_UI_ACTION_MISSING");
  target.click();
}

function typeTextInput(document: SmokeDocument, selector: string, value: string): void {
  const target = document.querySelector(selector);
  if (!(target instanceof HTMLInputElement) || target.hasAttribute("disabled")) {
    throw new Error("PACKAGED_UI_INPUT_MISSING");
  }
  const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set;
  if (!setter) throw new Error("PACKAGED_UI_INPUT_UNAVAILABLE");
  setter.call(target, value);
  target.dispatchEvent(new Event("input", { bubbles: true }));
}

async function waitUntil(
  document: SmokeDocument,
  predicate: () => boolean,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
): Promise<void> {
  void document;
  const started = performance.now();
  while (!predicate()) {
    if (performance.now() - started >= timeout) throw new Error("PACKAGED_UI_MARKER_TIMEOUT");
    await delay(10);
  }
}
