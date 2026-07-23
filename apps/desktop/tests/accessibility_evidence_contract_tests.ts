import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import test from "node:test";

import {
  AccessibilityEvidenceError,
  createAccessibilityEvidence,
} from "../src/accessibility_evidence_contract.ts";
import { createDesktopBuildIdentity } from "../src/desktop_build_identity_contract.ts";
import { RequirementEvidenceError } from "../src/requirement_evidence_contract.ts";
import {
  createReleaseEvidenceMetadata,
  mapAccessibilityEvidence,
} from "../src/release_evidence_mapper.ts";

const sha256 = (value: string): string => createHash("sha256").update(value).digest("hex");
const hash = (character: string): string => character.repeat(64);
const policy = Object.freeze({
  requiredRouteFocusCount: 6,
  requiredTextZoomPercent: 200,
  minimumKeyboardJourneyCount: 6,
  minimumFocusRestorationCount: 3,
});

test("accessibility evidence passes only complete current numeric coverage", () => {
  const evidence = createAccessibilityEvidence({
    sourceFingerprint: hash("a"),
    appFingerprint: hash("b"),
    policy,
    measurement: validMeasurement(),
  });

  assert.equal(evidence.status, "Passed");
  assert.equal(evidence.routeFocusCount, 6);
  assert.equal(evidence.visibleControlCount, evidence.namedControlCount);
  assert.deepEqual(Object.keys(evidence).sort(), [
    "appFingerprint", "focusErrorCount", "focusRestorationCount", "internalExposureCount",
    "keyboardErrorCount", "keyboardJourneyCount", "namedControlCount", "routeFocusCount",
    "sourceFingerprint", "status", "textZoomPercent", "visibleControlCount",
  ]);
  assert.equal(Object.isFrozen(evidence), true);
});

test("accessibility evidence rejects route zoom unnamed control and error mismatches", () => {
  expectCode({ routeFocusCount: 5 }, "ACCESSIBILITY_ROUTE_FOCUS_INCOMPLETE");
  expectCode({ textZoomPercent: 199 }, "ACCESSIBILITY_TEXT_ZOOM_INCOMPLETE");
  expectCode({ namedControlCount: 39 }, "ACCESSIBILITY_CONTROL_NAME_INCOMPLETE");
  expectCode({ keyboardJourneyCount: 5 }, "ACCESSIBILITY_KEYBOARD_COVERAGE_INCOMPLETE");
  expectCode({ focusRestorationCount: 2 }, "ACCESSIBILITY_FOCUS_RESTORATION_INCOMPLETE");
  expectCode({ keyboardErrorCount: 1 }, "ACCESSIBILITY_ERROR_REPORTED");
  expectCode({ focusErrorCount: 1 }, "ACCESSIBILITY_ERROR_REPORTED");
  expectCode({ internalExposureCount: 1 }, "ACCESSIBILITY_ERROR_REPORTED");
});

test("accessibility mapper requires verified metadata and matching fingerprints", () => {
  const identity = createDesktopBuildIdentity({
    sourceFingerprint: hash("a"), sourceFileCount: 89,
    appFingerprint: hash("b"), artifactCount: 3, totalArtifactBytes: 3_259_348,
    hash: sha256,
  });
  const metadata = createReleaseEvidenceMetadata({
    buildIdentity: identity,
    testOrJourneyName: "phase016.accessibility.current",
    harnessVersion: "1",
    fixtureClass: "clean-profile",
    startedAt: "2026-07-20T00:00:00.000Z",
    hash: sha256,
  });
  const evidence = createAccessibilityEvidence({
    sourceFingerprint: identity.sourceFingerprint,
    appFingerprint: identity.appFingerprint,
    policy,
    measurement: validMeasurement(),
  });
  const records = mapAccessibilityEvidence({
    requirementIds: ["NAV-002", "NAV-008"], metadata, evidence,
  });
  assert.equal(records.length, 2);
  assert.equal(records[0]?.measuredValues.namedControlCount, 40);

  assert.throws(() => mapAccessibilityEvidence({
    requirementIds: ["NAV-008"], metadata,
    evidence: { ...evidence, appFingerprint: hash("c") },
  }), evidenceError("REQUIREMENT_EVIDENCE_FINGERPRINT_MISMATCH"));
});

function validMeasurement() {
  return {
    routeFocusCount: 6,
    keyboardJourneyCount: 8,
    focusRestorationCount: 4,
    visibleControlCount: 40,
    namedControlCount: 40,
    textZoomPercent: 200,
    keyboardErrorCount: 0,
    focusErrorCount: 0,
    internalExposureCount: 0,
  };
}

function expectCode(overrides: Record<string, number>, code: string): void {
  assert.throws(() => createAccessibilityEvidence({
    sourceFingerprint: hash("a"),
    appFingerprint: hash("b"),
    policy,
    measurement: { ...validMeasurement(), ...overrides },
  }), (error) => error instanceof AccessibilityEvidenceError && error.code === code);
}

function evidenceError(code: string): (error: unknown) => boolean {
  return (error) => error instanceof RequirementEvidenceError && error.code === code;
}
