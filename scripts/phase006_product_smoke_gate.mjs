import { readFile, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

export const ProductSmokeGateState = Object.freeze({
  Pending: "Pending",
  ReadingSources: "ReadingSources",
  ValidatingEvidence: "ValidatingEvidence",
  WritingReport: "WritingReport",
  Passed: "Passed",
  Failed: "Failed",
});

export const ProductSmokeGateEvent = Object.freeze({
  Start: "Start",
  SourcesLoaded: "SourcesLoaded",
  EvidenceValidated: "EvidenceValidated",
  ReportWritten: "ReportWritten",
  Fail: "Fail",
});

export const ProductSmokeGateErrorCode = Object.freeze({
  RequiredEvidenceMissing: "PHASE006_PRODUCT_SMOKE_REQUIRED_EVIDENCE_MISSING",
  SourceReadFailed: "PHASE006_PRODUCT_SMOKE_SOURCE_READ_FAILED",
  InvalidTransition: "PHASE006_PRODUCT_SMOKE_INVALID_TRANSITION",
});

const requiredTargets = Object.freeze([
  target("lower_phase006_gates", "lower Phase 006 gates", {
    requiredFiles: [
      ".tasks/phase006-plan-validation-result.md",
      ".tasks/phase006-local-runtime-gate-result.md",
      ".tasks/phase006-workspace-shell-gate-result.md",
      ".tasks/phase006-document-ux-gate-result.md",
      ".tasks/phase006-search-graph-asset-gate-result.md",
      ".tasks/phase006-ai-ux-gate-result.md",
      ".tasks/phase006-backup-package-gate-result.md",
    ],
    evidence: [
      "phase006_plan_validation=passed",
      "phase006_local_runtime_gate=passed",
      "phase006_workspace_shell_gate=passed",
      "phase006_document_ux_gate=passed",
      "phase006_search_graph_asset_gate=passed",
      "phase006_ai_ux_gate=passed",
      "phase006_backup_package_gate=passed",
    ],
  }),
  target("data_ownership_report", "data ownership verification report", {
    requiredFiles: [".tasks/release/data-ownership-verification.md"],
    evidence: ["phase006_data_ownership_verification=passed"],
  }),
  target("phase006_performance_budgets", "Phase 006 p95 performance budgets", {
    requiredFiles: [".tasks/release/performance-budget-phase006.md"],
    evidence: [
      "phase006_document_query_budget=passed",
      "phase006_search_graph_asset_budget=passed",
      "phase006_ai_status_result_budget=passed",
    ],
  }),
  target("desktop_product_smoke_surfaces", "desktop product smoke surfaces", {
    requiredFiles: [
      "apps/desktop/tests/desktop_personal_workspace_shell_tests.ts",
      "apps/desktop/tests/desktop_document_ux_smoke_tests.ts",
      "apps/desktop/tests/desktop_discovery_smoke_tests.ts",
      "apps/desktop/tests/desktop_ai_local_ux_smoke_tests.ts",
      "apps/desktop/tests/desktop_backup_restore_staging_smoke_tests.ts",
      "apps/desktop/tests/desktop_import_preview_smoke_tests.ts",
    ],
    evidence: [
      "desktop current product shell",
      "desktop document UX smoke",
      "desktop local discovery smoke",
      "desktop AI local UX smoke",
      "desktop backup restore smoke",
      "desktop import preview smoke",
    ],
  }),
  target("local_desktop_runbook", "local desktop runbook", {
    requiredFiles: [".tasks/release/local-desktop-runbook.md"],
    evidence: ["clean install", "startup repair", "Index Rebuild", "backup", "restore"],
  }),
]);

export function transitionProductSmokeGateState(currentState, event, detail = {}) {
  if (currentState === ProductSmokeGateState.Pending && event === ProductSmokeGateEvent.Start) {
    return { state: ProductSmokeGateState.ReadingSources };
  }
  if (currentState === ProductSmokeGateState.ReadingSources && event === ProductSmokeGateEvent.SourcesLoaded) {
    return { state: ProductSmokeGateState.ValidatingEvidence };
  }
  if (
    currentState === ProductSmokeGateState.ValidatingEvidence &&
    event === ProductSmokeGateEvent.EvidenceValidated
  ) {
    return { state: ProductSmokeGateState.WritingReport };
  }
  if (currentState === ProductSmokeGateState.WritingReport && event === ProductSmokeGateEvent.ReportWritten) {
    return { state: ProductSmokeGateState.Passed };
  }
  if (
    [ProductSmokeGateState.ReadingSources, ProductSmokeGateState.ValidatingEvidence, ProductSmokeGateState.WritingReport].includes(
      currentState,
    ) &&
    event === ProductSmokeGateEvent.Fail
  ) {
    return {
      state: ProductSmokeGateState.Failed,
      errorCode: detail.errorCode ?? ProductSmokeGateErrorCode.RequiredEvidenceMissing,
      findingId: detail.findingId,
    };
  }
  return { state: ProductSmokeGateState.Failed, errorCode: ProductSmokeGateErrorCode.InvalidTransition };
}

export function analyzeProductSmokeEvidence({ sources }) {
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
    marker: "phase006_product_smoke_gate=passed",
    state: ProductSmokeGateState.Passed,
    summary: { requiredTargets: requiredTargets.length, missingRequiredEvidence: 0 },
    targetResults,
    missingEvidence: [],
  };
}

