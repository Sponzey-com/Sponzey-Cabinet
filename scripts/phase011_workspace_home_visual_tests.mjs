import assert from "node:assert/strict";
import test from "node:test";

import {
  WorkspaceHomeVisualState,
  transitionWorkspaceHomeVisualState,
  validateWorkspaceHomeVisualReport,
} from "./phase011_workspace_home_visual.mjs";

test("visual report requires all viewports, pixels, layout, landmarks, and focus", () => {
  assert.equal(validateWorkspaceHomeVisualReport(validReport()).passed, true);

  for (const mutate of [
    (report) => report.runs.pop(),
    (report) => { report.runs[0].nonBlankPixelCount = 0; },
    (report) => { report.runs[0].overlapCount = 1; },
    (report) => { report.runs[0].horizontalOverflow = true; },
    (report) => { report.runs[0].focusVisible = false; },
    (report) => { report.runs[0].mainLandmark = false; },
    (report) => { report.runs[0].readyState = false; },
    (report) => { report.runs[0].sensitiveDataExcluded = false; },
    (report) => { report.retryKeyboardFlow = false; },
    (report) => { report.navigatorInteractions.fiveViews = false; },
    (report) => { report.navigatorInteractions.filterEmpty = false; },
    (report) => { report.navigatorInteractions.retryKeyboardFlow = false; },
    (report) => { report.navigatorInteractions.homeReturn = false; },
  ]) {
    const report = validReport();
    mutate(report);
    assert.equal(validateWorkspaceHomeVisualReport(report).passed, false);
  }
});

test("visual report rejects sensitive text and stale source fingerprints", () => {
  const sensitive = validReport();
  sensitive.diagnostics = "/Users/private/app-data raw document body";
  assert.equal(validateWorkspaceHomeVisualReport(sensitive).passed, false);
  assert.equal(
    validateWorkspaceHomeVisualReport(validReport(), "different-fingerprint").passed,
    false,
  );
});

test("visual runner state machine reaches passed and fails invalid transitions", () => {
  let state = WorkspaceHomeVisualState.Pending;
  for (const event of ["Serve", "Launch", "Inject", "Navigate", "Validate", "Capture", "Pass"]) {
    state = transitionWorkspaceHomeVisualState(state, event).state;
  }
  assert.equal(state, WorkspaceHomeVisualState.Passed);
  assert.equal(
    transitionWorkspaceHomeVisualState(WorkspaceHomeVisualState.Pending, "Capture").state,
    WorkspaceHomeVisualState.Failed,
  );
});

function validReport() {
  return {
    marker: "phase011_workspace_home_visual=passed",
    sourceFingerprint: "a".repeat(64),
    diagnostics: "sanitized",
    retryKeyboardFlow: true,
    navigatorInteractions: {
      fiveViews: true,
      filterEmpty: true,
      retryKeyboardFlow: true,
      homeReturn: true,
    },
    runs: [
      viewport(1024, 700),
      viewport(1280, 800),
      viewport(1440, 900),
      viewport(1920, 1080),
    ],
  };
}

function viewport(width, height) {
  return {
    width,
    height,
    readyState: true,
    nonBlankPixelCount: 10000,
    overlapCount: 0,
    horizontalOverflow: false,
    focusVisible: true,
    navLandmark: true,
    mainLandmark: true,
    liveRegion: true,
    sensitiveDataExcluded: true,
    screenshot: `workspace-home-${width}x${height}.png`,
  };
}
