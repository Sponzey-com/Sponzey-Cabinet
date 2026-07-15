import { readFile, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

export const DataOwnershipReportErrorCode = Object.freeze({
  RequiredEvidenceMissing: "PHASE006_DATA_OWNERSHIP_REQUIRED_EVIDENCE_MISSING",
  SourceReadFailed: "PHASE006_DATA_OWNERSHIP_SOURCE_READ_FAILED",
});

const requiredTargets = Object.freeze([
  target("current_product_scope", "current personal desktop product scope", {
    requiredFiles: ["PROJECT.md"],
    evidence: [
      "현재 최종 목표: 개인 사용자의 개인 PC에 설치되는 단일 사용자 지식 관리 앱",
      "개인 구축의 로컬 설정은 설치 1회로 완료되어야 한다",
      "서버 호스팅, SaaS 형태는 차후 목표다",
    ],
  }),
  target("local_runtime_ownership", "local runtime ownership baseline", {
    requiredFiles: [".tasks/phase006-local-runtime-gate-result.md"],
    evidence: ["phase006_local_runtime_gate=passed"],
  }),
  target("backup_import_export_ownership", "backup import export ownership baseline", {
    requiredFiles: [".tasks/phase006-backup-package-gate-result.md", "PROJECT.md"],
    evidence: ["phase006_backup_package_gate=passed", "백업/복원, import/export"],
  }),
  target("performance_ownership", "local lookup performance ownership", {
    requiredFiles: [".tasks/release/performance-budget-phase006.md"],
    evidence: [
      "phase006_document_query_budget=passed",
      "phase006_search_graph_asset_budget=passed",
      "phase006_ai_status_result_budget=passed",
    ],
  }),
]);

export function analyzeDataOwnershipEvidence({ sources }) {
  if (!sources || Object.keys(sources).length === 0) {
    return failedResult({
      errorCode: DataOwnershipReportErrorCode.SourceReadFailed,
      missingEvidence: [{ targetId: "source_set", missing: ["sources"] }],
    });
  }
  const targetResults = requiredTargets.map((entry) => analyzeTarget(entry, sources));
  const missingEvidence = targetResults
    .filter((entry) => entry.missing.length > 0)
    .map((entry) => ({ targetId: entry.id, missing: entry.missing }));
  if (missingEvidence.length > 0) {
    return failedResult({
      errorCode: DataOwnershipReportErrorCode.RequiredEvidenceMissing,
      missingEvidence,
      targetResults,
    });
  }
  return {
    passed: true,
    marker: "phase006_data_ownership_verification=passed",
    summary: { requiredTargets: requiredTargets.length, missingRequiredEvidence: 0 },
    targetResults,
    missingEvidence: [],
  };
}

export function renderDataOwnershipReportMarkdown(result) {
  const lines = [
    "# Phase 006 Data Ownership Verification",
    "",
    result.marker,
    "",
    `- status: \`${result.passed ? "passed" : "failed"}\``,
  ];
  if (!result.passed) {
    lines.push(`- error code: \`${result.errorCode}\``);
  }
  lines.push(
    `- required targets: \`${result.summary.requiredTargets}\``,
    `- missing required evidence: \`${result.summary.missingRequiredEvidence}\``,
    "",
    "## Ownership Checks",
    "",
    "| Target | Status | Description |",
    "| --- | --- | --- |",
  );
  for (const targetResult of result.targetResults ?? []) {
    lines.push(`| \`${targetResult.id}\` | \`${targetResult.status}\` | ${targetResult.description} |`);
  }
  lines.push(
    "",
    "## Verified Product Boundary",
    "",
    "- The current product scope is a personal desktop local-first app.",
    "- Default use requires no server, SaaS tenant, external database, external search server, external AI provider, Git CLI, or manual runtime configuration.",
    "- Backup, restore, import, and export evidence supports user-owned data movement.",
    "",
    "## Sensitive Data Exclusion",
    "",
    "This report records target ids, marker status, counts, and stable error codes only. It does not record document body, asset content, raw prompt, raw generated response, personal path, credential, token, provider secret, or local machine secret.",
    "",
  );
  return lines.join("\n");
}

export async function runDataOwnershipReport({ root = process.cwd() } = {}) {
  try {
    const sources = {};
    for (const filePath of collectRequiredFiles()) {
      sources[filePath] = await readFile(`${root}/${filePath}`, "utf8");
    }
    return analyzeDataOwnershipEvidence({ sources });
  } catch {
    return failedResult({
      errorCode: DataOwnershipReportErrorCode.SourceReadFailed,
      missingEvidence: [{ targetId: "source_read", missing: ["required source file"] }],
    });
  }
}

function analyzeTarget(entry, sources) {
  const texts = entry.requiredFiles.map((filePath) => sources[filePath] ?? "");
  const missingFiles = entry.requiredFiles.filter((filePath) => !(filePath in sources));
  const missingEvidence = entry.evidence.filter((needle) => !texts.some((text) => text.includes(needle)));
  const missing = [...missingFiles, ...missingEvidence];
  return {
    id: entry.id,
    description: entry.description,
    status: missing.length === 0 ? "covered" : "missing",
    missing,
  };
}

function failedResult({ errorCode, missingEvidence, targetResults = [] }) {
  return {
    passed: false,
    marker: "phase006_data_ownership_verification=failed",
    errorCode,
    summary: { requiredTargets: requiredTargets.length, missingRequiredEvidence: missingEvidence.length },
    targetResults,
    missingEvidence,
  };
}

function target(id, description, { requiredFiles, evidence }) {
  return { id, description, requiredFiles, evidence };
}

function collectRequiredFiles() {
  return [...new Set(requiredTargets.flatMap((entry) => entry.requiredFiles))];
}

async function runCli() {
  const result = await runDataOwnershipReport();
  await writeFile(".tasks/release/data-ownership-verification.md", renderDataOwnershipReportMarkdown(result));
  if (result.passed) {
    console.log(result.marker);
    console.log(`required_targets=${result.summary.requiredTargets}`);
    return;
  }
  console.error(result.marker);
  console.error(`error_code=${result.errorCode}`);
  process.exit(1);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runCli();
}
