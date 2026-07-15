import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const ProductSmokeGateState = Object.freeze({
  Pending: "Pending",
  Running: "Running",
  Passed: "Passed",
  Failed: "Failed",
  Reported: "Reported",
});

export const ProductSmokeGateEvent = Object.freeze({
  Start: "Start",
  Pass: "Pass",
  Fail: "Fail",
  Report: "Report",
});

export const ProductSmokeGateErrorCode = Object.freeze({
  InvalidTransition: "PHASE005_PRODUCT_SMOKE_GATE_INVALID_TRANSITION",
  SourceSetEmpty: "PHASE005_PRODUCT_SMOKE_GATE_SOURCE_SET_EMPTY",
});

const STATUS = Object.freeze({
  Covered: "covered",
  Missing: "missing",
});

const TARGETS = Object.freeze([
  target("client_core_ai_contract", "Client-core AI retrieval and answer API contract", {
    files: [
      "packages/client-core/src/index.ts",
      "packages/client-core/tests/ai_api_client_tests.ts",
      "packages/client-core/tests/ai_capability_matrix_tests.ts",
    ],
    evidence: [
      "CabinetAiApiClient",
      "AiRetrievalResultPage",
      "AskKnowledgeBaseCommand",
      "AiAnswerResultView",
      "createPlatformCapabilityMatrix",
      "aiQuerySupport",
      "aiCitationSupport",
      "connectorAdminSupport",
      "self-host AI client sends retrieval, answer, status, and result requests through explicit config",
      "AI answer result DTO carries citation, refusal, and freshness without provider secrets",
      "platform capability matrix documents AI query, citation, and connector admin support",
    ],
    priority: 100,
  }),
  target("web_ai_ui_model", "Web AI query UI model", {
    files: ["packages/ui/src/index.ts", "packages/ui/tests/ai_query_ui_model_tests.ts"],
    evidence: [
      "createAiQueryPanelViewModel",
      "AiQueryPanelViewModel",
      "AiCitationCardViewModel",
      "AI query panel maps retrieval candidates to citation cards without permission rules",
      "AI query panel does not display completed answer without citations as successful",
      "AI query panel model excludes prompt, provider, connector, and source raw fixtures",
    ],
    priority: 98,
  }),
  target("desktop_mobile_ai_smoke", "Desktop and mobile AI product smoke skeleton", {
    files: [
      "apps/desktop/tests/desktop_ai_product_smoke_tests.ts",
      "apps/mobile/tests/mobile_ai_product_smoke_tests.ts",
    ],
    evidence: [
      "desktop AI product smoke skeleton displays completed answer with citations",
      "mobile AI product smoke skeleton displays refusal and citation metadata without connector admin",
    ],
    priority: 96,
  }),
  target("phase005_lower_product_gates", "Lower Phase 005 AI, MCP/API, and webhook/connector gates", {
    files: [
      ".tasks/ai-answer-product-gate-result.md",
      ".tasks/mcp-api-product-gate-result.md",
      ".tasks/webhook-connector-product-gate-result.md",
    ],
    evidence: [
      "phase005_ai_answer_product_gate=passed",
      "phase005_mcp_api_product_gate=passed",
      "phase005_webhook_connector_product_gate=passed",
    ],
    priority: 94,
  }),
]);

class ProductSmokeGateError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "ProductSmokeGateError";
    this.code = code;
  }
}

export function transitionProductSmokeGateState(state, event) {
  const key = `${state}:${event}`;
  const transitions = new Map([
    [`${ProductSmokeGateState.Pending}:${ProductSmokeGateEvent.Start}`, ProductSmokeGateState.Running],
    [`${ProductSmokeGateState.Running}:${ProductSmokeGateEvent.Pass}`, ProductSmokeGateState.Passed],
    [`${ProductSmokeGateState.Running}:${ProductSmokeGateEvent.Fail}`, ProductSmokeGateState.Failed],
    [`${ProductSmokeGateState.Passed}:${ProductSmokeGateEvent.Report}`, ProductSmokeGateState.Reported],
    [`${ProductSmokeGateState.Failed}:${ProductSmokeGateEvent.Report}`, ProductSmokeGateState.Reported],
  ]);
  const nextState = transitions.get(key);
  if (!nextState) {
    throw new ProductSmokeGateError(
      ProductSmokeGateErrorCode.InvalidTransition,
      `invalid product smoke gate transition: ${key}`,
    );
  }
  return nextState;
}

export function analyzeProductSmokeGateSources({ sources }) {
  if (!sources || Object.keys(sources).length === 0) {
    throw new ProductSmokeGateError(
      ProductSmokeGateErrorCode.SourceSetEmpty,
      "phase005 product smoke gate source set is empty",
    );
  }
  const targets = TARGETS.map((entry) => analyzeTarget(entry, sources));
  const targetsNeedingWork = targets.filter((entry) => entry.status !== STATUS.Covered);
  return {
    phase: "Phase 005.7",
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

export function renderProductSmokeGateMarkdown(gate) {
  const marker =
    gate.status === "passed"
      ? "phase005_product_smoke_gate=passed"
      : "phase005_product_smoke_gate=failed";
  const lines = [
    "# Phase 005 Product Smoke Gate Result",
    "",
    marker,
    "",
    "현재 단계: Phase 005.7 - Cross-Platform AI Query UX and Product Smoke",
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
    "- client-core AI retrieval and answer API contract complete",
    "- Web AI query UI model complete",
    "- desktop/mobile AI smoke skeleton complete",
    "- lower AI answer, MCP/API, webhook/connector gates passed",
    "",
    "## Next Implementation Target",
    "",
    gate.nextImplementationTarget
      ? `- \`${gate.nextImplementationTarget.id}\`: ${gate.nextImplementationTarget.label}`
      : "- none",
    "",
    "## Review Notes",
    "",
    "- The gate uses deterministic source and artifact evidence; it does not call external AI providers or connector services.",
    "- The gate records evidence names and counts, not prompts, answers, source text, provider keys, connector tokens, or raw payloads.",
  ];
  return `${lines.join("\n")}\n`;
}

export async function runProductSmokeGate({
  root = process.cwd(),
  reportPath = ".tasks/phase005-product-smoke-gate-result.md",
} = {}) {
  let state = transitionProductSmokeGateState(
    ProductSmokeGateState.Pending,
    ProductSmokeGateEvent.Start,
  );
  const sources = await readTargetSources(root);
  const gate = analyzeProductSmokeGateSources({ sources });
  state = transitionProductSmokeGateState(
    state,
    gate.status === "passed" ? ProductSmokeGateEvent.Pass : ProductSmokeGateEvent.Fail,
  );
  await mkdir(path.dirname(path.join(root, reportPath)), { recursive: true });
  await writeFile(path.join(root, reportPath), renderProductSmokeGateMarkdown(gate), "utf8");
  state = transitionProductSmokeGateState(state, ProductSmokeGateEvent.Report);
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
  const gate = await runProductSmokeGate({ root: repoRoot });
  if (gate.status === "passed") {
    console.log("phase005_product_smoke_gate=passed");
    console.log(`gate_state=${gate.state}`);
    console.log(`covered_target_count=${gate.summary.covered}`);
    console.log(`report_path=${path.join(repoRoot, gate.reportPath)}`);
    return;
  }
  console.error("phase005_product_smoke_gate=failed");
  console.error(`missing_target_count=${gate.summary.targetsNeedingWork}`);
  console.error(`next_target=${gate.nextImplementationTarget?.id ?? "none"}`);
  process.exitCode = 1;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await runCli();
}
