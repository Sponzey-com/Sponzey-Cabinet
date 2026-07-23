import assert from "node:assert/strict";
import test from "node:test";

import {
  RequirementEvidenceError,
  aggregateRequirementEvidence,
  createPhase016RequirementCatalog,
  createRequirementEvidenceRecord,
} from "../src/requirement_evidence_contract.ts";
import {
  createReleaseEvidenceMetadata,
  mapPackagedSmokeEvidence,
  mapQueryRenderEvidence,
  mapVisualEvidence,
} from "../src/release_evidence_mapper.ts";
import { createDesktopBuildIdentity } from "../src/desktop_build_identity_contract.ts";

const hash = (digit: string): string => digit.repeat(64);
const identityHash = (): string => hash("e");
const buildIdentity = createDesktopBuildIdentity({
  sourceFingerprint: hash("a"), sourceFileCount: 89,
  appFingerprint: hash("b"), artifactCount: 3, totalArtifactBytes: 3_259_348,
  hash: identityHash,
});
const metadata = createReleaseEvidenceMetadata({
  buildIdentity,
  testOrJourneyName: "phase016.package.initial-restart",
  harnessVersion: "1",
  fixtureClass: "clean-profile-2-documents",
  startedAt: "2026-07-20T00:00:00.000Z",
  hash: identityHash,
});
const recordMetadata = Object.freeze({
  sourceFingerprint: metadata.sourceFingerprint,
  appFingerprint: metadata.appFingerprint,
  testOrJourneyName: metadata.testOrJourneyName,
  harnessVersion: metadata.harnessVersion,
  fixtureClass: metadata.fixtureClass,
  startedAt: metadata.startedAt,
});

test("phase requirement catalog is complete, ordered, and immutable", () => {
  const catalog = createPhase016RequirementCatalog();
  assert.equal(catalog.length, 139);
  assert.deepEqual(catalog.slice(0, 3), ["NAV-001", "NAV-002", "NAV-003"]);
  assert.deepEqual(catalog.slice(-3), ["BACKUP-011", "BACKUP-012", "BACKUP-013"]);
  assert.equal(new Set(catalog).size, catalog.length);
  assert.equal(Object.isFrozen(catalog), true);
});

test("record accepts only stable safe fields and immutable numeric measurements", () => {
  const record = createRequirementEvidenceRecord({
    requirementId: "SEARCH-001",
    ...recordMetadata,
    result: "Passed",
    measuredValues: { sampleCount: 200, p95Ms: 34, markerMatched: true },
    artifactDigests: [hash("c")],
  });
  assert.deepEqual(record.measuredValues, { sampleCount: 200, p95Ms: 34, markerMatched: true });
  assert.equal(Object.isFrozen(record), true);
  assert.equal(Object.isFrozen(record.measuredValues), true);

  assert.throws(() => createRequirementEvidenceRecord({
    requirementId: "SEARCH-001",
    ...recordMetadata,
    result: "Passed",
    measuredValues: { rawQuery: "private content" },
    artifactDigests: [],
  } as never), evidenceError("REQUIREMENT_EVIDENCE_MEASUREMENT_INVALID"));
  assert.throws(() => createRequirementEvidenceRecord({
    requirementId: "SEARCH-001",
    ...recordMetadata,
    result: "Passed",
    measuredValues: {},
    artifactDigests: [],
    path: "/Users/private/workspace",
  } as never), evidenceError("REQUIREMENT_EVIDENCE_FIELD_UNEXPECTED"));
});

test("aggregate passes only exact current fingerprint coverage", () => {
  const expected = ["SEARCH-001", "DOC-001"] as const;
  const records = expected.map((requirementId) => createRequirementEvidenceRecord({
    requirementId,
    ...recordMetadata,
    result: "Passed",
    measuredValues: { errorCount: 0 },
    artifactDigests: [],
  }));
  assert.deepEqual(aggregateRequirementEvidence({
    expectedRequirementIds: expected,
    currentSourceFingerprint: metadata.sourceFingerprint,
    currentAppFingerprint: metadata.appFingerprint,
    records,
  }), {
    status: "Passed",
    requirementCount: 2,
    passedCount: 2,
    missingCount: 0,
    staleCount: 0,
    duplicateCount: 0,
    failedCount: 0,
    blockedCount: 0,
    contradictoryCount: 0,
    missingRequirementIds: [],
  });
});

