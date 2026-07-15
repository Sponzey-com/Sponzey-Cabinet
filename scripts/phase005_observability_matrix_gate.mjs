import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const ObservabilityMatrixGateState = Object.freeze({
  Pending: "Pending",
  Running: "Running",
  Passed: "Passed",
  Failed: "Failed",
  Reported: "Reported",
});

export const ObservabilityMatrixGateEvent = Object.freeze({
  Start: "Start",
  Pass: "Pass",
  Fail: "Fail",
  Report: "Report",
});

export const ObservabilityMatrixGateErrorCode = Object.freeze({
  InvalidTransition: "PHASE005_OBSERVABILITY_MATRIX_GATE_INVALID_TRANSITION",
  SourceSetEmpty: "PHASE005_OBSERVABILITY_MATRIX_GATE_SOURCE_SET_EMPTY",
});

const MATRIX_PATH = ".tasks/release/product-log-event-matrix.md";

const STATUS = Object.freeze({
  Covered: "covered",
  Missing: "missing",
});

const TARGETS = Object.freeze([
  target("phase005_product_log_events", "Phase 005 Product Log events", {
    files: [MATRIX_PATH],
    evidence: [
      "ai.retrieval.degraded",
      "ai.answer.requested",
      "ai.answer.completed",
      "ai.answer.failed",
      "mcp.tool.invocation.failed",
      "webhook.delivery.dead_lettered",
      "connector.authorization.failed",
      "connector.sync.failed",
      "AI_PROVIDER_UNAVAILABLE",
      "WEBHOOK_DEAD_LETTERED",
      "CONNECTOR_AUTHORIZATION_FAILED",
    ],
    priority: 100,
  }),
  target("phase005_field_debug_events", "Phase 005 Field Debug Log events", {
    files: [MATRIX_PATH],
    evidence: [
      "field.ai.retrieval",
      "field.ai.provider",
      "field.mcp.tool",
      "field.webhook.delivery",
      "field.connector.sync",
      "scope",
      "TTL",
      "query hash",
      "provider name",
      "connector id",
      "retry count",
      "retrieval count",
      "citation count",
    ],
    priority: 98,
  }),
  target("phase005_development_log_events", "Phase 005 Development Log events", {
    files: [MATRIX_PATH],
    evidence: [
      "dev.ai.fake_provider.called",
      "dev.webhook.fake_transport.called",
      "dev.connector.fake_gateway.called",
      "fixture id",
      "fake port",
      "call count",
      "local/test only",
      "production default behavior is forbidden",
    ],
    priority: 96,
  }),
  target("phase005_sensitive_denied_rules", "Phase 005 sensitive data denied rules", {
    files: [MATRIX_PATH],
    evidence: [
      "raw prompt",
      "raw answer",
      "retrieval source text",
      "embedding input",
      "provider API key",
      "connector access token",
      "connector refresh token",
      "connector client secret",
      "webhook signing secret",
      "raw payload body",
      "request body",
      "response body",
      "token",
      "credential",
      "secret",
    ],
    priority: 94,
  }),
]);

class ObservabilityMatrixGateError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "ObservabilityMatrixGateError";
    this.code = code;
  }
}

export function transitionObservabilityMatrixGateState(state, event) {
  const key = `${state}:${event}`;
  const transitions = new Map([
    [
      `${ObservabilityMatrixGateState.Pending}:${ObservabilityMatrixGateEvent.Start}`,
      ObservabilityMatrixGateState.Running,
    ],
    [
      `${ObservabilityMatrixGateState.Running}:${ObservabilityMatrixGateEvent.Pass}`,
      ObservabilityMatrixGateState.Passed,
    ],
    [
      `${ObservabilityMatrixGateState.Running}:${ObservabilityMatrixGateEvent.Fail}`,
      ObservabilityMatrixGateState.Failed,
    ],
    [
      `${ObservabilityMatrixGateState.Passed}:${ObservabilityMatrixGateEvent.Report}`,
      ObservabilityMatrixGateState.Reported,
    ],
    [
      `${ObservabilityMatrixGateState.Failed}:${ObservabilityMatrixGateEvent.Report}`,
      ObservabilityMatrixGateState.Reported,
    ],
  ]);
  const nextState = transitions.get(key);
  if (!nextState) {
    throw new ObservabilityMatrixGateError(
      ObservabilityMatrixGateErrorCode.InvalidTransition,
      `${ObservabilityMatrixGateErrorCode.InvalidTransition}: invalid observability matrix gate transition: ${key}`,
    );
  }
  return nextState;
}

