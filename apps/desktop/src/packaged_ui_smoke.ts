import type { TauriInvoke } from "./tauri_home_transport.ts";
import { requestCodeMirrorDocumentReplacement } from "./codemirror_document_editor.ts";
import {
  aggregateAccessibilityRouteMeasurements,
  type AccessibilityRouteMeasurement,
} from "./accessibility_evidence_collector.ts";
import { validateAccessibilityMeasurement } from "./accessibility_evidence_contract.ts";
import {
  measurePackagedAccessibilityRoute,
  packagedUnnamedControlDiagnostic,
  type PackagedAccessibilityDocument,
} from "./packaged_accessibility_measurement_port.ts";
import { collectPackagedVisualReceipts, type PackagedVisualCollectionPort } from "./packaged_visual_evidence_collector.ts";
import type { RendererVisualMeasurement, ViewportVisualMeasurement, VisualEvidencePolicy, VisualRect, VisualRoute, VisualViewport } from "./viewport_visual_evidence_contract.ts";

export const PackagedUiSmokeState = Object.freeze({
  Disabled: "Disabled",
  Booting: "Booting",
  HomeReady: "HomeReady",
  DocumentSaved: "DocumentSaved",
  DocumentReopened: "DocumentReopened",
  DocumentVersionWorkflowVerified: "DocumentVersionWorkflowVerified",
  GraphActionsVerified: "GraphActionsVerified",
  CanvasMutationsVerified: "CanvasMutationsVerified",
  AssetActionsVerified: "AssetActionsVerified",
  DocumentAttachmentWorkflowVerified: "DocumentAttachmentWorkflowVerified",
  CrossSurfaceVerified: "CrossSurfaceVerified",
  BackupRestoreVerified: "BackupRestoreVerified",
  CanvasLifecycleVerified: "CanvasLifecycleVerified",
  CanvasRecoveryVerified: "CanvasRecoveryVerified",
  VisualEvidenceVerified: "VisualEvidenceVerified",
  RoutesMeasured: "RoutesMeasured",
  NativeReadsMeasured: "NativeReadsMeasured",
  Reporting: "Reporting",
  Passed: "Passed",
  Failed: "Failed",
} as const);

export type PackagedUiSmokeStateValue = typeof PackagedUiSmokeState[keyof typeof PackagedUiSmokeState];
type PackagedUiSmokeEvent = "home_ready" | "document_saved" | "document_reopened" | "document_version_verified" | "graph_actions_verified" | "canvas_mutations_verified" | "asset_actions_verified" | "document_attachment_verified" | "cross_surface_verified" | "backup_restore_verified" | "canvas_lifecycle_verified" | "canvas_recovery_verified" | "visual_evidence_verified" | "route_ready" | "samples_ready" | "report_ready" | "reported" | "failed";

export const PackagedGraphEvidenceState = Object.freeze({
  TargetSaving: "TargetSaving",
  SourceSaving: "SourceSaving",
  ProjectionWaiting: "ProjectionWaiting",
  LocalVerifying: "LocalVerifying",
  GlobalVerifying: "GlobalVerifying",
  Verified: "Verified",
  Failed: "Failed",
} as const);
export type PackagedGraphEvidenceStateValue = typeof PackagedGraphEvidenceState[keyof typeof PackagedGraphEvidenceState];
type PackagedGraphEvidenceEvent = "target_saved" | "source_saved" | "projection_ready" | "local_verified" | "global_verified" | "failed";

export const PackagedGraphAttachmentEvidenceState = Object.freeze({
  LocalVerifying: "LocalVerifying",
  GlobalVerifying: "GlobalVerifying",
  RouteVerifying: "RouteVerifying",
  Verified: "Verified",
  Failed: "Failed",
} as const);
export type PackagedGraphAttachmentEvidenceStateValue = typeof PackagedGraphAttachmentEvidenceState[keyof typeof PackagedGraphAttachmentEvidenceState];
type PackagedGraphAttachmentEvidenceEvent = "local_verified" | "global_verified" | "route_verified" | "failed";

export function transitionPackagedGraphAttachmentEvidence(
  state: PackagedGraphAttachmentEvidenceStateValue,
  event: PackagedGraphAttachmentEvidenceEvent,
): PackagedGraphAttachmentEvidenceStateValue {
  if (event === "failed") return PackagedGraphAttachmentEvidenceState.Failed;
  const transitions = new Map<string, PackagedGraphAttachmentEvidenceStateValue>([
    [`${PackagedGraphAttachmentEvidenceState.LocalVerifying}:local_verified`, PackagedGraphAttachmentEvidenceState.GlobalVerifying],
    [`${PackagedGraphAttachmentEvidenceState.GlobalVerifying}:global_verified`, PackagedGraphAttachmentEvidenceState.RouteVerifying],
    [`${PackagedGraphAttachmentEvidenceState.RouteVerifying}:route_verified`, PackagedGraphAttachmentEvidenceState.Verified],
  ]);
  return transitions.get(`${state}:${event}`) ?? PackagedGraphAttachmentEvidenceState.Failed;
}

export function transitionPackagedGraphEvidence(
  state: PackagedGraphEvidenceStateValue,
  event: PackagedGraphEvidenceEvent,
): PackagedGraphEvidenceStateValue {
  if (event === "failed") return PackagedGraphEvidenceState.Failed;
  const transitions = new Map<string, PackagedGraphEvidenceStateValue>([
    [`${PackagedGraphEvidenceState.TargetSaving}:target_saved`, PackagedGraphEvidenceState.SourceSaving],
    [`${PackagedGraphEvidenceState.SourceSaving}:source_saved`, PackagedGraphEvidenceState.ProjectionWaiting],
    [`${PackagedGraphEvidenceState.ProjectionWaiting}:projection_ready`, PackagedGraphEvidenceState.LocalVerifying],
    [`${PackagedGraphEvidenceState.LocalVerifying}:local_verified`, PackagedGraphEvidenceState.GlobalVerifying],
    [`${PackagedGraphEvidenceState.GlobalVerifying}:global_verified`, PackagedGraphEvidenceState.Verified],
  ]);
  return transitions.get(`${state}:${event}`) ?? PackagedGraphEvidenceState.Failed;
}

