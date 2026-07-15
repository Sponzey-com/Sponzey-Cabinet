import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const RetrievalCoverageAuditState = Object.freeze({
  NotStarted: "NotStarted",
  ReadingSource: "ReadingSource",
  Auditing: "Auditing",
  Reported: "Reported",
  Failed: "Failed",
});

export const RetrievalCoverageAuditEvent = Object.freeze({
  Start: "Start",
  SourceLoaded: "SourceLoaded",
  AuditComplete: "AuditComplete",
  ReportWritten: "ReportWritten",
  Fail: "Fail",
});

export const RetrievalCoverageAuditErrorCode = Object.freeze({
  InvalidTransition: "PHASE005_RETRIEVAL_COVERAGE_INVALID_TRANSITION",
  SourceSetEmpty: "PHASE005_RETRIEVAL_COVERAGE_SOURCE_SET_EMPTY",
});

const STATUS = Object.freeze({
  Covered: "covered",
  Missing: "missing",
});

const TARGETS = Object.freeze([
  target("retrieval_domain_contract", "Retrieval domain value objects and state machine", {
    files: [
      "crates/cabinet-domain/src/retrieval.rs",
      "crates/cabinet-domain/tests/retrieval_tests.rs",
    ],
    evidence: [
      "RetrievalQuery",
      "RetrievalScope",
      "RetrievalCandidate",
      "CitationSpan",
      "ContextBudget",
      "RetrievalPipelineState",
      "retrieval_pipeline_uses_explicit_transitions",
    ],
    priority: 100,
  }),
  target("retrieval_usecase_port_contract", "Retrieval usecase and port contract", {
    files: [
      "crates/cabinet-ports/src/retrieval.rs",
      "crates/cabinet-usecases/src/retrieval.rs",
      "crates/cabinet-usecases/tests/retrieval_usecase_tests.rs",
    ],
    evidence: [
      "RetrievalSourcePort",
      "RetrievalPermissionPort",
      "BuildRetrievalContextUsecase",
      "BuildRetrievalContextInput",
      "RetrievalContextStats",
      "build_retrieval_context_filters_permission_denied_candidates",
      "build_retrieval_context_truncates_candidates_over_context_budget",
    ],
    priority: 98,
  }),
  target("retrieval_fake_source_adapter", "Deterministic local retrieval source adapter", {
    files: [
      "crates/cabinet-adapters/src/local_retrieval_source.rs",
      "crates/cabinet-adapters/tests/local_retrieval_source_tests.rs",
    ],
    evidence: [
      "LocalRetrievalSource",
      "LocalRetrievalSourceRecord",
      "RetrievalSourcePort",
      "local_retrieval_source_returns_matching_candidates_by_query_and_source_kind",
      "local_retrieval_source_excludes_source_kinds_outside_scope",
    ],
    priority: 96,
  }),
]);

class RetrievalCoverageAuditError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "RetrievalCoverageAuditError";
    this.code = code;
  }
}

export function transitionRetrievalCoverageAuditState(state, event) {
  const key = `${state}:${event}`;
  const transitions = new Map([
    [
      `${RetrievalCoverageAuditState.NotStarted}:${RetrievalCoverageAuditEvent.Start}`,
      RetrievalCoverageAuditState.ReadingSource,
    ],
    [
      `${RetrievalCoverageAuditState.ReadingSource}:${RetrievalCoverageAuditEvent.SourceLoaded}`,
      RetrievalCoverageAuditState.Auditing,
    ],
    [
      `${RetrievalCoverageAuditState.Auditing}:${RetrievalCoverageAuditEvent.AuditComplete}`,
      RetrievalCoverageAuditState.Reported,
    ],
    [
      `${RetrievalCoverageAuditState.Reported}:${RetrievalCoverageAuditEvent.ReportWritten}`,
      RetrievalCoverageAuditState.Reported,
    ],
    [
      `${RetrievalCoverageAuditState.ReadingSource}:${RetrievalCoverageAuditEvent.Fail}`,
      RetrievalCoverageAuditState.Failed,
    ],
    [
      `${RetrievalCoverageAuditState.Auditing}:${RetrievalCoverageAuditEvent.Fail}`,
      RetrievalCoverageAuditState.Failed,
    ],
  ]);
  const nextState = transitions.get(key);
  if (!nextState) {
    throw new RetrievalCoverageAuditError(
      RetrievalCoverageAuditErrorCode.InvalidTransition,
      `invalid retrieval coverage audit transition: ${key}`,
    );
  }
  return nextState;
}

export function analyzeRetrievalCoverageSources({ sources }) {
  if (!sources || Object.keys(sources).length === 0) {
    throw new RetrievalCoverageAuditError(
      RetrievalCoverageAuditErrorCode.SourceSetEmpty,
      "phase005 retrieval coverage audit source set is empty",
    );
  }

  const targets = TARGETS.map((entry) => analyzeTarget(entry, sources));
  const targetsNeedingWork = targets.filter((entry) => entry.status !== STATUS.Covered);

  return {
    phase: "Phase 005.1",
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
              id: "PHASE005_RETRIEVAL_COVERAGE_GAP",
              message: "Some Phase 005.1 retrieval coverage targets are missing evidence.",
              targetIds: targetsNeedingWork.map((entry) => entry.id),
            },
          ],
    nextImplementationTarget: pickNextImplementationTarget(targetsNeedingWork),
    targets,
  };
}

