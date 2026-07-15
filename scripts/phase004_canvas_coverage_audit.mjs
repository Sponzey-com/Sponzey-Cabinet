import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const CanvasCoverageAuditState = Object.freeze({
  NotStarted: "NotStarted",
  ReadingSource: "ReadingSource",
  Auditing: "Auditing",
  Reported: "Reported",
  Failed: "Failed",
});

export const CanvasCoverageAuditEvent = Object.freeze({
  Start: "Start",
  SourceLoaded: "SourceLoaded",
  AuditComplete: "AuditComplete",
  ReportWritten: "ReportWritten",
  Fail: "Fail",
});

export const CanvasCoverageAuditErrorCode = Object.freeze({
  InvalidTransition: "PHASE004_CANVAS_COVERAGE_INVALID_TRANSITION",
  SourceSetEmpty: "PHASE004_CANVAS_COVERAGE_SOURCE_SET_EMPTY",
});

const STATUS = Object.freeze({
  Covered: "covered",
  Missing: "missing",
});

const TARGETS = Object.freeze([
  target("canvas_domain_lifecycle_contract", "Canvas domain model and lifecycle contract", {
    files: ["crates/cabinet-domain/src/canvas.rs", "crates/cabinet-domain/tests/canvas_tests.rs"],
    evidence: [
      "Canvas",
      "CanvasNode",
      "CanvasEdge",
      "CanvasEmbed",
      "CanvasLifecycleState",
      "transition_canvas_lifecycle",
      "canvas_rejects_edge_with_missing_node_reference",
      "canvas_accepts_document_attachment_external_link_and_text_card_nodes",
      "canvas_embed_uses_stable_reference_without_raw_ui_state",
      "canvas_lifecycle_uses_explicit_transitions",
    ],
    priority: 100,
  }),
  target(
    "canvas_usecase_repository_contract",
    "Canvas usecase, repository port, and local adapter contract",
    {
      files: [
        "crates/cabinet-usecases/src/canvas.rs",
        "crates/cabinet-usecases/tests/canvas_usecase_tests.rs",
        "crates/cabinet-ports/src/canvas_repository.rs",
        "crates/cabinet-ports/tests/canvas_repository_contract_tests.rs",
        "crates/cabinet-adapters/src/local_canvas_repository.rs",
        "crates/cabinet-adapters/tests/local_canvas_repository_tests.rs",
      ],
      evidence: [
        "CreateCanvasUsecase",
        "AddCanvasNodeUsecase",
        "ConnectCanvasNodesUsecase",
        "EmbedCanvasInDocumentUsecase",
        "ConvertDocumentOutlineToCanvasUsecase",
        "CanvasRepository",
        "LocalCanvasRepository",
        "create_canvas_requires_write_permission_and_saves_draft_canvas",
        "add_canvas_node_updates_existing_canvas_without_raw_ui_state_output",
        "connect_canvas_nodes_rejects_missing_node_edge_without_save",
        "embed_canvas_in_document_returns_stable_reference_without_raw_ui_state",
        "convert_document_outline_to_canvas_preserves_heading_order",
        "canvas_repository_contract_preserves_workspace_canvas_and_lifecycle_state",
        "local_canvas_repository_keeps_workspace_canvases_separate",
      ],
      priority: 98,
    },
  ),
  target("canvas_security_log_policy_evidence", "Canvas security log policy scanner evidence", {
    files: [".tasks/release/security-log-policy-manifest.json", "scripts/security_log_scanner_tests.mjs"],
    evidence: [
      "phase004_canvas_domain_model",
      "phase004_canvas_usecase_contract",
      "phase004_canvas_local_adapter",
      "phase004_canvas_coverage_audit",
      "canvas_raw_ui_state_fixture",
      "canvas_text_card_fixture",
      "canvas_heading_title_fixture",
      "canvas_attachment_filename_fixture",
      "canvas_raw_ui_state",
      "card_text",
      "heading_title",
      "canvas_attachment_filename",
      "active security manifest includes Phase 004 Canvas artifacts and denied fixtures",
    ],
    priority: 96,
  }),
  target("canvas_runtime_ui_product_smoke", "Canvas runtime API, Web UI model, and product smoke", {
    files: [
      "crates/cabinet-server/tests/canvas_runtime_tests.rs",
      "packages/client-core/tests/canvas_client_tests.ts",
      "apps/web/tests/web_canvas_model_tests.ts",
      "scripts/run_phase004_canvas_product_smoke.sh",
      ".tasks/canvas-product-smoke-result.md",
    ],
    evidence: [
      "canvas.create",
      "canvas.add_node",
      "canvas.embed",
      "canvas_relation_graph_projection",
      "CanvasApiClient",
      "createWebCanvasViewModel",
      "phase004_canvas_product_smoke=passed",
    ],
    priority: 94,
  }),
]);

class CanvasCoverageAuditError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "CanvasCoverageAuditError";
    this.code = code;
  }
}

