import assert from "node:assert/strict";
import test from "node:test";

import {
  Phase011AuthoringBrowserState,
  transitionPhase011AuthoringBrowserState,
  validatePhase011AuthoringBrowserReport,
} from "./phase011_authoring_browser.mjs";

test("authoring browser report requires interaction, layout, screenshot, and sanitized evidence", () => {
  assert.equal(validatePhase011AuthoringBrowserReport(validReport()).passed, true);

  for (const mutate of [
    (report) => { report.marker = "phase011_authoring_browser=failed"; },
    (report) => { report.browserSurface = "iab"; },
    (report) => { report.interactions.codeMirrorMounted = false; },
    (report) => { report.interactions.createDocumentCount = 0; },
    (report) => { report.interactions.createdDocumentOpened = false; },
    (report) => { report.interactions.manualSaveCount = 0; },
    (report) => { report.interactions.autosaveCount = 0; },
    (report) => { report.interactions.keyboardSave = false; },
    (report) => { report.interactions.closeCancel = false; },
    (report) => { report.interactions.closeRetrySave = false; },
    (report) => { report.interactions.closeDiscard = false; },
    (report) => { report.interactions.rawBodyExcluded = false; },
    (report) => { report.runs.pop(); },
    (report) => { report.runs[0].nonBlankPixelCount = 0; },
    (report) => { report.runs[0].overlapCount = 1; },
    (report) => { report.runs[0].horizontalOverflow = true; },
    (report) => { report.runs[0].focusVisible = false; },
    (report) => { report.runs[0].screenshot = "authoring.txt"; },
  ]) {
    const report = validReport();
    mutate(report);
    assert.equal(validatePhase011AuthoringBrowserReport(report).passed, false);
  }
});

test("authoring browser report rejects sensitive text and stale source fingerprints", () => {
  const sensitive = validReport();
  sensitive.diagnostics = "/Users/private raw markdown body Secret Browser Body";
  assert.equal(validatePhase011AuthoringBrowserReport(sensitive).passed, false);
  assert.equal(
    validatePhase011AuthoringBrowserReport(validReport(), "different-fingerprint").passed,
    false,
  );
});

test("authoring browser state machine reaches passed and rejects invalid transitions", () => {
  let state = Phase011AuthoringBrowserState.Pending;
  for (const event of ["Serve", "Launch", "Inject", "Navigate", "Interact", "Capture", "Pass"]) {
    state = transitionPhase011AuthoringBrowserState(state, event).state;
  }
  assert.equal(state, Phase011AuthoringBrowserState.Passed);
  assert.equal(
    transitionPhase011AuthoringBrowserState(Phase011AuthoringBrowserState.Pending, "Capture").state,
    Phase011AuthoringBrowserState.Failed,
  );
});

function validReport() {
  return {
    marker: "phase011_authoring_browser=passed",
    sourceFingerprint: "a".repeat(64),
    browserSurface: "local_chrome_cdp",
    diagnostics: "sanitized",
    interactions: {
      documentOpened: true,
      codeMirrorMounted: true,
      createDocumentCount: 1,
      createdDocumentOpened: true,
      sourceMode: true,
      splitMode: true,
      previewMode: true,
      previewTableRendered: true,
      keyboardSave: true,
      manualSaveCount: 1,
      autosaveCount: 1,
      closeBlocked: true,
      closeCancel: true,
      closeRetrySave: true,
      closeDiscard: true,
      historyLoaded: true,
      restorePreviewReady: true,
      restoreApplyCount: 1,
      rawBodyExcluded: true,
      rawPathExcluded: true,
    },
    runs: [viewport(1024, 700), viewport(1280, 800)],
  };
}

function viewport(width, height) {
  return {
    width,
    height,
    readyState: true,
    codeMirrorMounted: true,
    previewTableRendered: true,
    nonBlankPixelCount: 10000,
    overlapCount: 0,
    horizontalOverflow: false,
    focusVisible: true,
    screenshot: `authoring-${width}x${height}.png`,
  };
}
