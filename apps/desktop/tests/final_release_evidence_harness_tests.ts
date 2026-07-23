import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import test from "node:test";

import { createDesktopBuildIdentity } from "../src/desktop_build_identity_contract.ts";
import {
  FinalReleaseEvidenceHarnessError,
  createFinalReleaseEvidenceReceipt,
  transitionFinalReleaseGate,
} from "../src/final_release_evidence_harness.ts";

const sha256 = (value: string): string => createHash("sha256").update(value).digest("hex");
const hash = (character: string): string => character.repeat(64);

test("final harness creates exact process-bound records without test paths or stdout", () => {
  const receipt = createFinalReleaseEvidenceReceipt(validInput());
  assert.equal(receipt.status, "Passed");
  assert.equal(receipt.aggregate.requirementCount, 2);
  assert.equal(receipt.aggregate.passedCount, 2);
  assert.equal(receipt.aggregate.missingCount, 0);
  assert.equal(JSON.stringify(receipt).includes("apps/desktop/tests"), false);
  assert.equal("stdout" in receipt, false);
});

test("final harness rejects failed or empty executed suites", () => {
  assert.throws(() => createFinalReleaseEvidenceReceipt({
    ...validInput(), typescript: { ...validInput().typescript, exitCode: 1 },
  }), harnessError("FINAL_RELEASE_TYPESCRIPT_FAILED"));
  assert.throws(() => createFinalReleaseEvidenceReceipt({
    ...validInput(), rust: { ...validInput().rust, executedCount: 0 },
  }), harnessError("FINAL_RELEASE_RUST_FAILED"));
});

test("final harness rejects incomplete mapping stale visual and failed query gate", () => {
  assert.throws(() => createFinalReleaseEvidenceReceipt({
    ...validInput(), mappings: validInput().mappings.slice(0, 1),
  }), harnessError("FINAL_RELEASE_MAPPING_INCOMPLETE"));
  assert.throws(() => createFinalReleaseEvidenceReceipt({
    ...validInput(), visual: { ...validInput().visual, appFingerprint: hash("f") },
  }), harnessError("FINAL_RELEASE_VISUAL_FAILED"));
  assert.throws(() => createFinalReleaseEvidenceReceipt({
    ...validInput(), query: { exitCode: 2, queryCount: 10, failedCount: 1 },
  }), harnessError("FINAL_RELEASE_QUERY_FAILED"));
});

test("final release gate permits only the explicit successful sequence", () => {
  let state = transitionFinalReleaseGate("Pending", "IdentityAccepted");
  state = transitionFinalReleaseGate(state, "TypeScriptAccepted");
  state = transitionFinalReleaseGate(state, "RustAccepted");
  state = transitionFinalReleaseGate(state, "PackageAccepted");
  state = transitionFinalReleaseGate(state, "ReceiptCreated");
  state = transitionFinalReleaseGate(state, "ReceiptAccepted");
  assert.equal(state, "Passed");

  assert.throws(
    () => transitionFinalReleaseGate("IdentityVerified", "RustAccepted"),
    harnessError("FINAL_RELEASE_GATE_TRANSITION_INVALID"),
  );
  assert.throws(
    () => transitionFinalReleaseGate("Passed", "ReceiptAccepted"),
    harnessError("FINAL_RELEASE_GATE_TRANSITION_INVALID"),
  );
});

function validInput() {
  const buildIdentity = createDesktopBuildIdentity({
    sourceFingerprint: hash("a"), sourceFileCount: 95,
    appFingerprint: hash("b"), artifactCount: 5, totalArtifactBytes: 44_000_000,
    hash: sha256,
  });
  return {
    expectedRequirementIds: ["NAV-001", "DOC-028"],
    buildIdentity,
    mappings: [
      { requirementId: "NAV-001", evidenceClass: "typescript_test" as const, source: "apps/desktop/tests/example_tests.ts", testName: "shell test" },
      { requirementId: "DOC-028", evidenceClass: "rust_test" as const, source: "crates/example/tests/diff.rs", testName: "diff_test" },
    ],
    inventory: [
      { source: "apps/desktop/tests/example_tests.ts", testName: "shell test" },
      { source: "crates/example/tests/diff.rs", testName: "diff_test" },
    ],
    allowedAggregateRequirementIds: [],
    typescript: { exitCode: 0, executedCount: 583 },
    rust: { exitCode: 0, executedCount: 400 },
    packaged: {
      status: "Passed" as const,
      sourceFingerprint: buildIdentity.sourceFingerprint,
      appFingerprint: buildIdentity.appFingerprint,
      profileFingerprint: hash("c"), sampleCount: 200, p95Ms: 27,
      actionCount: 135, durableReadbackCount: 61,
      accessibilityRouteFocusCount: 6, accessibilityKeyboardJourneyCount: 6,
      accessibilityFocusRestorationCount: 6, accessibilityVisibleControlCount: 179,
      accessibilityNamedControlCount: 179, accessibilityTextZoomPercent: 200,
      attachmentRestartReadbackVerified: true as const,
      canvasTextRestartReadbackVerified: true as const,
    },
    visual: {
      status: "Passed" as const,
      sourceFingerprint: buildIdentity.sourceFingerprint,
      appFingerprint: buildIdentity.appFingerprint,
      routeViewportCount: 30, rendererViewportCount: 10, artifactCount: 40,
    },
    query: { exitCode: 0, queryCount: 10, failedCount: 0 },
    harnessVersion: "1", fixtureClass: "current-local-macos",
    startedAt: "2026-07-20T17:00:00.000Z", hash: sha256,
  } as const;
}

function harnessError(code: string): (error: unknown) => boolean {
  return (error) => error instanceof FinalReleaseEvidenceHarnessError && error.code === code;
}
