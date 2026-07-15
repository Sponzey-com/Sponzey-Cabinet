import assert from "node:assert/strict";
import test from "node:test";

import {
  WorkspaceHomeGateState,
  transitionWorkspaceHomeGateState,
  validateWorkspaceHomeGateEvidence,
} from "./phase011_workspace_home_gate.mjs";

test("workspace home gate requires visual, performance, product, and matching fingerprints", () => {
  const evidence = validEvidence();
  assert.equal(validateWorkspaceHomeGateEvidence(evidence).passed, true);

  for (const mutate of [
    (value) => { value.visual.marker = "failed"; },
    (value) => { value.performance.p95Ms = 301; },
    (value) => { value.product.reactRootMounted = false; },
    (value) => { value.product.navigatorInteractionPassed = false; },
    (value) => { value.performance.sourceFingerprint = "c".repeat(64); },
    (value) => { value.requirementIds.pop(); },
  ]) {
    const value = validEvidence();
    mutate(value);
    assert.equal(validateWorkspaceHomeGateEvidence(value).passed, false);
  }
});

test("workspace home gate state machine passes in order and fails invalid event", () => {
  let state = WorkspaceHomeGateState.Pending;
  for (const event of ["Read", "VisualValid", "PerformanceValid", "ProductValid", "Write", "Pass"]) {
    state = transitionWorkspaceHomeGateState(state, event).state;
  }
  assert.equal(state, WorkspaceHomeGateState.Passed);
  assert.equal(
    transitionWorkspaceHomeGateState(WorkspaceHomeGateState.Pending, "Pass").state,
    WorkspaceHomeGateState.Failed,
  );
});

function validEvidence() {
  const sourceFingerprint = "a".repeat(64);
  return {
    sourceFingerprint,
    requirementIds: ["BOOT-01", "HOME-01", "NAV-01", "PERF-01", "UX-01"],
    visual: {
      marker: "phase011_workspace_home_visual=passed",
      sourceFingerprint,
      diagnostics: "sanitized",
      retryKeyboardFlow: true,
      navigatorInteractions: {
        fiveViews: true,
        filterEmpty: true,
        retryKeyboardFlow: true,
        homeReturn: true,
      },
      runs: [[1024, 700], [1280, 800], [1440, 900], [1920, 1080]].map(([width, height]) => ({
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
      })),
    },
    performance: {
      marker: "phase011_workspace_home_performance=passed",
      sourceFingerprint,
      fixtureHash: "b".repeat(64),
      currentDocumentCount: 10000,
      totalVersionCount: 100000,
      warmupCount: 20,
      sampleCount: 200,
      p50Ms: 1.2,
      p95Ms: 2.5,
      maxMs: 4.2,
      buildProfile: "release",
      queryPath: "bounded_workspace_home_projection",
      diagnostics: "sanitized",
    },
    product: {
      reactRootMounted: true,
      navigatorInteractionPassed: true,
      nativeCommandIntegrationPassed: true,
      packageSmokePassed: true,
      futureScopeExcluded: true,
      sensitiveDataExcluded: true,
    },
  };
}
