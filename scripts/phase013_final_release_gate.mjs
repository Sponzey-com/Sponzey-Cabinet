export const PHASE013_REQUIREMENT_FAMILIES = Object.freeze([
  "SHELL", "COPY", "IDENTITY", "ERROR", "ACTION", "A11Y", "LAYOUT",
  "PERF", "ARCH", "CONFIG", "LOG", "STATE", "TIDY", "COMPAT", "EVIDENCE",
]);

const SHA256 = /^[0-9a-f]{64}$/;
const ABSOLUTE_PATH = /^(?:\/(?:Users|home|private|tmp|var|opt)\/|[A-Za-z]:\\)/;
const SENSITIVE_KEYS = /^(?:documentBody|body|content|bytes|assetBytes|token|secret|password|apiKey|absolutePath)$/i;

export function validatePhase013FinalReleaseReport(report, expectedSourceFingerprint) {
  const findingIds = [];
  if (report?.marker !== "phase013_final_release_gate=passed") findingIds.push("marker");
  if (report?.state !== "Passed") findingIds.push("state");
  if (!SHA256.test(report?.sourceFingerprint ?? "")) findingIds.push("source_fingerprint");
  if (expectedSourceFingerprint && report?.sourceFingerprint !== expectedSourceFingerprint) {
    findingIds.push("stale_source_fingerprint");
  }
  if (!Number.isInteger(report?.taskCount) || report.taskCount < 46) findingIds.push("task_count");
  if (report?.diagnostics !== "sanitized") findingIds.push("diagnostics");

  const evidence = Array.isArray(report?.requirementEvidence) ? report.requirementEvidence : [];
  const familyCounts = new Map();
  for (const item of evidence) {
    familyCounts.set(item?.family, (familyCounts.get(item?.family) ?? 0) + 1);
    if (item?.state !== "Passed") findingIds.push("requirement_evidence_failed");
    if (typeof item?.evidence !== "string" || item.evidence.length === 0) findingIds.push("requirement_evidence_missing");
  }
  if (PHASE013_REQUIREMENT_FAMILIES.some((family) => !familyCounts.has(family))) {
    findingIds.push("requirement_family_missing");
  }
  if ([...familyCounts.values()].some((count) => count !== 1)) findingIds.push("requirement_family_duplicate");

  const commands = Array.isArray(report?.commands) ? report.commands : [];
  if (commands.length === 0) findingIds.push("commands_missing");
  if (commands.some((command) => command?.state !== "Passed")) findingIds.push("command_failed");
  inspect(report, findingIds);
  const unique = [...new Set(findingIds)];
  return Object.freeze({ passed: unique.length === 0, findingIds: Object.freeze(unique) });
}

export function renderPhase013RequirementMatrix(report) {
  const rows = report.requirementEvidence.map((item) =>
    `| ${item.family}-013-* | ${item.state} | \`${item.evidence}\` | ${item.summary} |`,
  );
  return [
    "# Phase 013 Requirement Evidence Matrix", "",
    report.marker,
    `state=${report.state}`,
    `source_fingerprint=${report.sourceFingerprint}`,
    `task_count=${report.taskCount}`,
    "diagnostics=sanitized", "",
    "| Requirement | State | Evidence | Verified outcome |",
    "| --- | --- | --- | --- |",
    ...rows, "",
    "No document body, user filename, asset bytes, credential, or absolute path is included.", "",
  ].join("\n");
}

function inspect(value, findingIds) {
  if (Array.isArray(value)) {
    for (const child of value) inspect(child, findingIds);
    return;
  }
  if (!value || typeof value !== "object") {
    if (typeof value === "string" && ABSOLUTE_PATH.test(value)) findingIds.push("absolute_path");
    return;
  }
  for (const [key, child] of Object.entries(value)) {
    if (SENSITIVE_KEYS.test(key)) findingIds.push("sensitive_evidence");
    inspect(child, findingIds);
  }
}
