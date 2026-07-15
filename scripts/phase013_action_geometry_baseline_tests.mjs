import assert from "node:assert/strict";
import test from "node:test";

import {
  compareActionInventory,
  transitionActionGeometryCapture,
  validateActionGeometryReport,
} from "./phase013_action_geometry_baseline.mjs";

const fingerprint = "a".repeat(64);
const routes = ["Home", "Search", "Document", "Graph", "Canvas", "Assets", "Backup"];
const viewports = [{ width: 1024, height: 768 }, { width: 1280, height: 800 }, { width: 1440, height: 900 }, { width: 1728, height: 1117 }, { width: 1920, height: 1080 }];

test("action comparison classifies unregistered and hidden controls while allowing repeated actions", () => {
  const rendered = [
    action("Home", "navigate-search"),
    action("Home", "new-document"),
    action("Search", "navigate-search", true),
    action("Search", "navigate-search", true),
    action("Search", "open-settings", true),
  ];
  const manifest = [
    { actionId: "navigate-search", state: "connected" },
    { actionId: "open-settings", availability: "hidden_out_of_scope" },
    { actionId: "manifest-only", availability: "connected", visibleCondition: "route_and_state_specific" },
  ];
  const gaps = compareActionInventory(rendered, manifest);
  assert.deepEqual(gaps.map((finding) => finding.category), [
    "rendered_unregistered",
    "hidden_action_rendered",
  ]);
  assert.ok(gaps.every((finding) => !JSON.stringify(finding).includes("사용자 문서")));
});

test("capture state machine accepts only explicit transitions", () => {
  assert.equal(transitionActionGeometryCapture("Pending", "Serve"), "Serving");
  assert.equal(transitionActionGeometryCapture("Serving", "Browse"), "Browsing");
  assert.equal(transitionActionGeometryCapture("Browsing", "Capture"), "Capturing");
  assert.equal(transitionActionGeometryCapture("Capturing", "Continue"), "Browsing");
  assert.equal(transitionActionGeometryCapture("Capturing", "Pass"), "Passed");
  assert.equal(transitionActionGeometryCapture("Pending", "Pass"), "Failed");
});

test("complete sanitized action and geometry report passes", () => {
  assert.deepEqual(validateActionGeometryReport(validReport(), { fingerprint, routes, viewports }), {
    passed: true,
    findingIds: [],
  });
});

test("validator rejects stale incomplete malformed and sensitive reports", () => {
  const report = validReport();
  report.sourceFingerprint = "b".repeat(64);
  report.runs.pop();
  report.runs[0].main.width = -1;
  report.runs[1].horizontalOverflow = true;
  report.runs[2].clippedActionCount = 1;
  report.gaps.push({ category: "rendered_unregistered", route: "Home", actionId: "unregistered" });
  report.diagnostics = "/Users/private/workspace";
  const result = validateActionGeometryReport(report, { fingerprint, routes, viewports });
  assert.equal(result.passed, false);
  for (const id of ["stale_source_fingerprint", "sensitive_data", "malformed_geometry_Home_1024x768", "horizontal_overflow_Search_1024x768", "clipped_action_Document_1024x768", "missing_Backup_1920x1080", "action_gap_rendered_unregistered_Home_unregistered"]) {
    assert.ok(result.findingIds.includes(id), id);
  }
});

test("validator rejects controls without action identity or accessible label", () => {
  const report = validReport();
  report.actions.push({ route: "Home", actionId: "", kind: "button", disabled: false, hasAccessibleName: true });
  report.actions.push({ route: "Home", actionId: "unnamed", kind: "button", disabled: false, hasAccessibleName: false });
  const result = validateActionGeometryReport(report, { fingerprint, routes, viewports });
  assert.ok(result.findingIds.includes("missing_action_identity_Home"));
  assert.ok(result.findingIds.includes("missing_accessible_name_Home_unnamed"));
});

test("validator accepts repeated action identities in one route", () => {
  const report = validReport();
  report.actions.push(action("Home", "action-home"));
  assert.equal(validateActionGeometryReport(report, { fingerprint, routes, viewports }).passed, true);
});

function action(route, actionId, disabled = false) {
  return { route, actionId, kind: "button", disabled, hasAccessibleName: true };
}

function validReport() {
  return {
    marker: "phase013_action_geometry_baseline=recorded",
    sourceFingerprint: fingerprint,
    fixtureVersion: "phase013-baseline-v1",
    diagnostics: "sanitized",
    actions: routes.map((route) => action(route, `action-${route.toLowerCase()}`)),
    gaps: [],
    runs: viewports.flatMap((viewport) => routes.map((route) => ({
      ...viewport,
      route,
      state: "Ready",
      sidebar: rect(0, 0, 250, viewport.height),
      topbar: rect(250, 0, viewport.width - 250, 60),
      main: rect(250, 60, viewport.width - 250, viewport.height - 60),
      horizontalOverflow: false,
      clippedActionCount: 0,
    }))),
  };
}

function rect(x, y, width, height) {
  return { x, y, width, height };
}
