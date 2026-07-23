const SHA256_PATTERN = /^[a-f0-9]{64}$/;
const REQUIREMENT_PATTERN = /^(NAV|HOME|SEARCH|DOC|GRAPH|CANVAS|ASSET|BACKUP)-(\d{3})$/;
const STABLE_NAME_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/;
const FAILURE_CODE_PATTERN = /^[A-Z][A-Z0-9_]{2,79}$/;

const REQUIREMENT_RANGES = Object.freeze([
  ["NAV", 10],
  ["HOME", 10],
  ["SEARCH", 12],
  ["DOC", 34],
  ["GRAPH", 17],
  ["CANVAS", 25],
  ["ASSET", 18],
  ["BACKUP", 13],
] as const);

export type RequirementEvidenceResult = "Passed" | "Failed" | "Blocked";

export interface RequirementEvidenceRecordInput {
  readonly requirementId: string;
  readonly sourceFingerprint: string;
  readonly appFingerprint: string;
  readonly testOrJourneyName: string;
  readonly harnessVersion: string;
  readonly fixtureClass: string;
  readonly startedAt: string;
  readonly result: RequirementEvidenceResult;
  readonly measuredValues: Readonly<Record<string, number | boolean>>;
  readonly artifactDigests: readonly string[];
  readonly failureCode?: string;
}

export type RequirementEvidenceRecord = Readonly<{
  requirementId: string;
  sourceFingerprint: string;
  appFingerprint: string;
  testOrJourneyName: string;
  harnessVersion: string;
  fixtureClass: string;
  startedAt: string;
  result: RequirementEvidenceResult;
  measuredValues: Readonly<Record<string, number | boolean>>;
  artifactDigests: readonly string[];
  failureCode?: string;
}>;

export type RequirementEvidenceAggregate = Readonly<{
  status: "Passed" | "Failed";
  requirementCount: number;
  passedCount: number;
  missingCount: number;
  staleCount: number;
  duplicateCount: number;
  failedCount: number;
  blockedCount: number;
  contradictoryCount: number;
  missingRequirementIds: readonly string[];
}>;

export class RequirementEvidenceError extends Error {
  readonly code: string;

  constructor(code: string) {
    super(code);
    this.name = "RequirementEvidenceError";
    this.code = code;
  }
}

const fail = (code: string): never => {
  throw new RequirementEvidenceError(code);
};

export function createPhase016RequirementCatalog(): readonly string[] {
  return Object.freeze(REQUIREMENT_RANGES.flatMap(([namespace, maximum]) =>
    Array.from({ length: maximum }, (_, index) => `${namespace}-${String(index + 1).padStart(3, "0")}`)
  ));
}

export function createRequirementEvidenceRecord(
  input: RequirementEvidenceRecordInput,
): RequirementEvidenceRecord {
  const allowedFields = new Set([
    "requirementId", "sourceFingerprint", "appFingerprint", "testOrJourneyName",
    "harnessVersion", "fixtureClass", "startedAt", "result", "measuredValues",
    "artifactDigests", "failureCode",
  ]);
  if (!isPlainObject(input) || Object.keys(input).some((key) => !allowedFields.has(key))) {
    fail("REQUIREMENT_EVIDENCE_FIELD_UNEXPECTED");
  }
  validateRequirementId(input.requirementId);
  validateFingerprint(input.sourceFingerprint);
  validateFingerprint(input.appFingerprint);
  for (const value of [input.testOrJourneyName, input.harnessVersion, input.fixtureClass]) {
    if (typeof value !== "string" || !STABLE_NAME_PATTERN.test(value)) {
      fail("REQUIREMENT_EVIDENCE_NAME_INVALID");
    }
  }
  if (typeof input.startedAt !== "string"
    || Number.isNaN(Date.parse(input.startedAt))
    || new Date(input.startedAt).toISOString() !== input.startedAt) {
    fail("REQUIREMENT_EVIDENCE_TIME_INVALID");
  }
  if (!(["Passed", "Failed", "Blocked"] as const).includes(input.result)) {
    fail("REQUIREMENT_EVIDENCE_RESULT_INVALID");
  }
  if (input.result === "Passed" && input.failureCode !== undefined) {
    fail("REQUIREMENT_EVIDENCE_FAILURE_CODE_INVALID");
  }
  if (input.result !== "Passed"
    && (typeof input.failureCode !== "string" || !FAILURE_CODE_PATTERN.test(input.failureCode))) {
    fail("REQUIREMENT_EVIDENCE_FAILURE_CODE_INVALID");
  }
  if (!isPlainObject(input.measuredValues)) {
    fail("REQUIREMENT_EVIDENCE_MEASUREMENT_INVALID");
  }
  const measuredValues: Record<string, number | boolean> = {};
  for (const [key, value] of Object.entries(input.measuredValues)) {
    if (!/^[a-z][A-Za-z0-9]{0,63}$/.test(key)
      || (typeof value !== "boolean" && (typeof value !== "number" || !Number.isFinite(value)))) {
      fail("REQUIREMENT_EVIDENCE_MEASUREMENT_INVALID");
    }
    measuredValues[key] = value;
  }
  if (!Array.isArray(input.artifactDigests)
    || input.artifactDigests.some((digest) => typeof digest !== "string" || !SHA256_PATTERN.test(digest))
    || new Set(input.artifactDigests).size !== input.artifactDigests.length) {
    fail("REQUIREMENT_EVIDENCE_ARTIFACT_INVALID");
  }
  return Object.freeze({
    requirementId: input.requirementId,
    sourceFingerprint: input.sourceFingerprint,
    appFingerprint: input.appFingerprint,
    testOrJourneyName: input.testOrJourneyName,
    harnessVersion: input.harnessVersion,
    fixtureClass: input.fixtureClass,
    startedAt: input.startedAt,
    result: input.result,
    measuredValues: Object.freeze(measuredValues),
    artifactDigests: Object.freeze([...input.artifactDigests]),
    ...(input.failureCode === undefined ? {} : { failureCode: input.failureCode }),
  });
}