export function renderProductSmokeGateMarkdown(result) {
  const lines = [
    "# Phase 006 Product Smoke Gate Result",
    "",
    result.marker,
    "",
    "- phase: `Phase 006`",
    "- gate: `Personal Desktop Product Smoke`",
    `- status: \`${result.passed ? "passed" : "failed"}\``,
    `- state: \`${result.state}\``,
  ];
  if (!result.passed) {
    lines.push(`- error code: \`${result.errorCode}\``);
  }
  lines.push(
    `- required targets: \`${result.summary.requiredTargets}\``,
    `- missing required evidence: \`${result.summary.missingRequiredEvidence}\``,
    "",
    "## Evidence",
    "",
    "| Target | Status | Description |",
    "| --- | --- | --- |",
  );
  for (const targetResult of result.targetResults ?? []) {
    lines.push(`| \`${targetResult.id}\` | \`${targetResult.status}\` | ${targetResult.description} |`);
  }
  lines.push(
    "",
    "## Sensitive Data Exclusion",
    "",
    "This artifact records target ids, marker status, counts, and stable error codes only. It does not record document body, asset content, raw prompt, raw generated response, personal path, credential, token, provider secret, or local machine secret.",
    "",
  );
  return lines.join("\n");
}

export async function runProductSmokeGate({ root = process.cwd() } = {}) {
  let state = transitionProductSmokeGateState(ProductSmokeGateState.Pending, ProductSmokeGateEvent.Start);
  try {
    const sources = {};
    for (const filePath of collectRequiredFiles()) {
      sources[filePath] = await readFile(`${root}/${filePath}`, "utf8");
    }
    state = transitionProductSmokeGateState(state.state, ProductSmokeGateEvent.SourcesLoaded);
    const result = analyzeProductSmokeEvidence({ sources });
    if (!result.passed) {
      state = transitionProductSmokeGateState(state.state, ProductSmokeGateEvent.Fail, {
        errorCode: result.errorCode,
        findingId: result.missingEvidence[0]?.targetId,
      });
      return { ...result, state: state.state };
    }
    state = transitionProductSmokeGateState(state.state, ProductSmokeGateEvent.EvidenceValidated);
    state = transitionProductSmokeGateState(state.state, ProductSmokeGateEvent.ReportWritten);
    return { ...result, state: state.state };
  } catch {
    state = transitionProductSmokeGateState(state.state, ProductSmokeGateEvent.Fail, {
      errorCode: ProductSmokeGateErrorCode.SourceReadFailed,
    });
    return failedResult({
      errorCode: state.errorCode,
      state: state.state,
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

function failedResult({ errorCode, state = ProductSmokeGateState.Failed, missingEvidence, targetResults = [] }) {
  return {
    passed: false,
    marker: "phase006_product_smoke_gate=failed",
    state,
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
  const result = await runProductSmokeGate();
  await writeFile(".tasks/phase006-product-smoke-gate-result.md", renderProductSmokeGateMarkdown(result));
  if (result.passed) {
    console.log(result.marker);
    console.log(`gate_state=${result.state}`);
    console.log(`required_targets=${result.summary.requiredTargets}`);
    return;
  }
  console.error(result.marker);
  console.error(`gate_state=${result.state}`);
  console.error(`error_code=${result.errorCode}`);
  process.exit(1);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runCli();
}
