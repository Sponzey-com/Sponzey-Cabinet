import { readFile, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

export const ReleaseGateState = Object.freeze({
  Pending: "Pending",
  ReadingEvidence: "ReadingEvidence",
  ValidatingEvidence: "ValidatingEvidence",
  WritingResult: "WritingResult",
  Passed: "Passed",
  Failed: "Failed",
});

export const ReleaseGateErrorCode = Object.freeze({
  RequiredEvidenceMissing: "PHASE007_RELEASE_REQUIRED_EVIDENCE_MISSING",
  SourceReadFailed: "PHASE007_RELEASE_SOURCE_READ_FAILED",
  InvalidTransition: "PHASE007_RELEASE_INVALID_TRANSITION",
});

const requiredTargets = Object.freeze([
  target("product_smoke_prerequisite", "Phase 007 product smoke prerequisite", {
    requiredFiles: [".tasks/phase007-product-smoke-gate-result.md"],
    evidence: ["phase007_product_smoke_gate=passed"],
  }),
  target("ownership_and_performance", "Phase 007 ownership and performance evidence", {
    requiredFiles: [
      ".tasks/phase007-data-ownership-gate-result.md",
      ".tasks/release/performance-budget-phase007.md",
      ".tasks/release/ai-status-result-budget-phase007.md",
    ],
    evidence: [
      "phase007_data_ownership_gate=passed",
      "phase007_performance_budget=passed",
      "phase007_ai_status_result_budget=passed",
    ],
  }),
  target("security_manifest", "Phase 007 security log policy manifest", {
    requiredFiles: [".tasks/release/security-log-policy-manifest.json"],
    evidence: [
      '"id": "phase007_release_gate_result"',
      '"path": ".tasks/phase007-release-gate-result.md"',
      '"required": true',
      '"deniedFixtures"',
      "ai_prompt_fixture",
      "provider_api_key_fixture",
    ],
  }),
  target("product_log_matrix", "Phase 007 product log event matrix", {
    requiredFiles: [".tasks/release/product-log-event-matrix.md"],
    evidence: [
      "Product Log",
      "Field Debug Log",
      "Development Log",
      "workspace.home.ready",
      "document.saved",
      "search.failed",
      "ai.answer.failed",
      "backup.created",
      "Do not record document body, asset content, prompt, answer, token, credential, secret, or raw path.",
    ],
  }),
  target("local_desktop_runbook", "Phase 007 local desktop runbook", {
    requiredFiles: [".tasks/release/local-desktop-runbook.md"],
    evidence: [
      "Phase 007 Local Desktop Runbook",
      "Clean Install",
      "Home",
      "Document Authoring",
      "Search Discovery",
      "AI Provider Disabled",
      "Backup Export Import Restore",
      "Sensitive Data Exclusion",
    ],
  }),
  target("release_tooling", "Phase 007 release tooling", {
    requiredFiles: ["package.json"],
    evidence: [
      "run:phase007-product-smoke",
      "run:phase007-release-gate",
      "run:phase007-data-ownership-gate",
    ],
  }),
]);

export function evaluateReleaseGate({ sources }) {
  if (!sources || Object.keys(sources).length === 0) {
    return failedResult({
      errorCode: ReleaseGateErrorCode.SourceReadFailed,
      missingEvidence: [{ targetId: "source_set", missing: ["sources"] }],
    });
  }
  const targetResults = requiredTargets.map((entry) => analyzeTarget(entry, sources));
  const missingEvidence = targetResults
    .filter((entry) => entry.missing.length > 0)
    .map((entry) => ({ targetId: entry.id, missing: entry.missing }));
  if (missingEvidence.length > 0) {
    return failedResult({
      errorCode: ReleaseGateErrorCode.RequiredEvidenceMissing,
      missingEvidence,
      targetResults,
    });
  }
  return {
    passed: true,
    marker: "phase007_release_gate=passed",
    state: ReleaseGateState.Passed,
    summary: { requiredTargets: requiredTargets.length, missingRequiredEvidence: 0 },
    targetResults,
    missingEvidence: [],
  };
}

export function renderReleaseGateResult(result) {
  const lines = [
    "# Phase 007 Final Release Gate Result",
    "",
    result.marker,
    "",
    "- phase: `Phase 007`",
    "- gate: `Final Personal Local Desktop Release`",
    `- status: \`${result.passed ? "passed" : "failed"}\``,
    `- state: \`${result.state}\``,
    `- required targets: \`${result.summary.requiredTargets}\``,
    `- missing required evidence: \`${result.summary.missingRequiredEvidence}\``,
    "",
    "## Evidence",
    "",
    "| Target | Status | Description |",
    "| --- | --- | --- |",
  ];
  for (const targetResult of result.targetResults ?? []) {
    lines.push(`| \`${targetResult.id}\` | \`${targetResult.status}\` | ${targetResult.description} |`);
  }
  lines.push(
    "",
    "## Release Boundary",
    "",
    "- Phase 007 release scope is the personal local desktop installable knowledge workspace.",
    "- Server hosting, SaaS, multi-user, tenant administration, billing, SSO, OS signing, notarization, and app store distribution remain outside this release gate.",
    "",
    "## Sensitive Data Exclusion",
    "",
    "This artifact records target ids, marker status, counts, and stable error codes only. It does not record document body, asset content, AI prompt, AI answer, provider key, token, credential, secret, or personal absolute path.",
    "",
  );
  return lines.join("\n");
}

export async function runReleaseGate({ root = process.cwd() } = {}) {
  try {
    const sources = {};
    for (const filePath of collectRequiredFiles()) {
      sources[filePath] = await readFile(`${root}/${filePath}`, "utf8");
    }
    const result = evaluateReleaseGate({ sources });
    await writeFile(`${root}/.tasks/phase007-release-gate-result.md`, renderReleaseGateResult(result));
    return result;
  } catch {
    return failedResult({
      errorCode: ReleaseGateErrorCode.SourceReadFailed,
      missingEvidence: [{ targetId: "source_read", missing: ["required source file"] }],
    });
  }
}

function analyzeTarget(entry, sources) {
  const texts = entry.requiredFiles.map((filePath) => sources[filePath] ?? "");
  const missingFiles = entry.requiredFiles.filter((filePath) => !(filePath in sources));
  const missingEvidence = entry.evidence.filter((needle) => !texts.some((text) => text.includes(needle)));
  const missing = [...missingFiles, ...missingEvidence];
  return { id: entry.id, description: entry.description, status: missing.length === 0 ? "covered" : "missing", missing };
}

function failedResult({ errorCode, missingEvidence, targetResults = [] }) {
  return {
    passed: false,
    marker: "phase007_release_gate=failed",
    state: ReleaseGateState.Failed,
    errorCode,
    summary: {
      requiredTargets: requiredTargets.length,
      missingRequiredEvidence: missingEvidence.length,
    },
    targetResults,
    missingEvidence,
  };
}

function collectRequiredFiles() {
  return [...new Set(requiredTargets.flatMap((entry) => entry.requiredFiles))];
}

function target(id, description, details) {
  return { id, description, ...details };
}

async function runCli() {
  const result = await runReleaseGate();
  if (result.passed) {
    console.log(result.marker);
    return;
  }
  console.error(result.marker);
  console.error(`error_code=${result.errorCode}`);
  process.exit(1);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runCli();
}
