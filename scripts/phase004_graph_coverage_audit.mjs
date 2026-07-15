import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const GraphCoverageAuditState = Object.freeze({
  NotStarted: "NotStarted",
  ReadingSource: "ReadingSource",
  Auditing: "Auditing",
  Reported: "Reported",
  Failed: "Failed",
});

export const GraphCoverageAuditEvent = Object.freeze({
  Start: "Start",
  SourceLoaded: "SourceLoaded",
  AuditComplete: "AuditComplete",
  ReportWritten: "ReportWritten",
  Fail: "Fail",
});

export const GraphCoverageAuditErrorCode = Object.freeze({
  InvalidTransition: "PHASE004_GRAPH_COVERAGE_INVALID_TRANSITION",
  SourceSetEmpty: "PHASE004_GRAPH_COVERAGE_SOURCE_SET_EMPTY",
});

const STATUS = Object.freeze({
  Covered: "covered",
  Missing: "missing",
});

const TARGETS = Object.freeze([
  target("graph_domain_usecase_contract", "Graph domain, usecase, port, and adapter contract", {
    files: [
      "crates/cabinet-domain/src/graph.rs",
      "crates/cabinet-domain/tests/graph_tests.rs",
      "crates/cabinet-ports/src/graph_projection.rs",
      "crates/cabinet-adapters/src/local_graph_projection.rs",
      "crates/cabinet-usecases/src/graph.rs",
      "crates/cabinet-usecases/tests/permission_aware_graph_tests.rs",
    ],
    evidence: [
      "KnowledgeGraph",
      "GraphProjectionStore",
      "LocalGraphProjectionStore",
      "PermissionAwareGraphUsecase",
      "permission_aware_graph_filters_denied_document_nodes_and_edges",
    ],
    priority: 100,
  }),
  target("graph_runtime_product_smoke", "Self-host graph runtime API and product smoke", {
    files: [
      "crates/cabinet-server/src/composition.rs",
      "crates/cabinet-server/src/runtime.rs",
      "crates/cabinet-server/tests/server_runtime_wiring_tests.rs",
      "scripts/run_self_host_e2e_smoke.mjs",
    ],
    evidence: [
      "graph.get_local",
      "HandlerKind::GraphLocal",
      "graph_handler_uses_projection_port_and_filters_denied_documents",
      "graph_hidden_node_filtered",
      "product_log_event=graph.query.completed",
      "graph_under_300ms_target",
    ],
    priority: 98,
  }),
  target("graph_client_web_desktop_contract", "Client-core, Web, and desktop graph contract", {
    files: [
      "packages/client-core/src/index.ts",
      "packages/client-core/tests/collaboration_api_client_tests.ts",
      "apps/web/src/index.ts",
      "apps/web/tests/web_graph_model_tests.ts",
      "apps/desktop/src/index.ts",
      "apps/desktop/tests/desktop_remote_product_smoke.ts",
    ],
    evidence: [
      "KnowledgeGraphQuery",
      "getKnowledgeGraph",
      "createWebGraphViewModel",
      "readDesktopRemoteKnowledgeGraph",
      "desktop_remote_product_step_passed=remote_graph_flow",
    ],
    priority: 96,
  }),
  target("graph_desktop_product_smoke", "Desktop remote graph product smoke", {
    files: [
      "scripts/run_desktop_remote_product_smoke.sh",
      "scripts/run_desktop_remote_product_smoke.mjs",
      "apps/desktop/tests/desktop_remote_product_smoke.ts",
    ],
    evidence: [
      "runDesktopSmoke",
      "desktop_remote_product_step_passed=remote_graph_flow",
      "doc-hidden",
      "observedMs",
      "assertSensitiveOutputClean",
    ],
    priority: 94,
  }),
  target("graph_performance_evidence", "Permission-aware graph p95 300ms benchmark evidence", {
    files: ["crates/cabinet-platform/tests/query_performance_benchmarks.rs"],
    evidence: [
      "PermissionAwareGraphLookup",
      "PermissionAwareGraphUsecase",
      "LocalGraphProjectionStore",
    ],
    priority: 92,
  }),
]);

class GraphCoverageAuditError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "GraphCoverageAuditError";
    this.code = code;
  }
}

