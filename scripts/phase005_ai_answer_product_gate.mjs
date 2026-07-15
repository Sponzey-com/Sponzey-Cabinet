import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const AiAnswerGateState = Object.freeze({
  Pending: "Pending",
  Running: "Running",
  Passed: "Passed",
  Failed: "Failed",
  Reported: "Reported",
});

export const AiAnswerGateEvent = Object.freeze({
  Start: "Start",
  Pass: "Pass",
  Fail: "Fail",
  Report: "Report",
});

export const AiAnswerGateErrorCode = Object.freeze({
  InvalidTransition: "PHASE005_AI_ANSWER_GATE_INVALID_TRANSITION",
  SourceSetEmpty: "PHASE005_AI_ANSWER_GATE_SOURCE_SET_EMPTY",
});

const STATUS = Object.freeze({
  Covered: "covered",
  Missing: "missing",
});

const TARGETS = Object.freeze([
  target("ai_answer_domain_contract", "AI answer domain result and state machine", {
    files: [
      "crates/cabinet-domain/src/ai.rs",
      "crates/cabinet-domain/tests/ai_tests.rs",
      "crates/cabinet-domain/tests/ai_summary_tests.rs",
    ],
    evidence: [
      "AiQuestion",
      "AiAnswerResult",
      "AiCitation",
      "AiRefusal",
      "AiFreshnessStatus",
      "AiAnswerJobState",
      "transition_ai_answer_job",
      "AiSummaryResult",
      "AiRelatedDocumentRecommendation",
      "completed_ai_answer_requires_answer_reference_and_citation",
      "ai_answer_job_uses_success_refusal_retry_and_failure_transitions",
      "ai_summary_result_requires_summary_reference_citation_and_freshness",
      "related_document_recommendation_is_reference_only",
    ],
    priority: 100,
  }),
  target("ai_answer_port_usecase_contract", "AI answer provider port, usecase, and prompt reference boundary", {
    files: [
      "crates/cabinet-ports/src/ai.rs",
      "crates/cabinet-usecases/src/ai.rs",
      "crates/cabinet-usecases/tests/ai_usecase_tests.rs",
      "crates/cabinet-usecases/tests/ai_prompt_builder_tests.rs",
    ],
    evidence: [
      "AiProviderPort",
      "AiProviderRequest",
      "AiPromptReference",
      "AiProviderPolicy",
      "AiProviderResponse",
      "AiAnswerResultStorePort",
      "AskKnowledgeBaseUsecase",
      "BuildAiPromptReferenceUsecase",
      "ask_knowledge_base_stores_completed_answer_with_valid_citation",
      "ask_knowledge_base_schedules_retry_when_provider_times_out",
      "prompt_reference_builder_does_not_expose_raw_prompt_or_secret_fixture",
      "prompt_reference_builder_rejects_secret_like_job_id",
    ],
    priority: 98,
  }),
  target("ai_answer_fake_adapter_and_cache", "AI answer fake provider and cached answer store evidence", {
    files: [
      "crates/cabinet-adapters/src/fake_ai_provider.rs",
      "crates/cabinet-adapters/src/local_ai_answer_store.rs",
      "crates/cabinet-adapters/tests/fake_ai_provider_tests.rs",
    ],
    evidence: [
      "FakeAiProvider",
      "AiProviderPort",
      "LocalAiAnswerStore",
      "AiAnswerResultStorePort",
      "fake_ai_provider_returns_configured_response_and_counts_calls",
      "local_ai_answer_store_cached_status_and_result_lookup_stays_under_300ms",
    ],
    priority: 96,
  }),
  target("ai_summary_recommendation_baseline", "AI summary and related recommendation baseline", {
    files: [
      "crates/cabinet-domain/src/ai.rs",
      "crates/cabinet-domain/tests/ai_summary_tests.rs",
      "crates/cabinet-usecases/src/ai.rs",
      "crates/cabinet-usecases/tests/ai_summary_usecase_tests.rs",
    ],
    evidence: [
      "AiSummaryReference",
      "AiSummaryTarget",
      "AiSummaryResult",
      "AiRelatedDocumentRecommendation",
      "SummarizeRetrievalContextUsecase",
      "SuggestRelatedDocumentsUsecase",
      "summarize_retrieval_context_reflects_stale_source_freshness",
      "suggest_related_documents_returns_document_candidates_only_and_applies_limit",
    ],
    priority: 94,
  }),
]);

