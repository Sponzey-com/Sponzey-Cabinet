import type { DesktopBuildIdentity } from "./desktop_build_identity_contract.ts";
import type { PackagedSmokeEvidence } from "./packaged_smoke_evidence_contract.ts";
import {
  auditRequirementEvidenceMappings,
  type RequirementEvidenceMapping,
  type RequirementTestInventoryEntry,
} from "./requirement_evidence_mapping_contract.ts";
import { createReleaseEvidenceMetadata, type ViewportVisualEvidence } from "./release_evidence_mapper.ts";
import { createReleaseEvidenceReceipt, type ReleaseEvidenceReceipt } from "./release_evidence_receipt.ts";
import { createRequirementEvidenceRecord } from "./requirement_evidence_contract.ts";

export class FinalReleaseEvidenceHarnessError extends Error {
  readonly code: string;

  constructor(code: string) {
    super(code);
    this.name = "FinalReleaseEvidenceHarnessError";
    this.code = code;
  }
}

type SuiteResult = Readonly<{ exitCode: number; executedCount: number }>;

export type FinalReleaseGateState =
  | "Pending"
  | "IdentityVerified"
  | "TypeScriptPassed"
  | "RustPassed"
  | "PackageFresh"
  | "ReceiptCreated"
  | "Passed";

export type FinalReleaseGateEvent =
  | "IdentityAccepted"
  | "TypeScriptAccepted"
  | "RustAccepted"
  | "PackageAccepted"
  | "ReceiptCreated"
  | "ReceiptAccepted";

const FINAL_RELEASE_TRANSITIONS: Readonly<Record<FinalReleaseGateState, Partial<Record<FinalReleaseGateEvent, FinalReleaseGateState>>>> = Object.freeze({
  Pending: Object.freeze({ IdentityAccepted: "IdentityVerified" }),
  IdentityVerified: Object.freeze({ TypeScriptAccepted: "TypeScriptPassed" }),
  TypeScriptPassed: Object.freeze({ RustAccepted: "RustPassed" }),
  RustPassed: Object.freeze({ PackageAccepted: "PackageFresh" }),
  PackageFresh: Object.freeze({ ReceiptCreated: "ReceiptCreated" }),
  ReceiptCreated: Object.freeze({ ReceiptAccepted: "Passed" }),
  Passed: Object.freeze({}),
});

export function transitionFinalReleaseGate(
  state: FinalReleaseGateState,
  event: FinalReleaseGateEvent,
): FinalReleaseGateState {
  const next = FINAL_RELEASE_TRANSITIONS[state]?.[event];
  if (next === undefined) fail("FINAL_RELEASE_GATE_TRANSITION_INVALID");
  return next;
}

