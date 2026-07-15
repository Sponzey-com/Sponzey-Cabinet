const requiredAuthoringViewports = Object.freeze([
  [1024, 700],
  [1280, 800],
]);

export const Phase011AuthoringBrowserState = Object.freeze({
  Pending: "Pending",
  Serving: "Serving",
  Launching: "Launching",
  Injecting: "Injecting",
  Navigating: "Navigating",
  Interacting: "Interacting",
  Capturing: "Capturing",
  Passed: "Passed",
  Failed: "Failed",
});

const transitions = Object.freeze({
  Pending: { Serve: "Serving" },
  Serving: { Launch: "Launching" },
  Launching: { Inject: "Injecting" },
  Injecting: { Navigate: "Navigating" },
  Navigating: { Interact: "Interacting" },
  Interacting: { Capture: "Capturing" },
  Capturing: { Pass: "Passed" },
});

export function transitionPhase011AuthoringBrowserState(state, event) {
  return { state: transitions[state]?.[event] ?? Phase011AuthoringBrowserState.Failed };
}

export function validatePhase011AuthoringBrowserReport(report, expectedSourceFingerprint) {
  const findingIds = [];
  if (report?.marker !== "phase011_authoring_browser=passed") findingIds.push("marker");
  if (!/^[a-f0-9]{64}$/.test(report?.sourceFingerprint ?? "")) findingIds.push("source_fingerprint");
  if (expectedSourceFingerprint && report?.sourceFingerprint !== expectedSourceFingerprint) {
    findingIds.push("stale_source_fingerprint");
  }
  if (report?.browserSurface !== "local_chrome_cdp") findingIds.push("browser_surface");
  if (containsAuthoringSensitiveData(JSON.stringify(report ?? {}))) findingIds.push("sensitive_data");

  const interactions = report?.interactions ?? {};
  for (const key of [
    "documentOpened",
    "codeMirrorMounted",
    "createdDocumentOpened",
    "sourceMode",
    "splitMode",
    "previewMode",
    "previewTableRendered",
    "keyboardSave",
    "closeBlocked",
    "closeCancel",
    "closeRetrySave",
    "closeDiscard",
    "historyLoaded",
    "restorePreviewReady",
    "rawBodyExcluded",
    "rawPathExcluded",
  ]) {
    if (interactions[key] !== true) findingIds.push(`interaction_${key}`);
  }
  if (!(interactions.manualSaveCount >= 1)) findingIds.push("manual_save_count");
  if (!(interactions.autosaveCount >= 1)) findingIds.push("autosave_count");
  if (!(interactions.createDocumentCount >= 1)) findingIds.push("create_document_count");
  if (!(interactions.restoreApplyCount >= 1)) findingIds.push("restore_apply_count");

  const runs = Array.isArray(report?.runs) ? report.runs : [];
  for (const [width, height] of requiredAuthoringViewports) {
    const run = runs.find((candidate) => candidate.width === width && candidate.height === height);
    if (!run) {
      findingIds.push(`viewport_${width}x${height}`);
      continue;
    }
    if (!run.readyState) findingIds.push(`ready_${width}x${height}`);
    if (!run.codeMirrorMounted) findingIds.push(`codemirror_${width}x${height}`);
    if (!run.previewTableRendered) findingIds.push(`preview_table_${width}x${height}`);
    if (!(run.nonBlankPixelCount >= 10000)) findingIds.push(`pixels_${width}x${height}`);
    if (run.overlapCount !== 0) findingIds.push(`overlap_${width}x${height}`);
    if (run.horizontalOverflow) findingIds.push(`overflow_${width}x${height}`);
    if (!run.focusVisible) findingIds.push(`focus_${width}x${height}`);
    if (typeof run.screenshot !== "string" || !run.screenshot.endsWith(".png")) {
      findingIds.push(`screenshot_${width}x${height}`);
    }
  }

  return { passed: findingIds.length === 0, findingIds };
}

export function containsAuthoringSensitiveData(text) {
  return [
    "/Users/",
    "C:\\Users\\",
    "raw markdown body",
    "Secret Browser Body",
    "notes/private.md",
    "provider_api_key",
    "sessionToken",
  ].some((token) => text.includes(token));
}

export function authoringBrowserViewports() {
  return requiredAuthoringViewports.map(([width, height]) => ({ width, height }));
}
