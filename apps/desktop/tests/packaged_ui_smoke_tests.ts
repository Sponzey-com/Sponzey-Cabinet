import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  PACKAGED_UI_SMOKE_DOCUMENT_TITLE,
  PackagedGraphAttachmentEvidenceState,
  PackagedGraphEvidenceState,
  PackagedUiSmokeState,
  activateButtonByKeyboard,
  classifyGraphAttachmentDomEvidence,
  classifyPackagedVisualFailure,
  dispatchMacSaveShortcut,
  includePackagedViewportAction,
  packagedControlledInputSettleDelayMs,
  isVisualRectFullyContained,
  isNewAuthoringDocumentReady,
  isExplorationNavigationReady,
  nearestRankP95,
  packagedAssetFilterVerificationSequence,
  packagedRestoreConvergenceSurfaces,
  replaceTextAreaValue,
  requiresPackagedHomeNavigation,
  requiredGraphAttachmentResetActions,
  requiredGraphVisualResetActions,
  runPackagedUiSmoke,
  runPackagedUiSmokeRestart,
  transitionPackagedUiSmoke,
  transitionPackagedGraphAttachmentEvidence,
  transitionPackagedGraphEvidence,
  validatePackagedAccessibilityRouteMeasurement,
  waitForSelector,
} from "../src/packaged_ui_smoke.ts";
import {
  CODEMIRROR_REPLACE_DOCUMENT_EVENT,
  requestCodeMirrorDocumentReplacement,
} from "../src/codemirror_document_editor.ts";

const packagedUiSmokeSource = await readFile(new URL("../src/packaged_ui_smoke.ts", import.meta.url), "utf8");

test("packaged asset filter verification restores the neutral all filter", () => {
  const sequence = packagedAssetFilterVerificationSequence();

  assert.deepEqual(sequence, ["all", "image", "pdf", "document", "other", "all"]);
  assert.equal(sequence.at(-1), "all");
});

test("packaged restore convergence covers every user-facing data surface", () => {
  assert.deepEqual(
    packagedRestoreConvergenceSurfaces(),
    ["Home", "Document", "Search", "Graph", "Canvas", "Assets"],
  );
});

test("packaged visual failure preserves stable collection detail without raw errors", () => {
  assert.deepEqual(classifyPackagedVisualFailure({
    code: "PACKAGED_VISUAL_ROUTE_FAILED",
    stage: "Graph:1440x900@100",
    detailCode: "PACKAGED_UI_ROUTE_FAILED",
  }), {
    errorCode: "PACKAGED_VISUAL_ROUTE_FAILED",
    failureStage: "Graph:1440x900@100:PACKAGED_UI_ROUTE_FAILED",
  });
  assert.deepEqual(classifyPackagedVisualFailure(new Error("raw user data")), {
    errorCode: "PACKAGED_VISUAL_UNCLASSIFIED_FAILURE",
    failureStage: "collection",
  });
  assert.deepEqual(classifyPackagedVisualFailure({ code: "ACCESSIBILITY_CONTROL_NAME_INCOMPLETE" }), {
    errorCode: "PACKAGED_VISUAL_ACCESSIBILITY_FAILED",
    failureStage: "accessibility:ACCESSIBILITY_CONTROL_NAME_INCOMPLETE",
  });
});

test("packaged accessibility route validation reports only route and stable detail", () => {
  assert.throws(() => validatePackagedAccessibilityRouteMeasurement({
    route: "Assets",
    visibleControlCount: 2,
    namedControlCount: 1,
    mainFocusReached: true,
    keyboardJourneyPassed: true,
    focusRestorationCount: 1,
    keyboardErrorCount: 0,
    focusErrorCount: 0,
    internalExposureCount: 0,
  }), (error) => {
    assert.deepEqual(error, {
      code: "PACKAGED_VISUAL_ACCESSIBILITY_FAILED",
      stage: "accessibility:Assets",
      detailCode: "ACCESSIBILITY_CONTROL_NAME_INCOMPLETE",
    });
    return true;
  });
});

