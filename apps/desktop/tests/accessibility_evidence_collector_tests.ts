import assert from "node:assert/strict";
import test from "node:test";

import {
  AccessibilityEvidenceCollectorError,
  aggregateAccessibilityRouteMeasurements,
  collectAccessibilityEvidence,
  createAccessibilityCollectionTiming,
  type AccessibilityRoute,
  type AccessibilityRouteMeasurement,
} from "../src/accessibility_evidence_collector.ts";
import { AccessibilityEvidenceError } from "../src/accessibility_evidence_contract.ts";

const hash = (character: string): string => character.repeat(64);
const routes = ["Home", "Document", "Graph", "Canvas", "Assets", "Backup"] as const;
const policy = Object.freeze({
  routes,
  requiredTextZoomPercent: 200,
  minimumKeyboardJourneyCount: 6,
  minimumFocusRestorationCount: 3,
  routeTimeoutMs: 100,
});

test("collector measures six primary routes once in order and creates strict evidence", async () => {
  const calls: string[] = [];
  const evidence = await collectAccessibilityEvidence({
    sourceFingerprint: hash("a"),
    appFingerprint: hash("b"),
    policy,
    timing: createAccessibilityCollectionTiming(),
    port: {
      async measureRoute(route, textZoomPercent) {
        calls.push(`${route}:${textZoomPercent}`);
        return measurement(route);
      },
    },
  });

  assert.deepEqual(calls, routes.map((route) => `${route}:200`));
  assert.equal(evidence.status, "Passed");
  assert.equal(evidence.routeFocusCount, 6);
  assert.equal(evidence.keyboardJourneyCount, 6);
  assert.equal(evidence.focusRestorationCount, 6);
  assert.equal(evidence.visibleControlCount, 60);
  assert.equal(evidence.namedControlCount, 60);
});

test("collector rejects mismatched route and stops before later I/O", async () => {
  const calls: AccessibilityRoute[] = [];
  await assert.rejects(() => collectAccessibilityEvidence({
    sourceFingerprint: hash("a"), appFingerprint: hash("b"), policy,
    timing: createAccessibilityCollectionTiming(),
    port: {
      async measureRoute(route) {
        calls.push(route);
        return measurement(route === "Graph" ? "Canvas" : route);
      },
    },
  }), collectorError("ACCESSIBILITY_COLLECTION_ROUTE_MISMATCH"));
  assert.deepEqual(calls, ["Home", "Document", "Graph"]);
});

test("collector forwards unnamed control and focus failures to strict evidence", async () => {
  await assert.rejects(() => collectAccessibilityEvidence({
    sourceFingerprint: hash("a"), appFingerprint: hash("b"), policy,
    timing: createAccessibilityCollectionTiming(),
    port: {
      async measureRoute(route) {
        return route === "Assets" ? { ...measurement(route), namedControlCount: 9 } : measurement(route);
      },
    },
  }), (error) => error instanceof AccessibilityEvidenceError
    && error.code === "ACCESSIBILITY_CONTROL_NAME_INCOMPLETE");
});

test("collector bounds a stalled route and does not start later routes", async () => {
  const calls: AccessibilityRoute[] = [];
  await assert.rejects(() => collectAccessibilityEvidence({
    sourceFingerprint: hash("a"), appFingerprint: hash("b"),
    policy: { ...policy, routeTimeoutMs: 10 },
    timing: createAccessibilityCollectionTiming(),
    port: {
      async measureRoute(route) {
        calls.push(route);
        if (route === "Graph") return new Promise<AccessibilityRouteMeasurement>(() => {});
        return measurement(route);
      },
    },
  }), collectorError("ACCESSIBILITY_COLLECTION_TIMEOUT"));
  assert.deepEqual(calls, ["Home", "Document", "Graph"]);
});

test("aggregate requires the exact six-route order and preserves only numeric evidence", () => {
  const aggregate = aggregateAccessibilityRouteMeasurements(routes.map(measurement), 200);
  assert.deepEqual(aggregate, {
    routeFocusCount: 6,
    keyboardJourneyCount: 6,
    focusRestorationCount: 6,
    visibleControlCount: 60,
    namedControlCount: 60,
    textZoomPercent: 200,
    keyboardErrorCount: 0,
    focusErrorCount: 0,
    internalExposureCount: 0,
  });
  assert.equal(Object.isFrozen(aggregate), true);

  assert.throws(
    () => aggregateAccessibilityRouteMeasurements(routes.slice(0, -1).map(measurement), 200),
    collectorError("ACCESSIBILITY_COLLECTION_ROUTE_SET_INVALID"),
  );
  assert.throws(
    () => aggregateAccessibilityRouteMeasurements(
      [...routes.slice(0, -1), "Assets"].map(measurement),
      200,
    ),
    collectorError("ACCESSIBILITY_COLLECTION_ROUTE_SET_INVALID"),
  );
});

function measurement(route: AccessibilityRoute): AccessibilityRouteMeasurement {
  return Object.freeze({
    route,
    visibleControlCount: 10,
    namedControlCount: 10,
    mainFocusReached: true,
    keyboardJourneyPassed: true,
    focusRestorationCount: 1,
    keyboardErrorCount: 0,
    focusErrorCount: 0,
    internalExposureCount: 0,
  });
}

function collectorError(code: string): (error: unknown) => boolean {
  return (error) => error instanceof AccessibilityEvidenceCollectorError && error.code === code;
}