export function transitionGraphCoverageAuditState(state, event) {
  const key = `${state}:${event}`;
  const transitions = new Map([
    [
      `${GraphCoverageAuditState.NotStarted}:${GraphCoverageAuditEvent.Start}`,
      GraphCoverageAuditState.ReadingSource,
    ],
    [
      `${GraphCoverageAuditState.ReadingSource}:${GraphCoverageAuditEvent.SourceLoaded}`,
      GraphCoverageAuditState.Auditing,
    ],
    [
      `${GraphCoverageAuditState.Auditing}:${GraphCoverageAuditEvent.AuditComplete}`,
      GraphCoverageAuditState.Reported,
    ],
    [
      `${GraphCoverageAuditState.Reported}:${GraphCoverageAuditEvent.ReportWritten}`,
      GraphCoverageAuditState.Reported,
    ],
    [
      `${GraphCoverageAuditState.ReadingSource}:${GraphCoverageAuditEvent.Fail}`,
      GraphCoverageAuditState.Failed,
    ],
    [
      `${GraphCoverageAuditState.Auditing}:${GraphCoverageAuditEvent.Fail}`,
      GraphCoverageAuditState.Failed,
    ],
  ]);
  const nextState = transitions.get(key);
  if (!nextState) {
    throw new GraphCoverageAuditError(
      GraphCoverageAuditErrorCode.InvalidTransition,
      `invalid graph coverage audit transition: ${key}`,
    );
  }
  return nextState;
}

export function analyzeGraphCoverageSources({ sources }) {
  if (!sources || Object.keys(sources).length === 0) {
    throw new GraphCoverageAuditError(
      GraphCoverageAuditErrorCode.SourceSetEmpty,
      "phase004 graph coverage audit source set is empty",
    );
  }

  const targets = TARGETS.map((entry) => analyzeTarget(entry, sources));
  const targetsNeedingWork = targets.filter((entry) => entry.status !== STATUS.Covered);

  return {
    phase: "Phase 004.2",
    sourceFiles: Object.keys(sources).sort(),
    summary: {
      totalTargets: targets.length,
      covered: targets.filter((entry) => entry.status === STATUS.Covered).length,
      missing: targets.filter((entry) => entry.status === STATUS.Missing).length,
      targetsNeedingWork: targetsNeedingWork.length,
    },
    findings:
      targetsNeedingWork.length === 0
        ? []
        : [
            {
              id: "PHASE004_GRAPH_COVERAGE_GAP",
              message: "Some Phase 004.2 graph coverage targets are missing evidence.",
              targetIds: targetsNeedingWork.map((entry) => entry.id),
            },
          ],
    nextImplementationTarget: pickNextImplementationTarget(targetsNeedingWork),
    targets,
  };
}

export function renderGraphCoverageAuditMarkdown(audit) {
  const lines = [
    "# Phase 004 Graph Coverage Audit",
    "",
    "현재 단계: Phase 004.2 - Graph Runtime API, Client Model, and Product Smoke",
    "",
    "## Purpose",
    "",
    "- graph domain/usecase/port/adapter/runtime/client/Web/desktop/performance evidence를 코드 기준으로 고정한다.",
    "- static contract만 있는 상태를 product smoke 또는 performance evidence complete로 오판하지 않는다.",
    "- 다음 task는 모든 graph gap을 한 번에 구현하지 않고 가장 높은 우선순위 target 하나를 선택한다.",
    "",
    "## Summary",
    "",
    "| Metric | Count |",
    "| --- | ---: |",
    `| total targets | ${audit.summary.totalTargets} |`,
    `| covered | ${audit.summary.covered} |`,
    `| missing | ${audit.summary.missing} |`,
    `| targets needing work | ${audit.summary.targetsNeedingWork} |`,
    "",
    "## Target Status",
    "",
    "| Target | Label | Status | Missing Files | Missing Evidence |",
    "| --- | --- | --- | --- | --- |",
    ...audit.targets.map((entry) => {
      const missingFiles =
        entry.missingFiles.length > 0 ? entry.missingFiles.map(code).join(", ") : "none";
      const missingEvidence =
        entry.missingEvidence.length > 0 ? entry.missingEvidence.map(code).join(", ") : "none";
      return `| \`${entry.id}\` | ${entry.label} | ${entry.status} | ${missingFiles} | ${missingEvidence} |`;
    }),
    "",
    "## Findings",
    "",
  ];

  if (audit.findings.length === 0) {
    lines.push("- No graph coverage gap was detected.");
  } else {
    for (const finding of audit.findings) {
      lines.push(`- ${finding.id}: ${finding.message}`);
      lines.push(`- affected target count: ${finding.targetIds.length}`);
      lines.push(`- affected targets: ${finding.targetIds.map(code).join(", ")}`);
    }
  }

  lines.push("", "## Next Implementation Target", "");
  if (audit.nextImplementationTarget) {
    lines.push(
      `- target id: \`${audit.nextImplementationTarget.id}\``,
      `- label: ${audit.nextImplementationTarget.label}`,
      `- current status: ${audit.nextImplementationTarget.status}`,
      "- selected reason: highest priority Phase 004.2 graph target that is not covered.",
      "- next task should implement or harden this selected target only.",
    );
  } else {
    lines.push("- No next graph coverage target remains.");
  }

  lines.push(
    "",
    "## Review Notes",
    "",
    "- Graph permission filtering must remain in server usecase logic, not Web or desktop UI.",
    "- Graph product smoke must run through actual process/client boundaries.",
    "- Permission-aware graph performance must be measured as a separate p95 300ms target.",
    "- Audit output must not include document bodies, tokens, secrets, credentials, raw link text, or attachment content.",
  );

  return `${lines.join("\n")}\n`;
}

