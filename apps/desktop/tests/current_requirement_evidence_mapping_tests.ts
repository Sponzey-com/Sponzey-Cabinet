import assert from "node:assert/strict";
import { readdir, readFile } from "node:fs/promises";
import { join } from "node:path";
import test from "node:test";

import {
  RequirementEvidenceMappingError,
  auditRequirementEvidenceMappings,
  type RequirementEvidenceMapping,
} from "../src/requirement_evidence_mapping_contract.ts";
import { createCurrentRequirementEvidenceMappings } from "../src/current_requirement_evidence_catalog.ts";
import { createPhase016RequirementCatalog } from "../src/requirement_evidence_contract.ts";

test("mapping audit accepts exact current test names and reports complete immutable coverage", async () => {
  const mappings = createCurrentRequirementEvidenceMappings();
  const result = auditRequirementEvidenceMappings({
    expectedRequirementIds: createPhase016RequirementCatalog(),
    mappings,
    inventory: await currentTestInventory(),
    allowedAggregateRequirementIds: aggregateRequirementIds(),
  });

  assert.equal(result.state, "Passed");
  assert.equal(result.requirementCount, 139);
  assert.equal(result.verifiedCount, 139);
  assert.equal(result.missingCount, 0);
  assert.equal(result.staleCount, 0);
  assert.equal(Object.isFrozen(result), true);
});

test("current document authoring evidence describes WYSIWYG and plain text editing instead of legacy modes", () => {
  const mappings = createCurrentRequirementEvidenceMappings();
  const authoringEvidence = mappings
    .filter((mapping) => mapping.requirementId.startsWith("DOC-"))
    .map((mapping) => `${mapping.source} ${mapping.testName}`)
    .join("\n");

  assert.match(authoringEvidence, /WYSIWYG/);
  assert.match(authoringEvidence, /plain text|원문/);
  assert.doesNotMatch(authoringEvidence, /source preview modes|split source preview|source\/split\/preview/i);
});

test("mapping audit fails closed for duplicate unknown and stale test claims", () => {
  const inventory = [{ source: "apps/desktop/tests/example_tests.ts", testName: "exact current behavior" }];
  const valid = mapping("NAV-001", "exact current behavior");
  assert.throws(() => auditRequirementEvidenceMappings({
    expectedRequirementIds: ["NAV-001"], mappings: [valid, valid], inventory,
    allowedAggregateRequirementIds: [],
  }), mappingError("REQUIREMENT_EVIDENCE_MAPPING_DUPLICATE"));
  assert.throws(() => auditRequirementEvidenceMappings({
    expectedRequirementIds: ["NAV-001"], mappings: [mapping("HOME-001", "exact current behavior")], inventory,
    allowedAggregateRequirementIds: [],
  }), mappingError("REQUIREMENT_EVIDENCE_MAPPING_UNKNOWN_REQUIREMENT"));

  const stale = auditRequirementEvidenceMappings({
    expectedRequirementIds: ["NAV-001"], mappings: [mapping("NAV-001", "removed behavior")], inventory,
    allowedAggregateRequirementIds: [],
  });
  assert.equal(stale.state, "GapsFound");
  assert.equal(stale.staleCount, 1);
  assert.deepEqual(stale.staleRequirementIds, ["NAV-001"]);
});

test("aggregate package visual and benchmark evidence can map only an explicit allowlist", () => {
  const aggregate: RequirementEvidenceMapping = {
    requirementId: "NAV-008", evidenceClass: "package",
    source: "phase016.package", testName: "phase016.package.initial-restart",
  };
  assert.throws(() => auditRequirementEvidenceMappings({
    expectedRequirementIds: ["NAV-008"], mappings: [aggregate], inventory: [],
    allowedAggregateRequirementIds: [],
  }), mappingError("REQUIREMENT_EVIDENCE_MAPPING_AGGREGATE_NOT_ALLOWED"));

  const accepted = auditRequirementEvidenceMappings({
    expectedRequirementIds: ["NAV-008"], mappings: [aggregate], inventory: [],
    allowedAggregateRequirementIds: ["NAV-008"],
  });
  assert.equal(accepted.state, "Passed");
});

function mapping(requirementId: string, testName: string): RequirementEvidenceMapping {
  return {
    requirementId,
    evidenceClass: "typescript_test",
    source: "apps/desktop/tests/example_tests.ts",
    testName,
  };
}

async function currentTestInventory() {
  const directory = join(process.cwd(), "apps/desktop/tests");
  const files = (await readdir(directory)).filter((file) => file.endsWith(".ts")).sort();
  const inventory: { source: string; testName: string }[] = [];
  for (const file of files) {
    const source = `apps/desktop/tests/${file}`;
    const content = await readFile(join(directory, file), "utf8");
    for (const match of content.matchAll(/test\("([^"]+)"/gu)) {
      inventory.push({ source, testName: match[1] });
    }
  }
  const rustSources = [
    "crates/cabinet-usecases/tests/compare_document_versions_tests.rs",
    "crates/cabinet-usecases/tests/document_diff_tests.rs",
  ];
  for (const source of rustSources) {
    const content = await readFile(join(process.cwd(), source), "utf8");
    for (const match of content.matchAll(/#\[test\]\s*fn\s+([A-Za-z0-9_]+)/gu)) {
      inventory.push({ source, testName: match[1] });
    }
  }
  return inventory;
}

function aggregateRequirementIds(): readonly string[] {
  return ["NAV-008", "DOC-009", "DOC-021", "GRAPH-004", "CANVAS-004", "ASSET-011", "BACKUP-011"];
}

function mappingError(code: string): (error: unknown) => boolean {
  return (error) => error instanceof RequirementEvidenceMappingError && error.code === code;
}