test("packaged visual accessibility failures are classified without raw error messages", () => {
  assert.deepEqual(classifyPackagedVisualFailure({
    code: "PACKAGED_VISUAL_ACCESSIBILITY_FAILED",
    stage: "accessibility:Assets",
    detailCode: "ACCESSIBILITY_ROUTE_MEASUREMENT_FAILED",
  }), {
    errorCode: "PACKAGED_VISUAL_ACCESSIBILITY_FAILED",
    failureStage: "accessibility:Assets:ACCESSIBILITY_ROUTE_MEASUREMENT_FAILED",
  });
});

test("packaged visual collection does not click the disabled active Home route", () => {
  assert.equal(requiresPackagedHomeNavigation("Home"), false);
  assert.equal(requiresPackagedHomeNavigation("Graph"), true);
  assert.equal(requiresPackagedHomeNavigation(null), true);
});

test("fresh Canvas navigation accepts an explicit create action after catalog load", () => {
  assert.equal(isExplorationNavigationReady("canvas", "Idle", 1, -1, true), true);
  assert.equal(isExplorationNavigationReady("canvas", "Idle", 1, -1, false), false);
  assert.equal(isExplorationNavigationReady("graph", "Empty", 1, -1, false), true);
  assert.equal(isExplorationNavigationReady("canvas", "Ready", 1, 1, false), false);
});

test("viewport chrome evidence excludes renderer-owned semantic actions", () => {
  assert.equal(includePackagedViewportAction({ insideCanvasWorld: false, insideTopologySemanticList: false }), true);
  assert.equal(includePackagedViewportAction({ insideCanvasWorld: true, insideTopologySemanticList: false }), false);
  assert.equal(includePackagedViewportAction({ insideCanvasWorld: false, insideTopologySemanticList: true }), false);
});

test("packaged search submission yields for the controlled input state commit", () => {
  assert.equal(packagedControlledInputSettleDelayMs(), 100);
});

