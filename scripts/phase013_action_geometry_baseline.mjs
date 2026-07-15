const transitions = Object.freeze({
  Pending: Object.freeze({ Serve: "Serving", Fail: "Failed" }),
  Serving: Object.freeze({ Browse: "Browsing", Fail: "Failed" }),
  Browsing: Object.freeze({ Capture: "Capturing", Fail: "Failed" }),
  Capturing: Object.freeze({ Continue: "Browsing", Pass: "Passed", Fail: "Failed" }),
});

export function transitionActionGeometryCapture(state, event) {
  return transitions[state]?.[event] ?? "Failed";
}

export function compareActionInventory(renderedActions, manifestEntries) {
  const findings = [];
  const manifestById = new Map(manifestEntries.map((entry) => [entry.actionId, entry]));

  for (const action of renderedActions) {
    if (action.actionId && !manifestById.has(action.actionId)) {
      findings.push(finding("rendered_unregistered", action.route, action.actionId));
    }
    if (manifestById.get(action.actionId)?.availability === "hidden_out_of_scope") {
      findings.push(finding("hidden_action_rendered", action.route, action.actionId));
    }
  }
  return Object.freeze(findings.map(Object.freeze));
}

export function validateActionGeometryReport(report, expected) {
  const findingIds = [];
  if (report?.marker !== "phase013_action_geometry_baseline=recorded") findingIds.push("marker");
  if (!/^[a-f0-9]{64}$/.test(report?.sourceFingerprint ?? "")) findingIds.push("source_fingerprint");
  if (report?.sourceFingerprint !== expected.fingerprint) findingIds.push("stale_source_fingerprint");
  if (report?.fixtureVersion !== "phase013-baseline-v1") findingIds.push("fixture_version");
  if (containsSensitiveData(JSON.stringify(report ?? {}))) findingIds.push("sensitive_data");

  const actions = Array.isArray(report?.actions) ? report.actions : [];
  const recordedGaps = Array.isArray(report?.gaps) ? report.gaps : [];
  for (const action of actions) {
    if (!action.actionId || action.identityMissing === true) findingIds.push(`missing_action_identity_${action.route ?? "unknown"}`);
    if (!action.hasAccessibleName) findingIds.push(`missing_accessible_name_${action.route ?? "unknown"}_${action.actionId || "unknown"}`);
  }
  for (const gap of recordedGaps) {
    findingIds.push(`action_gap_${gap.category ?? "unknown"}_${gap.route ?? "unknown"}_${gap.actionId ?? "unknown"}`);
  }

  const runs = Array.isArray(report?.runs) ? report.runs : [];
  for (const viewport of expected.viewports) {
    for (const route of expected.routes) {
      const run = runs.find((candidate) => candidate.route === route && candidate.width === viewport.width && candidate.height === viewport.height);
      const suffix = `${route}_${viewport.width}x${viewport.height}`;
      if (!run) {
        findingIds.push(`missing_${suffix}`);
      } else if (![run.sidebar, run.topbar, run.main].every(validRect)) {
        findingIds.push(`malformed_geometry_${suffix}`);
      } else {
        if (run.horizontalOverflow === true) findingIds.push(`horizontal_overflow_${suffix}`);
        if (!Number.isInteger(run.clippedActionCount) || run.clippedActionCount > 0) findingIds.push(`clipped_action_${suffix}`);
      }
    }
  }
  return Object.freeze({ passed: findingIds.length === 0, findingIds: Object.freeze(findingIds) });
}

export function summarizeGeometryDeltas(runs, routes, viewports) {
  return viewports.map((viewport) => {
    const matching = routes.map((route) => runs.find((run) => run.route === route && run.width === viewport.width && run.height === viewport.height)).filter(Boolean);
    const baseline = matching[0];
    const maximumDelta = matching.reduce((maximum, run) => Math.max(maximum, geometryDelta(baseline, run)), 0);
    return Object.freeze({ ...viewport, maximumDelta });
  });
}

function geometryDelta(left, right) {
  if (!left || !right) return Number.POSITIVE_INFINITY;
  return Math.max(...["sidebar", "topbar", "main"].flatMap((part) => ["x", "y", "width", "height"].map((field) => Math.abs(left[part][field] - right[part][field]))));
}

function validRect(rect) {
  return rect && [rect.x, rect.y, rect.width, rect.height].every(Number.isFinite) && rect.width >= 0 && rect.height >= 0;
}

function finding(category, route, actionId) {
  return { category, route, actionId };
}

function containsSensitiveData(value) {
  return ["/Users/", "C:\\Users\\", "raw document body", "provider_api_key", "sessionToken"].some((token) => value.includes(token));
}
