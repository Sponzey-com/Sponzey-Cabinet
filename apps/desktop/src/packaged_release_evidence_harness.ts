import type { DesktopBuildIdentity } from "./desktop_build_identity_contract.ts";
import {
  createPackagedSmokeEvidence,
  parseInitialPackagedSmokeOutput,
  parseRestartPackagedSmokeOutput,
} from "./packaged_smoke_evidence_contract.ts";
import {
  createReleaseEvidenceMetadata,
  mapPackagedSmokeEvidence,
} from "./release_evidence_mapper.ts";
import {
  createReleaseEvidenceReceipt,
  type ReleaseEvidenceReceipt,
} from "./release_evidence_receipt.ts";

export class PackagedReleaseEvidenceHarnessError extends Error {
  readonly code: string;

  constructor(code: string) {
    super(code);
    this.name = "PackagedReleaseEvidenceHarnessError";
    this.code = code;
  }
}

export function createPackagedReleaseEvidenceReceipt(input: Readonly<{
  requirementIds: readonly string[];
  buildIdentity: DesktopBuildIdentity;
  initialProfileFingerprint: string;
  restartProfileFingerprint: string;
  p95BudgetMs: number;
  initialRun: Readonly<{ exitCode: number; stdout: string }>;
  restartRun: Readonly<{ exitCode: number; stdout: string }>;
  testOrJourneyName: string;
  harnessVersion: string;
  fixtureClass: string;
  startedAt: string;
  hash: (framedIdentity: string) => string;
}>): ReleaseEvidenceReceipt {
  if (input.initialRun.exitCode !== 0) {
    fail("PACKAGED_RELEASE_INITIAL_PROCESS_FAILED");
  }
  if (input.restartRun.exitCode !== 0) {
    fail("PACKAGED_RELEASE_RESTART_PROCESS_FAILED");
  }
  if (input.initialProfileFingerprint !== input.restartProfileFingerprint) {
    fail("PACKAGED_RELEASE_PROFILE_MISMATCH");
  }

  const initial = parseInitialPackagedSmokeOutput(
    input.initialRun.stdout,
    input.p95BudgetMs,
    input.initialProfileFingerprint,
  );
  const restart = parseRestartPackagedSmokeOutput(
    input.restartRun.stdout,
    input.restartProfileFingerprint,
  );
  const evidence = createPackagedSmokeEvidence({
    sourceFingerprint: input.buildIdentity.sourceFingerprint,
    appFingerprint: input.buildIdentity.appFingerprint,
    initial,
    restart,
  });
  const metadata = createReleaseEvidenceMetadata({
    buildIdentity: input.buildIdentity,
    testOrJourneyName: input.testOrJourneyName,
    harnessVersion: input.harnessVersion,
    fixtureClass: input.fixtureClass,
    startedAt: input.startedAt,
    hash: input.hash,
  });
  const records = mapPackagedSmokeEvidence({
    requirementIds: input.requirementIds,
    metadata,
    evidence,
  });
  return createReleaseEvidenceReceipt({
    expectedRequirementIds: input.requirementIds,
    currentSourceFingerprint: input.buildIdentity.sourceFingerprint,
    currentAppFingerprint: input.buildIdentity.appFingerprint,
    batches: [{ name: "packaged-initial-restart", records }],
  });
}

function fail(code: string): never {
  throw new PackagedReleaseEvidenceHarnessError(code);
}