export function aggregateRequirementEvidence(input: Readonly<{
  expectedRequirementIds: readonly string[];
  currentSourceFingerprint: string;
  currentAppFingerprint: string;
  records: readonly RequirementEvidenceRecord[];
}>): RequirementEvidenceAggregate {
  validateFingerprint(input.currentSourceFingerprint);
  validateFingerprint(input.currentAppFingerprint);
  if (input.expectedRequirementIds.length === 0) fail("REQUIREMENT_EVIDENCE_EXPECTED_EMPTY");
  input.expectedRequirementIds.forEach(validateRequirementId);
  if (new Set(input.expectedRequirementIds).size !== input.expectedRequirementIds.length) {
    fail("REQUIREMENT_EVIDENCE_EXPECTED_DUPLICATE");
  }
  const expected = new Set(input.expectedRequirementIds);
  const records = input.records.map((record) => createRequirementEvidenceRecord(record));
  if (records.some((record) => !expected.has(record.requirementId))) {
    fail("REQUIREMENT_EVIDENCE_REQUIREMENT_UNEXPECTED");
  }
  const staleCount = records.filter((record) =>
    record.sourceFingerprint !== input.currentSourceFingerprint
      || record.appFingerprint !== input.currentAppFingerprint
  ).length;
  const currentRecords = records.filter((record) =>
    record.sourceFingerprint === input.currentSourceFingerprint
      && record.appFingerprint === input.currentAppFingerprint
  );
  const byRequirement = new Map<string, RequirementEvidenceRecord[]>();
  for (const record of currentRecords) {
    const grouped = byRequirement.get(record.requirementId) ?? [];
    grouped.push(record);
    byRequirement.set(record.requirementId, grouped);
  }
  const missingRequirementIds = input.expectedRequirementIds.filter((id) => !byRequirement.has(id));
  let passedCount = 0;
  let duplicateCount = 0;
  let failedCount = 0;
  let blockedCount = 0;
  let contradictoryCount = 0;
  for (const grouped of byRequirement.values()) {
    if (grouped.length > 1) duplicateCount += 1;
    const results = new Set(grouped.map((record) => record.result));
    if (results.size > 1) contradictoryCount += 1;
    if (grouped.some((record) => record.result === "Failed")) failedCount += 1;
    if (grouped.some((record) => record.result === "Blocked")) blockedCount += 1;
    if (grouped.length === 1 && grouped[0]?.result === "Passed") passedCount += 1;
  }
  const failureTotal = missingRequirementIds.length + staleCount + duplicateCount
    + failedCount + blockedCount + contradictoryCount;
  return Object.freeze({
    status: failureTotal === 0 && passedCount === input.expectedRequirementIds.length ? "Passed" : "Failed",
    requirementCount: input.expectedRequirementIds.length,
    passedCount,
    missingCount: missingRequirementIds.length,
    staleCount,
    duplicateCount,
    failedCount,
    blockedCount,
    contradictoryCount,
    missingRequirementIds: Object.freeze([...missingRequirementIds]),
  });
}

function validateRequirementId(value: string): void {
  const match = typeof value === "string" ? REQUIREMENT_PATTERN.exec(value) : null;
  if (!match) fail("REQUIREMENT_EVIDENCE_REQUIREMENT_INVALID");
  const range = REQUIREMENT_RANGES.find(([namespace]) => namespace === match[1]);
  const ordinal = Number(match[2]);
  if (!range || ordinal < 1 || ordinal > range[1]) {
    fail("REQUIREMENT_EVIDENCE_REQUIREMENT_INVALID");
  }
}

function validateFingerprint(value: string): void {
  if (typeof value !== "string" || !SHA256_PATTERN.test(value)) {
    fail("REQUIREMENT_EVIDENCE_FINGERPRINT_INVALID");
  }
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    && (Object.getPrototypeOf(value) === Object.prototype || Object.getPrototypeOf(value) === null);
}
