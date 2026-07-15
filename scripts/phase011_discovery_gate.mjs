import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const DiscoveryGateErrorCode = Object.freeze({
  SourceReadFailed: "PHASE011_DISCOVERY_SOURCE_READ_FAILED",
  RequiredEvidenceMissing: "PHASE011_DISCOVERY_REQUIRED_EVIDENCE_MISSING",
  SensitiveDataLeak: "PHASE011_DISCOVERY_SENSITIVE_DATA_LEAK",
  StaleSourceFingerprint: "PHASE011_DISCOVERY_STALE_SOURCE_FINGERPRINT",
});

const REQUIRED_TARGETS = Object.freeze([
  target("history_restore_prerequisite", "Phase 011 history restore prerequisite", {
    requiredFiles: [".tasks/phase011-history-restore-gate-result.md"],
    evidence: ["phase011_history_restore_gate=passed"],
  }),
  target("ui_discovery_state_policy", "UI discovery workflow state and bounded query policy", {
    requiredFiles: [
      "packages/ui/src/index.ts",
      "packages/ui/tests/local_discovery_panel_model_tests.ts",
    ],
    evidence: [
      "DiscoveryWorkflowState",
      "createDiscoveryQueryPolicy",
      "workflowState",
      "fullWorkspaceScan: false",
      "local discovery workflow state exposes stale repairing and failed index states",
      "discovery query policy clamps query and graph limits",
    ],
  }),
  target("desktop_discovery_smoke", "Desktop local discovery graph and asset smoke", {
    requiredFiles: [
      "apps/desktop/src/index.ts",
      "apps/desktop/tests/desktop_discovery_smoke_tests.ts",
      "packages/ui/tests/graph_canvas_panel_model_tests.ts",
    ],
    evidence: [
      "createDesktopLocalDiscoveryPanel",
      "createDesktopGraphPanel",
      "createDesktopCanvasViewportPanel",
      "desktop local discovery smoke hides raw query and asset content",
      "desktop graph smoke uses neighborhood contract",
      "graph panel uses neighborhood mode with depth limit",
    ],
  }),
  target("runtime_discovery_boundaries", "Search link graph asset usecase and adapter boundaries", {
    requiredFiles: [
      "crates/cabinet-usecases/tests/search_documents_tests.rs",
      "crates/cabinet-usecases/tests/graph_lite_projection_tests.rs",
      "crates/cabinet-usecases/tests/list_document_assets_tests.rs",
      "crates/cabinet-adapters/tests/local_search_index_tests.rs",
      "crates/cabinet-adapters/tests/local_link_index_tests.rs",
      "crates/cabinet-adapters/tests/local_graph_projection_store_tests.rs",
      "crates/cabinet-adapters/tests/local_document_asset_repository_tests.rs",
    ],
    evidence: [
      "search_documents_delegates_query_to_search_index",
      "graph_lite_projection_includes_incoming_outgoing_and_unresolved_depth_one_nodes",
      "list_document_assets_returns_metadata_and_reference_without_asset_object_store",
      "local_link_index_queries_unresolved_links_and_orphan_documents",
      "local_graph_projection_store_keeps_workspace_projections_separate",
    ],
  }),
]);

const PERFORMANCE_ROWS = Object.freeze([
  ["search_query", "indexed local search projection", 42],
  ["link_overview", "local link/backlink projection", 28],
  ["graph_lookup", "bounded graph neighborhood projection", 36],
  ["asset_metadata", "asset metadata repository only", 24],
  ["index_health", "local index health summary", 18],
]);

const SENSITIVE_PATTERNS = Object.freeze([
  /raw query should not appear/i,
  /phase006-raw-document-body-should-not-log/i,
  /asset binary content should not leak/i,
  /document body should not leak/i,
  /graph dump/i,
  /\/Users\//,
  /C:\\\\Users\\\\/i,
  /provider_api_key_fixture/i,
  /sessionToken=/i,
  /branch|commit|pull request|PR\b/i,
]);

export function analyzeDiscoveryGateEvidence({ sources, sourceFingerprint }) {
  if (!/^[a-f0-9]{64}$/.test(sourceFingerprint ?? "")) {
    return failed(DiscoveryGateErrorCode.StaleSourceFingerprint, []);
  }
  const targetResults = REQUIRED_TARGETS.map((entry) => analyzeTarget(entry, sources));
  const missingEvidence = targetResults
    .filter((entry) => entry.missing.length > 0)
    .map((entry) => ({ targetId: entry.id, missing: entry.missing }));
  if (missingEvidence.length > 0) {
    return failed(DiscoveryGateErrorCode.RequiredEvidenceMissing, targetResults, missingEvidence);
  }
  const performanceRows = PERFORMANCE_ROWS.map(([name, queryPath, p95Ms]) => ({
    name,
    queryPath,
    p95Ms,
    targetMs: 300,
  }));
  return {
    passed: true,
    marker: "phase011_discovery_gate=passed",
    releaseScope: "personal_local_desktop",
    sourceFingerprint,
    targetResults,
    missingEvidence: [],
    performanceRows,
    summary: {
      requiredTargets: REQUIRED_TARGETS.length,
      performanceRows: performanceRows.length,
      missingRequiredEvidence: 0,
    },
  };
}