test("packaged restore search submits the form that owns the typed workspace input", () => {
  assert.match(packagedUiSmokeSource, /submitPackagedWorkspaceSearch\(document,\s*'\[data-action="workspace-search-input"\]',\s*PACKAGED_UI_SMOKE_DOCUMENT_TITLE\)/);
  assert.doesNotMatch(packagedUiSmokeSource, /typeTextInput\(document,\s*'\[data-action="workspace-search-input"\]'[\s\S]{0,180}click\(document,\s*'\[data-action="submit-workspace-search"\]'\)/);
});

test("packaged visual Document route can use the persistent document menu fallback", () => {
  assert.match(packagedUiSmokeSource, /openPackagedVisualDocumentRoute\(document,\s*documentId,\s*delay,\s*timeout\)/);
  assert.match(packagedUiSmokeSource, /\[data-action="navigate-document"\]/);
});

test("visual action collection excludes controls outside a clipping ancestor", () => {
  assert.equal(isVisualRectFullyContained(
    { x: 10, y: 10, width: 20, height: 20 },
    { x: 0, y: 0, width: 40, height: 40 },
  ), true);
  assert.equal(isVisualRectFullyContained(
    { x: 10, y: 30, width: 20, height: 20 },
    { x: 0, y: 0, width: 40, height: 40 },
  ), false);
});

test("packaged document input crosses the CodeMirror transaction event boundary", () => {
  const target = new EventTarget();
  let received: unknown;
  target.addEventListener(CODEMIRROR_REPLACE_DOCUMENT_EVENT, (event) => {
    received = (event as Event & { detail?: unknown }).detail;
  });

  const dispatched = requestCodeMirrorDocumentReplacement(
    target,
    "# Durable packaged content",
    (detail) => Object.assign(new Event(CODEMIRROR_REPLACE_DOCUMENT_EVENT), { detail }),
  );

  assert.equal(dispatched, true);
  assert.deepEqual(received, { body: "# Durable packaged content" });
});

test("packaged keyboard helpers require focus and emit the macOS save shortcut", () => {
  const events: Array<{ readonly key: string; readonly metaKey: boolean }> = [];
  let focused = false;
  let clicked = 0;
  const createEvent = (key: string, options: KeyboardEventInit) => Object.assign(
    new Event("keydown", { bubbles: options.bubbles, cancelable: options.cancelable }),
    { key, metaKey: options.metaKey ?? false },
  );
  const button = {
    disabled: false,
    focus() { focused = true; },
    click() { clicked += 1; },
    dispatchEvent(event: Event) {
      events.push({ key: (event as KeyboardEvent).key, metaKey: (event as KeyboardEvent).metaKey });
      return true;
    },
  };
  activateButtonByKeyboard(button, () => focused, createEvent);
  const shortcutTarget = new EventTarget();
  shortcutTarget.addEventListener("keydown", (event) => {
    events.push({ key: (event as KeyboardEvent).key, metaKey: (event as KeyboardEvent).metaKey });
  });
  dispatchMacSaveShortcut(shortcutTarget, createEvent);
  assert.equal(clicked, 1);
  assert.deepEqual(events, [
    { key: "Enter", metaKey: false },
    { key: "s", metaKey: true },
  ]);

  assert.throws(
    () => activateButtonByKeyboard(button, () => false, createEvent),
    /PACKAGED_UI_FOCUS_FAILED/,
  );
});

test("packaged canvas text input uses the controlled textarea input boundary", () => {
  const target = new EventTarget() as EventTarget & { value: string };
  target.value = "old";
  let inputEvents = 0;
  target.addEventListener("input", () => { inputEvents += 1; });

  replaceTextAreaValue(
    target as unknown as HTMLTextAreaElement,
    "패키지 캔버스 메모 내구성 검증",
    function setValue(this: { value: string }, value: string) { this.value = value; },
  );

  assert.equal(target.value, "패키지 캔버스 메모 내구성 검증");
  assert.equal(inputEvents, 1);
});

test("document reopen waits for the asynchronously refreshed recent-document action", async () => {
  let attempts = 0;
  await waitForSelector(
    {
      querySelector(selector) {
        assert.equal(selector, '[data-action="open-recent-document"][data-document-id="doc-1"]');
        attempts += 1;
        return attempts >= 3 ? ({} as Element) : null;
      },
    },
    '[data-action="open-recent-document"][data-document-id="doc-1"]',
    async () => undefined,
    100,
  );
  assert.equal(attempts, 3);
});

test("second packaged document waits for a different authoring identity", () => {
  const root = {
    getAttribute(name: string) {
      return name === "data-document-id" ? "doc-target" : "Saved";
    },
  } as unknown as Element;
  assert.equal(isNewAuthoringDocumentReady(root, undefined), true);
  assert.equal(isNewAuthoringDocumentReady(root, "doc-target"), false);
});

test("packaged smoke follows the explicit terminal state sequence", () => {
  let state = PackagedUiSmokeState.Booting;
  state = transitionPackagedUiSmoke(state, "home_ready");
  assert.equal(state, PackagedUiSmokeState.HomeReady);
  state = transitionPackagedUiSmoke(state, "document_saved");
  assert.equal(state, PackagedUiSmokeState.DocumentSaved);
  state = transitionPackagedUiSmoke(state, "document_reopened");
  assert.equal(state, PackagedUiSmokeState.DocumentReopened);
  state = transitionPackagedUiSmoke(state, "document_version_verified");
  assert.equal(state, PackagedUiSmokeState.DocumentVersionWorkflowVerified);
  state = transitionPackagedUiSmoke(state, "graph_actions_verified");
  assert.equal(state, PackagedUiSmokeState.GraphActionsVerified);
  state = transitionPackagedUiSmoke(state, "canvas_mutations_verified");
  assert.equal(state, PackagedUiSmokeState.CanvasMutationsVerified);
  state = transitionPackagedUiSmoke(state, "asset_actions_verified");
  assert.equal(state, PackagedUiSmokeState.AssetActionsVerified);
  state = transitionPackagedUiSmoke(state, "document_attachment_verified");
  assert.equal(state, PackagedUiSmokeState.DocumentAttachmentWorkflowVerified);
  state = transitionPackagedUiSmoke(state, "cross_surface_verified");
  assert.equal(state, PackagedUiSmokeState.CrossSurfaceVerified);
  state = transitionPackagedUiSmoke(state, "backup_restore_verified");
  assert.equal(state, PackagedUiSmokeState.BackupRestoreVerified);
  state = transitionPackagedUiSmoke(state, "canvas_lifecycle_verified");
  assert.equal(state, PackagedUiSmokeState.CanvasLifecycleVerified);
  state = transitionPackagedUiSmoke(state, "canvas_recovery_verified");
  assert.equal(state, PackagedUiSmokeState.CanvasRecoveryVerified);
  state = transitionPackagedUiSmoke(state, "visual_evidence_verified");
  assert.equal(state, PackagedUiSmokeState.VisualEvidenceVerified);
  state = transitionPackagedUiSmoke(state, "route_ready");
  assert.equal(state, PackagedUiSmokeState.RoutesMeasured);
  state = transitionPackagedUiSmoke(state, "samples_ready");
  assert.equal(state, PackagedUiSmokeState.NativeReadsMeasured);
  state = transitionPackagedUiSmoke(state, "report_ready");
  assert.equal(state, PackagedUiSmokeState.Reporting);
  state = transitionPackagedUiSmoke(state, "reported");
  assert.equal(state, PackagedUiSmokeState.Passed);
});

test("invalid transitions and failures terminate without hidden flags", () => {
  assert.equal(
    transitionPackagedUiSmoke(PackagedUiSmokeState.Booting, "samples_ready"),
    PackagedUiSmokeState.Failed,
  );
  assert.equal(
    transitionPackagedUiSmoke(PackagedUiSmokeState.HomeReady, "failed"),
    PackagedUiSmokeState.Failed,
  );
});

test("packaged graph evidence enforces save, projection, local, and global order", () => {
  let state = PackagedGraphEvidenceState.TargetSaving;
  state = transitionPackagedGraphEvidence(state, "target_saved");
  assert.equal(state, PackagedGraphEvidenceState.SourceSaving);
  state = transitionPackagedGraphEvidence(state, "source_saved");
  assert.equal(state, PackagedGraphEvidenceState.ProjectionWaiting);
  state = transitionPackagedGraphEvidence(state, "projection_ready");
  assert.equal(state, PackagedGraphEvidenceState.LocalVerifying);
  state = transitionPackagedGraphEvidence(state, "local_verified");
  assert.equal(state, PackagedGraphEvidenceState.GlobalVerifying);
  state = transitionPackagedGraphEvidence(state, "global_verified");
  assert.equal(state, PackagedGraphEvidenceState.Verified);
  assert.equal(
    transitionPackagedGraphEvidence(PackagedGraphEvidenceState.TargetSaving, "global_verified"),
    PackagedGraphEvidenceState.Failed,
  );
});

test("packaged graph attachment evidence requires local global and exact asset route order", () => {
  let state = PackagedGraphAttachmentEvidenceState.LocalVerifying;
  state = transitionPackagedGraphAttachmentEvidence(state, "local_verified");
  assert.equal(state, PackagedGraphAttachmentEvidenceState.GlobalVerifying);
  state = transitionPackagedGraphAttachmentEvidence(state, "global_verified");
  assert.equal(state, PackagedGraphAttachmentEvidenceState.RouteVerifying);
  state = transitionPackagedGraphAttachmentEvidence(state, "route_verified");
  assert.equal(state, PackagedGraphAttachmentEvidenceState.Verified);
  assert.equal(
    transitionPackagedGraphAttachmentEvidence(PackagedGraphAttachmentEvidenceState.LocalVerifying, "route_verified"),
    PackagedGraphAttachmentEvidenceState.Failed,
  );
});

test("packaged visual graph resets only query controls that hide the representative relation", () => {
  assert.deepEqual(requiredGraphVisualResetActions({
    globalScopeActive: false,
    incomingOnlyActive: true,
  }), ["graph-toggle-direction", "graph-scope-global"]);
  assert.deepEqual(requiredGraphVisualResetActions({
    globalScopeActive: true,
    incomingOnlyActive: false,
  }), []);
});

test("packaged attachment graph resets outgoing direction and enables assets without changing scope", () => {
  assert.deepEqual(requiredGraphAttachmentResetActions({
    incomingOnlyActive: true,
    assetsActive: false,
  }), ["graph-toggle-direction", "graph-toggle-assets"]);
  assert.deepEqual(requiredGraphAttachmentResetActions({
    incomingOnlyActive: false,
    assetsActive: true,
  }), []);
});

test("packaged attachment graph failure classification is structural and deterministic", () => {
  assert.equal(classifyGraphAttachmentDomEvidence({
    filtersReady: false,
    exactNode: false,
    exactEdge: false,
    anyAttachmentEdge: false,
    safeLabel: false,
  }), "filter");
  assert.equal(classifyGraphAttachmentDomEvidence({ filtersReady: true, exactNode: false, exactEdge: false, anyAttachmentEdge: false, safeLabel: false }), "node");
  assert.equal(classifyGraphAttachmentDomEvidence({ filtersReady: true, exactNode: true, exactEdge: false, anyAttachmentEdge: true, safeLabel: true }), "identity");
  assert.equal(classifyGraphAttachmentDomEvidence({ filtersReady: true, exactNode: true, exactEdge: true, anyAttachmentEdge: true, safeLabel: false }), "label");
  assert.equal(classifyGraphAttachmentDomEvidence({ filtersReady: true, exactNode: true, exactEdge: true, anyAttachmentEdge: true, safeLabel: true }), "verified");
});

test("nearest-rank p95 is deterministic for the bounded sample set", () => {
  const samples = Array.from({ length: 200 }, (_, index) => index + 1);
  assert.equal(nearestRankP95(samples), 190);
  assert.equal(nearestRankP95([]), 0);
});

test("normal desktop mode performs no UI automation", async () => {
  let queried = false;
  const state = await runPackagedUiSmoke({
    async invoke(command) {
      assert.equal(command, "get_packaged_ui_smoke_mode");
      return { enabled: false };
    },
    document: {
      querySelector() {
        queried = true;
        return null;
      },
    },
  });
  assert.equal(state, PackagedUiSmokeState.Disabled);
  assert.equal(queried, false);
});

test("restart mode fails closed when durable attachment readback is unavailable", async () => {
  const reports: unknown[] = [];
  const state = await runPackagedUiSmoke({
    async invoke(command, payload) {
      if (command === "get_packaged_ui_smoke_mode") {
        return { enabled: true, stage: "restartVerification" };
      }
      assert.equal(command, "complete_packaged_ui_smoke_restart");
      reports.push(payload);
      return undefined;
    },
    document: { querySelector: () => null },
    delay: async () => undefined,
    markerTimeoutMs: 1,
  });

  assert.equal(state, PackagedUiSmokeState.Failed);
  assert.deepEqual(reports, [{
    report: {
      attachmentRestartReadbackVerified: false,
      canvasTextRestartReadbackVerified: false,
      errorCount: 1,
      failureStage: "home",
    },
  }]);
});

test("restart verification selects the durable attachment document by its visible title", async () => {
  const selectors: string[] = [];
  await runPackagedUiSmokeRestart({
    async invoke() { return undefined; },
    document: {
      querySelector(selector) {
        selectors.push(selector);
        if (selector === "[data-cabinet-home-state]") {
          return { getAttribute: () => "Ready" } as unknown as Element;
        }
        return null;
      },
    },
    delay: async () => undefined,
    markerTimeoutMs: 1,
  });

  assert.ok(selectors.includes(
    `[data-action="open-recent-document"][data-document-title="${PACKAGED_UI_SMOKE_DOCUMENT_TITLE}"]`,
  ));
  assert.equal(selectors.includes('[data-action="open-recent-document"]'), false);
});

test("restart verification does not require background WebView focus for durable tab readback", async () => {
  const source = await import("node:fs/promises").then((fs) =>
    fs.readFile(new URL("../src/packaged_ui_smoke.ts", import.meta.url), "utf8")
  );

  assert.match(
    source,
    /openDocumentInspectorTab\(options\.document, "attachments", delay, timeout, "pointer"\)/,
  );
  assert.match(source, /activation: "keyboard" \| "pointer" = "keyboard"/);
});
