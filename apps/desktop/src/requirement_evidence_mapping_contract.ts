export type RequirementEvidenceClass = "typescript_test" | "rust_test" | "package" | "visual" | "benchmark";

export type RequirementEvidenceMapping = Readonly<{
  requirementId: string;
  evidenceClass: RequirementEvidenceClass;
  source: string;
  testName: string;
}>;

export type RequirementTestInventoryEntry = Readonly<{
  source: string;
  testName: string;
}>;

export class RequirementEvidenceMappingError extends Error {
  readonly code: string;

  constructor(code: string) {
    super(code);
    this.name = "RequirementEvidenceMappingError";
    this.code = code;
  }
}

export function auditRequirementEvidenceMappings(input: Readonly<{
  expectedRequirementIds: readonly string[];
  mappings: readonly RequirementEvidenceMapping[];
  inventory: readonly RequirementTestInventoryEntry[];
  allowedAggregateRequirementIds: readonly string[];
}>): Readonly<{
  state: "Passed" | "GapsFound";
  requirementCount: number;
  verifiedCount: number;
  missingCount: number;
  staleCount: number;
  missingRequirementIds: readonly string[];
  staleRequirementIds: readonly string[];
}> {
  const expected = validateUniqueIds(input.expectedRequirementIds, "REQUIREMENT_EVIDENCE_MAPPING_EXPECTED_INVALID");
  const allowedAggregate = input.allowedAggregateRequirementIds.length === 0
    ? Object.freeze([] as string[])
    : validateUniqueIds(
        input.allowedAggregateRequirementIds,
        "REQUIREMENT_EVIDENCE_MAPPING_AGGREGATE_ALLOWLIST_INVALID",
      );
  const expectedSet = new Set(expected);
  if (allowedAggregate.some((requirementId) => !expectedSet.has(requirementId))) {
    fail("REQUIREMENT_EVIDENCE_MAPPING_AGGREGATE_ALLOWLIST_INVALID");
  }

  const inventory = new Set(input.inventory.map((entry) => inventoryKey(entry.source, entry.testName)));
  const mapped = new Map<string, RequirementEvidenceMapping>();
  const stale: string[] = [];
  for (const candidate of input.mappings) {
    const mapping = validateMapping(candidate);
    if (!expectedSet.has(mapping.requirementId)) fail("REQUIREMENT_EVIDENCE_MAPPING_UNKNOWN_REQUIREMENT");
    if (mapped.has(mapping.requirementId)) fail("REQUIREMENT_EVIDENCE_MAPPING_DUPLICATE");
    mapped.set(mapping.requirementId, mapping);
    if (mapping.evidenceClass === "typescript_test" || mapping.evidenceClass === "rust_test") {
      if (!inventory.has(inventoryKey(mapping.source, mapping.testName))) stale.push(mapping.requirementId);
    } else if (!allowedAggregate.includes(mapping.requirementId)) {
      fail("REQUIREMENT_EVIDENCE_MAPPING_AGGREGATE_NOT_ALLOWED");
    }
  }

  const staleSet = new Set(stale);
  const missing = expected.filter((requirementId) => !mapped.has(requirementId));
  const verifiedCount = expected.length - missing.length - staleSet.size;
  return Object.freeze({
    state: missing.length === 0 && stale.length === 0 ? "Passed" : "GapsFound",
    requirementCount: expected.length,
    verifiedCount,
    missingCount: missing.length,
    staleCount: stale.length,
    missingRequirementIds: Object.freeze(missing),
    staleRequirementIds: Object.freeze([...stale].sort()),
  });
}

function validateMapping(value: RequirementEvidenceMapping): RequirementEvidenceMapping {
  if (!value || typeof value !== "object"
    || !/^(NAV|HOME|SEARCH|DOC|GRAPH|CANVAS|ASSET|BACKUP)-\d{3}$/u.test(value.requirementId)
    || !["typescript_test", "rust_test", "package", "visual", "benchmark"].includes(value.evidenceClass)
    || !isSafeName(value.testName)) {
    fail("REQUIREMENT_EVIDENCE_MAPPING_INVALID");
  }
  if (value.evidenceClass === "typescript_test"
    && (!/^apps\/desktop\/tests\/[A-Za-z0-9_.-]+\.ts$/u.test(value.source))) {
    fail("REQUIREMENT_EVIDENCE_MAPPING_SOURCE_INVALID");
  }
  if (value.evidenceClass === "rust_test"
    && !/^(?:crates|apps)\/[A-Za-z0-9_./-]+\.rs$/u.test(value.source)) {
    fail("REQUIREMENT_EVIDENCE_MAPPING_SOURCE_INVALID");
  }
  if (!value.evidenceClass.endsWith("_test") && !/^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/u.test(value.source)) {
    fail("REQUIREMENT_EVIDENCE_MAPPING_SOURCE_INVALID");
  }
  return Object.freeze({ ...value });
}

function validateUniqueIds(values: readonly string[], code: string): readonly string[] {
  if (!Array.isArray(values) || values.length === 0
    || values.some((value) => !/^(NAV|HOME|SEARCH|DOC|GRAPH|CANVAS|ASSET|BACKUP)-\d{3}$/u.test(value))
    || new Set(values).size !== values.length) {
    fail(code);
  }
  return Object.freeze([...values]);
}

function inventoryKey(source: string, testName: string): string {
  return `${source}\u0000${testName}`;
}

function isSafeName(value: unknown): value is string {
  return typeof value === "string" && value.length > 0 && value.length <= 240 && !/[\r\n\u0000]/u.test(value);
}

function fail(code: string): never {
  throw new RequirementEvidenceMappingError(code);
}
