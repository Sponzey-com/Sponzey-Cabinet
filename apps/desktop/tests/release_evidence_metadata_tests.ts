import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import test from "node:test";

import { createDesktopBuildIdentity } from "../src/desktop_build_identity_contract.ts";
import { RequirementEvidenceError } from "../src/requirement_evidence_contract.ts";
import {
  createReleaseEvidenceMetadata,
  mapVisualEvidence,
} from "../src/release_evidence_mapper.ts";

const sha256 = (value: string): string => createHash("sha256").update(value).digest("hex");
const hash = (character: string): string => character.repeat(64);

test("release metadata is created only from a validated immutable build identity", () => {
  const identity = buildIdentity();
  const metadata = createReleaseEvidenceMetadata({
    buildIdentity: identity,
    testOrJourneyName: "phase016.visual.current",
    harnessVersion: "1",
    fixtureClass: "clean-profile",
    startedAt: "2026-07-20T00:00:00.000Z",
    hash: sha256,
  });

  assert.equal(metadata.sourceFingerprint, identity.sourceFingerprint);
  assert.equal(metadata.appFingerprint, identity.appFingerprint);
  assert.equal(metadata.buildIdentityFingerprint, identity.identityFingerprint);
  assert.equal(metadata.sourceFileCount, identity.sourceFileCount);
  assert.equal(metadata.artifactCount, identity.artifactCount);
  assert.equal(Object.isFrozen(metadata), true);
});

test("release metadata rejects a tampered combined build identity", () => {
  assert.throws(() => createReleaseEvidenceMetadata({
    buildIdentity: { ...buildIdentity(), identityFingerprint: hash("f") },
    testOrJourneyName: "phase016.visual.current",
    harnessVersion: "1",
    fixtureClass: "clean-profile",
    startedAt: "2026-07-20T00:00:00.000Z",
    hash: sha256,
  }), evidenceError("REQUIREMENT_EVIDENCE_BUILD_IDENTITY_MISMATCH"));
});

test("release mapper rejects forged plain metadata", () => {
  const identity = buildIdentity();
  assert.throws(() => mapVisualEvidence({
    requirementIds: ["NAV-001"],
    metadata: {
      sourceFingerprint: identity.sourceFingerprint,
      appFingerprint: identity.appFingerprint,
      testOrJourneyName: "phase016.visual.current",
      harnessVersion: "1",
      fixtureClass: "clean-profile",
      startedAt: "2026-07-20T00:00:00.000Z",
    } as never,
    evidence: {
      status: "Passed",
      sourceFingerprint: identity.sourceFingerprint,
      appFingerprint: identity.appFingerprint,
      routeViewportCount: 30,
      rendererViewportCount: 10,
      artifactCount: 40,
    },
  }), evidenceError("REQUIREMENT_EVIDENCE_METADATA_UNVERIFIED"));
});

test("validated metadata maps evidence without leaking build-only fields into records", () => {
  const identity = buildIdentity();
  const metadata = createReleaseEvidenceMetadata({
    buildIdentity: identity,
    testOrJourneyName: "phase016.visual.current",
    harnessVersion: "1",
    fixtureClass: "clean-profile",
    startedAt: "2026-07-20T00:00:00.000Z",
    hash: sha256,
  });
  const [record] = mapVisualEvidence({
    requirementIds: ["NAV-001"],
    metadata,
    evidence: {
      status: "Passed",
      sourceFingerprint: identity.sourceFingerprint,
      appFingerprint: identity.appFingerprint,
      routeViewportCount: 30,
      rendererViewportCount: 10,
      artifactCount: 40,
    },
  });

  assert.equal(record?.sourceFingerprint, identity.sourceFingerprint);
  assert.equal("buildIdentityFingerprint" in (record ?? {}), false);
  assert.equal("sourceFileCount" in (record ?? {}), false);
});

function buildIdentity() {
  return createDesktopBuildIdentity({
    sourceFingerprint: hash("a"),
    sourceFileCount: 89,
    appFingerprint: hash("b"),
    artifactCount: 3,
    totalArtifactBytes: 3_259_348,
    hash: sha256,
  });
}

function evidenceError(code: string): (error: unknown) => boolean {
  return (error) => error instanceof RequirementEvidenceError && error.code === code;
}
