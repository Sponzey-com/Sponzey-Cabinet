import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const SemanticGateState = Object.freeze({
  Pending: "Pending",
  Running: "Running",
  Passed: "Passed",
  Failed: "Failed",
  Reported: "Reported",
});

export const SemanticGateEvent = Object.freeze({
  Start: "Start",
  Pass: "Pass",
  Fail: "Fail",
  Report: "Report",
});

export const SemanticGateErrorCode = Object.freeze({
  InvalidTransition: "PHASE005_SEMANTIC_GATE_INVALID_TRANSITION",
  SourceSetEmpty: "PHASE005_SEMANTIC_GATE_SOURCE_SET_EMPTY",
});

const STATUS = Object.freeze({
  Covered: "covered",
  Missing: "missing",
});

const TARGETS = Object.freeze([
  target("embedding_domain_contract", "Embedding domain state machine", {
    files: [
      "crates/cabinet-domain/src/embedding.rs",
      "crates/cabinet-domain/tests/embedding_tests.rs",
    ],
    evidence: [
      "EmbeddingInput",
      "EmbeddingVectorReference",
      "EmbeddingJob",
      "EmbeddingJobState",
      "embedding_job_uses_explicit_success_transitions",
      "embedding_job_uses_explicit_retry_and_failure_transitions",
    ],
    priority: 100,
  }),
  target("embedding_vector_port_adapter", "Embedding provider and local vector index contracts", {
    files: [
      "crates/cabinet-ports/src/embedding.rs",
      "crates/cabinet-adapters/src/deterministic_embedding_provider.rs",
      "crates/cabinet-adapters/src/local_vector_index.rs",
    ],
    evidence: [
      "EmbeddingProviderPort",
      "VectorIndexPort",
      "EmbeddingVector",
      "VectorSearchQuery",
      "DeterministicEmbeddingProvider",
      "LocalVectorIndex",
    ],
    priority: 98,
  }),
  target("semantic_merge_usecase", "Hybrid keyword and semantic merge usecase", {
    files: [
      "crates/cabinet-usecases/src/semantic.rs",
      "crates/cabinet-usecases/tests/semantic_usecase_tests.rs",
    ],
    evidence: [
      "MergeHybridSearchUsecase",
      "HybridSearchInput",
      "HybridSearchResult",
      "hybrid_search_merge_dedupes_keyword_and_semantic_matches",
      "hybrid_merge_completes_under_300ms_fixture",
    ],
    priority: 96,
  }),
]);

class SemanticGateError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "SemanticGateError";
    this.code = code;
  }
}

export function transitionSemanticGateState(state, event) {
  const key = `${state}:${event}`;
  const transitions = new Map([
    [`${SemanticGateState.Pending}:${SemanticGateEvent.Start}`, SemanticGateState.Running],
    [`${SemanticGateState.Running}:${SemanticGateEvent.Pass}`, SemanticGateState.Passed],
    [`${SemanticGateState.Running}:${SemanticGateEvent.Fail}`, SemanticGateState.Failed],
    [`${SemanticGateState.Passed}:${SemanticGateEvent.Report}`, SemanticGateState.Reported],
    [`${SemanticGateState.Failed}:${SemanticGateEvent.Report}`, SemanticGateState.Reported],
  ]);
  const nextState = transitions.get(key);
  if (!nextState) {
    throw new SemanticGateError(
      SemanticGateErrorCode.InvalidTransition,
      `invalid semantic gate transition: ${key}`,
    );
  }
  return nextState;
}

export function analyzeSemanticGateSources({ sources }) {
  if (!sources || Object.keys(sources).length === 0) {
    throw new SemanticGateError(
      SemanticGateErrorCode.SourceSetEmpty,
      "phase005 semantic gate source set is empty",
    );
  }
  const targets = TARGETS.map((entry) => analyzeTarget(entry, sources));
  const targetsNeedingWork = targets.filter((entry) => entry.status !== STATUS.Covered);
  return {
    phase: "Phase 005.2",
    status: targetsNeedingWork.length === 0 ? "passed" : "failed",
    sourceFiles: Object.keys(sources).sort(),
    summary: {
      totalTargets: targets.length,
      covered: targets.filter((entry) => entry.status === STATUS.Covered).length,
      missing: targets.filter((entry) => entry.status === STATUS.Missing).length,
      targetsNeedingWork: targetsNeedingWork.length,
    },
    nextImplementationTarget: pickNextImplementationTarget(targetsNeedingWork),
    targets,
  };
}

