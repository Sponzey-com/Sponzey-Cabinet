import assert from "node:assert/strict";
import test from "node:test";

import { validateResponsiveStressReport } from "./phase013_responsive_stress.mjs";

const fingerprint = "a".repeat(64);
const routes = ["Home", "Search", "Document", "Graph", "Canvas", "Assets", "Backup"];

test("complete responsive stress report passes", () => {
  assert.deepEqual(validateResponsiveStressReport(validReport(), { fingerprint, routes }), { passed: true, findingIds: [] });
});

test("validator rejects stale weak incomplete and overflowing stress evidence", () => {
  const report = validReport();
  report.sourceFingerprint = "b".repeat(64);
  report.textZoomPercent = 100;
  report.fixtureItemCount = 99;
  report.longContentFixture = false;
  report.runs.shift();
  report.runs[0].horizontalOverflow = true;
  report.runs[1].clippedActionCount = 2;
  report.actions.push({ route: "Document", actionId: "", identityMissing: true, hasAccessibleName: true });
  report.gaps.push({ category: "rendered_unregistered" });
  const result = validateResponsiveStressReport(report, { fingerprint, routes });
  for (const id of ["stale_source_fingerprint", "text_zoom_percent", "fixture_item_count", "long_content_fixture", "missing_Home", "horizontal_overflow_Search", "clipped_action_Document", "missing_action_identity_Document", "action_contract_gap"]) {
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
    runs: routes.map((route) => ({ route, horizontalOverflow: false, clippedActionCount: 0 })),
  };
}
