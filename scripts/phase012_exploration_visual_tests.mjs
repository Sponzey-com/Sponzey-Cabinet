import assert from "node:assert/strict";
import test from "node:test";

import {
  explorationVisualViewports,
  transitionExplorationVisualState,
  validateExplorationVisualReport,
} from "./phase012_exploration_visual.mjs";

const fingerprint = "a".repeat(64);

test("exploration visual state machine rejects invalid transitions", () => {
  assert.equal(transitionExplorationVisualState("Pending", "Serve").state, "Serving");
  assert.equal(transitionExplorationVisualState("Serving", "Browse").state, "Browsing");
  assert.equal(transitionExplorationVisualState("Browsing", "Capture").state, "Capturing");
  assert.equal(transitionExplorationVisualState("Capturing", "Pass").state, "Passed");
  assert.equal(transitionExplorationVisualState("Pending", "Pass").state, "Failed");
});

test("complete sanitized exploration visual report passes", () => {
  const report = validReport();
  assert.deepEqual(validateExplorationVisualReport(report, fingerprint), { passed: true, findingIds: [] });
});

test("blank overflow focus landmark route and stale evidence fail closed", () => {
  const report = validReport();
  report.sourceFingerprint = "b".repeat(64);
  report.runs[0].nonBlankPixelCount = 0;
  report.runs[1].horizontalOverflow = true;
  report.runs[2].clippedControlCount = 1;
  report.runs[3].overlapCount = 1;
  report.runs[4].focusVisible = false;
  report.runs[5].mainLandmark = false;
  report.interactions.canvasKeyboardSelection = false;
  report.interactions.graphOpenedDocumentId = "wrong";
  const validation = validateExplorationVisualReport(report, fingerprint);
  assert.equal(validation.passed, false);
  for (const finding of ["stale_source_fingerprint", "pixels_graph_1280x800", "overflow_canvas_1280x800", "clipped_assets_1280x800", "overlap_graph_1440x900", "focus_canvas_1440x900", "landmarks_assets_1440x900", "canvas_keyboard_selection", "graph_route_identity"]) {
    assert.ok(validation.findingIds.includes(finding), finding);
  }
});

test("missing viewport and sensitive artifact fail closed", () => {
  const report = validReport();
  report.runs.pop();
  report.diagnostics = "/Users/private/source.md";
  const validation = validateExplorationVisualReport(report, fingerprint);
  assert.equal(validation.passed, false);
  assert.ok(validation.findingIds.includes("sensitive_data"));
  assert.ok(validation.findingIds.includes("viewport_assets_1728x1117"));
});

function validReport() {
  const runs = [];
  for (const viewport of explorationVisualViewports()) {
    for (const surface of ["graph", "canvas", "assets"]) {
      runs.push({
        ...viewport,
        surface,
        readyState: true,
        nonBlankPixelCount: 50_000,
        overlapCount: 0,
        horizontalOverflow: false,
        clippedControlCount: 0,
        focusVisible: true,
        navLandmark: true,
        mainLandmark: true,
        sensitiveDataExcluded: true,
        screenshot: `${surface}-${viewport.width}x${viewport.height}.png`,
      });
    }
  }
  return {
    marker: "phase012_exploration_visual=passed",
    sourceFingerprint: fingerprint,
    diagnostics: "sanitized",
    interactions: {
      graphOpenedDocumentId: "doc-001",
      canvasOpenedDocumentId: "doc-001",
      assetOpenedDocumentId: "doc-001",
      canvasKeyboardSelection: true,
    },
    runs,
  };
}