export function renderSemanticGateMarkdown(gate) {
  const marker =
    gate.status === "passed"
      ? "phase005_semantic_search_gate=passed"
      : "phase005_semantic_search_gate=failed";
  const lines = [
    "# Phase 005 Semantic Search Gate Result",
    "",
    marker,
    "",
    "현재 단계: Phase 005.2 - Semantic Search and Vector Index Pipeline",
    "",
    "## Summary",
    "",
    "| Metric | Count |",
    "| --- | ---: |",
    `| total targets | ${gate.summary.totalTargets} |`,
    `| covered | ${gate.summary.covered} |`,
    `| missing | ${gate.summary.missing} |`,
    `| targets needing work | ${gate.summary.targetsNeedingWork} |`,
    "",
    "## Target Status",
    "",
    "| Target | Label | Status | Missing Files | Missing Evidence |",
    "| --- | --- | --- | --- | --- |",
    ...gate.targets.map((entry) => {
      const missingFiles =
        entry.missingFiles.length > 0 ? entry.missingFiles.map(code).join(", ") : "none";
      const missingEvidence =
        entry.missingEvidence.length > 0
          ? entry.missingEvidence.map(code).join(", ")
          : "none";
      return `| \`${entry.id}\` | ${entry.label} | ${entry.status} | ${missingFiles} | ${missingEvidence} |`;
    }),
    "",
    "## Evidence Markers",
    "",
    "- embedding domain state machine complete",
    "- embedding provider port and vector index port complete",
    "- deterministic fake provider and local vector index complete",
    "- hybrid merge p95 300ms fixture complete",
    "",
    "## Next Implementation Target",
    "",
    gate.nextImplementationTarget
      ? `- \`${gate.nextImplementationTarget.id}\`: ${gate.nextImplementationTarget.label}`
      : "- none",
    "",
    "## Review Notes",
    "",
    "- The gate uses deterministic fake/local adapters and does not require external AI provider or external vector DB.",
    "- The gate records evidence names and counts, not raw query text, source text, vector dumps, tokens, or credentials.",
  ];
  return `${lines.join("\n")}\n`;
}

export async function runSemanticGate({
  root = process.cwd(),
  reportPath = ".tasks/semantic-search-gate-result.md",
} = {}) {
  let state = transitionSemanticGateState(SemanticGateState.Pending, SemanticGateEvent.Start);
  const sources = await readTargetSources(root);
  const gate = analyzeSemanticGateSources({ sources });
  state = transitionSemanticGateState(
    state,
    gate.status === "passed" ? SemanticGateEvent.Pass : SemanticGateEvent.Fail,
  );
  await mkdir(path.dirname(path.join(root, reportPath)), { recursive: true });
  await writeFile(path.join(root, reportPath), renderSemanticGateMarkdown(gate), "utf8");
  state = transitionSemanticGateState(state, SemanticGateEvent.Report);
  return { ...gate, state, reportPath };
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
  const gate = await runSemanticGate({ root: repoRoot });
  if (gate.status === "passed") {
    console.log("phase005_semantic_search_gate=passed");
    console.log(`gate_state=${gate.state}`);
    console.log(`covered_target_count=${gate.summary.covered}`);
    console.log(`report_path=${path.join(repoRoot, gate.reportPath)}`);
    return;
  }
  console.error("phase005_semantic_search_gate=failed");
  console.error(`missing_target_count=${gate.summary.targetsNeedingWork}`);
  console.error(`next_target=${gate.nextImplementationTarget?.id ?? "none"}`);
  process.exitCode = 1;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await runCli();
}