test("aggregate separates missing stale duplicate failed blocked and contradictory evidence", () => {
  const passed = (requirementId: string, overrides: Record<string, unknown> = {}) =>
    createRequirementEvidenceRecord({
      requirementId,
      ...recordMetadata,
      result: "Passed",
      measuredValues: {},
      artifactDigests: [],
      ...overrides,
    } as never);
  const records = [
    passed("SEARCH-001"),
    passed("SEARCH-001", { result: "Failed", failureCode: "SEARCH_RESULT_MISMATCH" }),
    passed("DOC-001", { result: "Blocked", failureCode: "FIXTURE_UNAVAILABLE" }),
    passed("HOME-001", { sourceFingerprint: hash("c") }),
  ];
  const aggregate = aggregateRequirementEvidence({
    expectedRequirementIds: ["SEARCH-001", "DOC-001", "HOME-001", "GRAPH-001"],
    currentSourceFingerprint: metadata.sourceFingerprint,
    currentAppFingerprint: metadata.appFingerprint,
    records,
  });
  assert.deepEqual(aggregate, {
    status: "Failed",
    requirementCount: 4,
    passedCount: 0,
    missingCount: 2,
    staleCount: 1,
    duplicateCount: 1,
    failedCount: 1,
    blockedCount: 1,
    contradictoryCount: 1,
    missingRequirementIds: ["HOME-001", "GRAPH-001"],
  });
});

test("release mappers preserve assertions and never infer Passed from process exit alone", () => {
  const packageRecords = mapPackagedSmokeEvidence({
    requirementIds: ["DOC-001", "BACKUP-001"],
    metadata,
    evidence: {
      status: "Passed",
      sourceFingerprint: metadata.sourceFingerprint,
      appFingerprint: metadata.appFingerprint,
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
      attachmentRestartReadbackVerified: true,
      canvasTextRestartReadbackVerified: true,
    },
  });
  assert.equal(packageRecords.length, 2);
  assert.deepEqual(packageRecords[0]?.measuredValues, {
    sampleCount: 200,
    p95Ms: 34,
    actionCount: 135,
    durableReadbackCount: 61,
    attachmentRestartReadbackVerified: true,
    canvasTextRestartReadbackVerified: true,
    accessibilityRouteFocusCount: 6,
    accessibilityKeyboardJourneyCount: 6,
    accessibilityFocusRestorationCount: 6,
    accessibilityVisibleControlCount: 84,
    accessibilityNamedControlCount: 84,
    accessibilityTextZoomPercent: 200,
  });

  const visualRecords = mapVisualEvidence({
    requirementIds: ["NAV-001"],
    metadata,
    evidence: {
      status: "Passed",
      sourceFingerprint: metadata.sourceFingerprint,
      appFingerprint: metadata.appFingerprint,
      routeViewportCount: 30,
      rendererViewportCount: 10,
      artifactCount: 40,
    },
  });
  assert.equal(visualRecords[0]?.measuredValues.artifactCount, 40);

  assert.throws(() => mapQueryRenderEvidence({
    requirementIds: ["SEARCH-001"],
    metadata,
    measurement: {
      queryId: "search",
      markerMatched: false,
      resultCountMatched: true,
      errorCount: 0,
      p95Ms: 20,
      budgetMs: 300,
      sampleCount: 200,
      expectedSampleCount: 200,
    },
    processExitCode: 0,
  }), evidenceError("REQUIREMENT_EVIDENCE_QUERY_ASSERTION_FAILED"));
});

function evidenceError(code: string): (error: unknown) => boolean {
  return (error) => error instanceof RequirementEvidenceError && error.code === code;
}
