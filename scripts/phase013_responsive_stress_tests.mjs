import assert from "node:assert/strict";
import test from "node:test";

import { validateResponsiveStressReport } from "./phase013_responsive_stress.mjs";

const fingerprint = "a".repeat(64);
const routes = ["Home", "Search", "Document", "Graph", "Canvas", "Assets", "Backup"];
const viewports = [
  { width: 1440, height: 900 },
  { width: 1180, height: 800 },
  { width: 960, height: 720 },
  { width: 760, height: 640 },
];

test("complete responsive stress report passes", () => {
  assert.deepEqual(validateResponsiveStressReport(validReport(), { fingerprint, routes, viewports }), { passed: true, findingIds: [] });
});

test("validator rejects stale weak incomplete and overflowing stress evidence", () => {
  const report = validReport();
  report.sourceFingerprint = "b".repeat(64);
  report.textZoomPercent = 100;
  report.fixtureItemCount = 99;
  report.longContentFixture = false;
  report.runs = report.runs.filter((run) => !(run.route === "Home" && run.width === 760));
  report.runs.find((run) => run.route === "Search" && run.width === 760).horizontalOverflow = true;
  report.runs.find((run) => run.route === "Document" && run.width === 760).clippedActionCount = 2;
  report.actions.push({ route: "Document", actionId: "", identityMissing: true, hasAccessibleName: true });
  report.gaps.push({ category: "rendered_unregistered" });
  const result = validateResponsiveStressReport(report, { fingerprint, routes, viewports });
  for (const id of ["stale_source_fingerprint", "text_zoom_percent", "fixture_item_count", "long_content_fixture", "missing_Home_760x640", "horizontal_overflow_Search_760x640", "clipped_action_Document_760x640", "missing_action_identity_Document", "action_contract_gap"]) {
    assert.ok(result.findingIds.includes(id), id);
  }
});

function validReport() {
  return {
    marker: "phase013_responsive_stress=recorded",
    sourceFingerprint: fingerprint,
    fixtureVersion: "phase013-responsive-stress-v1",
    textZoomPercent: 200,
    fixtureItemCount: 100,
    longContentFixture: true,
    actions: [{ route: "Home", actionId: "navigate-home", identityMissing: false, hasAccessibleName: true }],
    gaps: [],
    runs: viewports.flatMap((viewport) => routes.map((route) => ({
      ...viewport,
      route,
      horizontalOverflow: false,
      clippedActionCount: 0,
    }))),
  };
}
