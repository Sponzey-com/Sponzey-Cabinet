import assert from "node:assert/strict";
import test from "node:test";

import {
  createRequirementEvidenceRecord,
  type RequirementEvidenceRecord,
} from "../src/requirement_evidence_contract.ts";
import {
  ReleaseEvidenceReceiptError,
  createReleaseEvidenceReceipt,
  releaseEvidenceProcessExitCode,
} from "../src/release_evidence_receipt.ts";

const fingerprint = (character: string): string => character.repeat(64);
const sourceFingerprint = fingerprint("a");
const appFingerprint = fingerprint("b");

test("receipt merges named batches into deterministic current coverage", () => {
  const receipt = createReleaseEvidenceReceipt({
    expectedRequirementIds: ["NAV-001", "DOC-001", "SEARCH-001"],
    currentSourceFingerprint: sourceFingerprint,
    currentAppFingerprint: appFingerprint,
    batches: [
      { name: "visual", records: [passed("NAV-001")] },
      { name: "package", records: [passed("DOC-001")] },
      { name: "query", records: [passed("SEARCH-001")] },
    ],
  });

  assert.equal(receipt.status, "Passed");
  assert.equal(receipt.batchCount, 3);
  assert.equal(receipt.recordCount, 3);
  assert.deepEqual(receipt.records.map((record) => record.requirementId), [
    "DOC-001", "NAV-001", "SEARCH-001",
  ]);
  assert.equal(receipt.sourceFingerprint, sourceFingerprint);
  assert.equal(receipt.appFingerprint, appFingerprint);
  assert.equal(receipt.aggregate.missingCount, 0);
  assert.equal(Object.isFrozen(receipt), true);
  assert.equal(Object.isFrozen(receipt.records), true);
  assert.equal(releaseEvidenceProcessExitCode(receipt), 0);
});

test("receipt fails closed and preserves missing stale and duplicate counts", () => {
  const receipt = createReleaseEvidenceReceipt({
    expectedRequirementIds: ["NAV-001", "DOC-001", "SEARCH-001"],
    currentSourceFingerprint: sourceFingerprint,
    currentAppFingerprint: appFingerprint,
    batches: [
      { name: "package", records: [passed("DOC-001"), passed("DOC-001")] },
      {
        name: "query",
        records: [passed("SEARCH-001", { sourceFingerprint: fingerprint("c") })],
      },
    ],
  });

  assert.equal(receipt.status, "Failed");
  assert.equal(receipt.aggregate.missingCount, 2);
  assert.equal(receipt.aggregate.staleCount, 1);
  assert.equal(receipt.aggregate.duplicateCount, 1);
  assert.deepEqual(receipt.aggregate.missingRequirementIds, ["NAV-001", "SEARCH-001"]);
  assert.equal(releaseEvidenceProcessExitCode(receipt), 1);
});

test("receipt preserves failed blocked and contradictory evidence", () => {
  const receipt = createReleaseEvidenceReceipt({
    expectedRequirementIds: ["DOC-001", "BACKUP-001"],
    currentSourceFingerprint: sourceFingerprint,
    currentAppFingerprint: appFingerprint,
    batches: [
      {
        name: "package",
        records: [
          passed("DOC-001"),
          passed("DOC-001", { result: "Failed", failureCode: "DURABLE_READBACK_FAILED" }),
          passed("BACKUP-001", { result: "Blocked", failureCode: "PROFILE_UNAVAILABLE" }),
        ],
      },
    ],
  });

  assert.equal(receipt.status, "Failed");
  assert.equal(receipt.aggregate.failedCount, 1);
  assert.equal(receipt.aggregate.blockedCount, 1);
  assert.equal(receipt.aggregate.contradictoryCount, 1);
  assert.equal(receipt.aggregate.duplicateCount, 1);
});

test("receipt rejects empty and duplicate named batches", () => {
  assert.throws(() => createReleaseEvidenceReceipt({
    expectedRequirementIds: ["NAV-001"],
    currentSourceFingerprint: sourceFingerprint,
    currentAppFingerprint: appFingerprint,
    batches: [{ name: "visual", records: [] }],
  }), receiptError("RELEASE_EVIDENCE_BATCH_EMPTY"));

  assert.throws(() => createReleaseEvidenceReceipt({
    expectedRequirementIds: ["NAV-001"],
    currentSourceFingerprint: sourceFingerprint,
    currentAppFingerprint: appFingerprint,
    batches: [
      { name: "visual", records: [passed("NAV-001")] },
      { name: "visual", records: [passed("NAV-001")] },
    ],
  }), receiptError("RELEASE_EVIDENCE_BATCH_DUPLICATE"));
});

test("receipt ordering is independent of batch input order", () => {
  const input = {
    expectedRequirementIds: ["NAV-001", "DOC-001"],
    currentSourceFingerprint: sourceFingerprint,
    currentAppFingerprint: appFingerprint,
  } as const;
  const first = createReleaseEvidenceReceipt({
    ...input,
    batches: [
      { name: "visual", records: [passed("NAV-001")] },
      { name: "package", records: [passed("DOC-001")] },
    ],
  });
  const second = createReleaseEvidenceReceipt({
    ...input,
    batches: [
      { name: "package", records: [passed("DOC-001")] },
      { name: "visual", records: [passed("NAV-001")] },
    ],
  });

  assert.deepEqual(first.records, second.records);
  assert.deepEqual(first.aggregate, second.aggregate);
});

function passed(
  requirementId: string,
  overrides: Partial<RequirementEvidenceRecord> = {},
): RequirementEvidenceRecord {
  return createRequirementEvidenceRecord({
    requirementId,
    sourceFingerprint,
    appFingerprint,
    testOrJourneyName: "phase016.focused",
    harnessVersion: "1",
    fixtureClass: "clean-profile",
    startedAt: "2026-07-20T00:00:00.000Z",
    result: "Passed",
    measuredValues: { errorCount: 0 },
    artifactDigests: [],
    ...overrides,
  });
}

function receiptError(code: string): (error: unknown) => boolean {
  return (error) => error instanceof ReleaseEvidenceReceiptError && error.code === code;
}
