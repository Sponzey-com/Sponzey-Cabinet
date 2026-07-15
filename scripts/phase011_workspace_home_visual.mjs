const requiredViewports = Object.freeze([
  [1024, 700],
  [1280, 800],
  [1440, 900],
  [1920, 1080],
]);

export const WorkspaceHomeVisualState = Object.freeze({
  Pending: "Pending",
  Serving: "Serving",
  Launching: "Launching",
  Injecting: "Injecting",
  Navigating: "Navigating",
  Validating: "Validating",
  Capturing: "Capturing",
  Passed: "Passed",
  Failed: "Failed",
});

const transitions = Object.freeze({
  Pending: { Serve: "Serving" },
  Serving: { Launch: "Launching" },
  Launching: { Inject: "Injecting" },
  Injecting: { Navigate: "Navigating" },
  Navigating: { Validate: "Validating" },
  Validating: { Capture: "Capturing" },
  Capturing: { Pass: "Passed" },
});

export function transitionWorkspaceHomeVisualState(state, event) {
  return { state: transitions[state]?.[event] ?? WorkspaceHomeVisualState.Failed };
}

export function validateWorkspaceHomeVisualReport(report, expectedSourceFingerprint) {
  const findingIds = [];
  if (report?.marker !== "phase011_workspace_home_visual=passed") findingIds.push("marker");
  if (!/^[a-f0-9]{64}$/.test(report?.sourceFingerprint ?? "")) findingIds.push("source_fingerprint");
  if (expectedSourceFingerprint && report?.sourceFingerprint !== expectedSourceFingerprint) {
    findingIds.push("stale_source_fingerprint");
  }
  if (containsSensitiveData(JSON.stringify(report ?? {}))) findingIds.push("sensitive_data");
  if (report?.retryKeyboardFlow !== true) findingIds.push("retry_keyboard_flow");
  for (const key of ["fiveViews", "filterEmpty", "retryKeyboardFlow", "homeReturn"]) {
    if (report?.navigatorInteractions?.[key] !== true) {
      findingIds.push(`navigator_${key}`);
    }
  }

  const runs = Array.isArray(report?.runs) ? report.runs : [];
  for (const [width, height] of requiredViewports) {
    const run = runs.find((candidate) => candidate.width === width && candidate.height === height);
    if (!run) {
      findingIds.push(`viewport_${width}x${height}`);
      continue;
    }
    if (!run.readyState) findingIds.push(`ready_${width}x${height}`);
    if (!(run.nonBlankPixelCount >= 10000)) findingIds.push(`pixels_${width}x${height}`);
    if (run.overlapCount !== 0) findingIds.push(`overlap_${width}x${height}`);
    if (run.horizontalOverflow) findingIds.push(`overflow_${width}x${height}`);
    if (!run.focusVisible) findingIds.push(`focus_${width}x${height}`);
    if (!run.navLandmark || !run.mainLandmark || !run.liveRegion) {
      findingIds.push(`landmarks_${width}x${height}`);
    }
    if (run.sensitiveDataExcluded !== true) findingIds.push(`sensitive_data_${width}x${height}`);
    if (typeof run.screenshot !== "string" || !run.screenshot.endsWith(".png")) {
      findingIds.push(`screenshot_${width}x${height}`);
    }
  }
  return { passed: findingIds.length === 0, findingIds };
}

export function containsSensitiveData(text) {
  return [
    "/Users/",
    "C:\\Users\\",
    "raw document body",
    "notes/private.md",
    "provider_api_key",
    "sessionToken",
  ].some((token) => text.includes(token));
}

export function workspaceHomeVisualViewports() {
  return requiredViewports.map(([width, height]) => ({ width, height }));
}
