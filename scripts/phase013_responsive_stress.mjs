export function validateResponsiveStressReport(report, expected) {
  const findingIds = [];
  if (report?.marker !== "phase013_responsive_stress=recorded") findingIds.push("marker");
  if (report?.sourceFingerprint !== expected.fingerprint) findingIds.push("stale_source_fingerprint");
  if (report?.fixtureVersion !== "phase013-responsive-stress-v1") findingIds.push("fixture_version");
  if (report?.textZoomPercent !== 200) findingIds.push("text_zoom_percent");
  if (!Number.isInteger(report?.fixtureItemCount) || report.fixtureItemCount < 100) findingIds.push("fixture_item_count");
  if (report?.longContentFixture !== true) findingIds.push("long_content_fixture");

  const runs = Array.isArray(report?.runs) ? report.runs : [];
  for (const route of expected.routes) {
    const run = runs.find((candidate) => candidate.route === route);
    if (!run) findingIds.push(`missing_${route}`);
    else {
      if (run.horizontalOverflow === true) findingIds.push(`horizontal_overflow_${route}`);
      if (!Number.isInteger(run.clippedActionCount) || run.clippedActionCount > 0) findingIds.push(`clipped_action_${route}`);
    }
  }
  for (const action of Array.isArray(report?.actions) ? report.actions : []) {
    if (!action.actionId || action.identityMissing === true) findingIds.push(`missing_action_identity_${action.route ?? "unknown"}`);
    if (!action.hasAccessibleName) findingIds.push(`missing_accessible_name_${action.route ?? "unknown"}_${action.actionId || "unknown"}`);
  }
  if (Array.isArray(report?.gaps) && report.gaps.length > 0) findingIds.push("action_contract_gap");
  return Object.freeze({ passed: findingIds.length === 0, findingIds: Object.freeze(findingIds) });
}