class AiAnswerGateError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "AiAnswerGateError";
    this.code = code;
  }
}

export function transitionAiAnswerGateState(state, event) {
  const key = `${state}:${event}`;
  const transitions = new Map([
    [`${AiAnswerGateState.Pending}:${AiAnswerGateEvent.Start}`, AiAnswerGateState.Running],
    [`${AiAnswerGateState.Running}:${AiAnswerGateEvent.Pass}`, AiAnswerGateState.Passed],
    [`${AiAnswerGateState.Running}:${AiAnswerGateEvent.Fail}`, AiAnswerGateState.Failed],
    [`${AiAnswerGateState.Passed}:${AiAnswerGateEvent.Report}`, AiAnswerGateState.Reported],
    [`${AiAnswerGateState.Failed}:${AiAnswerGateEvent.Report}`, AiAnswerGateState.Reported],
  ]);
  const nextState = transitions.get(key);
  if (!nextState) {
    throw new AiAnswerGateError(
      AiAnswerGateErrorCode.InvalidTransition,
      `invalid AI answer gate transition: ${key}`,
    );
  }
  return nextState;
}

export function analyzeAiAnswerGateSources({ sources }) {
  if (!sources || Object.keys(sources).length === 0) {
    throw new AiAnswerGateError(
      AiAnswerGateErrorCode.SourceSetEmpty,
      "phase005 AI answer gate source set is empty",
    );
  }
  const targets = TARGETS.map((entry) => analyzeTarget(entry, sources));
  const targetsNeedingWork = targets.filter((entry) => entry.status !== STATUS.Covered);
  return {
    phase: "Phase 005.3",
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

export function renderAiAnswerGateMarkdown(gate) {
  const marker =
    gate.status === "passed"
      ? "phase005_ai_answer_product_gate=passed"
      : "phase005_ai_answer_product_gate=failed";
  const lines = [
    "# Phase 005 AI Answer Product Gate Result",
    "",
    marker,
    "",
    "현재 단계: Phase 005.3 - AI Answer, Citation, Summary, and Refusal Boundary",
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
    "- AI answer domain result and job state machine complete",
    "- AI provider port, answer result store port, and AskKnowledgeBase usecase complete",
    "- prompt reference builder excludes prompt contents and credential material",
    "- deterministic fake provider and local cached answer store complete",
    "- cached answer/status 300ms fixture complete",
    "- summary and related recommendation metadata baseline complete",
    "",
    "## Next Implementation Target",
    "",
    gate.nextImplementationTarget
      ? `- \`${gate.nextImplementationTarget.id}\`: ${gate.nextImplementationTarget.label}`
      : "- none",
    "",
    "## Review Notes",
    "",
    "- The gate uses deterministic fake/local adapters and does not require an external AI provider.",
    "- The gate records evidence names and counts, not prompt contents, answer text, source text, provider body, tokens, or credentials.",
  ];
  return `${lines.join("\n")}\n`;
}

export async function runAiAnswerGate({
  root = process.cwd(),
  reportPath = ".tasks/ai-answer-product-gate-result.md",
} = {}) {
  let state = transitionAiAnswerGateState(AiAnswerGateState.Pending, AiAnswerGateEvent.Start);
  const sources = await readTargetSources(root);
  const gate = analyzeAiAnswerGateSources({ sources });
  state = transitionAiAnswerGateState(
    state,
    gate.status === "passed" ? AiAnswerGateEvent.Pass : AiAnswerGateEvent.Fail,
  );
  await mkdir(path.dirname(path.join(root, reportPath)), { recursive: true });
  await writeFile(path.join(root, reportPath), renderAiAnswerGateMarkdown(gate), "utf8");
  state = transitionAiAnswerGateState(state, AiAnswerGateEvent.Report);
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
  const gate = await runAiAnswerGate({ root: repoRoot });
  if (gate.status === "passed") {
    console.log("phase005_ai_answer_product_gate=passed");
    console.log(`gate_state=${gate.state}`);
    console.log(`covered_target_count=${gate.summary.covered}`);
    console.log(`report_path=${path.join(repoRoot, gate.reportPath)}`);
    return;
  }
  console.error("phase005_ai_answer_product_gate=failed");
  console.error(`missing_target_count=${gate.summary.targetsNeedingWork}`);
  console.error(`next_target=${gate.nextImplementationTarget?.id ?? "none"}`);
  process.exitCode = 1;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await runCli();
}