export function renderRetrievalCoverageAuditMarkdown(audit) {
  const marker =
    audit.summary.targetsNeedingWork === 0
      ? "phase005_retrieval_coverage_audit=passed"
      : "phase005_retrieval_coverage_audit=failed";
  const lines = [
    "# Phase 005 Retrieval Coverage Audit",
    "",
    marker,
    "",
    "현재 단계: Phase 005.1 - Permission-Aware Retrieval Domain and Source Contract",
    "",
    "## Purpose",
    "",
    "- retrieval domain/usecase/port/fake adapter evidence를 코드 기준으로 고정한다.",
    "- provider, vector index, server route, MCP, AI answer 구현으로 조기 확장하지 않는다.",
    "- 다음 task는 모든 retrieval 후속 범위를 한 번에 구현하지 않고 가장 높은 우선순위 target 하나를 선택한다.",
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
        entry.missingEvidence.length > 0
          ? entry.missingEvidence.map(code).join(", ")
          : "none";
      return `| \`${entry.id}\` | ${entry.label} | ${entry.status} | ${missingFiles} | ${missingEvidence} |`;
    }),
    "",
    "## Next Implementation Target",
    "",
    audit.nextImplementationTarget
      ? `- \`${audit.nextImplementationTarget.id}\`: ${audit.nextImplementationTarget.label}`
      : "- none",
    "",
    "## Review Notes",
    "",
    "- Retrieval candidate evidence uses source ids, citation references, snippet references, freshness, permission decision summary, and context budget metadata.",
    "- Raw document body, raw comment body, attachment content, prompt text, answer text, token, and credential are outside Phase 005.1 retrieval coverage evidence.",
  ];
  return `${lines.join("\n")}\n`;
}

export async function runRetrievalCoverageAudit({
  root = process.cwd(),
  reportPath = ".tasks/retrieval-coverage-audit.md",
} = {}) {
  let state = transitionRetrievalCoverageAuditState(
    RetrievalCoverageAuditState.NotStarted,
    RetrievalCoverageAuditEvent.Start,
  );
  const sources = await readTargetSources(root);
  state = transitionRetrievalCoverageAuditState(
    state,
    RetrievalCoverageAuditEvent.SourceLoaded,
  );
  const audit = analyzeRetrievalCoverageSources({ sources });
  state = transitionRetrievalCoverageAuditState(state, RetrievalCoverageAuditEvent.AuditComplete);
  await mkdir(path.dirname(path.join(root, reportPath)), { recursive: true });
  await writeFile(path.join(root, reportPath), renderRetrievalCoverageAuditMarkdown(audit), "utf8");
  state = transitionRetrievalCoverageAuditState(state, RetrievalCoverageAuditEvent.ReportWritten);
  return { ...audit, state, reportPath };
}

function analyzeTarget(entry, sources) {
  const missingFiles = entry.files.filter((file) => !(file in sources));
  const combined = entry.files.map((file) => sources[file] ?? "").join("\n");
  const missingEvidence = entry.evidence.filter((evidence) => !combined.includes(evidence));
  const status =
    missingFiles.length === 0 && missingEvidence.length === 0 ? STATUS.Covered : STATUS.Missing;
  return { ...entry, status, missingFiles, missingEvidence };
}

function pickNextImplementationTarget(targetsNeedingWork) {
  if (targetsNeedingWork.length === 0) {
    return null;
  }
  return [...targetsNeedingWork].sort((left, right) => right.priority - left.priority)[0];
}

async function readTargetSources(root) {
  const paths = new Set(TARGETS.flatMap((entry) => entry.files));
  const sources = {};
  for (const relativePath of paths) {
    try {
      sources[relativePath] = await readFile(path.join(root, relativePath), "utf8");
    } catch {
      // Missing files are represented by absence in the source map.
    }
  }
  return sources;
}

function target(id, label, { files, evidence, priority }) {
  return { id, label, files, evidence, priority };
}

function code(value) {
  return `\`${value}\``;
}

async function runCli() {
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const audit = await runRetrievalCoverageAudit({ root: repoRoot });
  if (audit.summary.targetsNeedingWork === 0) {
    console.log("phase005_retrieval_coverage_audit=passed");
    console.log(`coverage_state=${audit.state}`);
    console.log(`covered_target_count=${audit.summary.covered}`);
    console.log(`report_path=${path.join(repoRoot, audit.reportPath)}`);
    return;
  }
  console.error("phase005_retrieval_coverage_audit=failed");
  console.error(`missing_target_count=${audit.summary.targetsNeedingWork}`);
  console.error(`next_target=${audit.nextImplementationTarget?.id ?? "none"}`);
  process.exitCode = 1;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await runCli();
}