export function transitionCanvasCoverageAuditState(state, event) {
  const key = `${state}:${event}`;
  const transitions = new Map([
    [
      `${CanvasCoverageAuditState.NotStarted}:${CanvasCoverageAuditEvent.Start}`,
      CanvasCoverageAuditState.ReadingSource,
    ],
    [
      `${CanvasCoverageAuditState.ReadingSource}:${CanvasCoverageAuditEvent.SourceLoaded}`,
      CanvasCoverageAuditState.Auditing,
    ],
    [
      `${CanvasCoverageAuditState.Auditing}:${CanvasCoverageAuditEvent.AuditComplete}`,
      CanvasCoverageAuditState.Reported,
    ],
    [
      `${CanvasCoverageAuditState.Reported}:${CanvasCoverageAuditEvent.ReportWritten}`,
      CanvasCoverageAuditState.Reported,
    ],
    [
      `${CanvasCoverageAuditState.ReadingSource}:${CanvasCoverageAuditEvent.Fail}`,
      CanvasCoverageAuditState.Failed,
    ],
    [
      `${CanvasCoverageAuditState.Auditing}:${CanvasCoverageAuditEvent.Fail}`,
      CanvasCoverageAuditState.Failed,
    ],
  ]);
  const nextState = transitions.get(key);
  if (!nextState) {
    throw new CanvasCoverageAuditError(
      CanvasCoverageAuditErrorCode.InvalidTransition,
      `invalid canvas coverage audit transition: ${key}`,
    );
  }
  return nextState;
}

export function analyzeCanvasCoverageSources({ sources }) {
  if (!sources || Object.keys(sources).length === 0) {
    throw new CanvasCoverageAuditError(
      CanvasCoverageAuditErrorCode.SourceSetEmpty,
      "phase004 canvas coverage audit source set is empty",
    );
  }

  const targets = TARGETS.map((entry) => analyzeTarget(entry, sources));
  const targetsNeedingWork = targets.filter((entry) => entry.status !== STATUS.Covered);

  return {
    phase: "Phase 004.5",
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
              id: "PHASE004_CANVAS_COVERAGE_GAP",
              message: "Some Phase 004.5 Canvas coverage targets are missing evidence.",
              targetIds: targetsNeedingWork.map((entry) => entry.id),
            },
          ],
    nextImplementationTarget: pickNextImplementationTarget(targetsNeedingWork),
    targets,
  };
}

export function renderCanvasCoverageAuditMarkdown(audit) {
  const lines = [
    "# Phase 004 Canvas Coverage Audit",
    "",
    "현재 단계: Phase 004.5 - Canvas and Edgeless Baseline",
    "",
    "## Purpose",
    "",
    "- Canvas domain/usecase/repository/security/runtime/UI evidence를 코드 기준으로 고정한다.",
    "- core contract만 있는 상태를 Canvas product smoke complete로 오판하지 않는다.",
    "- 다음 task는 모든 Canvas gap을 한 번에 구현하지 않고 가장 높은 우선순위 target 하나를 선택한다.",
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
    lines.push("- No Canvas coverage gap was detected.");
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
      "- selected reason: highest priority Phase 004.5 Canvas target that is not covered.",
      "- next task should implement or harden this selected target only.",
    );
  } else {
    lines.push("- No next Canvas coverage target remains.");
  }

  lines.push(
    "",
    "## Review Notes",
    "",
    "- Canvas domain/usecase rules must remain independent from React, CodeMirror, Tauri, HTTP, filesystem, and network runtime types.",
    "- Canvas raw UI state must not be dumped into document body; use stable canvas references only.",
    "- Canvas repository stores source Canvas data, while graph projection stores derived relation edges.",
    "- Audit output must not include document bodies, card text, heading titles, attachment filenames, tokens, secrets, credentials, or raw UI JSON.",
  );

  return `${lines.join("\n")}\n`;
}

async function runCli() {
  let state = transitionCanvasCoverageAuditState(
    CanvasCoverageAuditState.NotStarted,
    CanvasCoverageAuditEvent.Start,
  );
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const resultPath = path.join(repoRoot, ".tasks/canvas-coverage-audit.md");
  const sources = await readAuditSources(repoRoot);
  state = transitionCanvasCoverageAuditState(state, CanvasCoverageAuditEvent.SourceLoaded);
  const audit = analyzeCanvasCoverageSources({ sources });
  state = transitionCanvasCoverageAuditState(state, CanvasCoverageAuditEvent.AuditComplete);
  await mkdir(path.dirname(resultPath), { recursive: true });
  await writeFile(resultPath, renderCanvasCoverageAuditMarkdown(audit));
  state = transitionCanvasCoverageAuditState(state, CanvasCoverageAuditEvent.ReportWritten);

  if (audit.summary.targetsNeedingWork === 0) {
    console.log("phase004_canvas_coverage_audit=passed");
    console.log(`state=${state}`);
    console.log(`target_count=${audit.summary.totalTargets}`);
    console.log(`result_path=${resultPath}`);
    return;
  }

  console.error("phase004_canvas_coverage_audit=failed");
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