export function createFinalReleaseEvidenceReceipt(input: Readonly<{
  expectedRequirementIds: readonly string[];
  buildIdentity: DesktopBuildIdentity;
  mappings: readonly RequirementEvidenceMapping[];
  inventory: readonly RequirementTestInventoryEntry[];
  allowedAggregateRequirementIds: readonly string[];
  typescript: SuiteResult;
  rust: SuiteResult;
  packaged: PackagedSmokeEvidence;
  visual: ViewportVisualEvidence;
  query: Readonly<{ exitCode: number; queryCount: number; failedCount: number }>;
  harnessVersion: string;
  fixtureClass: string;
  startedAt: string;
  hash: (framedIdentity: string) => string;
}>): ReleaseEvidenceReceipt {
  let gate = transitionFinalReleaseGate("Pending", "IdentityAccepted");
  validateSuite(input.typescript, "FINAL_RELEASE_TYPESCRIPT_FAILED");
  gate = transitionFinalReleaseGate(gate, "TypeScriptAccepted");
  validateSuite(input.rust, "FINAL_RELEASE_RUST_FAILED");
  gate = transitionFinalReleaseGate(gate, "RustAccepted");

  const audit = auditRequirementEvidenceMappings({
    expectedRequirementIds: input.expectedRequirementIds,
    mappings: input.mappings,
    inventory: input.inventory,
    allowedAggregateRequirementIds: input.allowedAggregateRequirementIds,
  });
  if (audit.state !== "Passed" || audit.verifiedCount !== input.expectedRequirementIds.length) {
    fail("FINAL_RELEASE_MAPPING_INCOMPLETE");
  }
  if (input.packaged.status !== "Passed"
    || input.packaged.sourceFingerprint !== input.buildIdentity.sourceFingerprint
    || input.packaged.appFingerprint !== input.buildIdentity.appFingerprint
    || input.packaged.sampleCount !== 200
    || input.packaged.p95Ms > 300
    || !input.packaged.attachmentRestartReadbackVerified
    || !input.packaged.canvasTextRestartReadbackVerified) {
    fail("FINAL_RELEASE_PACKAGE_FAILED");
  }
  if (input.visual.status !== "Passed"
    || input.visual.sourceFingerprint !== input.buildIdentity.sourceFingerprint
    || input.visual.appFingerprint !== input.buildIdentity.appFingerprint
    || input.visual.routeViewportCount !== 30
    || input.visual.rendererViewportCount !== 10
    || input.visual.artifactCount !== 40) {
    fail("FINAL_RELEASE_VISUAL_FAILED");
  }
  if (input.query.exitCode !== 0
    || !Number.isSafeInteger(input.query.queryCount) || input.query.queryCount <= 0
    || input.query.failedCount !== 0) {
    fail("FINAL_RELEASE_QUERY_FAILED");
  }
  gate = transitionFinalReleaseGate(gate, "PackageAccepted");

  const metadata = createReleaseEvidenceMetadata({
    buildIdentity: input.buildIdentity,
    testOrJourneyName: "phase016.final.current-gate",
    harnessVersion: input.harnessVersion,
    fixtureClass: input.fixtureClass,
    startedAt: input.startedAt,
    hash: input.hash,
  });
  const records = input.mappings.map((mapping) => createRequirementEvidenceRecord({
    requirementId: mapping.requirementId,
    sourceFingerprint: metadata.sourceFingerprint,
    appFingerprint: metadata.appFingerprint,
    testOrJourneyName: metadata.testOrJourneyName,
    harnessVersion: metadata.harnessVersion,
    fixtureClass: metadata.fixtureClass,
    startedAt: metadata.startedAt,
    result: "Passed",
    measuredValues: measuredValues(mapping, input),
    artifactDigests: [],
  }));
  const receipt = createReleaseEvidenceReceipt({
    expectedRequirementIds: input.expectedRequirementIds,
    currentSourceFingerprint: input.buildIdentity.sourceFingerprint,
    currentAppFingerprint: input.buildIdentity.appFingerprint,
    batches: [{ name: "current-process-bound-requirements", records }],
  });
  gate = transitionFinalReleaseGate(gate, "ReceiptCreated");
  if (receipt.status !== "Passed") {
    fail("FINAL_RELEASE_RECEIPT_FAILED");
  }
  gate = transitionFinalReleaseGate(gate, "ReceiptAccepted");
  if (gate !== "Passed") {
    fail("FINAL_RELEASE_RECEIPT_FAILED");
  }
  return receipt;
}

function measuredValues(
  mapping: RequirementEvidenceMapping,
  input: Readonly<{
    typescript: SuiteResult;
    rust: SuiteResult;
    packaged: PackagedSmokeEvidence;
    visual: ViewportVisualEvidence;
    query: Readonly<{ queryCount: number }>;
  }>,
): Readonly<Record<string, number | boolean>> {
  if (mapping.evidenceClass === "typescript_test") {
    return { suitePassed: true, executedCount: input.typescript.executedCount };
  }
  if (mapping.evidenceClass === "rust_test") {
    return { suitePassed: true, executedCount: input.rust.executedCount };
  }
  if (mapping.evidenceClass === "package") {
    return {
      sampleCount: input.packaged.sampleCount,
      p95Ms: input.packaged.p95Ms,
      durableReadbackCount: input.packaged.durableReadbackCount,
      attachmentRestartReadbackVerified: true,
      canvasTextRestartReadbackVerified: true,
      visualArtifactCount: input.visual.artifactCount,
      queryCount: input.query.queryCount,
    };
  }
  return { suitePassed: true };
}

function validateSuite(result: SuiteResult, code: string): void {
  if (result.exitCode !== 0 || !Number.isSafeInteger(result.executedCount) || result.executedCount <= 0) {
    fail(code);
  }
}

function fail(code: string): never {
  throw new FinalReleaseEvidenceHarnessError(code);
}
