import type { DesktopBuildIdentity } from "./desktop_build_identity_contract.ts";
import type { PackagedSmokeEvidence } from "./packaged_smoke_evidence_contract.ts";
import type { QueryRenderMeasurementInput } from "./query_render_benchmark_contract.ts";
import {
  createReleaseEvidenceMetadata,
  mapPackagedSmokeEvidence,
  mapQueryRenderEvidence,
  mapVisualEvidence,
  type ViewportVisualEvidence,
} from "./release_evidence_mapper.ts";
import {
  createReleaseEvidenceReceipt,
  type ReleaseEvidenceBatch,
  type ReleaseEvidenceReceipt,
} from "./release_evidence_receipt.ts";

type EvidenceBatchInput<T> = Readonly<{
  requirementIds: readonly string[];
  testOrJourneyName: string;
  evidence: T;
}>;

export function createCurrentReleaseEvidenceReceipt(input: Readonly<{
  expectedRequirementIds: readonly string[];
  buildIdentity: DesktopBuildIdentity;
  harnessVersion: string;
  fixtureClass: string;
  startedAt: string;
  hash: (framedIdentity: string) => string;
  packaged: EvidenceBatchInput<PackagedSmokeEvidence>;
  visual: EvidenceBatchInput<ViewportVisualEvidence>;
  queries: readonly Readonly<{
    requirementIds: readonly string[];
    testOrJourneyName: string;
    processExitCode: number;
    measurement: QueryRenderMeasurementInput;
  }>[];
}>): ReleaseEvidenceReceipt {
  const metadata = (testOrJourneyName: string) => createReleaseEvidenceMetadata({
    buildIdentity: input.buildIdentity,
    testOrJourneyName,
    harnessVersion: input.harnessVersion,
    fixtureClass: input.fixtureClass,
    startedAt: input.startedAt,
    hash: input.hash,
  });

  const batches: ReleaseEvidenceBatch[] = [
    {
      name: "packaged",
      records: mapPackagedSmokeEvidence({
        requirementIds: input.packaged.requirementIds,
        metadata: metadata(input.packaged.testOrJourneyName),
        evidence: input.packaged.evidence,
      }),
    },
    {
      name: "visual",
      records: mapVisualEvidence({
        requirementIds: input.visual.requirementIds,
        metadata: metadata(input.visual.testOrJourneyName),
        evidence: input.visual.evidence,
      }),
    },
    ...input.queries.map((query, index) => ({
      name: `query-${String(index + 1).padStart(3, "0")}`,
      records: mapQueryRenderEvidence({
        requirementIds: query.requirementIds,
        metadata: metadata(query.testOrJourneyName),
        measurement: query.measurement,
        processExitCode: query.processExitCode,
      }),
    })),
  ];

  return createReleaseEvidenceReceipt({
    expectedRequirementIds: input.expectedRequirementIds,
    currentSourceFingerprint: input.buildIdentity.sourceFingerprint,
    currentAppFingerprint: input.buildIdentity.appFingerprint,
    batches,
  });
}