export function transitionPackagedUiSmoke(
  state: PackagedUiSmokeStateValue,
  event: PackagedUiSmokeEvent,
): PackagedUiSmokeStateValue {
  if (event === "failed") return PackagedUiSmokeState.Failed;
  const transitions = new Map<string, PackagedUiSmokeStateValue>([
    [`${PackagedUiSmokeState.Booting}:home_ready`, PackagedUiSmokeState.HomeReady],
    [`${PackagedUiSmokeState.HomeReady}:document_saved`, PackagedUiSmokeState.DocumentSaved],
    [`${PackagedUiSmokeState.DocumentSaved}:document_reopened`, PackagedUiSmokeState.DocumentReopened],
    [`${PackagedUiSmokeState.DocumentReopened}:document_version_verified`, PackagedUiSmokeState.DocumentVersionWorkflowVerified],
    [`${PackagedUiSmokeState.DocumentVersionWorkflowVerified}:graph_actions_verified`, PackagedUiSmokeState.GraphActionsVerified],
    [`${PackagedUiSmokeState.GraphActionsVerified}:canvas_mutations_verified`, PackagedUiSmokeState.CanvasMutationsVerified],
    [`${PackagedUiSmokeState.CanvasMutationsVerified}:asset_actions_verified`, PackagedUiSmokeState.AssetActionsVerified],
    [`${PackagedUiSmokeState.AssetActionsVerified}:document_attachment_verified`, PackagedUiSmokeState.DocumentAttachmentWorkflowVerified],
    [`${PackagedUiSmokeState.DocumentAttachmentWorkflowVerified}:cross_surface_verified`, PackagedUiSmokeState.CrossSurfaceVerified],
    [`${PackagedUiSmokeState.CrossSurfaceVerified}:backup_restore_verified`, PackagedUiSmokeState.BackupRestoreVerified],
    [`${PackagedUiSmokeState.BackupRestoreVerified}:canvas_lifecycle_verified`, PackagedUiSmokeState.CanvasLifecycleVerified],
    [`${PackagedUiSmokeState.CanvasLifecycleVerified}:canvas_recovery_verified`, PackagedUiSmokeState.CanvasRecoveryVerified],
    [`${PackagedUiSmokeState.CanvasRecoveryVerified}:visual_evidence_verified`, PackagedUiSmokeState.VisualEvidenceVerified],
    [`${PackagedUiSmokeState.VisualEvidenceVerified}:route_ready`, PackagedUiSmokeState.RoutesMeasured],
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

interface KeyboardActivatable {
  readonly disabled: boolean;
  focus(): void;
  click(): void;
  dispatchEvent(event: Event): boolean;
}

type KeyboardEventFactory = (key: string, options: KeyboardEventInit) => Event;

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

export function packagedAssetFilterVerificationSequence(): readonly ("all" | "image" | "pdf" | "document" | "other")[] {
  return Object.freeze(["all", "image", "pdf", "document", "other", "all"] as const);
}

export function packagedRestoreConvergenceSurfaces(): readonly ("Home" | "Document" | "Search" | "Graph" | "Canvas" | "Assets")[] {
  return Object.freeze(["Home", "Document", "Search", "Graph", "Canvas", "Assets"] as const);
}

export const PACKAGED_UI_SMOKE_DOCUMENT_TITLE = "Packaged Workflow";
export const PACKAGED_UI_SMOKE_TARGET_TITLE = "Packaged Target";
export const PACKAGED_UI_SMOKE_CANVAS_TEXT = "패키지 캔버스 메모 내구성 검증";
const PACKAGED_VISUAL_POLICY: VisualEvidencePolicy = Object.freeze({
  routes: Object.freeze(["Home", "Document", "Graph", "Canvas", "Assets", "Backup"]),
  viewports: Object.freeze([
    Object.freeze({ width: 1440, height: 900, zoomPercent: 100 }),
    Object.freeze({ width: 1180, height: 800, zoomPercent: 100 }),
    Object.freeze({ width: 960, height: 720, zoomPercent: 100 }),
    Object.freeze({ width: 760, height: 640, zoomPercent: 100 }),
    Object.freeze({ width: 760, height: 640, zoomPercent: 200 }),
  ]),
  rendererRoutes: Object.freeze(["Graph", "Canvas"]),
  minimumNonBackgroundPixels: 8,
  requiredActionsByRoute: Object.freeze({
    Home: Object.freeze(["new-document", "open-recent-document"]),
    Document: Object.freeze(["save-document", "open-plain-text-editor"]),
    Graph: Object.freeze(["graph-scope-global", "graph-scope-local", "graph-fit-view"]),
    Canvas: Object.freeze(["auto-arrange-canvas", "rename-canvas", "archive-canvas", "zoom-canvas-in", "zoom-canvas-out"]),
    Assets: Object.freeze(["import-asset", "search-assets"]),
    Backup: Object.freeze(["create-backup"]),
  }),
});

export async function runPackagedUiSmoke(options: SmokeOptions): Promise<PackagedUiSmokeStateValue> {
  const mode = await options.invoke("get_packaged_ui_smoke_mode");
  if (!isEnabledMode(mode)) return PackagedUiSmokeState.Disabled;
  if (mode.stage === "restartVerification") {
    return runPackagedUiSmokeRestart(options);
  }
  if (mode.stage === "visualEvidence") {
    return runPackagedVisualSmoke(options);
  }
  if (mode.stage !== "initial" && mode.stage !== "upgradeVerification") {
    return PackagedUiSmokeState.Failed;
  }

  const now = options.now ?? (() => performance.now());
  const delay = options.delay ?? ((milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds)));
  const timeout = options.markerTimeoutMs ?? 5_000;
  const warmups = options.warmupCount ?? 30;
  const sampleCount = options.sampleCount ?? 200;
  let state: PackagedUiSmokeStateValue = PackagedUiSmokeState.Booting;
  let failureStage: "home" | "documentCreate" | "documentEdit" | "documentSave" | "documentReopen" | "graphTargetSave" | "graphSourceSave" | "graphProjection" | "graphLocalEdge" | "graphGlobalEdge" | "graphSafeLabels" | "documentHistoryTab" | "documentHistoryLoad" | "documentHistoryReadback" | "documentDiff" | "documentRestorePreviewAction" | "documentRestorePreviewReadback" | "documentRestoreReview" | "documentRestoreCancel" | "documentRestoreConfirm" | "documentRestoreReadback" | "documentAttachmentTab" | "documentAttachmentOpen" | "documentAttachmentUnlinkRequest" | "documentAttachmentUnlinkCancel" | "graphOpen" | "graphScopeGlobal" | "graphScopeLocal" | "graphDepth" | "graphDirection" | "graphUnresolved" | "graphAssets" | "graphZoomIn" | "graphZoomOut" | "graphFitView" | "graphNode" | "graphDocumentRoute" | "graphAttachmentOpen" | "graphAttachmentLocalEdge" | "graphAttachmentLocalFilter" | "graphAttachmentLocalNode" | "graphAttachmentLocalIdentity" | "graphAttachmentLocalLabel" | "graphAttachmentGlobalEdge" | "graphAttachmentRoute" | "canvas" | "canvasOpen" | "canvasCreate" | "canvasNote" | "canvasTextEdit" | "canvasPan" | "canvasZoom" | "canvasArrange" | "canvasDocument" | "canvasEdge" | "canvasDrag" | "canvasResize" | "canvasReopen" | "canvasRename" | "canvasArchive" | "canvasArchiveReopen" | "canvasRecovery" | "canvasRecoveryOpen" | "canvasRecoveryDetect" | "canvasRecoveryApply" | "assets" | "assetOpen" | "assetImport" | "assetImportReadback" | "assetImportOperation" | "assetImportScope" | "assetImportPresentation" | "assetDetail" | "assetPreview" | "assetUnlink" | "assetLibrary" | "assetDetachedDetail" | "assetRelink" | "assetFilters" | "assetFilterAll" | "assetFilterImage" | "assetFilterPdf" | "assetFilterDocument" | "assetFilterOther" | "canvasAsset" | "canvasAssetRoute" | "assetDocumentRoute" | "backupOpen" | "backupCreate" | "restorePreview" | "restoreConfirm" | "restoreReopen" | "restoreHome" | "restoreDocument" | "restoreSearch" | "restoreGraph" | "restoreCanvas" | "restoreAssets" | "visualEvidence" | "measurement" = "home";
  const ready = { home: false, graph: false, canvas: false, assets: false };
  const generations = { graph: -1, canvas: -1, assets: -1 };
  let actionCount = 0;
  let durableReadbackCount = 0;
  let documentVersionWorkflowVerified = false;
  let documentAttachmentWorkflowVerified = false;
  let attachmentImportCompleted = false;
  let attachmentCurrentReadbackVerified = false;
  let attachmentDocumentReadbackVerified = false;
  const attachmentRestartReadbackVerified = false;
  let keyboardDocumentWorkflowVerified = false;
  let graphLinkFixtureSaved = false;
  let graphLocalEdgeVerified = false;
  let graphGlobalEdgeVerified = false;
  let graphSafeLabelsVerified = false;
  let canvasTextEditReadbackVerified = false;

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
    graphLinkFixtureSaved = documentResult.graphLinkFixtureSaved;
    state = transitionPackagedUiSmoke(state, "document_saved");
    state = transitionPackagedUiSmoke(state, "document_reopened");

    const versionResult = await runDocumentVersionWorkflow(options.document, delay, timeout, (stage) => {
      failureStage = stage;
    });
    actionCount += versionResult.actionCount;
    durableReadbackCount += versionResult.durableReadbackCount;
    documentVersionWorkflowVerified = true;
    state = transitionPackagedUiSmoke(state, "document_version_verified");

    failureStage = "graphOpen";
    const graphResult = await runGraphActionWorkflow(options.document, generations, documentResult.documentId, documentResult.targetDocumentId, documentResult.graphEvidenceState, delay, timeout, (stage) => {
      failureStage = stage;
    });
    actionCount += graphResult.actionCount;
    durableReadbackCount += graphResult.durableReadbackCount;
    ready.graph = true;
    graphLocalEdgeVerified = graphResult.graphLocalEdgeVerified;
    graphGlobalEdgeVerified = graphResult.graphGlobalEdgeVerified;
    graphSafeLabelsVerified = graphResult.graphSafeLabelsVerified;
    state = transitionPackagedUiSmoke(state, "graph_actions_verified");

    failureStage = "canvas";
    const canvasResult = await runCanvasActionWorkflow(options.document, generations, documentResult.documentId, delay, timeout, (stage) => {
      failureStage = stage;
    });
    actionCount += canvasResult.actionCount;
    durableReadbackCount += canvasResult.durableReadbackCount;
    canvasTextEditReadbackVerified = canvasResult.textEditReadbackVerified;
    ready.canvas = true;
    state = transitionPackagedUiSmoke(state, "canvas_mutations_verified");

    failureStage = "assets";
    const assetResult = await runAssetActionWorkflow(options.document, generations, documentResult.documentId, delay, timeout, (stage) => {
      failureStage = stage;
    });
    actionCount += assetResult.actionCount;
    durableReadbackCount += assetResult.durableReadbackCount;
    attachmentImportCompleted = true;
    attachmentCurrentReadbackVerified = true;
    ready.assets = true;
    state = transitionPackagedUiSmoke(state, "asset_actions_verified");

    const documentAttachmentResult = await runDocumentAttachmentWorkflow(
      options.document,
      documentResult.documentId,
      delay,
      timeout,
      (stage) => { failureStage = stage; },
    );
    actionCount += documentAttachmentResult.actionCount;
    durableReadbackCount += documentAttachmentResult.durableReadbackCount;
    documentAttachmentWorkflowVerified = true;
    attachmentDocumentReadbackVerified = true;
    keyboardDocumentWorkflowVerified = true;
    state = transitionPackagedUiSmoke(state, "document_attachment_verified");

    const crossSurfaceResult = await runCrossSurfaceWorkflow(
      options.document,
      generations,
      documentResult.documentId,
      documentResult.targetDocumentId,
      assetResult.assetId,
      delay,
      timeout,
      (stage) => { failureStage = stage; },
    );
    actionCount += crossSurfaceResult.actionCount;
    durableReadbackCount += crossSurfaceResult.durableReadbackCount;
    state = transitionPackagedUiSmoke(state, "cross_surface_verified");

    const backupResult = await runBackupRestoreWorkflow(
      options.invoke,
      options.document,
      generations,
      documentResult.documentId,
      documentResult.targetDocumentId,
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
      (stage) => { failureStage = stage; },
    );
    actionCount += recoveryResult.actionCount;
    durableReadbackCount += recoveryResult.durableReadbackCount;
    state = transitionPackagedUiSmoke(state, "canvas_recovery_verified");

    failureStage = "visualEvidence";
    try {
      await collectAndReportPackagedVisualEvidence(
        options.invoke,
        options.document as Document,
        documentResult.documentId,
        documentResult.targetDocumentId,
        generations,
        delay,
        timeout,
      );
    } catch (error) {
      const { errorCode, failureStage } = classifyPackagedVisualFailure(error);
      await options.invoke("report_packaged_ui_smoke_visual_failure", { errorCode, failureStage }).catch(() => undefined);
      throw error;
    }
    state = transitionPackagedUiSmoke(state, "visual_evidence_verified");

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
        graphLinkFixtureSaved,
        graphLocalEdgeVerified,
        graphGlobalEdgeVerified,
        graphSafeLabelsVerified,
        canvasReady: ready.canvas,
        canvasTextEditReadbackVerified,
        assetsReady: ready.assets,
        documentVersionWorkflowVerified,
        documentAttachmentWorkflowVerified,
        attachmentImportCompleted,
        attachmentCurrentReadbackVerified,
        attachmentDocumentReadbackVerified,
        attachmentRestartReadbackVerified,
        keyboardDocumentWorkflowVerified,
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
    if (failureStage === "assetImport") {
      const root = options.document.querySelector("[data-asset-import-state]");
      const importState = root?.getAttribute("data-asset-import-state");
      const importError = root?.getAttribute("data-asset-import-error-code");
      const scope = root?.getAttribute("data-asset-scope");
      if (scope !== "Document") failureStage = "assetImportScope";
      else if (importError === "ASSET_IMPORT_READBACK_MISMATCH") failureStage = "assetImportReadback";
      else if (importState === "Failed") failureStage = "assetImportOperation";
      else if (importState === "Completed") failureStage = "assetImportPresentation";
    }
    if ([
      "documentAttachmentTab",
      "documentHistoryTab",
      "graphTargetSave",
      "graphSourceSave",
      "graphAttachmentLocalEdge",
      "graphAttachmentGlobalEdge",
      "graphAttachmentRoute",
      "restoreSearch",
    ].includes(failureStage)) {
      const browserDocument = options.document as Document;
      await options.invoke("capture_packaged_ui_smoke_window", {
        request: {
          artifactKey: `failure-${failureStage.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`)}`,
          viewportWidth: browserDocument.documentElement.clientWidth,
          viewportHeight: browserDocument.documentElement.clientHeight,
        },
      }).catch(() => undefined);
    }
    await options.invoke("complete_packaged_ui_smoke", {
      report: {
        homeReady: ready.home,
        graphReady: ready.graph,
        graphLinkFixtureSaved,
        graphLocalEdgeVerified,
        graphGlobalEdgeVerified,
        graphSafeLabelsVerified,
        canvasReady: ready.canvas,
        canvasTextEditReadbackVerified,
        assetsReady: ready.assets,
        documentVersionWorkflowVerified,
        documentAttachmentWorkflowVerified,
        attachmentImportCompleted,
        attachmentCurrentReadbackVerified,
        attachmentDocumentReadbackVerified,
        attachmentRestartReadbackVerified,
        keyboardDocumentWorkflowVerified,
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

async function runPackagedVisualSmoke(options: SmokeOptions): Promise<PackagedUiSmokeStateValue> {
  const delay = options.delay ?? ((milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds)));
  const timeout = options.markerTimeoutMs ?? 5_000;
  try {
    await waitForHome(options.document, delay, timeout);
    const source = await packagedDocumentIdentityByTitle(
      options.document,
      PACKAGED_UI_SMOKE_DOCUMENT_TITLE,
      delay,
      timeout,
    );
    const target = await packagedDocumentIdentityByTitle(
      options.document,
      PACKAGED_UI_SMOKE_TARGET_TITLE,
      delay,
      timeout,
    );
    await collectAndReportPackagedVisualEvidence(
      options.invoke,
      options.document as Document,
      source,
      target,
      { graph: -1, canvas: -1, assets: -1 },
      delay,
      timeout,
    );
    await options.invoke("complete_packaged_ui_visual_smoke");
    return PackagedUiSmokeState.Passed;
  } catch (error) {
    const { errorCode, failureStage } = classifyPackagedVisualFailure(error);
    await options.invoke("report_packaged_ui_smoke_visual_failure", { errorCode, failureStage }).catch(() => undefined);
    await options.invoke("complete_packaged_ui_visual_smoke").catch(() => undefined);
    return PackagedUiSmokeState.Failed;
  }
}

export function classifyPackagedVisualFailure(error: unknown): Readonly<{
  errorCode: string
  failureStage: string
}> {
  if (error && typeof error === "object" && "code" in error
    && typeof (error as { code?: unknown }).code === "string"
    && /^ACCESSIBILITY_[A-Z0-9_]+$/.test((error as { code: string }).code)) {
    return Object.freeze({
      errorCode: "PACKAGED_VISUAL_ACCESSIBILITY_FAILED",
      failureStage: `accessibility:${(error as { code: string }).code}`,
    });
  }
  const errorCode = error && typeof error === "object" && "code" in error
    && typeof (error as { code?: unknown }).code === "string"
    && /^[A-Z0-9_]+$/.test((error as { code: string }).code)
    ? (error as { code: string }).code
    : "PACKAGED_VISUAL_UNCLASSIFIED_FAILURE";
  let failureStage = error && typeof error === "object" && "stage" in error
    && typeof (error as { stage?: unknown }).stage === "string"
    && /^[A-Za-z0-9:@_-]+$/.test((error as { stage: string }).stage)
    ? (error as { stage: string }).stage
    : "collection";
  if (error && typeof error === "object" && "detailCode" in error
    && typeof (error as { detailCode?: unknown }).detailCode === "string"
    && /^[A-Z0-9_]+$/.test((error as { detailCode: string }).detailCode)) {
    failureStage = `${failureStage}:${(error as { detailCode: string }).detailCode}`;
  }
  return Object.freeze({ errorCode, failureStage });
}

export function validatePackagedAccessibilityRouteMeasurement(
  measurement: AccessibilityRouteMeasurement,
  diagnostic?: string,
): void {
  try {
    validateAccessibilityMeasurement({
      requiredRouteFocusCount: 1,
      requiredTextZoomPercent: 200,
      minimumKeyboardJourneyCount: 1,
      minimumFocusRestorationCount: 1,
    }, {
      routeFocusCount: measurement.mainFocusReached ? 1 : 0,
      keyboardJourneyCount: measurement.keyboardJourneyPassed ? 1 : 0,
      focusRestorationCount: measurement.focusRestorationCount,
      visibleControlCount: measurement.visibleControlCount,
      namedControlCount: measurement.namedControlCount,
      textZoomPercent: 200,
      keyboardErrorCount: measurement.keyboardErrorCount,
      focusErrorCount: measurement.focusErrorCount,
      internalExposureCount: measurement.internalExposureCount,
    });
  } catch (error) {
    const detailCode = error && typeof error === "object" && "code" in error
      && typeof (error as { code?: unknown }).code === "string"
      ? (error as { code: string }).code
      : "ACCESSIBILITY_MEASUREMENT_INVALID";
    throw Object.freeze({
      code: "PACKAGED_VISUAL_ACCESSIBILITY_FAILED",
      stage: `accessibility:${measurement.route}${diagnostic ? `:${diagnostic}` : ""}`,
      detailCode,
    });
  }
}

async function packagedDocumentIdentityByTitle(
  document: SmokeDocument,
  title: string,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
): Promise<string> {
  const selector = `[data-action="open-recent-document"][data-document-title="${title}"]`;
  await waitForSelector(document, selector, delay, timeout);
  const identity = document.querySelector(selector)?.getAttribute("data-document-id")?.trim() ?? "";
  if (!identity) throw new Error("PACKAGED_UI_VISUAL_DOCUMENT_MISSING");
  return identity;
}

export async function runPackagedUiSmokeRestart(
  options: SmokeOptions,
): Promise<PackagedUiSmokeStateValue> {
  const delay = options.delay ?? ((milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds)));
  const timeout = options.markerTimeoutMs ?? 5_000;
  let failureStage: "home" | "document" | "attachmentTab" | "attachmentList" | "attachmentListLoading" | "attachmentListEmpty" | "attachmentListFailed" | "attachmentListMissing" | "attachmentDetail" | "canvasOpen" | "canvasCatalogSelect" | "canvasTextReadback" = "home";
  try {
    await waitForHome(options.document, delay, timeout);
    failureStage = "document";
    const durableDocumentSelector = `[data-action="open-recent-document"][data-document-title="${PACKAGED_UI_SMOKE_DOCUMENT_TITLE}"]`;
    await waitForSelector(options.document, durableDocumentSelector, delay, timeout);
    click(options.document, durableDocumentSelector);
    await waitUntil(options.document, () => {
      const root = options.document.querySelector("[data-cabinet-authoring-state]");
      return Boolean(root?.getAttribute("data-document-id"))
        && ["Clean", "Saved"].includes(root?.getAttribute("data-cabinet-authoring-state") ?? "");
    }, delay, timeout);
    failureStage = "attachmentTab";
    await openDocumentInspectorTab(options.document, "attachments", delay, timeout, "pointer");
    failureStage = "attachmentList";
    await waitForSelector(options.document, '[data-document-inspector-tab="attachments"] [data-action="select-document-asset"]', delay, timeout);
    click(options.document, '[data-action="select-document-asset"]');
    failureStage = "attachmentDetail";
    await waitForSelector(
      options.document,
      'button[data-action="preview-document-asset"]:not([disabled])',
      delay,
      timeout,
    );
    failureStage = "canvasOpen";
    const generations = { graph: -1, canvas: -1, assets: -1 };
    await navigateAndWait(options.document, "canvas", generations, delay, timeout);
    if (options.document.querySelector('[data-canvas-target-kind="text"] p')?.textContent?.trim() !== PACKAGED_UI_SMOKE_CANVAS_TEXT) {
      failureStage = "canvasCatalogSelect";
      selectFirstCanvasCatalogEntry(options.document);
      await waitUntil(options.document, () =>
        options.document.querySelector('[data-exploration-surface="canvas"]')?.getAttribute("data-exploration-state") === "Ready",
      delay, timeout);
    }
    failureStage = "canvasTextReadback";
    await waitForCanvasText(options.document, PACKAGED_UI_SMOKE_CANVAS_TEXT, delay, timeout);
    await options.invoke("complete_packaged_ui_smoke_restart", {
      report: {
        attachmentRestartReadbackVerified: true,
        canvasTextRestartReadbackVerified: true,
        errorCount: 0,
        failureStage: null,
      },
    });
    return PackagedUiSmokeState.Passed;
  } catch {
    if (failureStage === "attachmentList") {
      const panel = options.document.querySelector("[data-document-attachment-state]");
      const state = panel?.getAttribute("data-document-attachment-state");
      failureStage = state === "Loading"
        ? "attachmentListLoading"
        : state === "Failed"
          ? "attachmentListFailed"
          : state === "Ready" || state === "Empty"
            ? "attachmentListEmpty"
            : "attachmentListMissing";
    }
    await options.invoke("complete_packaged_ui_smoke_restart", {
      report: {
        attachmentRestartReadbackVerified: false,
        canvasTextRestartReadbackVerified: false,
        errorCount: 1,
        failureStage,
      },
    }).catch(() => undefined);
    return PackagedUiSmokeState.Failed;
  }
}

async function runDocumentAttachmentWorkflow(
  document: SmokeDocument,
  documentId: string,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
  setFailureStage: (stage: "documentAttachmentTab" | "documentAttachmentOpen" | "documentAttachmentUnlinkRequest" | "documentAttachmentUnlinkCancel") => void,
): Promise<{ readonly actionCount: number; readonly durableReadbackCount: number }> {
  setFailureStage("documentAttachmentTab");
  click(document, `[data-action="open-linked-document"][data-linked-document-id="${documentId}"]`);
  await waitForAuthoringDocument(document, documentId, delay, timeout);
  // The route can first render the retained snapshot for the same document.
  // Wait for the command-backed open before changing inspector query state.
  await delay(100);
  await waitForAuthoringDocument(document, documentId, delay, timeout);
  await openDocumentInspectorTab(document, "attachments", delay, timeout);
  setFailureStage("documentAttachmentOpen");
  await waitForSelector(document, '[data-document-inspector-tab="attachments"] [data-action="select-document-asset"]', delay, timeout);
  focusAndClick(document, '[data-action="select-document-asset"]');

  focusAndClick(document, '[data-action="open-document-asset-externally"]');
  await waitForSelector(document, '[data-document-asset-open-state="Opened"]', delay, timeout);

  setFailureStage("documentAttachmentUnlinkRequest");
  focusAndClick(document, '[data-action="unlink-document-asset"]');
  await waitForSelector(document, '[data-document-asset-unlink-state="Confirming"]', delay, timeout);

  setFailureStage("documentAttachmentUnlinkCancel");
  focusAndClick(document, '[data-action="cancel-document-asset-unlink"]');
  await waitUntil(document, () => document.querySelector('[data-document-asset-unlink-state]') === null
    && document.querySelector('[data-action="select-document-asset"]') !== null, delay, timeout);
  return { actionCount: 8, durableReadbackCount: 2 };
}

async function openDocumentInspectorTab(
  document: SmokeDocument,
  tab: "attachments" | "history",
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
  activation: "keyboard" | "pointer" = "keyboard",
): Promise<void> {
  const action = `[data-action="select-document-inspector-${tab}"]`;
  const panel = `[data-document-inspector-tab="${tab}"]`;
  await waitUntil(document, () => {
    if (document.querySelector(panel)) return true;
    const target = document.querySelector(action);
    if (!(target instanceof HTMLButtonElement) || target.disabled) return false;
    if (activation === "keyboard") {
      activateButtonByKeyboard(target, () => document.querySelector(":focus") === target);
    } else {
      target.click();
    }
    return document.querySelector(panel) !== null;
  }, delay, timeout);
}

async function runDocumentVersionWorkflow(
  document: SmokeDocument,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
  setFailureStage: (stage: "documentHistoryTab" | "documentHistoryLoad" | "documentHistoryReadback" | "documentDiff" | "documentRestorePreviewAction" | "documentRestorePreviewReadback" | "documentRestoreReview" | "documentRestoreCancel" | "documentRestoreConfirm" | "documentRestoreReadback") => void,
): Promise<{ readonly actionCount: number; readonly durableReadbackCount: number }> {
  setFailureStage("documentHistoryTab");
  await openDocumentInspectorTab(document, "history", delay, timeout);
  await waitForSelector(
    document,
    '[data-document-inspector-tab="history"] [data-action="load-history"]',
    delay,
    timeout,
  );
  setFailureStage("documentHistoryLoad");
  focusAndClick(document, '[data-action="load-history"]');
  setFailureStage("documentHistoryReadback");
  await waitForSelector(document, '.history-list li:nth-child(2)', delay, timeout);

  setFailureStage("documentDiff");
  focusAndClick(document, '.history-list li:last-child [data-action="compare-current-version"]');
  await waitForSelector(document, '[data-document-diff-state="Ready"]', delay, timeout);
  focusAndClick(document, '[data-action="close-document-diff"]');
  await waitUntil(document, () => document.querySelector('[data-document-diff-state]') === null, delay, timeout);

  setFailureStage("documentRestorePreviewAction");
  focusAndClick(document, '.history-list li:last-child [data-action="preview-restore"]');
  setFailureStage("documentRestorePreviewReadback");
  await waitForSelector(document, '[data-restore-state="PreviewReady"]', delay, timeout);
  setFailureStage("documentRestoreReview");
  focusAndClick(document, '[data-action="review-restore"]');
  await waitForSelector(document, '[data-restore-state="Confirming"]', delay, timeout);

  setFailureStage("documentRestoreCancel");
  focusAndClick(document, '[data-action="cancel-restore-confirmation"]');
  await waitForSelector(document, '[data-restore-state="PreviewReady"]', delay, timeout);

  setFailureStage("documentRestoreConfirm");
  focusAndClick(document, '[data-action="review-restore"]');
  await waitForSelector(document, '[data-restore-state="Confirming"]', delay, timeout);
  focusAndClick(document, '[data-action="confirm-restore"]');
  await waitForSelector(document, '[data-restore-state="Applied"]', delay, timeout);

  setFailureStage("documentRestoreReadback");
  await waitUntil(document, () => {
    const root = document.querySelector("[data-cabinet-authoring-state]");
    return ["Clean", "Saved"].includes(root?.getAttribute("data-cabinet-authoring-state") ?? "")
      && document.querySelector('.history-list li:nth-child(3)') !== null;
  }, delay, timeout);
  return { actionCount: 9, durableReadbackCount: 2 };
}

async function runDocumentWorkflow(
  document: SmokeDocument,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
  setFailureStage: (stage: "documentCreate" | "documentEdit" | "documentSave" | "documentReopen" | "graphTargetSave" | "graphSourceSave") => void,
): Promise<{ readonly documentId: string; readonly targetDocumentId: string; readonly graphLinkFixtureSaved: boolean; readonly graphEvidenceState: PackagedGraphEvidenceStateValue; readonly actionCount: number; readonly durableReadbackCount: number }> {
  let graphEvidenceState: PackagedGraphEvidenceStateValue = PackagedGraphEvidenceState.TargetSaving;
  setFailureStage("graphTargetSave");
  const targetDocumentId = await createPackagedDocument(
    document,
    PACKAGED_UI_SMOKE_TARGET_TITLE,
    `# ${PACKAGED_UI_SMOKE_TARGET_TITLE}\n\nTopology target.\n`,
    delay,
    timeout,
    undefined,
  );
  graphEvidenceState = transitionPackagedGraphEvidence(graphEvidenceState, "target_saved");
  await navigateHome(document, delay, timeout);

  setFailureStage("graphSourceSave");
  const documentId = await createPackagedDocument(
    document,
    PACKAGED_UI_SMOKE_DOCUMENT_TITLE,
    `# ${PACKAGED_UI_SMOKE_DOCUMENT_TITLE}\n\nDurable readback marker.\n\n[[${PACKAGED_UI_SMOKE_TARGET_TITLE}]]\n`,
    delay,
    timeout,
    targetDocumentId,
  );
  graphEvidenceState = transitionPackagedGraphEvidence(graphEvidenceState, "source_saved");
  setFailureStage("documentReopen");
  await navigateHome(document, delay, timeout);
  await reopenDocument(document, documentId, delay, timeout);
  return {
    documentId,
    targetDocumentId,
    graphLinkFixtureSaved: graphEvidenceState === PackagedGraphEvidenceState.ProjectionWaiting,
    graphEvidenceState,
    actionCount: 9,
    durableReadbackCount: 3,
  };
}

async function createPackagedDocument(
  document: SmokeDocument,
  title: string,
  body: string,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
  previousDocumentId: string | undefined,
): Promise<string> {
  focusAndClick(document, '[data-action="new-document"]');
  await waitUntil(document, () => {
    const root = document.querySelector("[data-cabinet-authoring-state]");
    return isNewAuthoringDocumentReady(root, previousDocumentId);
  }, delay, timeout);
  const root = document.querySelector("[data-cabinet-authoring-state]");
  const documentId = root?.getAttribute("data-document-id") ?? "";
  await waitForSelector(
    document,
    `[data-cabinet-authoring-state][data-document-id="${documentId}"] [data-codemirror-host="mounted"]`,
    delay,
    timeout,
  );
  const editorHost = document.querySelector(
    `[data-cabinet-authoring-state][data-document-id="${documentId}"] [data-codemirror-host="mounted"]`,
  );
  if (!(editorHost instanceof HTMLElement)) throw new Error("PACKAGED_UI_EDITOR_MISSING");
  requestCodeMirrorDocumentReplacement(editorHost, body);
  await waitUntil(document, () => document.querySelector("[data-cabinet-authoring-state]")?.getAttribute("data-cabinet-authoring-state") === "Dirty", delay, timeout);
  dispatchMacSaveShortcut(editorHost);
  await waitUntil(document, () => {
    const current = document.querySelector("[data-cabinet-authoring-state]");
    return current?.getAttribute("data-cabinet-authoring-state") === "Saved"
      && current.getAttribute("data-document-revision") === current.getAttribute("data-persisted-revision");
  }, delay, timeout);
  const savedTitle = document.querySelector("[data-document-title]")?.getAttribute("data-document-title");
  if (savedTitle && savedTitle !== title) throw new Error("PACKAGED_UI_DOCUMENT_TITLE_READBACK_MISMATCH");
  return documentId;
}

export function isNewAuthoringDocumentReady(
  root: Element | null,
  previousDocumentId: string | undefined,
): boolean {
  const documentId = root?.getAttribute("data-document-id") ?? "";
  return Boolean(documentId
    && documentId !== previousDocumentId
    && ["Clean", "Saved"].includes(root?.getAttribute("data-cabinet-authoring-state") ?? ""));
}

async function runGraphActionWorkflow(
  document: SmokeDocument,
  generations: Record<"graph" | "canvas" | "assets", number>,
  documentId: string,
  targetDocumentId: string,
  initialEvidenceState: PackagedGraphEvidenceStateValue,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
  setFailureStage: (stage: "graphOpen" | "graphProjection" | "graphLocalEdge" | "graphGlobalEdge" | "graphSafeLabels" | "graphScopeGlobal" | "graphScopeLocal" | "graphDepth" | "graphDirection" | "graphUnresolved" | "graphAssets" | "graphZoomIn" | "graphZoomOut" | "graphFitView" | "graphNode" | "graphDocumentRoute") => void,
): Promise<{ readonly actionCount: number; readonly durableReadbackCount: number; readonly graphLocalEdgeVerified: boolean; readonly graphGlobalEdgeVerified: boolean; readonly graphSafeLabelsVerified: boolean }> {
  let evidenceState = initialEvidenceState;
  setFailureStage("graphOpen");
  await navigateAndWait(document, "graph", generations, delay, timeout);
  setFailureStage("graphProjection");
  evidenceState = transitionPackagedGraphEvidence(evidenceState, "projection_ready");
  if (evidenceState !== PackagedGraphEvidenceState.LocalVerifying) throw new Error("PACKAGED_UI_GRAPH_EVIDENCE_TRANSITION_FAILED");
  let actionCount = 1;
  const localScope = requireButton(document, '[data-action="graph-scope-local"]');
  if (!localScope.classList.contains("active")) {
    setFailureStage("graphScopeLocal");
    actionCount += await clickAndWaitGeneration(document, "graph", "graph-scope-local", generations, delay, timeout);
  }
  setFailureStage("graphLocalEdge");
  await waitForGraphEdge(document, documentId, targetDocumentId, delay, timeout);
  setFailureStage("graphSafeLabels");
  await waitForGraphSafeLabels(document, documentId, targetDocumentId, delay, timeout);
  evidenceState = transitionPackagedGraphEvidence(evidenceState, "local_verified");

  setFailureStage("graphScopeGlobal");
  actionCount += await clickAndWaitGeneration(document, "graph", "graph-scope-global", generations, delay, timeout);
  setFailureStage("graphGlobalEdge");
  await waitForGraphEdge(document, documentId, targetDocumentId, delay, timeout);
  setFailureStage("graphSafeLabels");
  await waitForGraphSafeLabels(document, documentId, targetDocumentId, delay, timeout);
  evidenceState = transitionPackagedGraphEvidence(evidenceState, "global_verified");
  if (evidenceState !== PackagedGraphEvidenceState.Verified) throw new Error("PACKAGED_UI_GRAPH_EVIDENCE_TRANSITION_FAILED");

  setFailureStage("graphScopeLocal");
  actionCount += await clickAndWaitGeneration(document, "graph", "graph-scope-local", generations, delay, timeout);
  for (const [action, stage] of [
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
  return {
    actionCount: actionCount + 6,
    durableReadbackCount: 5,
    graphLocalEdgeVerified: true,
    graphGlobalEdgeVerified: true,
    graphSafeLabelsVerified: true,
  };
}

async function waitForGraphEdge(
  document: SmokeDocument,
  sourceDocumentId: string,
  targetDocumentId: string,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
): Promise<void> {
  await waitForSelector(
    document,
    `[data-edge-kind="document_link"][data-edge-source-id="${sourceDocumentId}"][data-edge-target-id="${targetDocumentId}"]`,
    delay,
    timeout,
  );
}

async function waitForGraphSafeLabels(
  document: SmokeDocument,
  sourceDocumentId: string,
  targetDocumentId: string,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
): Promise<void> {
  await waitUntil(document, () => {
    const source = document.querySelector(`[data-action="select-graph-node"][data-graph-node-id="${sourceDocumentId}"] strong`);
    const target = document.querySelector(`[data-action="select-graph-node"][data-graph-node-id="${targetDocumentId}"] strong`);
    return source?.textContent?.trim() === PACKAGED_UI_SMOKE_DOCUMENT_TITLE
      && target?.textContent?.trim() === PACKAGED_UI_SMOKE_TARGET_TITLE;
  }, delay, timeout);
}

async function runCanvasActionWorkflow(
  document: SmokeDocument,
  generations: Record<"graph" | "canvas" | "assets", number>,
  documentId: string,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
  setFailureStage: (stage: "canvasOpen" | "canvasCreate" | "canvasNote" | "canvasTextEdit" | "canvasPan" | "canvasZoom" | "canvasArrange" | "canvasDocument" | "canvasEdge" | "canvasDrag" | "canvasResize" | "canvasReopen") => void,
): Promise<{ readonly actionCount: number; readonly durableReadbackCount: number; readonly textEditReadbackVerified: true }> {
  setFailureStage("canvasOpen");
  await navigateHome(document, delay, timeout);
  await reopenDocument(document, documentId, delay, timeout);
  await navigateAndWait(document, "canvas", generations, delay, timeout);
  const neededCreate = document.querySelector('[data-action="create-canvas"]') instanceof HTMLElement;
  setFailureStage("canvasCreate");
  await ensureCanvasReady(document, delay, timeout);
  generations.canvas = currentGeneration(document, "canvas");
  let revision = currentCanvasRevision(document);
  setFailureStage("canvasNote");
  click(document, '[data-action="add-canvas-note"]');
  revision = await waitForCanvasRevision(document, revision, delay, timeout);
  const createdNote = requireElement(document, '[data-action="select-canvas-node"][data-canvas-target-kind="text"]');
  const createdNoteId = createdNote.getAttribute("data-canvas-node-id") ?? "";
  if (!createdNoteId) throw new Error("PACKAGED_UI_CANVAS_TEXT_NODE_MISSING");

  setFailureStage("canvasTextEdit");
  const editAction = createdNote.querySelector('[data-action="edit-canvas-text-card"]');
  if (!(editAction instanceof HTMLButtonElement) || editAction.disabled) {
    throw new Error("PACKAGED_UI_CANVAS_TEXT_EDIT_ACTION_MISSING");
  }
  editAction.click();
  await waitForSelector(document, '[data-action="edit-canvas-text"]', delay, timeout);
  const textarea = requireElement(document, '[data-action="edit-canvas-text"]');
  if (!(textarea instanceof HTMLTextAreaElement)) throw new Error("PACKAGED_UI_TEXTAREA_MISSING");
  replaceTextAreaValue(textarea, PACKAGED_UI_SMOKE_CANVAS_TEXT);
  await waitUntil(document, () => {
    const action = document.querySelector('[data-action="confirm-canvas-text-edit"]');
    return action instanceof HTMLButtonElement && !action.disabled;
  }, delay, timeout);
  const confirmEdit = requireButton(document, '[data-action="confirm-canvas-text-edit"]');
  confirmEdit.click();
  revision = await waitForCanvasRevision(document, revision, delay, timeout);
  await waitForCanvasText(document, PACKAGED_UI_SMOKE_CANVAS_TEXT, delay, timeout, createdNoteId);

  for (const [action, stage] of [
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
  requireVisiblePrimaryButton(document, '[data-action="apply-canvas-arrange"]');
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
  await waitForCanvasText(document, PACKAGED_UI_SMOKE_CANVAS_TEXT, delay, timeout, createdNoteId);
  return { actionCount: neededCreate ? 22 : 21, durableReadbackCount: 9, textEditReadbackVerified: true };
}

async function waitForCanvasText(
  document: SmokeDocument,
  expected: string,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
  nodeId?: string,
): Promise<void> {
  const selector = nodeId
    ? `[data-canvas-node-id="${nodeId}"][data-canvas-target-kind="text"] p`
    : '[data-canvas-target-kind="text"] p';
  await waitUntil(document, () => document.querySelector(selector)?.textContent?.trim() === expected, delay, timeout);
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

function requireVisiblePrimaryButton(document: SmokeDocument, selector: string): HTMLButtonElement {
  const button = requireButton(document, selector);
  const bounds = button.getBoundingClientRect();
  const style = getComputedStyle(button);
  if (bounds.width <= 0 || bounds.height <= 0 || style.visibility === "hidden" || style.display === "none"
    || style.opacity === "0" || style.backgroundColor === "transparent" || style.backgroundColor === "rgba(0, 0, 0, 0)") {
    throw new Error("PACKAGED_UI_PRIMARY_ACTION_HIDDEN");
  }
  return button;
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
  setFailureStage: (stage: "assetOpen" | "assetImport" | "assetDetail" | "assetPreview" | "assetUnlink" | "assetLibrary" | "assetDetachedDetail" | "assetRelink" | "assetFilters" | "assetFilterAll" | "assetFilterImage" | "assetFilterPdf" | "assetFilterDocument" | "assetFilterOther") => void,
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
  const filterLabels = Object.freeze({ all: "전체 파일", image: "이미지", pdf: "PDF", document: "문서", other: "기타" } as const);
  for (const filter of packagedAssetFilterVerificationSequence()) {
    const label = filterLabels[filter];
    setFailureStage(({ all: "assetFilterAll", image: "assetFilterImage", pdf: "assetFilterPdf", document: "assetFilterDocument", other: "assetFilterOther" } as const)[filter]);
    const selector = `[data-action="filter-assets-${filter}"]`;
    click(document, selector);
    await waitUntil(document, () => document.querySelector("[data-asset-filter]")?.getAttribute("data-asset-filter") === label
      && document.querySelector(selector)?.getAttribute("aria-pressed") === "true", delay, timeout);
  }
  click(document, '[data-action="select-asset"]');
  await waitForAssetDetail(document, 1, delay, timeout);
  return { assetId, actionCount: 19, durableReadbackCount: 6 };
}

async function runCrossSurfaceWorkflow(
  document: SmokeDocument,
  generations: Record<"graph" | "canvas" | "assets", number>,
  documentId: string,
  targetDocumentId: string,
  assetId: string,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
  setFailureStage: (stage: "canvasAsset" | "canvasAssetRoute" | "assetDocumentRoute" | "graphAttachmentOpen" | "graphAttachmentLocalEdge" | "graphAttachmentLocalFilter" | "graphAttachmentLocalNode" | "graphAttachmentLocalIdentity" | "graphAttachmentLocalLabel" | "graphAttachmentGlobalEdge" | "graphAttachmentRoute") => void,
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

  let graphEvidence: PackagedGraphAttachmentEvidenceStateValue = PackagedGraphAttachmentEvidenceState.LocalVerifying;
  setFailureStage("graphAttachmentOpen");
  await waitForSelector(document, '[data-action="open-authoring-graph"]', delay, timeout);
  click(document, '[data-action="open-authoring-graph"]');
  await waitForNewGeneration(document, "graph", generations, delay, timeout);
  let graphActionCount = 1;
  const assetsToggle = requireButton(document, '[data-action="graph-toggle-assets"]');
  const directionToggle = requireButton(document, '[data-action="graph-toggle-direction"]');
  for (const action of requiredGraphAttachmentResetActions({
    incomingOnlyActive: directionToggle.classList.contains("active"),
    assetsActive: assetsToggle.classList.contains("active"),
  })) {
    graphActionCount += await clickAndWaitGeneration(document, "graph", action, generations, delay, timeout);
  }
  graphActionCount += await recoverGraphIfNeeded(document, generations, delay, timeout);

  setFailureStage("graphAttachmentLocalEdge");
  try {
    await waitForGraphAttachment(document, documentId, assetId, delay, timeout);
  } catch (error) {
    const evidence = graphAttachmentDomEvidence(document, documentId, assetId);
    const reason = classifyGraphAttachmentDomEvidence(evidence);
    if (reason === "filter") setFailureStage("graphAttachmentLocalFilter");
    if (reason === "node") setFailureStage("graphAttachmentLocalNode");
    if (reason === "identity") setFailureStage("graphAttachmentLocalIdentity");
    if (reason === "label") setFailureStage("graphAttachmentLocalLabel");
    throw error;
  }
  graphEvidence = transitionPackagedGraphAttachmentEvidence(graphEvidence, "local_verified");

  graphActionCount += await clickAndWaitGeneration(document, "graph", "graph-scope-global", generations, delay, timeout);
  setFailureStage("graphAttachmentGlobalEdge");
  await waitForGraphAttachment(document, documentId, assetId, delay, timeout);
  graphEvidence = transitionPackagedGraphAttachmentEvidence(graphEvidence, "global_verified");

  setFailureStage("graphAttachmentRoute");
  click(document, `[data-action="select-graph-node"][data-graph-node-id="${assetId}"]`);
  click(document, '.graph-detail [data-action="open-graph-asset"]');
  await waitUntil(document, () => {
    const root = document.querySelector('[data-exploration-surface="assets"]');
    const detail = document.querySelector("[data-asset-detail-state]");
    return root !== null
      && detail?.getAttribute("data-selected-asset-id") === assetId
      && detail.getAttribute("data-asset-detail-state") === "Ready";
  }, delay, timeout);
  graphEvidence = transitionPackagedGraphAttachmentEvidence(graphEvidence, "route_verified");
  if (graphEvidence !== PackagedGraphAttachmentEvidenceState.Verified) {
    throw new Error("PACKAGED_UI_GRAPH_ATTACHMENT_EVIDENCE_TRANSITION_FAILED");
  }
  return { actionCount: 11 + graphActionCount, durableReadbackCount: 7, canvasRevision: revision };
}

async function waitForGraphAttachment(
  document: SmokeDocument,
  documentId: string,
  assetId: string,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
): Promise<void> {
  await waitForSelector(
    document,
    `[data-edge-kind="attachment_reference"][data-edge-source-id="${documentId}"][data-edge-target-id="${assetId}"]`,
    delay,
    timeout,
  );
  await waitUntil(document, () => {
    const label = document.querySelector(`[data-action="select-graph-node"][data-graph-node-id="${assetId}"] strong`)?.textContent?.trim();
    return Boolean(label && label !== assetId);
  }, delay, timeout);
}

async function runBackupRestoreWorkflow(
  invoke: TauriInvoke,
  document: SmokeDocument,
  generations: Record<"graph" | "canvas" | "assets", number>,
  documentId: string,
  targetDocumentId: string,
  assetId: string,
  canvasRevision: number,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
  setFailureStage: (stage: "backupOpen" | "backupCreate" | "restorePreview" | "restoreConfirm" | "restoreReopen" | "restoreHome" | "restoreDocument" | "restoreSearch" | "restoreGraph" | "restoreCanvas" | "restoreAssets") => void,
): Promise<{ readonly actionCount: number; readonly durableReadbackCount: number }> {
  const advance = async (stage: Parameters<typeof setFailureStage>[0]): Promise<void> => {
    setFailureStage(stage);
    await invoke("report_packaged_ui_smoke_progress", { stage });
  };
  await advance("backupOpen");
  await navigateHome(document, delay, timeout);
  click(document, '[data-action="navigate-backup"]');
  await waitUntil(document, () => document.querySelector("[data-backup-state]") !== null, delay, timeout);

  await advance("backupCreate");
  click(document, '[data-action="create-backup"]');
  await waitUntil(document, () => {
    const root = document.querySelector("[data-backup-state]");
    const classes = new Set((root?.getAttribute("data-backup-manifest-classes") ?? "").split(","));
    return root?.getAttribute("data-backup-state") === "Ready"
      && Number(root.getAttribute("data-backup-manifest-entry-count")) > 0
      && Number(root.getAttribute("data-backup-catalog-count")) > 0
      && document.querySelector('[data-action="select-backup-catalog"]') !== null
      && ["current_documents", "canvas_records", "asset_metadata", "asset_objects", "asset_associations"]
        .every((dataClass) => classes.has(dataClass));
  }, delay, timeout);
  click(document, '[data-action="select-backup-catalog"]');

  await advance("restorePreview");
  click(document, '[data-action="preview-backup-restore"]');
  await waitUntil(document, () => document.querySelector("[data-backup-state]")?.getAttribute("data-backup-state") === "AwaitingConfirmation", delay, timeout);

  await advance("restoreConfirm");
  click(document, '[data-action="confirm-backup-restore"]');
  await waitUntil(document, () => {
    const root = document.querySelector("[data-backup-state]");
    return root?.getAttribute("data-backup-state") === "Completed"
      && root.getAttribute("data-restore-operation-state") === "Completed";
  }, delay, timeout);

  await advance("restoreReopen");
  let convergenceActionCount = 0;
  let convergenceReadbackCount = 0;

  await advance("restoreHome");
  await navigateHome(document, delay, timeout);
  convergenceActionCount += 1;
  await waitForSelector(
    document,
    `[data-action="open-recent-document"][data-document-title="${PACKAGED_UI_SMOKE_DOCUMENT_TITLE}"]`,
    delay,
    timeout,
  );
  convergenceReadbackCount += 1;

  await advance("restoreDocument");
  await reopenDocument(document, documentId, delay, timeout);
  convergenceActionCount += 1;
  convergenceReadbackCount += 1;

  await advance("restoreSearch");
  submitPackagedWorkspaceSearch(document, '[data-action="workspace-search-input"]', PACKAGED_UI_SMOKE_DOCUMENT_TITLE);
  await invoke("report_packaged_ui_smoke_progress", { stage: "restoreSearchInput" });
  await delay(packagedControlledInputSettleDelayMs());
  await invoke("report_packaged_ui_smoke_progress", { stage: "restoreSearchSubmitted" });
  convergenceActionCount += 1;
  await waitUntil(document, () => document.querySelector('[data-cabinet-navigator-state="Ready"]') !== null, delay, timeout);
  await invoke("report_packaged_ui_smoke_progress", { stage: "restoreSearchReady" });
  await waitForSelector(document, `[data-action="open-navigator-document"][data-document-id="${documentId}"]`, delay, timeout);
  await invoke("report_packaged_ui_smoke_progress", { stage: "restoreSearchResult" });
  convergenceReadbackCount += 1;
  click(document, `[data-action="open-navigator-document"][data-document-id="${documentId}"]`);
  await waitForAuthoringDocument(document, documentId, delay, timeout);
  await invoke("report_packaged_ui_smoke_progress", { stage: "restoreSearchDocument" });
  convergenceActionCount += 1;
  convergenceReadbackCount += 1;

  await advance("restoreGraph");
  await navigateAndWait(document, "graph", generations, delay, timeout);
  convergenceActionCount += 1;
  const localScope = requireButton(document, '[data-action="graph-scope-local"]');
  if (!localScope.classList.contains("active")) {
    convergenceActionCount += await clickAndWaitGeneration(document, "graph", "graph-scope-local", generations, delay, timeout);
  }
  const directionToggle = requireButton(document, '[data-action="graph-toggle-direction"]');
  const assetsToggle = requireButton(document, '[data-action="graph-toggle-assets"]');
  for (const action of requiredGraphAttachmentResetActions({
    incomingOnlyActive: directionToggle.classList.contains("active"),
    assetsActive: assetsToggle.classList.contains("active"),
  })) {
    convergenceActionCount += await clickAndWaitGeneration(document, "graph", action, generations, delay, timeout);
  }
  convergenceActionCount += await recoverGraphIfNeeded(document, generations, delay, timeout);
  await waitForGraphEdge(document, documentId, targetDocumentId, delay, timeout);
  await waitForGraphSafeLabels(document, documentId, targetDocumentId, delay, timeout);
  await waitForGraphAttachment(document, documentId, assetId, delay, timeout);
  convergenceReadbackCount += 3;
  convergenceActionCount += await clickAndWaitGeneration(document, "graph", "graph-scope-global", generations, delay, timeout);
  await waitForGraphEdge(document, documentId, targetDocumentId, delay, timeout);
  await waitForGraphSafeLabels(document, documentId, targetDocumentId, delay, timeout);
  await waitForGraphAttachment(document, documentId, assetId, delay, timeout);
  convergenceReadbackCount += 3;

  await advance("restoreCanvas");
  await navigateAndWait(document, "canvas", generations, delay, timeout);
  convergenceActionCount += 1;
  await ensureCanvasReady(document, delay, timeout);
  if (currentCanvasRevision(document) < canvasRevision) throw new Error("PACKAGED_UI_RESTORE_CANVAS_STALE");
  await waitForSelector(document, `[data-canvas-target-kind="attachment"][data-canvas-target-id="${assetId}"]`, delay, timeout);
  convergenceReadbackCount += 2;

  await advance("restoreAssets");
  await navigateHome(document, delay, timeout);
  await reopenDocument(document, documentId, delay, timeout);
  click(document, '[data-action="navigate-assets"]');
  convergenceActionCount += 3;
  await waitForNewGeneration(document, "assets", generations, delay, timeout);
  click(document, `[data-action="select-asset"][data-asset-id="${assetId}"]`);
  convergenceActionCount += 1;
  await waitForAssetDetail(document, 1, delay, timeout);
  await waitForSelector(document, `[data-action="open-linked-document"][data-linked-document-id="${documentId}"]`, delay, timeout);
  convergenceReadbackCount += 2;
  return { actionCount: 12 + convergenceActionCount, durableReadbackCount: 8 + convergenceReadbackCount };
}

export function packagedControlledInputSettleDelayMs(): number {
  return 100;
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
  setFailureStage: (stage: "canvasRecoveryOpen" | "canvasRecoveryDetect" | "canvasRecoveryApply") => void,
): Promise<{ readonly actionCount: number; readonly durableReadbackCount: number }> {
  await invoke("corrupt_packaged_ui_smoke_canvas");
  await navigateHome(document, delay, timeout);
  setFailureStage("canvasRecoveryOpen");
  await navigateAndWait(document, "canvas", generations, delay, timeout);
  setFailureStage("canvasRecoveryDetect");
  await waitUntil(document, () => {
    const root = document.querySelector('[data-exploration-surface="canvas"]');
    return root?.getAttribute("data-exploration-state") === "RecoveryRequired"
      && document.querySelector('[data-action="recover-canvas"]') !== null;
  }, delay, timeout);
  click(document, '[data-action="recover-canvas"]');
  setFailureStage("canvasRecoveryApply");
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
  // The route first renders the retained authoring snapshot; wait for the
  // command-backed open to replace it before accepting the durable state.
  await delay(100);
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
  return 1 + await recoverGraphIfNeeded(document, generations, delay, timeout);
}

async function recoverGraphIfNeeded(
  document: SmokeDocument,
  generations: Record<"graph" | "canvas" | "assets", number>,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
): Promise<number> {
  const root = document.querySelector('[data-exploration-surface="graph"]');
  const state = root?.getAttribute("data-exploration-state");
  const recovery = state === "Failed"
    ? document.querySelector('[data-action="reindex-graph"]') ?? document.querySelector('[data-action="retry-graph"]')
    : state === "Stale" ? document.querySelector('[data-action="reindex-graph"]') : null;
  if (state === "Failed" && !(recovery instanceof HTMLElement)) {
    throw new Error("PACKAGED_UI_GRAPH_RECOVERY_MISSING");
  }
  if (!(recovery instanceof HTMLElement)) return 0;
  const previous = generations.graph;
  recovery.click();
  await waitUntil(document, () => {
    const next = document.querySelector('[data-exploration-surface="graph"]');
    const nextState = next?.getAttribute("data-exploration-state");
    const generation = Number(next?.getAttribute("data-exploration-generation"));
    return Boolean(nextState && terminalStates.graph.has(nextState) && generation > previous);
  }, delay, timeout);
  generations.graph = currentGeneration(document, "graph");
  return 1;
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

function isEnabledMode(value: unknown): value is {
  readonly enabled: true;
  readonly stage: "initial" | "upgradeVerification" | "restartVerification" | "visualEvidence";
} {
  if (typeof value !== "object" || value === null) return false;
  const mode = value as { enabled?: unknown; stage?: unknown };
  return mode.enabled === true
    && (mode.stage === "initial"
      || mode.stage === "upgradeVerification"
      || mode.stage === "restartVerification"
      || mode.stage === "visualEvidence");
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
    return isExplorationNavigationReady(
      surface,
      state,
      generation,
      generations[surface],
      surface === "canvas" && document.querySelector('[data-action="create-canvas"]') instanceof HTMLElement,
    );
  }, delay, timeout);
  generations[surface] = currentGeneration(document, surface);
  const root = document.querySelector(`[data-exploration-surface="${surface}"]`);
  if (root?.getAttribute("data-exploration-state") === "Failed" && surface !== "canvas") {
    throw new Error("PACKAGED_UI_ROUTE_FAILED");
  }
}

export function isExplorationNavigationReady(
  surface: "graph" | "canvas" | "assets",
  state: string | null | undefined,
  generation: number,
  previousGeneration: number,
  canvasCreateAvailable: boolean,
): boolean {
  if (!Number.isFinite(generation) || generation <= previousGeneration) return false;
  return Boolean(state && terminalStates[surface].has(state))
    || (surface === "canvas" && canvasCreateAvailable);
}

function currentGeneration(document: SmokeDocument, surface: "graph" | "canvas" | "assets"): number {
  return Number(document.querySelector(`[data-exploration-surface="${surface}"]`)?.getAttribute("data-exploration-generation"));
}

async function collectAndReportPackagedVisualEvidence(
  invoke: TauriInvoke,
  document: Document,
  documentId: string,
  targetDocumentId: string,
  generations: Record<"graph" | "canvas" | "assets", number>,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
): Promise<void> {
  const screenshotDigests: string[] = [];
  const artifactDigestByKey = new Map<string, string>();
  const captureByKey = new Map<string, PackagedVisualCapture>();
  const accessibilityByRoute = new Map<VisualRoute, AccessibilityRouteMeasurement>();
  const port: PackagedVisualCollectionPort = {
    async configureViewport(viewport) {
      await invoke("configure_packaged_ui_smoke_viewport", { request: viewport });
      document.documentElement.style.setProperty(
        "-webkit-text-size-adjust",
        `${viewport.zoomPercent}%`,
      );
      await delay(300);
    },
    async openRoute(route) {
      await openPackagedVisualRoute(route, document, documentId, targetDocumentId, generations, delay, timeout);
      await delay(200);
    },
    async measureViewport(route, viewport) {
      const artifactKey = packagedVisualArtifactKey(route, viewport);
      const rendererBounds = visualRendererBounds(document, route);
      const capture = await invoke("capture_packaged_ui_smoke_window", {
        request: {
          artifactKey,
          sampleBounds: rendererBounds,
          viewportWidth: document.documentElement.clientWidth,
          viewportHeight: document.documentElement.clientHeight,
        },
      });
      if (!isPackagedVisualCapture(capture)) throw new Error("PACKAGED_UI_VISUAL_CAPTURE_INVALID");
      screenshotDigests.push(capture.digest);
      artifactDigestByKey.set(artifactKey, capture.digest);
      captureByKey.set(artifactKey, capture);
      return measurePackagedViewport(document, route, viewport, capture.digest);
    },
    async measureRenderer(route, viewport) {
      const artifactKey = packagedVisualArtifactKey(route, viewport);
      const artifactDigest = artifactDigestByKey.get(artifactKey);
      const capture = captureByKey.get(artifactKey);
      if (!artifactDigest || !capture) throw new Error("PACKAGED_UI_VISUAL_CAPTURE_MISSING");
      return measurePackagedRenderer(document, route, viewport, artifactDigest, capture);
    },
  };
  try {
    const receipts = await collectPackagedVisualReceipts(PACKAGED_VISUAL_POLICY, port);
    const accessibilityViewport = PACKAGED_VISUAL_POLICY.viewports.find(
      (viewport) => viewport.zoomPercent === 200,
    );
    if (!accessibilityViewport) throw new Error("PACKAGED_VISUAL_ACCESSIBILITY_VIEWPORT_MISSING");
    await port.configureViewport(accessibilityViewport);
    for (const route of PACKAGED_VISUAL_POLICY.routes) {
      try {
        await port.openRoute(route);
      } catch (error) {
        throw new PackagedVisualAccessibilityFailure(
          route,
          stableAccessibilityDetailCode(error, "ACCESSIBILITY_ROUTE_OPEN_FAILED"),
        );
      }
      try {
        if (accessibilityByRoute.has(route)) {
          throw new PackagedVisualAccessibilityFailure(route, "ACCESSIBILITY_ROUTE_DUPLICATE");
        }
        const measurement = measurePackagedAccessibilityRoute(
          document as unknown as PackagedAccessibilityDocument,
          route,
        );
        validatePackagedAccessibilityRouteMeasurement(
          measurement,
          packagedUnnamedControlDiagnostic(document as unknown as PackagedAccessibilityDocument),
        );
        accessibilityByRoute.set(route, measurement);
      } catch (error) {
        if (error instanceof PackagedVisualAccessibilityFailure) throw error;
        throw new PackagedVisualAccessibilityFailure(
          route,
          stableAccessibilityDetailCode(error, "ACCESSIBILITY_ROUTE_MEASUREMENT_FAILED"),
        );
      }
    }
    const accessibility = aggregateAccessibilityRouteMeasurements(
      PACKAGED_VISUAL_POLICY.routes.map((route) => {
        const measurement = accessibilityByRoute.get(route);
        if (!measurement) throw new PackagedVisualAccessibilityFailure(route, "ACCESSIBILITY_ROUTE_MISSING");
        return measurement;
      }),
      200,
    );
    try {
      validateAccessibilityMeasurement({
        requiredRouteFocusCount: 6,
        requiredTextZoomPercent: 200,
        minimumKeyboardJourneyCount: 6,
        minimumFocusRestorationCount: 6,
      }, accessibility);
    } catch (error) {
      throw new PackagedVisualAccessibilityFailure(
        "Aggregate",
        stableAccessibilityDetailCode(error, "ACCESSIBILITY_AGGREGATE_VALIDATION_FAILED"),
      );
    }
    const aggregateDigest = await sha256Hex(screenshotDigests.join(""));
    await invoke("report_packaged_ui_smoke_visual_evidence", {
      summary: {
        routeViewportCount: receipts.viewportReceipts.length,
        rendererViewportCount: receipts.rendererReceipts.length,
        artifactCount: receipts.viewportReceipts.length + receipts.rendererReceipts.length,
        screenshotCount: screenshotDigests.length,
        screenshotAggregateDigest: aggregateDigest,
        accessibilityRouteFocusCount: accessibility.routeFocusCount,
        accessibilityKeyboardJourneyCount: accessibility.keyboardJourneyCount,
        accessibilityFocusRestorationCount: accessibility.focusRestorationCount,
        accessibilityVisibleControlCount: accessibility.visibleControlCount,
        accessibilityNamedControlCount: accessibility.namedControlCount,
        accessibilityTextZoomPercent: accessibility.textZoomPercent,
        accessibilityKeyboardErrorCount: accessibility.keyboardErrorCount,
        accessibilityFocusErrorCount: accessibility.focusErrorCount,
        accessibilityInternalExposureCount: accessibility.internalExposureCount,
      },
    });
  } finally {
    document.documentElement.style.removeProperty("-webkit-text-size-adjust");
  }
}

class PackagedVisualAccessibilityFailure extends Error {
  readonly code = "PACKAGED_VISUAL_ACCESSIBILITY_FAILED";
  readonly stage: string;
  readonly detailCode: string;

  constructor(route: VisualRoute | "Aggregate", detailCode: string) {
    super("PACKAGED_VISUAL_ACCESSIBILITY_FAILED");
    this.name = "PackagedVisualAccessibilityFailure";
    this.stage = `accessibility:${route}`;
    this.detailCode = detailCode;
  }
}

function stableAccessibilityDetailCode(error: unknown, fallback: string): string {
  if (error && typeof error === "object" && "code" in error
    && typeof (error as { code?: unknown }).code === "string"
    && /^[A-Z0-9_]+$/.test((error as { code: string }).code)) {
    return (error as { code: string }).code;
  }
  return fallback;
}

async function openPackagedVisualRoute(
  route: VisualRoute,
  document: Document,
  documentId: string,
  targetDocumentId: string,
  generations: Record<"graph" | "canvas" | "assets", number>,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
): Promise<void> {
  if (route === "Home") return navigateHome(document, delay, timeout);
  if (route === "Document") return openPackagedVisualDocumentRoute(document, documentId, delay, timeout);
  if (route === "Backup") {
    click(document, '[data-action="navigate-backup"]');
    return waitForSelector(document, '[data-shell-route="Backup"]', delay, timeout).then(() => undefined);
  }
  const surface = route.toLowerCase() as "graph" | "canvas" | "assets";
  await navigateAndWait(document, surface, generations, delay, timeout);
  if (surface === "canvas") {
    await ensureCanvasReady(document, delay, timeout);
    await waitForSelector(document, ".canvas-stage [data-canvas-node-id]", delay, timeout);
  }
  if (surface === "graph") {
    await waitForSelector(document, '[data-topology-visual-state="Ready"]', delay, timeout);
    const globalScope = requireButton(document, '[data-action="graph-scope-global"]');
    const direction = requireButton(document, '[data-action="graph-toggle-direction"]');
    for (const action of requiredGraphVisualResetActions({
      globalScopeActive: globalScope.classList.contains("active"),
      incomingOnlyActive: direction.classList.contains("active"),
    })) {
      await clickAndWaitGeneration(document, "graph", action, generations, delay, timeout);
    }
    try {
      await waitForGraphEdge(document, documentId, targetDocumentId, delay, timeout);
    } catch {
      if (!document.querySelector('[data-action="select-graph-node"]')) {
        throw new Error("PACKAGED_UI_VISUAL_GRAPH_NODES_MISSING");
      }
      throw new Error("PACKAGED_UI_VISUAL_GRAPH_EDGE_MISSING");
    }
  }
}

async function openPackagedVisualDocumentRoute(
  document: SmokeDocument,
  documentId: string,
  delay: (milliseconds: number) => Promise<void>,
  timeout: number,
): Promise<void> {
  const authoring = document.querySelector("[data-cabinet-authoring-state]");
  if (authoring?.getAttribute("data-document-id") === documentId
    && ["Clean", "Saved"].includes(authoring.getAttribute("data-cabinet-authoring-state") ?? "")) {
    return;
  }
  const shortcutSelector = `[data-action="open-recent-document"][data-document-id="${documentId}"]`;
  const shortcut = document.querySelector(shortcutSelector);
  if (shortcut instanceof HTMLElement && !shortcut.hasAttribute("disabled")) {
    click(document, shortcutSelector);
    await waitForAuthoringDocument(document, documentId, delay, timeout);
    return;
  }
  click(document, '[data-action="navigate-document"]');
  await waitForAuthoringDocument(document, documentId, delay, timeout);
}

export function requiredGraphVisualResetActions(input: Readonly<{
  globalScopeActive: boolean;
  incomingOnlyActive: boolean;
}>): readonly ("graph-scope-global" | "graph-toggle-direction")[] {
  const actions: ("graph-scope-global" | "graph-toggle-direction")[] = [];
  if (input.incomingOnlyActive) actions.push("graph-toggle-direction");
  if (!input.globalScopeActive) actions.push("graph-scope-global");
  return Object.freeze(actions);
}

export function requiredGraphAttachmentResetActions(input: Readonly<{
  incomingOnlyActive: boolean;
  assetsActive: boolean;
}>): readonly ("graph-toggle-direction" | "graph-toggle-assets")[] {
  const actions: ("graph-toggle-direction" | "graph-toggle-assets")[] = [];
  if (input.incomingOnlyActive) actions.push("graph-toggle-direction");
  if (!input.assetsActive) actions.push("graph-toggle-assets");
  return Object.freeze(actions);
}

export function classifyGraphAttachmentDomEvidence(input: Readonly<{
  filtersReady: boolean;
  exactNode: boolean;
  exactEdge: boolean;
  anyAttachmentEdge: boolean;
  safeLabel: boolean;
}>): "filter" | "node" | "identity" | "edge" | "label" | "verified" {
  if (!input.filtersReady) return "filter";
  if (!input.exactNode) return "node";
  if (!input.exactEdge) return input.anyAttachmentEdge ? "identity" : "edge";
  if (!input.safeLabel) return "label";
  return "verified";
}

function graphAttachmentDomEvidence(
  document: SmokeDocument,
  documentId: string,
  assetId: string,
): Readonly<{
  filtersReady: boolean;
  exactNode: boolean;
  exactEdge: boolean;
  anyAttachmentEdge: boolean;
  safeLabel: boolean;
}> {
  const direction = document.querySelector('[data-action="graph-toggle-direction"]');
  const assets = document.querySelector('[data-action="graph-toggle-assets"]');
  const node = document.querySelector(`[data-action="select-graph-node"][data-graph-node-id="${assetId}"]`);
  const exactEdge = document.querySelector(
    `[data-edge-kind="attachment_reference"][data-edge-source-id="${documentId}"][data-edge-target-id="${assetId}"]`,
  );
  const label = node?.querySelector("strong")?.textContent?.trim();
  return Object.freeze({
    filtersReady: !direction?.classList.contains("active") && Boolean(assets?.classList.contains("active")),
    exactNode: node !== null,
    exactEdge: exactEdge !== null,
    anyAttachmentEdge: document.querySelector('[data-edge-kind="attachment_reference"]') !== null,
    safeLabel: Boolean(label && label !== assetId),
  });
}

function measurePackagedViewport(
  document: Document,
  route: VisualRoute,
  viewport: VisualViewport,
  artifactDigest: string,
): ViewportVisualMeasurement {
  const shell = document.querySelector(".desktop-shell");
  if (!(shell instanceof HTMLElement)) throw new Error("PACKAGED_UI_VISUAL_SHELL_MISSING");
  const actionElements = [...document.querySelectorAll<HTMLElement>(
    'button[data-action], input[data-action], select[data-action], [role="button"][data-action]',
  )].filter((element) => isVisibleVisualElement(element)
    && includePackagedViewportAction({
      insideCanvasWorld: element.closest(".canvas-world") !== null,
      insideTopologySemanticList: element.closest(".topology-semantic-list") !== null,
    }));
  return {
    route,
    viewport,
    bodyScrollWidth: document.documentElement.scrollWidth,
    bodyClientWidth: document.documentElement.clientWidth,
    shellBounds: visualRect(shell.getBoundingClientRect()),
    visibleActions: actionElements.map((element, index) => ({
      actionId: `${element.dataset.action ?? "action"}-${index}`,
      bounds: visualRect(element.getBoundingClientRect()),
    })),
    focusTargetCount: actionElements.filter((element) => element.tabIndex >= 0 && !element.hasAttribute("disabled")).length,
    artifactDigest,
  };
}

export function includePackagedViewportAction(input: Readonly<{
  insideCanvasWorld: boolean;
  insideTopologySemanticList: boolean;
}>): boolean {
  return !input.insideCanvasWorld && !input.insideTopologySemanticList;
}

function measurePackagedRenderer(
  document: Document,
  route: "Graph" | "Canvas",
  viewport: VisualViewport,
  artifactDigest: string,
  capture: PackagedVisualCapture,
): RendererVisualMeasurement {
  if (route === "Canvas") {
    const stage = document.querySelector(".canvas-stage");
    if (!(stage instanceof HTMLElement)) throw new Error("PACKAGED_UI_VISUAL_CANVAS_MISSING");
    const stageRect = stage.getBoundingClientRect();
    const nodes = [...stage.querySelectorAll<HTMLElement>("[data-canvas-node-id]")];
    return {
      route,
      viewport,
      canvasBounds: visualRect(stageRect),
      sampledPixelCount: capture.sampledPixelCount,
      nonBackgroundPixelCount: capture.nonBackgroundPixelCount,
      semanticFallbackCount: nodes.length,
      safeLabelsVerified: nodes.every((node) => Boolean(node.textContent?.trim())
        && !node.textContent?.includes(node.dataset.canvasNodeId ?? "\u0000")),
      artifactDigest,
    };
  }

  const host = document.querySelector(".topology-renderer-host");
  if (!(host instanceof HTMLElement)) throw new Error("PACKAGED_UI_VISUAL_GRAPH_MISSING");
  const canvas = host.querySelector("canvas");
  if (!(canvas instanceof HTMLCanvasElement)) throw new Error("PACKAGED_UI_VISUAL_GRAPH_CANVAS_MISSING");
  const semanticNodes = [...document.querySelectorAll<HTMLElement>('[data-action="select-graph-node"]')];
  return {
    route,
    viewport,
    canvasBounds: visualRect(host.getBoundingClientRect()),
    sampledPixelCount: capture.sampledPixelCount,
    nonBackgroundPixelCount: capture.nonBackgroundPixelCount,
    semanticFallbackCount: semanticNodes.length,
    safeLabelsVerified: semanticNodes.every((node) => {
      const label = node.querySelector("strong")?.textContent?.trim() ?? "";
      return Boolean(label) && label !== node.dataset.graphNodeId;
    }),
    artifactDigest,
  };
}

function visualRendererBounds(document: Document, route: VisualRoute): VisualRect | undefined {
  const selector = route === "Graph" ? ".topology-renderer-host" : route === "Canvas" ? ".canvas-stage" : undefined;
  if (!selector) return undefined;
  const element = document.querySelector(selector);
  if (!(element instanceof HTMLElement)) throw new Error("PACKAGED_UI_VISUAL_RENDERER_MISSING");
  return visualRect(element.getBoundingClientRect());
}

function isVisibleVisualElement(element: HTMLElement): boolean {
  const style = globalThis.getComputedStyle(element);
  const rect = element.getBoundingClientRect();
  return style.display !== "none"
    && style.visibility !== "hidden"
    && Number(style.opacity) !== 0
    && rect.width > 0
    && rect.height > 0
    && isInsideClippingAncestors(element, rect);
}

function isInsideClippingAncestors(element: HTMLElement, rect: DOMRect): boolean {
  let ancestor = element.parentElement;
  while (ancestor) {
    const style = globalThis.getComputedStyle(ancestor);
    const overflow = `${style.overflow} ${style.overflowX} ${style.overflowY}`;
    if (/(?:auto|scroll|hidden|clip)/.test(overflow)
      && !isVisualRectFullyContained(visualRect(rect), visualRect(ancestor.getBoundingClientRect()))) {
      return false;
    }
    ancestor = ancestor.parentElement;
  }
  return true;
}

export function isVisualRectFullyContained(inner: VisualRect, outer: VisualRect): boolean {
  const tolerance = 0.5;
  return inner.x >= outer.x - tolerance
    && inner.y >= outer.y - tolerance
    && inner.x + inner.width <= outer.x + outer.width + tolerance
    && inner.y + inner.height <= outer.y + outer.height + tolerance;
}

function visualRect(rect: DOMRect): VisualRect {
  return { x: rect.x, y: rect.y, width: rect.width, height: rect.height };
}

function packagedVisualArtifactKey(route: VisualRoute, viewport: VisualViewport): string {
  return `${route.toLowerCase()}-${viewport.width}-${viewport.height}-${viewport.zoomPercent}`;
}

type PackagedVisualCapture = Readonly<{
  digest: string;
  byteCount: number;
  sampledPixelCount: number;
  nonBackgroundPixelCount: number;
}>;

function isPackagedVisualCapture(value: unknown): value is PackagedVisualCapture {
  if (!value || typeof value !== "object") return false;
  const capture = value as Partial<PackagedVisualCapture>;
  return typeof capture.digest === "string"
    && /^[a-f0-9]{64}$/.test(capture.digest)
    && typeof capture.byteCount === "number"
    && Number.isSafeInteger(capture.byteCount)
    && (capture.byteCount ?? 0) > 0
    && typeof capture.sampledPixelCount === "number"
    && Number.isSafeInteger(capture.sampledPixelCount)
    && (capture.sampledPixelCount ?? -1) >= 0
    && typeof capture.nonBackgroundPixelCount === "number"
    && Number.isSafeInteger(capture.nonBackgroundPixelCount)
    && (capture.nonBackgroundPixelCount ?? -1) >= 0
    && (capture.nonBackgroundPixelCount ?? 0) <= (capture.sampledPixelCount ?? 0);
}

async function sha256Hex(value: string): Promise<string> {
  const digest = await globalThis.crypto.subtle.digest("SHA-256", new TextEncoder().encode(value));
  return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, "0")).join("");
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
  const currentRoute = document.querySelector("[data-shell-route]")?.getAttribute("data-shell-route");
  if (!requiresPackagedHomeNavigation(currentRoute)) {
    await waitForHome(document, delay, timeout);
    return;
  }
  click(document, '[data-action="navigate-home"]');
  await waitForHome(document, delay, timeout);
}

export function requiresPackagedHomeNavigation(currentRoute: string | null): boolean {
  return currentRoute !== "Home";
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

function focusAndClick(document: SmokeDocument, selector: string): void {
  const target = document.querySelector(selector);
  if (!(target instanceof HTMLButtonElement) || target.disabled) throw new Error("PACKAGED_UI_ACTION_MISSING");
  activateButtonByKeyboard(target, () => document.querySelector(":focus") === target);
}

export function activateButtonByKeyboard(
  target: KeyboardActivatable,
  isFocused: () => boolean,
  createEvent: KeyboardEventFactory = (key, options) => new KeyboardEvent("keydown", { ...options, key }),
): void {
  if (target.disabled) throw new Error("PACKAGED_UI_ACTION_MISSING");
  target.focus();
  if (!isFocused()) throw new Error("PACKAGED_UI_FOCUS_FAILED");
  target.dispatchEvent(createEvent("Enter", { bubbles: true, cancelable: true }));
  // Synthetic keyboard events do not trigger the browser's trusted default click.
  target.click();
}

export function dispatchMacSaveShortcut(
  target: EventTarget,
  createEvent: KeyboardEventFactory = (key, options) => new KeyboardEvent("keydown", { ...options, key }),
): void {
  target.dispatchEvent(createEvent("s", {
    bubbles: true,
    cancelable: true,
    metaKey: true,
  }));
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

function submitPackagedWorkspaceSearch(document: SmokeDocument, selector: string, value: string): void {
  const target = document.querySelector(selector);
  if (!(target instanceof HTMLInputElement) || target.hasAttribute("disabled")) {
    throw new Error("PACKAGED_UI_INPUT_MISSING");
  }
  const form = target.closest("form");
  if (!(form instanceof HTMLFormElement)) {
    throw new Error("PACKAGED_UI_SEARCH_FORM_MISSING");
  }
  const submit = form.querySelector('[data-action="submit-workspace-search"]');
  if (!(submit instanceof HTMLButtonElement) || submit.disabled) {
    throw new Error("PACKAGED_UI_SEARCH_SUBMIT_MISSING");
  }
  const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set;
  if (!setter) throw new Error("PACKAGED_UI_INPUT_UNAVAILABLE");
  setter.call(target, value);
  target.dispatchEvent(new Event("input", { bubbles: true }));
  form.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
}

export function replaceTextAreaValue(
  target: HTMLTextAreaElement,
  value: string,
  setter = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set,
): void {
  if (!setter) throw new Error("PACKAGED_UI_TEXTAREA_UNAVAILABLE");
  setter.call(target, value);
  target.dispatchEvent(new Event("input", { bubbles: true }));
}

function selectFirstCanvasCatalogEntry(document: SmokeDocument): void {
  const picker = document.querySelector('[data-action="select-canvas-catalog"]');
  if (!(picker instanceof HTMLSelectElement) || picker.options.length === 0) {
    throw new Error("PACKAGED_UI_CANVAS_CATALOG_ENTRY_MISSING");
  }
  const value = picker.options[0]?.value.trim() ?? "";
  const setter = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value")?.set;
  if (!value || !setter) throw new Error("PACKAGED_UI_CANVAS_CATALOG_ENTRY_INVALID");
  setter.call(picker, value);
  picker.dispatchEvent(new Event("change", { bubbles: true }));
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