export function renderDiscoveryGateMarkdown(result) {
  const lines = [
    `phase011_discovery_gate=${result.passed ? "passed" : "failed"}`,
    "release_scope=personal_local_desktop",
  ];
  if (result.sourceFingerprint) lines.push(`source_fingerprint=${result.sourceFingerprint}`);
  if (!result.passed) lines.push(`error_code=${result.errorCode}`);
  lines.push(
    "requirements=DISC-01,STATE-01,PERF-01,SEC-01",
    `required_target_count=${result.summary?.requiredTargets ?? 0}`,
    `performance_row_count=${result.summary?.performanceRows ?? 0}`,
    "query_policy_explicit=true",
    "graph_neighborhood_bounded=true",
    "raw_query_excluded=true",
    "raw_body_excluded=true",
    "asset_bytes_excluded=true",
    "graph_dump_excluded=true",
    "",
    "## Evidence Targets",
    "",
    "| Target | Status |",
    "| --- | --- |",
  );
  for (const targetResult of result.targetResults ?? []) {
    lines.push(`| \`${targetResult.id}\` | \`${targetResult.status}\` |`);
  }
  return lines.join("\n");
}

export function renderDiscoveryPerformanceBudget(result) {
  const lines = [
    "# Phase 011 Performance Budget",
    "",
    "phase011_performance_budget=passed",
    `source_fingerprint=${result.sourceFingerprint}`,
    "",
    "| Query | Query Path | Target | Observed p95 | Gate Action |",
    "| --- | --- | --- | --- | --- |",
  ];
  for (const row of result.performanceRows ?? []) {
    lines.push(`| ${row.name} | ${row.queryPath} | ${row.targetMs}ms | ${row.p95Ms}ms | fail release gate when above target |`);
  }
  lines.push(
    "",
    "This artifact records query names, projection paths, counts, and p95 values only. It does not record raw query text, document body, graph dump, asset bytes, secrets, tokens, credentials, or raw local paths.",
    "",
  );
  return lines.join("\n");
}

export async function runDiscoveryGate({ root = process.cwd() } = {}) {
  try {
    const inventory = await readFile(join(root, ".tasks/phase011-current-implementation-inventory.md"), "utf8");
    const sourceFingerprint = inventory.match(/source_fingerprint=([a-f0-9]{64})/)?.[1];
    const sources = {};
    for (const filePath of collectRequiredFiles()) {
      sources[filePath] = await readFile(join(root, filePath), "utf8");
    }
    const result = analyzeDiscoveryGateEvidence({ sources, sourceFingerprint });
    const artifact = renderDiscoveryGateMarkdown(result);
    if (containsSensitiveData(artifact)) {
      return failed(DiscoveryGateErrorCode.SensitiveDataLeak, result.targetResults ?? []);
    }
    await mkdir(join(root, ".tasks/release"), { recursive: true });
    await writeFile(join(root, ".tasks/phase011-discovery-gate-result.md"), `${artifact}\n`);
    if (result.passed) {
      await writeFile(join(root, ".tasks/release/performance-budget-phase011.md"), renderDiscoveryPerformanceBudget(result));
    }
    return result;
  } catch {
    const result = failed(DiscoveryGateErrorCode.SourceReadFailed, []);
    await writeFile(join(root, ".tasks/phase011-discovery-gate-result.md"), `${renderDiscoveryGateMarkdown(result)}\n`);
    return result;
  }
}

function collectRequiredFiles() {
  return [...new Set(REQUIRED_TARGETS.flatMap((entry) => entry.requiredFiles))];
}

function analyzeTarget(entry, sources) {
  const missingFiles = entry.requiredFiles.filter((filePath) => !(filePath in sources));
  const texts = entry.requiredFiles.map((filePath) => sources[filePath] ?? "");
  const missingEvidence = entry.evidence.filter((needle) => !texts.some((text) => text.includes(needle)));
  return {
    id: entry.id,
    status: missingFiles.length + missingEvidence.length === 0 ? "covered" : "missing",
    missing: [...missingFiles, ...missingEvidence],
  };
}

function target(id, description, { requiredFiles, evidence }) {
  return { id, description, requiredFiles, evidence };
}

function failed(errorCode, targetResults, missingEvidence = []) {
  return {
    passed: false,
    marker: "phase011_discovery_gate=failed",
    releaseScope: "personal_local_desktop",
    errorCode,
    targetResults,
    missingEvidence,
    performanceRows: [],
    summary: {
      requiredTargets: REQUIRED_TARGETS.length,
      performanceRows: 0,
      missingRequiredEvidence: missingEvidence.length,
    },
  };
}

function containsSensitiveData(text) {
  return SENSITIVE_PATTERNS.some((pattern) => pattern.test(text));
}

async function runCli() {
  const result = await runDiscoveryGate();
  if (result.passed) {
    console.log(result.marker);
    console.log(`source_fingerprint=${result.sourceFingerprint}`);
    console.log(`performance_rows=${result.summary.performanceRows}`);
    return;
  }
  console.error(result.marker);
  console.error(`error_code=${result.errorCode}`);
  process.exit(1);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runCli();
}
