import type { PackagedSmokeEvidence } from "./packaged_smoke_evidence_contract.ts";
import type { AccessibilityEvidence } from "./accessibility_evidence_contract.ts";
import {
  createDesktopBuildIdentity,
  type DesktopBuildIdentity,
} from "./desktop_build_identity_contract.ts";
import {
  benchmarkProcessExitCode,
  aggregateQueryRenderBenchmark,
  evaluateQueryRenderMeasurement,
  type QueryRenderMeasurementInput,
} from "./query_render_benchmark_contract.ts";
import {
  RequirementEvidenceError,
  createRequirementEvidenceRecord,
  type RequirementEvidenceRecord,
} from "./requirement_evidence_contract.ts";

export type ViewportVisualEvidence = Readonly<{
  status: "Passed";
  sourceFingerprint: string;
  appFingerprint: string;
  routeViewportCount: number;
  rendererViewportCount: number;
  artifactCount: number;
}>;

export class ReleaseEvidenceMetadata {
  readonly sourceFingerprint: string;
  readonly appFingerprint: string;
  readonly buildIdentityFingerprint: string;
  readonly sourceFileCount: number;
  readonly artifactCount: number;
  readonly totalArtifactBytes: number;
  readonly testOrJourneyName: string;
  readonly harnessVersion: string;
  readonly fixtureClass: string;
  readonly startedAt: string;

  private constructor(input: Readonly<{
    buildIdentity: DesktopBuildIdentity;
    testOrJourneyName: string;
    harnessVersion: string;
    fixtureClass: string;
    startedAt: string;
  }>) {
    this.sourceFingerprint = input.buildIdentity.sourceFingerprint;
    this.appFingerprint = input.buildIdentity.appFingerprint;
    this.buildIdentityFingerprint = input.buildIdentity.identityFingerprint;
    this.sourceFileCount = input.buildIdentity.sourceFileCount;
    this.artifactCount = input.buildIdentity.artifactCount;
    this.totalArtifactBytes = input.buildIdentity.totalArtifactBytes;
    this.testOrJourneyName = input.testOrJourneyName;
    this.harnessVersion = input.harnessVersion;
    this.fixtureClass = input.fixtureClass;
    this.startedAt = input.startedAt;
    Object.freeze(this);
  }

  static create(input: Readonly<{
    buildIdentity: DesktopBuildIdentity;
    testOrJourneyName: string;
    harnessVersion: string;
    fixtureClass: string;
    startedAt: string;
    hash: (framedIdentity: string) => string;
  }>): ReleaseEvidenceMetadata {
    const verifiedIdentity = createDesktopBuildIdentity({
      sourceFingerprint: input.buildIdentity.sourceFingerprint,
      sourceFileCount: input.buildIdentity.sourceFileCount,
      appFingerprint: input.buildIdentity.appFingerprint,
      artifactCount: input.buildIdentity.artifactCount,
      totalArtifactBytes: input.buildIdentity.totalArtifactBytes,
      hash: input.hash,
    });
    if (verifiedIdentity.identityFingerprint !== input.buildIdentity.identityFingerprint) {
      throw new RequirementEvidenceError("REQUIREMENT_EVIDENCE_BUILD_IDENTITY_MISMATCH");
    }
    createRequirementEvidenceRecord({
      requirementId: "NAV-001",
      sourceFingerprint: verifiedIdentity.sourceFingerprint,
      appFingerprint: verifiedIdentity.appFingerprint,
      testOrJourneyName: input.testOrJourneyName,
      harnessVersion: input.harnessVersion,
      fixtureClass: input.fixtureClass,
      startedAt: input.startedAt,
      result: "Passed",
      measuredValues: {},
      artifactDigests: [],
    });
    return new ReleaseEvidenceMetadata({ ...input, buildIdentity: verifiedIdentity });
  }
}

export const createReleaseEvidenceMetadata = (
  input: Parameters<typeof ReleaseEvidenceMetadata.create>[0],
): ReleaseEvidenceMetadata => ReleaseEvidenceMetadata.create(input);

export function mapPackagedSmokeEvidence(input: Readonly<{
  requirementIds: readonly string[];
  metadata: ReleaseEvidenceMetadata;
  evidence: PackagedSmokeEvidence;
}>): readonly RequirementEvidenceRecord[] {
  assertFingerprints(input.metadata, input.evidence);
  return records(input.requirementIds, input.metadata, {
    sampleCount: input.evidence.sampleCount,
    p95Ms: input.evidence.p95Ms,
    actionCount: input.evidence.actionCount,
    durableReadbackCount: input.evidence.durableReadbackCount,
    attachmentRestartReadbackVerified: input.evidence.attachmentRestartReadbackVerified,
    canvasTextRestartReadbackVerified: input.evidence.canvasTextRestartReadbackVerified,
    accessibilityRouteFocusCount: input.evidence.accessibilityRouteFocusCount,
    accessibilityKeyboardJourneyCount: input.evidence.accessibilityKeyboardJourneyCount,
    accessibilityFocusRestorationCount: input.evidence.accessibilityFocusRestorationCount,
    accessibilityVisibleControlCount: input.evidence.accessibilityVisibleControlCount,
    accessibilityNamedControlCount: input.evidence.accessibilityNamedControlCount,
    accessibilityTextZoomPercent: input.evidence.accessibilityTextZoomPercent,
  });
}

