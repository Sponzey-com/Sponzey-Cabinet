export const PHASE014_DOD_REQUIREMENTS = Object.freeze(
  Array.from({ length: 34 }, (_, index) => `DOD-${String(index + 1).padStart(3, "0")}`),
);

const SHA256 = /^[0-9a-f]{64}$/;
const REQUIRED_RECEIPTS = Object.freeze([
  "desktop", "rust", "boundary", "geometry", "responsive", "performance", "packaged",
]);

export function validatePhase014CompletionReport(report, expectedFingerprint) {
  const findingIds = [];
  if (report?.marker !== "phase014_completion_evidence=passed") findingIds.push("marker");
  if (report?.state !== "Passed") findingIds.push("state");
  if (!SHA256.test(report?.sourceFingerprint ?? "")) findingIds.push("source_fingerprint");
  if (expectedFingerprint && report?.sourceFingerprint !== expectedFingerprint) findingIds.push("stale_source_fingerprint");
  if (report?.diagnostics !== "sanitized") findingIds.push("diagnostics");

  const requirements = Array.isArray(report?.requirements) ? report.requirements : [];
  const counts = new Map();
  for (const requirement of requirements) {
    counts.set(requirement?.id, (counts.get(requirement?.id) ?? 0) + 1);
    if (requirement?.state !== "Passed") findingIds.push("requirement_failed");
    if (!Array.isArray(requirement?.evidence) || requirement.evidence.length === 0) {
      findingIds.push("requirement_evidence_missing");
    }
    for (const path of requirement?.evidence ?? []) {
      if (typeof path !== "string" || path.startsWith("/") || path.includes("..")) {
        findingIds.push("requirement_evidence_unsafe");
      }
    }
  }
  if (PHASE014_DOD_REQUIREMENTS.some((id) => !counts.has(id))) findingIds.push("requirement_missing");
  if ([...counts.values()].some((count) => count !== 1)) findingIds.push("requirement_duplicate");

  for (const name of REQUIRED_RECEIPTS) {
    const receipt = report?.receipts?.[name];
    if (!receipt || receipt.state !== "Passed") findingIds.push("receipt_failed");
  }
  for (const name of ["desktop", "rust", "boundary"]) {
    if (report?.receipts?.[name]?.sourceFingerprint !== report?.sourceFingerprint) {
      findingIds.push("receipt_stale");
    }
  }
  if (report?.receipts?.packaged?.keyboardDocumentWorkflowVerified !== true) {
    findingIds.push("keyboard_document_workflow");
  }
  const serialized = JSON.stringify(report ?? {});
  if (/\/(?:Users|home|private|tmp|var)\/|[A-Za-z]:\\Users\\|documentBody|assetBytes|sessionToken|provider_api_key/.test(serialized)) {
    findingIds.push("sensitive_evidence");
  }
  const unique = [...new Set(findingIds)];
  return Object.freeze({ passed: unique.length === 0, findingIds: Object.freeze(unique) });
}

export function renderPhase014CompletionMarkdown(report) {
  return [
    "# Phase 014 Completion Evidence",
    "",
    report.marker,
    `state=${report.state}`,
    `source_fingerprint=${report.sourceFingerprint}`,
    `requirement_count=${report.requirements.length}`,
    "diagnostics=sanitized",
    "",
    "| Requirement | State | Evidence |",
    "| --- | --- | --- |",
    ...report.requirements.map((item) => `| ${item.id} | ${item.state} | ${item.evidence.map((path) => `\`${path}\``).join("<br>")} |`),
    "",
    "Deferred server, SaaS, multi-user, mobile, Windows, Linux, and independent Web products are not included in this completion decision.",
    "",
  ].join("\n");
}
