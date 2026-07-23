import {
  aggregateRequirementEvidence,
  createRequirementEvidenceRecord,
  type RequirementEvidenceAggregate,
  type RequirementEvidenceRecord,
} from "./requirement_evidence_contract.ts";

const BATCH_NAME_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/;

export type ReleaseEvidenceBatch = Readonly<{
  name: string;
  records: readonly RequirementEvidenceRecord[];
}>;

export type ReleaseEvidenceReceipt = Readonly<{
  status: "Passed" | "Failed";
  sourceFingerprint: string;
  appFingerprint: string;
  batchCount: number;
  recordCount: number;
  records: readonly RequirementEvidenceRecord[];
  aggregate: RequirementEvidenceAggregate;
}>;

export class ReleaseEvidenceReceiptError extends Error {
  readonly code: string;

  constructor(code: string) {
    super(code);
    this.name = "ReleaseEvidenceReceiptError";
    this.code = code;
  }
}

export function createReleaseEvidenceReceipt(input: Readonly<{
  expectedRequirementIds: readonly string[];
  currentSourceFingerprint: string;
  currentAppFingerprint: string;
  batches: readonly ReleaseEvidenceBatch[];
}>): ReleaseEvidenceReceipt {
  if (!Array.isArray(input.batches) || input.batches.length === 0) {
    fail("RELEASE_EVIDENCE_BATCHES_EMPTY");
  }

  const batchNames = new Set<string>();
  const records: RequirementEvidenceRecord[] = [];
  for (const batch of input.batches) {
    if (typeof batch?.name !== "string" || !BATCH_NAME_PATTERN.test(batch.name)) {
      fail("RELEASE_EVIDENCE_BATCH_NAME_INVALID");
    }
    if (batchNames.has(batch.name)) {
      fail("RELEASE_EVIDENCE_BATCH_DUPLICATE");
    }
    batchNames.add(batch.name);
    if (!Array.isArray(batch.records) || batch.records.length === 0) {
      fail("RELEASE_EVIDENCE_BATCH_EMPTY");
    }
    records.push(...batch.records.map(createRequirementEvidenceRecord));
  }

  records.sort(compareRecords);
  const immutableRecords = Object.freeze([...records]);
  const aggregate = aggregateRequirementEvidence({
    expectedRequirementIds: input.expectedRequirementIds,
    currentSourceFingerprint: input.currentSourceFingerprint,
    currentAppFingerprint: input.currentAppFingerprint,
    records: immutableRecords,
  });

  return Object.freeze({
    status: aggregate.status,
    sourceFingerprint: input.currentSourceFingerprint,
    appFingerprint: input.currentAppFingerprint,
    batchCount: input.batches.length,
    recordCount: immutableRecords.length,
    records: immutableRecords,
    aggregate,
  });
}

export function releaseEvidenceProcessExitCode(receipt: ReleaseEvidenceReceipt): 0 | 1 {
  return receipt.status === "Passed" ? 0 : 1;
}

function compareRecords(left: RequirementEvidenceRecord, right: RequirementEvidenceRecord): number {
  return left.requirementId.localeCompare(right.requirementId)
    || left.testOrJourneyName.localeCompare(right.testOrJourneyName)
    || left.startedAt.localeCompare(right.startedAt)
    || left.result.localeCompare(right.result)
    || left.sourceFingerprint.localeCompare(right.sourceFingerprint)
    || left.appFingerprint.localeCompare(right.appFingerprint)
    || left.failureCode?.localeCompare(right.failureCode ?? "")
    || 0;
}

function fail(code: string): never {
  throw new ReleaseEvidenceReceiptError(code);
}