export function analyzeObservabilityMatrixGateSources({ sources }) {
  if (!sources || Object.keys(sources).length === 0) {
    throw new ObservabilityMatrixGateError(
      ObservabilityMatrixGateErrorCode.SourceSetEmpty,
      "phase005 observability matrix gate source set is empty",
    );
  }
  const targets = TARGETS.map((entry) => analyzeTarget(entry, sources));
  const targetsNeedingWork = targets.filter((entry) => entry.status !== STATUS.Covered);
  return {
    phase: "Phase 005.8",
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

export function renderObservabilityMatrixGateMarkdown(gate) {
  const marker =
    gate.status === "passed"
      ? "phase005_observability_matrix_gate=passed"
      : "phase005_observability_matrix_gate=failed";
  const lines = [
    "# Phase 005 Observability Matrix Gate Result",
    "",
    marker,
    "",
    "현재 단계: Phase 005.8 - AI and Integration Observability, Runbooks, and Release Gate",
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
    "- Phase 005 Product Log event names and stable error codes are documented.",
    "- Phase 005 Field Debug Log scope, TTL, and non-sensitive metadata are documented.",
    "- Phase 005 Development Log evidence remains local/test only.",
    "- Sensitive AI/provider/webhook/connector data is denied by matrix rule.",
    "",
    "## Next Implementation Target",
    "",
    gate.nextImplementationTarget
      ? `- \`${gate.nextImplementationTarget.id}\`: ${gate.nextImplementationTarget.label}`
      : "- none",
    "",
    "## Review Notes",
    "",
    "- This gate validates release evidence only; it does not configure runtime logging.",
    "- The report records event names and policy phrases, not prompt, answer, source text, provider key, connector token, or payload values.",
  ];
  return `${lines.join("\n")}\n`;
}

export async function runObservabilityMatrixGate({
  root = process.cwd(),
  reportPath = ".tasks/phase005-observability-matrix-gate-result.md",
} = {}) {
  let state = transitionObservabilityMatrixGateState(
    ObservabilityMatrixGateState.Pending,
    ObservabilityMatrixGateEvent.Start,
  );
  const sources = await readTargetSources(root);
  const gate = analyzeObservabilityMatrixGateSources({ sources });
  state = transitionObservabilityMatrixGateState(
    state,
    gate.status === "passed" ? ObservabilityMatrixGateEvent.Pass : ObservabilityMatrixGateEvent.Fail,
  );
  await mkdir(path.dirname(path.join(root, reportPath)), { recursive: true });
  await writeFile(path.join(root, reportPath), renderObservabilityMatrixGateMarkdown(gate), "utf8");
  state = transitionObservabilityMatrixGateState(state, ObservabilityMatrixGateEvent.Report);
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
  const gate = await runObservabilityMatrixGate({ root: repoRoot });
  if (gate.status === "passed") {
    console.log("phase005_observability_matrix_gate=passed");
    console.log(`gate_state=${gate.state}`);
    console.log(`covered_target_count=${gate.summary.covered}`);
    return;
  }

  console.error("phase005_observability_matrix_gate=failed");
  console.error(`missing_target_count=${gate.summary.targetsNeedingWork}`);
  if (gate.nextImplementationTarget) {
    console.error(`next_target=${gate.nextImplementationTarget.id}`);
  }
  process.exitCode = 1;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await runCli();
}
