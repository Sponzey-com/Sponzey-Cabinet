import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import test from "node:test";

import { createDesktopBuildIdentity } from "../src/desktop_build_identity_contract.ts";
import { createCurrentReleaseEvidenceReceipt } from "../src/current_release_evidence_harness.ts";
import { RequirementEvidenceError } from "../src/requirement_evidence_contract.ts";

const sha256 = (value: string): string => createHash("sha256").update(value).digest("hex");
const hash = (character: string): string => character.repeat(64);

test("current release harness combines package visual and query exact coverage", () => {
  const receipt = createCurrentReleaseEvidenceReceipt(validInput());

  assert.equal(receipt.status, "Passed");
  assert.equal(receipt.batchCount, 3);
  assert.equal(receipt.recordCount, 3);
  assert.equal(receipt.aggregate.missingCount, 0);
  assert.equal(receipt.aggregate.duplicateCount, 0);
  assert.deepEqual(receipt.records.map((record) => record.requirementId), [
    "DOC-001", "NAV-001", "SEARCH-001",
  ]);
  assert.equal("stdout" in receipt, false);
  assert.equal("query" in receipt, false);
});

test("current release harness preserves missing and duplicate requirement mapping failures", () => {
  const missing = createCurrentReleaseEvidenceReceipt({
    ...validInput(),
    expectedRequirementIds: ["NAV-001", "SEARCH-001", "DOC-001", "HOME-001"],
  });
  assert.equal(missing.status, "Failed");
  assert.deepEqual(missing.aggregate.missingRequirementIds, ["HOME-001"]);

  const duplicate = createCurrentReleaseEvidenceReceipt({
    ...validInput(),
    packaged: { ...validInput().packaged, requirementIds: ["DOC-001", "NAV-001"] },
  });
  assert.equal(duplicate.status, "Failed");
  assert.equal(duplicate.aggregate.duplicateCount, 1);
});

test("current release harness rejects stale visual fingerprint and failed query assertions", () => {
  const stale = validInput();
  assert.throws(() => createCurrentReleaseEvidenceReceipt({
    ...stale,
    visual: {
      ...stale.visual,
      evidence: { ...stale.visual.evidence, appFingerprint: hash("f") },
    },
  }), evidenceError("REQUIREMENT_EVIDENCE_FINGERPRINT_MISMATCH"));

  const failed = validInput();
  assert.throws(() => createCurrentReleaseEvidenceReceipt({
    ...failed,
    queries: [{ ...failed.queries[0]!, processExitCode: 1 }],
  }), evidenceError("REQUIREMENT_EVIDENCE_QUERY_ASSERTION_FAILED"));
});

function validInput() {
  const buildIdentity = createDesktopBuildIdentity({
    sourceFingerprint: hash("a"), sourceFileCount: 89,
    appFingerprint: hash("b"), artifactCount: 3, totalArtifactBytes: 3_259_348,
    hash: sha256,
  });
  return {
    expectedRequirementIds: ["NAV-001", "SEARCH-001", "DOC-001"],
    buildIdentity,
    harnessVersion: "1",
    fixtureClass: "clean-profile",
    startedAt: "2026-07-20T00:00:00.000Z",
    hash: sha256,
    packaged: {
      requirementIds: ["DOC-001"],
      testOrJourneyName: "phase016.package.current",
      evidence: {
        status: "Passed" as const,
        sourceFingerprint: buildIdentity.sourceFingerprint,
        appFingerprint: buildIdentity.appFingerprint,
        profileFingerprint: hash("d"),
        sampleCount: 200,
        p95Ms: 34,
        actionCount: 135,
        durableReadbackCount: 61,
        accessibilityRouteFocusCount: 6,
        accessibilityKeyboardJourneyCount: 6,
        accessibilityFocusRestorationCount: 6,
        accessibilityVisibleControlCount: 84,
        accessibilityNamedControlCount: 84,
        accessibilityTextZoomPercent: 200,
        attachmentRestartReadbackVerified: true as const,
        canvasTextRestartReadbackVerified: true as const,
      },
    },
    visual: {
      requirementIds: ["NAV-001"],
      testOrJourneyName: "phase016.visual.current",
      evidence: {
        status: "Passed" as const,
        sourceFingerprint: buildIdentity.sourceFingerprint,
        appFingerprint: buildIdentity.appFingerprint,
        routeViewportCount: 30,
        rendererViewportCount: 10,
        artifactCount: 40,
      },
    },
    queries: [{
      requirementIds: ["SEARCH-001"],
      testOrJourneyName: "phase016.query.current",
      processExitCode: 0,
      measurement: {
        queryId: "workspace-search",
        markerMatched: true,
        resultCountMatched: true,
        errorCount: 0,
        p95Ms: 34,
        budgetMs: 300,
        sampleCount: 200,
        expectedSampleCount: 200,
      },
    }],
  } as const;
}

function evidenceError(code: string): (error: unknown) => boolean {
  return (error) => error instanceof RequirementEvidenceError && error.code === code;
}
