export const Phase012ReleaseState = Object.freeze({
  NotStarted: "NotStarted",
  RequirementsValidated: "RequirementsValidated",
  PlatformValidated: "PlatformValidated",
  SecurityValidated: "SecurityValidated",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase012ReleaseEvent = Object.freeze({
  RequirementsAccepted: "RequirementsAccepted",
  PlatformAccepted: "PlatformAccepted",
  SecurityAccepted: "SecurityAccepted",
  Complete: "Complete",
  Fail: "Fail",
});

export const Phase012ReleaseErrorCode = Object.freeze({
  SourceFingerprintInvalid: "PHASE012_RELEASE_SOURCE_FINGERPRINT_INVALID",
  SourceFingerprintMismatch: "PHASE012_RELEASE_SOURCE_FINGERPRINT_MISMATCH",
  RequirementMissing: "PHASE012_RELEASE_REQUIREMENT_MISSING",
  RequirementDuplicate: "PHASE012_RELEASE_REQUIREMENT_DUPLICATE",
  RequirementFailed: "PHASE012_RELEASE_REQUIREMENT_FAILED",
  NativeMacosEvidenceMissing: "PHASE012_RELEASE_NATIVE_MACOS_EVIDENCE_MISSING",
  DeferredPlatformClaimInvalid: "PHASE012_RELEASE_DEFERRED_PLATFORM_CLAIM_INVALID",
  UnsafeArtifactContent: "PHASE012_RELEASE_UNSAFE_ARTIFACT_CONTENT",
  InvalidTransition: "PHASE012_RELEASE_INVALID_TRANSITION",
});

export const phase012RequirementIds = Object.freeze([
  "SCOPE-012-01", "BASE-012-01", "SAVE-012-01", "GRAPH-012-01", "GRAPH-012-02",
  "CANVAS-012-01", "CANVAS-012-02", "CANVAS-012-03", "ASSET-012-01", "ASSET-012-02",
  "ASSET-012-03", "ASSET-012-04", "PROJ-012-01", "PROJ-012-02", "UX-012-01", "UX-012-02",
  "UI-CONN-012-01", "UI-CONN-012-02", "UI-CONN-012-03", "UI-CONN-012-04", "PERF-012-01",
  "CFG-012-01", "LOG-012-01", "RECOVERY-012-01", "BACKUP-012-01", "PLAT-012-01",
  "DATA-012-01", "DATA-012-02", "ROUTE-012-01", "EVID-012-01", "OPS-012-01", "ERR-012-01",
  "SEC-012-01",
]);

const unsafePatterns = Object.freeze([
  /(?:document|raw)_body\s*=/i,
  /asset_bytes\s*=/i,
  /(?:secret|token|api[_-]?key|password)\s*=/i,
  /\/Users\/[^/\s]+\//,
  /[A-Z]:\\Users\\[^\\\s]+\\/i,
]);

export function transitionPhase012ReleaseState(state, event, failure = {}) {
  const next = new Map([
    [`${Phase012ReleaseState.NotStarted}:${Phase012ReleaseEvent.RequirementsAccepted}`, Phase012ReleaseState.RequirementsValidated],
    [`${Phase012ReleaseState.RequirementsValidated}:${Phase012ReleaseEvent.PlatformAccepted}`, Phase012ReleaseState.PlatformValidated],
    [`${Phase012ReleaseState.PlatformValidated}:${Phase012ReleaseEvent.SecurityAccepted}`, Phase012ReleaseState.SecurityValidated],
    [`${Phase012ReleaseState.SecurityValidated}:${Phase012ReleaseEvent.Complete}`, Phase012ReleaseState.Passed],
  ]).get(`${state}:${event}`);
  if (event === Phase012ReleaseEvent.Fail && state !== Phase012ReleaseState.NotStarted) {
    return { state: Phase012ReleaseState.Failed, ...failure };
  }
  return next
    ? { state: next }
    : { state: Phase012ReleaseState.Failed, errorCode: Phase012ReleaseErrorCode.InvalidTransition };
}

export function analyzePhase012ReleaseEvidence(input) {
  const fingerprint = input?.expectedSourceFingerprint ?? "";
  if (!/^[a-f0-9]{64}$/.test(fingerprint)) {
    return failed(Phase012ReleaseErrorCode.SourceFingerprintInvalid, "source_fingerprint");
  }

  const evidence = input?.requirementEvidence ?? [];
  const counts = new Map();
  for (const record of evidence) {
    counts.set(record.requirementId, (counts.get(record.requirementId) ?? 0) + 1);
  }
  const missing = phase012RequirementIds.find((id) => !counts.has(id));
  if (missing) return failed(Phase012ReleaseErrorCode.RequirementMissing, missing);
  const duplicate = phase012RequirementIds.find((id) => counts.get(id) !== 1);
  if (duplicate) return failed(Phase012ReleaseErrorCode.RequirementDuplicate, duplicate);
  const unexpected = evidence.find((record) => !phase012RequirementIds.includes(record.requirementId));
  if (unexpected) return failed(Phase012ReleaseErrorCode.RequirementDuplicate, unexpected.requirementId);
  const failedRecord = evidence.find((record) => record.status !== "passed");
  if (failedRecord) return failed(Phase012ReleaseErrorCode.RequirementFailed, failedRecord.requirementId);
  const stale = evidence.find((record) => record.sourceFingerprint !== fingerprint);
  if (stale) return failed(Phase012ReleaseErrorCode.SourceFingerprintMismatch, stale.requirementId);

  let state = transitionPhase012ReleaseState(
    Phase012ReleaseState.NotStarted,
    Phase012ReleaseEvent.RequirementsAccepted,
  );
  const platforms = input?.platformEvidence ?? {};
  if (
    platforms.macos?.status !== "passed" ||
    platforms.macos?.sourceFingerprint !== fingerprint ||
    platforms.macos?.evidenceId !== "phase012-macos-packaged-ui-smoke"
  ) {
    return failed(Phase012ReleaseErrorCode.NativeMacosEvidenceMissing, "macos");
  }
  for (const platform of ["windows", "linux"]) {
    if (platforms[platform]?.status !== "deferred_future") {
      return failed(Phase012ReleaseErrorCode.DeferredPlatformClaimInvalid, platform);
    }
  }
  state = transitionPhase012ReleaseState(state.state, Phase012ReleaseEvent.PlatformAccepted);

  const unsafe = (input?.artifactTexts ?? []).find((text) =>
    unsafePatterns.some((pattern) => pattern.test(text))
  );
  if (unsafe) return failed(Phase012ReleaseErrorCode.UnsafeArtifactContent, "artifact_content");
  state = transitionPhase012ReleaseState(state.state, Phase012ReleaseEvent.SecurityAccepted);
  state = transitionPhase012ReleaseState(state.state, Phase012ReleaseEvent.Complete);

  return {
    passed: true,
    state: state.state,
    marker: "phase012_release_gate=passed",
    sourceFingerprint: fingerprint,
    requirementCount: phase012RequirementIds.length,
    requirementEvidence: evidence.map((record) => ({ ...record })),
    platformEvidence: structuredClone(platforms),
  };
}

export function renderPhase012RequirementMatrix(result) {
  return [
    "# Phase 012 Requirement Evidence Matrix",
    "",
    "phase012_requirement_evidence=passed",
    `source_fingerprint=${result.sourceFingerprint}`,
    `requirement_count=${result.requirementCount}`,
    "",
    "| Requirement | Status | Evidence | Command |",
    "| --- | --- | --- | --- |",
    ...result.requirementEvidence.map((record) =>
      `| \`${record.requirementId}\` | \`passed\` | \`${record.artifactId}\` | \`${record.commandId}\` |`
    ),
    "",
  ].join("\n");
}

export function renderPhase012PlatformMatrix(result) {
  return [
    "# Phase 012 Native Platform Matrix",
    "",
    "phase012_native_platform_matrix=passed",
    `source_fingerprint=${result.sourceFingerprint}`,
    "",
    "| Platform | Status | Evidence |",
    "| --- | --- | --- |",
    `| \`macos\` | \`passed\` | \`${result.platformEvidence.macos.evidenceId}\` |`,
    "| `windows` | `deferred_future` | `user-directed deferral` |",
    "| `linux` | `deferred_future` | `user-directed deferral` |",
    "",
  ].join("\n");
}

export function renderPhase012ReleaseResult(result) {
  return [
    "# Phase 012 Final Release Gate Result",
    "",
    result.marker,
    `state=${result.state}`,
    "release_scope=personal_local_macos_desktop",
    `source_fingerprint=${result.sourceFingerprint}`,
    `requirement_count=${result.requirementCount}`,
    "macos_native_evidence=passed",
    "windows_evidence=deferred_future",
    "linux_evidence=deferred_future",
    "task_checkboxes_used_as_evidence=false",
    "phase011_markers_used_as_current_evidence=false",
    "raw_body_excluded=true",
    "raw_path_excluded=true",
    "asset_bytes_excluded=true",
    "",
  ].join("\n");
}

function failed(errorCode, findingId) {
  return {
    passed: false,
    state: Phase012ReleaseState.Failed,
    marker: "phase012_release_gate=failed",
    errorCode,
    findingId,
  };
}
