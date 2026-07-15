import { readFile, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

export const ReleaseGateState = Object.freeze({
  Pending: "Pending",
  ReadingSources: "ReadingSources",
  ValidatingEvidence: "ValidatingEvidence",
  WritingReport: "WritingReport",
  Passed: "Passed",
  Failed: "Failed",
});

export const ReleaseGateEvent = Object.freeze({
  Start: "Start",
  SourcesLoaded: "SourcesLoaded",
  EvidenceValidated: "EvidenceValidated",
  ReportWritten: "ReportWritten",
  Fail: "Fail",
});

export const ReleaseGateErrorCode = Object.freeze({
  RequiredEvidenceMissing: "PHASE006_RELEASE_REQUIRED_EVIDENCE_MISSING",
  SourceReadFailed: "PHASE006_RELEASE_SOURCE_READ_FAILED",
  InvalidTransition: "PHASE006_RELEASE_INVALID_TRANSITION",
});

const requiredTargets = Object.freeze([
  target("product_smoke_prerequisite", "Phase 006 product smoke prerequisite", {
    requiredFiles: [".tasks/phase006-product-smoke-gate-result.md"],
    evidence: ["phase006_product_smoke_gate=passed"],
  }),
  target("lower_final_prerequisites", "lower final prerequisites", {
    requiredFiles: [
      ".tasks/phase006-backup-package-gate-result.md",
      ".tasks/phase006-ai-ux-gate-result.md",
    ],
    evidence: ["phase006_backup_package_gate=passed", "phase006_ai_ux_gate=passed"],
  }),
  target("ownership_and_performance_evidence", "ownership and performance evidence", {
    requiredFiles: [
      ".tasks/release/data-ownership-verification.md",
      ".tasks/release/performance-budget-phase006.md",
    ],
    evidence: [
      "phase006_data_ownership_verification=passed",
      "phase006_document_query_budget=passed",
      "phase006_search_graph_asset_budget=passed",
      "phase006_ai_status_result_budget=passed",
    ],
  }),
  target("local_desktop_runbook_evidence", "local desktop runbook evidence", {
    requiredFiles: [".tasks/release/local-desktop-runbook.md"],
    evidence: [
      "Clean Install",
      "Startup Repair",
      "Index Rebuild",
      "Read-Only Recovery",
      "Sensitive Data Exclusion",
    ],
  }),
  target("security_manifest_final_targets", "security manifest final targets", {
    requiredFiles: [".tasks/release/security-log-policy-manifest.json"],
    evidence: [
      '"id": "phase006_final_release_gate_result"',
      '"path": ".tasks/phase006-release-gate-result.md"',
      '"required": true',
      '"id": "phase006_product_smoke_gate_result"',
      '"id": "phase006_data_ownership_verification"',
    ],
  }),
  target("release_tooling", "release tooling package scripts", {
    requiredFiles: ["package.json"],
    evidence: [
      "run:phase006-release-gate-tests",
      "run:phase006-release-gate",
      "run:security-log-scanner",
      "run:runbook-validator",
    ],
  }),
]);

export function transitionReleaseGateState(currentState, event, detail = {}) {
  if (currentState === ReleaseGateState.Pending && event === ReleaseGateEvent.Start) {
    return { state: ReleaseGateState.ReadingSources };
  }
  if (currentState === ReleaseGateState.ReadingSources && event === ReleaseGateEvent.SourcesLoaded) {
    return { state: ReleaseGateState.ValidatingEvidence };
  }
  if (currentState === ReleaseGateState.ValidatingEvidence && event === ReleaseGateEvent.EvidenceValidated) {
    return { state: ReleaseGateState.WritingReport };
  }
  if (currentState === ReleaseGateState.WritingReport && event === ReleaseGateEvent.ReportWritten) {
    return { state: ReleaseGateState.Passed };
  }
  if (
    [ReleaseGateState.ReadingSources, ReleaseGateState.ValidatingEvidence, ReleaseGateState.WritingReport].includes(
      currentState,
    ) &&
    event === ReleaseGateEvent.Fail
  ) {
    return {
      state: ReleaseGateState.Failed,
      errorCode: detail.errorCode ?? ReleaseGateErrorCode.RequiredEvidenceMissing,
      findingId: detail.findingId,
    };
  }
  return { state: ReleaseGateState.Failed, errorCode: ReleaseGateErrorCode.InvalidTransition };
}

export function analyzeReleaseEvidence({ sources }) {
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
    marker: "phase006_release_gate=passed",
    state: ReleaseGateState.Passed,
    summary: { requiredTargets: requiredTargets.length, missingRequiredEvidence: 0 },
    targetResults,
    missingEvidence: [],
  };
}

export function renderReleaseGateMarkdown(result) {
  const lines = [
    "# Phase 006 Final Release Gate Result",
    "",
    result.marker,
    "",
    "- phase: `Phase 006`",
    "- gate: `Final Personal Desktop Release`",
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
    "This artifact records target ids, marker status, counts, and stable error codes only. It does not record document body, asset content, raw prompt, raw generated response, personal path, credential, token, provider secret, local machine secret, or package internal file contents.",
    "",
    "## Release Boundary",
    "",
    "- Phase 006 release scope is the personal PC installable desktop knowledge management app evidence set.",
    "- Server hosting, SaaS, multi-user, OS signing, notarization, and app store distribution remain outside this release gate.",
    "",
  );
  return lines.join("\n");
}

export async function runReleaseGate({ root = process.cwd() } = {}) {
  let state = transitionReleaseGateState(ReleaseGateState.Pending, ReleaseGateEvent.Start);
  try {
    const sources = {};
    for (const filePath of collectRequiredFiles()) {
      sources[filePath] = await readFile(`${root}/${filePath}`, "utf8");
    }
    state = transitionReleaseGateState(state.state, ReleaseGateEvent.SourcesLoaded);
    const result = analyzeReleaseEvidence({ sources });
    if (!result.passed) {
      state = transitionReleaseGateState(state.state, ReleaseGateEvent.Fail, {
        errorCode: result.errorCode,
        findingId: result.missingEvidence[0]?.targetId,
      });
      return { ...result, state: state.state };
    }
    state = transitionReleaseGateState(state.state, ReleaseGateEvent.EvidenceValidated);
    state = transitionReleaseGateState(state.state, ReleaseGateEvent.ReportWritten);
    return { ...result, state: state.state };
  } catch {
    state = transitionReleaseGateState(state.state, ReleaseGateEvent.Fail, {
      errorCode: ReleaseGateErrorCode.SourceReadFailed,
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

function failedResult({ errorCode, state = ReleaseGateState.Failed, missingEvidence, targetResults = [] }) {
  return {
    passed: false,
    marker: "phase006_release_gate=failed",
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
  const result = await runReleaseGate();
  await writeFile(".tasks/phase006-release-gate-result.md", renderReleaseGateMarkdown(result));
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