export function mapVisualEvidence(input: Readonly<{
  requirementIds: readonly string[];
  metadata: ReleaseEvidenceMetadata;
  evidence: ViewportVisualEvidence;
}>): readonly RequirementEvidenceRecord[] {
  assertFingerprints(input.metadata, input.evidence);
  if (input.evidence.status !== "Passed") {
    throw new RequirementEvidenceError("REQUIREMENT_EVIDENCE_VISUAL_ASSERTION_FAILED");
  }
  return records(input.requirementIds, input.metadata, {
    routeViewportCount: input.evidence.routeViewportCount,
    rendererViewportCount: input.evidence.rendererViewportCount,
    artifactCount: input.evidence.artifactCount,
  });
}

export function mapQueryRenderEvidence(input: Readonly<{
  requirementIds: readonly string[];
  metadata: ReleaseEvidenceMetadata;
  measurement: QueryRenderMeasurementInput;
  processExitCode: number;
}>): readonly RequirementEvidenceRecord[] {
  const result = evaluateQueryRenderMeasurement(input.measurement);
  const expectedExitCode = benchmarkProcessExitCode(aggregateQueryRenderBenchmark([result]));
  if (result.status !== "Passed" || expectedExitCode !== 0 || input.processExitCode !== 0) {
    throw new RequirementEvidenceError("REQUIREMENT_EVIDENCE_QUERY_ASSERTION_FAILED");
  }
  return records(input.requirementIds, input.metadata, {
    markerMatched: input.measurement.markerMatched,
    resultCountMatched: input.measurement.resultCountMatched,
    errorCount: input.measurement.errorCount,
    p95Ms: input.measurement.p95Ms,
    budgetMs: input.measurement.budgetMs,
    sampleCount: input.measurement.sampleCount,
  });
}

export function mapAccessibilityEvidence(input: Readonly<{
  requirementIds: readonly string[];
  metadata: ReleaseEvidenceMetadata;
  evidence: AccessibilityEvidence;
}>): readonly RequirementEvidenceRecord[] {
  assertFingerprints(input.metadata, input.evidence);
  return records(input.requirementIds, input.metadata, {
    routeFocusCount: input.evidence.routeFocusCount,
    keyboardJourneyCount: input.evidence.keyboardJourneyCount,
    focusRestorationCount: input.evidence.focusRestorationCount,
    visibleControlCount: input.evidence.visibleControlCount,
    namedControlCount: input.evidence.namedControlCount,
    textZoomPercent: input.evidence.textZoomPercent,
    keyboardErrorCount: input.evidence.keyboardErrorCount,
    focusErrorCount: input.evidence.focusErrorCount,
    internalExposureCount: input.evidence.internalExposureCount,
  });
}

function records(
  requirementIds: readonly string[],
  metadata: ReleaseEvidenceMetadata,
  measuredValues: Readonly<Record<string, number | boolean>>,
): readonly RequirementEvidenceRecord[] {
  assertVerifiedMetadata(metadata);
  if (requirementIds.length === 0 || new Set(requirementIds).size !== requirementIds.length) {
    throw new RequirementEvidenceError("REQUIREMENT_EVIDENCE_MAPPING_REQUIREMENTS_INVALID");
  }
  return Object.freeze(requirementIds.map((requirementId) => createRequirementEvidenceRecord({
    requirementId,
    sourceFingerprint: metadata.sourceFingerprint,
    appFingerprint: metadata.appFingerprint,
    testOrJourneyName: metadata.testOrJourneyName,
    harnessVersion: metadata.harnessVersion,
    fixtureClass: metadata.fixtureClass,
    startedAt: metadata.startedAt,
    result: "Passed",
    measuredValues,
    artifactDigests: [],
  })));
}

function assertFingerprints(
  metadata: ReleaseEvidenceMetadata,
  evidence: Readonly<{ sourceFingerprint: string; appFingerprint: string; status: string }>,
): void {
  assertVerifiedMetadata(metadata);
  if (evidence.status !== "Passed"
    || evidence.sourceFingerprint !== metadata.sourceFingerprint
    || evidence.appFingerprint !== metadata.appFingerprint) {
    throw new RequirementEvidenceError("REQUIREMENT_EVIDENCE_FINGERPRINT_MISMATCH");
  }
}

function assertVerifiedMetadata(metadata: ReleaseEvidenceMetadata): void {
  if (!(metadata instanceof ReleaseEvidenceMetadata)) {
    throw new RequirementEvidenceError("REQUIREMENT_EVIDENCE_METADATA_UNVERIFIED");
  }
}