async function runCli() {
  let state = transitionGraphCoverageAuditState(
    GraphCoverageAuditState.NotStarted,
    GraphCoverageAuditEvent.Start,
  );
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const resultPath = path.join(repoRoot, ".tasks/graph-coverage-audit.md");
  const sources = await readAuditSources(repoRoot);
  state = transitionGraphCoverageAuditState(state, GraphCoverageAuditEvent.SourceLoaded);
  const audit = analyzeGraphCoverageSources({ sources });
  state = transitionGraphCoverageAuditState(state, GraphCoverageAuditEvent.AuditComplete);
  await mkdir(path.dirname(resultPath), { recursive: true });
  await writeFile(resultPath, renderGraphCoverageAuditMarkdown(audit));
  state = transitionGraphCoverageAuditState(state, GraphCoverageAuditEvent.ReportWritten);

  if (audit.summary.targetsNeedingWork === 0) {
    console.log("phase004_graph_coverage_audit=passed");
    console.log(`state=${state}`);
    console.log(`target_count=${audit.summary.totalTargets}`);
    console.log(`result_path=${resultPath}`);
    return;
  }

  console.error("phase004_graph_coverage_audit=failed");
  console.error(`state=${state}`);
  console.error(`targets_needing_work=${audit.summary.targetsNeedingWork}`);
  console.error(`result_path=${resultPath}`);
  process.exitCode = 1;
}

async function readAuditSources(repoRoot) {
  const paths = new Set(TARGETS.flatMap((entry) => entry.files));
  const sources = {};
  for (const relativePath of paths) {
    try {
      sources[relativePath] = await readFile(path.join(repoRoot, relativePath), "utf8");
    } catch {
      // Missing files are represented by absence from the source map.
    }
  }
  return sources;
}

function analyzeTarget(entry, sources) {
  const combined = entry.files.map((filePath) => sources[filePath] ?? "").join("\n");
  const missingFiles = entry.files.filter((filePath) => !(filePath in sources));
  const missingEvidence = entry.evidence.filter((needle) => !combined.includes(needle));
  const status =
    missingFiles.length === 0 && missingEvidence.length === 0
      ? STATUS.Covered
      : STATUS.Missing;
  return {
    id: entry.id,
    label: entry.label,
    priority: entry.priority,
    status,
    missingFiles,
    missingEvidence,
  };
}

function pickNextImplementationTarget(targetsNeedingWork) {
  if (targetsNeedingWork.length === 0) {
    return null;
  }
  return [...targetsNeedingWork].sort((a, b) => b.priority - a.priority)[0];
}

function target(id, label, options) {
  return {
    id,
    label,
    files: options.files,
    evidence: options.evidence,
    priority: options.priority,
  };
}

function code(value) {
  return `\`${value}\``;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await runCli();
}
