const viewports = Object.freeze([
  Object.freeze({ width: 1280, height: 800 }),
  Object.freeze({ width: 1440, height: 900 }),
  Object.freeze({ width: 1728, height: 1117 }),
]);
const surfaces = Object.freeze(["graph", "canvas", "assets"]);
const transitions = Object.freeze({
  Pending: Object.freeze({ Serve: "Serving" }),
  Serving: Object.freeze({ Browse: "Browsing" }),
  Browsing: Object.freeze({ Capture: "Capturing" }),
  Capturing: Object.freeze({ Pass: "Passed" }),
});

export function transitionExplorationVisualState(state, event) {
  return Object.freeze({ state: transitions[state]?.[event] ?? "Failed" });
}

export function explorationVisualViewports() {
  return viewports.map((viewport) => ({ ...viewport }));
}

export function validateExplorationVisualReport(report, expectedSourceFingerprint) {
  const findingIds = [];
  if (report?.marker !== "phase012_exploration_visual=passed") findingIds.push("marker");
  if (!/^[a-f0-9]{64}$/.test(report?.sourceFingerprint ?? "")) findingIds.push("source_fingerprint");
  if (expectedSourceFingerprint && report?.sourceFingerprint !== expectedSourceFingerprint) findingIds.push("stale_source_fingerprint");
  if (containsSensitiveData(JSON.stringify(report ?? {}))) findingIds.push("sensitive_data");

  const interactions = report?.interactions ?? {};
  if (interactions.graphOpenedDocumentId !== "doc-001") findingIds.push("graph_route_identity");
  if (interactions.canvasOpenedDocumentId !== "doc-001") findingIds.push("canvas_route_identity");
  if (interactions.assetOpenedDocumentId !== "doc-001") findingIds.push("asset_route_identity");
  if (interactions.canvasKeyboardSelection !== true) findingIds.push("canvas_keyboard_selection");

  const runs = Array.isArray(report?.runs) ? report.runs : [];
  for (const viewport of viewports) {
    for (const surface of surfaces) {
      const suffix = `${surface}_${viewport.width}x${viewport.height}`;
      const run = runs.find((candidate) => candidate.surface === surface && candidate.width === viewport.width && candidate.height === viewport.height);
      if (!run) {
        findingIds.push(`viewport_${suffix}`);
        continue;
      }
      if (!run.readyState) findingIds.push(`ready_${suffix}`);
      if (!(run.nonBlankPixelCount >= 10_000)) findingIds.push(`pixels_${suffix}`);
      if (run.overlapCount !== 0) findingIds.push(`overlap_${suffix}`);
      if (run.horizontalOverflow) findingIds.push(`overflow_${suffix}`);
      if (run.clippedControlCount !== 0) findingIds.push(`clipped_${suffix}`);
      if (!run.focusVisible) findingIds.push(`focus_${suffix}`);
      if (!run.navLandmark || !run.mainLandmark) findingIds.push(`landmarks_${suffix}`);
      if (run.sensitiveDataExcluded !== true) findingIds.push(`sensitive_data_${suffix}`);
      if (typeof run.screenshot !== "string" || !run.screenshot.endsWith(".png")) findingIds.push(`screenshot_${suffix}`);
    }
  }
  return Object.freeze({ passed: findingIds.length === 0, findingIds: Object.freeze(findingIds) });
}

function containsSensitiveData(text) {
  return ["/Users/", "C:\\Users\\", "raw document body", "provider_api_key", "sessionToken", "source.md"].some((token) => text.includes(token));
}
