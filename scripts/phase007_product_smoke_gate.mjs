import { readFile, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

export const ProductSmokeGateState = Object.freeze({
  Pending: "Pending",
  ReadingEvidence: "ReadingEvidence",
  ValidatingEvidence: "ValidatingEvidence",
  WritingResult: "WritingResult",
  Passed: "Passed",
  Failed: "Failed",
});

export const ProductSmokeGateErrorCode = Object.freeze({
  RequiredEvidenceMissing: "PHASE007_PRODUCT_SMOKE_REQUIRED_EVIDENCE_MISSING",
  SourceReadFailed: "PHASE007_PRODUCT_SMOKE_SOURCE_READ_FAILED",
  InvalidTransition: "PHASE007_PRODUCT_SMOKE_INVALID_TRANSITION",
});

const requiredTargets = Object.freeze([
  target("phase007_lower_gates", "Phase 007 lower gate markers", {
    requiredFiles: [
      ".tasks/phase007-plan-validation-result.md",
      ".tasks/phase007-workspace-home-gate-result.md",
      ".tasks/phase007-document-authoring-gate-result.md",
      ".tasks/phase007-local-persistence-gate-result.md",
      ".tasks/phase007-discovery-gate-result.md",
      ".tasks/phase007-ai-assistant-gate-result.md",
      ".tasks/phase007-data-ownership-gate-result.md",
    ],
    evidence: [
      "phase007_plan_validation=passed",
      "phase007_workspace_home_gate=passed",
      "phase007_document_authoring_gate=passed",
      "phase007_local_persistence_gate=passed",
      "phase007_discovery_gate=passed",
      "phase007_ai_assistant_gate=passed",
      "phase007_data_ownership_gate=passed",
    ],
  }),
  target("phase007_performance_budgets", "Phase 007 performance budgets", {
    requiredFiles: [
      ".tasks/release/performance-budget-phase007.md",
      ".tasks/release/ai-status-result-budget-phase007.md",
    ],
    evidence: ["phase007_performance_budget=passed", "phase007_ai_status_result_budget=passed"],
  }),
  target("desktop_daily_workspace_flow", "desktop daily workspace flow smoke coverage", {
    requiredFiles: [
      "packages/ui/tests/personal_workspace_home_model_tests.ts",
      "packages/ui/tests/document_authoring_preview_model_tests.ts",
      "apps/desktop/tests/desktop_local_persistence_flow_tests.ts",
      "apps/desktop/tests/desktop_discovery_smoke_tests.ts",
      "apps/desktop/tests/desktop_ai_local_ux_smoke_tests.ts",
      "apps/desktop/tests/desktop_backup_restore_staging_smoke_tests.ts",
      "apps/desktop/tests/desktop_import_preview_smoke_tests.ts",
    ],
    evidence: [
      "recent-documents",
      "| 항목 | 내용 |",
      "version append",
      "desktop local discovery smoke",
      "desktop AI local UX smoke",
      "desktop backup settings smoke",
      "desktop import preview smoke",
    ],
  }),
  target("phase007_release_tooling", "Phase 007 package scripts", {
    requiredFiles: ["package.json"],
    evidence: ["run:phase007-product-smoke", "run:phase007-release-gate"],
  }),
]);

export function evaluateProductSmokeGate({ sources }) {
  if (!sources || Object.keys(sources).length === 0) {
    return failedResult({
      errorCode: ProductSmokeGateErrorCode.SourceReadFailed,
      missingEvidence: [{ targetId: "source_set", missing: ["sources"] }],
    });
  }
  const targetResults = requiredTargets.map((entry) => analyzeTarget(entry, sources));
  const missingEvidence = targetResults
    .filter((entry) => entry.missing.length > 0)
    .map((entry) => ({ targetId: entry.id, missing: entry.missing }));
  if (missingEvidence.length > 0) {
    return failedResult({
      errorCode: ProductSmokeGateErrorCode.RequiredEvidenceMissing,
      missingEvidence,
      targetResults,
    });
  }
  return {
    passed: true,
    marker: "phase007_product_smoke_gate=passed",
    state: ProductSmokeGateState.Passed,
    summary: { requiredTargets: requiredTargets.length, missingRequiredEvidence: 0 },
    targetResults,
    missingEvidence: [],
  };
}

export function renderProductSmokeGateResult(result) {
  const lines = [
    "# Phase 007 Product Smoke Gate Result",
    "",
    result.marker,
    "",
    "- phase: `Phase 007.7`",
    "- gate: `Desktop Product Smoke`",
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
    "## Sensitive Data Exclusion",
    "",
    "This artifact records target ids, marker status, counts, and stable error codes only. It does not record document body, asset content, AI prompt, AI answer, provider key, token, credential, secret, or personal absolute path.",
    "",
  );
  return lines.join("\n");
}

export async function runProductSmokeGate({ root = process.cwd() } = {}) {
  try {
    const sources = {};
    for (const filePath of collectRequiredFiles()) {
      sources[filePath] = await readFile(`${root}/${filePath}`, "utf8");
    }
    const result = evaluateProductSmokeGate({ sources });
    await writeFile(`${root}/.tasks/phase007-product-smoke-gate-result.md`, renderProductSmokeGateResult(result));
    return result;
  } catch {
    return failedResult({
      errorCode: ProductSmokeGateErrorCode.SourceReadFailed,
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
    marker: "phase007_product_smoke_gate=failed",
    state: ProductSmokeGateState.Failed,
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
  const result = await runProductSmokeGate();
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
