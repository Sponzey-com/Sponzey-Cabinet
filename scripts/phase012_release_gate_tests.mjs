import assert from "node:assert/strict";
import test from "node:test";

import {
  Phase012ReleaseErrorCode,
  Phase012ReleaseEvent,
  Phase012ReleaseState,
  analyzePhase012ReleaseEvidence,
  phase012RequirementIds,
  renderPhase012PlatformMatrix,
  renderPhase012ReleaseResult,
  renderPhase012RequirementMatrix,
  transitionPhase012ReleaseState,
} from "./phase012_release_gate.mjs";

const fingerprint = "a".repeat(64);

test("rejects missing, duplicate, failed, and stale requirement evidence", () => {
  const cases = [
    ["missing", (records) => records.pop(), Phase012ReleaseErrorCode.RequirementMissing],
    ["duplicate", (records) => records.push({ ...records[0] }), Phase012ReleaseErrorCode.RequirementDuplicate],
    ["failed", (records) => { records[0].status = "failed"; }, Phase012ReleaseErrorCode.RequirementFailed],
    ["stale", (records) => { records[0].sourceFingerprint = "b".repeat(64); }, Phase012ReleaseErrorCode.SourceFingerprintMismatch],
  ];
  for (const [name, mutate, errorCode] of cases) {
    const input = completeInput();
    mutate(input.requirementEvidence);
    const result = analyzePhase012ReleaseEvidence(input);
    assert.equal(result.passed, false, name);
    assert.equal(result.errorCode, errorCode, name);
  }
});

test("rejects a missing macOS pass or a current pass claim for a deferred OS", () => {
  const noMac = completeInput();
  noMac.platformEvidence.macos.status = "deferred_future";
  assert.equal(
    analyzePhase012ReleaseEvidence(noMac).errorCode,
    Phase012ReleaseErrorCode.NativeMacosEvidenceMissing,
  );

  const fileOnlyMac = completeInput();
  fileOnlyMac.platformEvidence.macos.evidenceId = "phase012-macos-packaged-smoke";
  assert.equal(
    analyzePhase012ReleaseEvidence(fileOnlyMac).errorCode,
    Phase012ReleaseErrorCode.NativeMacosEvidenceMissing,
  );

  const falseWindowsClaim = completeInput();
  falseWindowsClaim.platformEvidence.windows.status = "passed";
  assert.equal(
    analyzePhase012ReleaseEvidence(falseWindowsClaim).errorCode,
    Phase012ReleaseErrorCode.DeferredPlatformClaimInvalid,
  );
});

test("rejects unsafe evidence content and absolute user paths", () => {
  for (const unsafeArtifact of [
    "document_body=private text",
    "asset_bytes=deadbeef",
    "secret=token-value",
    "/Users/example/private/document.md",
  ]) {
    const input = completeInput();
    input.artifactTexts.push(unsafeArtifact);
    const result = analyzePhase012ReleaseEvidence(input);
    assert.equal(result.errorCode, Phase012ReleaseErrorCode.UnsafeArtifactContent);
  }
});

test("passes complete fresh evidence and renders sanitized final artifacts", () => {
  const result = analyzePhase012ReleaseEvidence(completeInput());
  assert.equal(result.passed, true);
  assert.equal(result.state, Phase012ReleaseState.Passed);
  assert.equal(result.requirementCount, 33);

  const requirementMatrix = renderPhase012RequirementMatrix(result);
  const platformMatrix = renderPhase012PlatformMatrix(result);
  const releaseResult = renderPhase012ReleaseResult(result);
  assert.match(requirementMatrix, /phase012_requirement_evidence=passed/);
  assert.match(platformMatrix, /\| `macos` \| `passed` \|/);
  assert.match(platformMatrix, /\| `windows` \| `deferred_future` \|/);
  assert.match(platformMatrix, /\| `linux` \| `deferred_future` \|/);
  assert.match(releaseResult, /phase012_release_gate=passed/);
  for (const text of [requirementMatrix, platformMatrix, releaseResult]) {
    assert.doesNotMatch(text, /private text|token-value|\/Users\//);
  }
});

test("release gate state machine accepts only the ordered path", () => {
  let state = transitionPhase012ReleaseState(Phase012ReleaseState.NotStarted, Phase012ReleaseEvent.RequirementsAccepted);
  assert.equal(state.state, Phase012ReleaseState.RequirementsValidated);
  state = transitionPhase012ReleaseState(state.state, Phase012ReleaseEvent.PlatformAccepted);
  assert.equal(state.state, Phase012ReleaseState.PlatformValidated);
  state = transitionPhase012ReleaseState(state.state, Phase012ReleaseEvent.SecurityAccepted);
  assert.equal(state.state, Phase012ReleaseState.SecurityValidated);
  state = transitionPhase012ReleaseState(state.state, Phase012ReleaseEvent.Complete);
  assert.equal(state.state, Phase012ReleaseState.Passed);

  const invalid = transitionPhase012ReleaseState(Phase012ReleaseState.NotStarted, Phase012ReleaseEvent.Complete);
  assert.equal(invalid.state, Phase012ReleaseState.Failed);
  assert.equal(invalid.errorCode, Phase012ReleaseErrorCode.InvalidTransition);
});

function completeInput() {
  return {
    expectedSourceFingerprint: fingerprint,
    requirementEvidence: phase012RequirementIds.map((requirementId, index) => ({
      requirementId,
      status: "passed",
      sourceFingerprint: fingerprint,
      artifactId: `evidence-${String(index + 1).padStart(2, "0")}`,
      commandId: `command-${String(index + 1).padStart(2, "0")}`,
    })),
    platformEvidence: {
      macos: { status: "passed", sourceFingerprint: fingerprint, evidenceId: "phase012-macos-packaged-ui-smoke" },
      windows: { status: "deferred_future" },
      linux: { status: "deferred_future" },
    },
    artifactTexts: [
      "event=phase012.workflow.completed duration_ms=42 count=33",
      "raw_body_excluded=true raw_path_excluded=true",
    ],
  };
}
