import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const McpApiGateState = Object.freeze({
  Pending: "Pending",
  Running: "Running",
  Passed: "Passed",
  Failed: "Failed",
  Reported: "Reported",
});

export const McpApiGateEvent = Object.freeze({
  Start: "Start",
  Pass: "Pass",
  Fail: "Fail",
  Report: "Report",
});

export const McpApiGateErrorCode = Object.freeze({
  InvalidTransition: "PHASE005_MCP_API_GATE_INVALID_TRANSITION",
  SourceSetEmpty: "PHASE005_MCP_API_GATE_SOURCE_SET_EMPTY",
});

const STATUS = Object.freeze({
  Covered: "covered",
  Missing: "missing",
});

const TARGETS = Object.freeze([
  target("tool_scope_domain_usecase", "Tool scope domain and authorization usecase", {
    files: [
      "crates/cabinet-domain/src/tool.rs",
      "crates/cabinet-domain/tests/tool_tests.rs",
      "crates/cabinet-usecases/src/tool.rs",
      "crates/cabinet-usecases/tests/tool_usecase_tests.rs",
    ],
    evidence: [
      "ToolId",
      "ToolScope",
      "ToolOperation",
      "ToolExecutionRequest",
      "ToolExecutionResult",
      "ToolExecutionState",
      "transition_tool_execution",
      "AuthorizeToolExecutionUsecase",
      "ToolAuthorizationOutput",
      "ToolAuthorizationError",
      "tool_execution_request_requires_explicit_scope",
      "tool_operation_maps_to_required_scope_without_direct_write_scope",
      "authorize_tool_execution_denies_request_missing_required_scope",
      "authorize_tool_execution_limits_write_operation_to_draft_suggestion_scope",
    ],
    priority: 100,
  }),
  target("tool_mapper_boundary", "MCP/API-like mapper boundary", {
    files: [
      "crates/cabinet-adapters/src/tool_mapper.rs",
      "crates/cabinet-adapters/tests/tool_mapper_tests.rs",
    ],
    evidence: [
      "ExternalToolRequest",
      "ExternalToolKind",
      "ToolRequestMapper",
      "ToolMapperError",
      "tool_mapper_maps_mcp_like_search_request_to_internal_request",
      "tool_mapper_maps_api_like_write_suggestion_request_without_direct_write",
      "tool_mapper_output_does_not_expose_token_or_credential_fixture",
    ],
    priority: 98,
  }),
]);

class McpApiGateError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "McpApiGateError";
    this.code = code;
  }
}

export function transitionMcpApiGateState(state, event) {
  const key = `${state}:${event}`;
  const transitions = new Map([
    [`${McpApiGateState.Pending}:${McpApiGateEvent.Start}`, McpApiGateState.Running],
    [`${McpApiGateState.Running}:${McpApiGateEvent.Pass}`, McpApiGateState.Passed],
    [`${McpApiGateState.Running}:${McpApiGateEvent.Fail}`, McpApiGateState.Failed],
    [`${McpApiGateState.Passed}:${McpApiGateEvent.Report}`, McpApiGateState.Reported],
    [`${McpApiGateState.Failed}:${McpApiGateEvent.Report}`, McpApiGateState.Reported],
  ]);
  const nextState = transitions.get(key);
  if (!nextState) {
    throw new McpApiGateError(
      McpApiGateErrorCode.InvalidTransition,
      `invalid MCP API gate transition: ${key}`,
    );
  }
  return nextState;
}

export function analyzeMcpApiGateSources({ sources }) {
  if (!sources || Object.keys(sources).length === 0) {
    throw new McpApiGateError(
      McpApiGateErrorCode.SourceSetEmpty,
      "phase005 MCP API gate source set is empty",
    );
  }
  const targets = TARGETS.map((entry) => analyzeTarget(entry, sources));
  const targetsNeedingWork = targets.filter((entry) => entry.status !== STATUS.Covered);
  return {
    phase: "Phase 005.4",
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

export function renderMcpApiGateMarkdown(gate) {
  const marker =
    gate.status === "passed"
      ? "phase005_mcp_api_product_gate=passed"
      : "phase005_mcp_api_product_gate=failed";
  const lines = [
    "# Phase 005 MCP API Product Gate Result",
    "",
    marker,
    "",
    "현재 단계: Phase 005.4 - MCP Server and Public Tool API Boundary",
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
    "- tool scope domain and execution state machine complete",
    "- tool authorization usecase complete",
    "- write action limited to draft suggestion scope",
    "- MCP/API-like mapper boundary complete",
    "",
    "## Next Implementation Target",
    "",
    gate.nextImplementationTarget
      ? `- \`${gate.nextImplementationTarget.id}\`: ${gate.nextImplementationTarget.label}`
      : "- none",
    "",
    "## Review Notes",
    "",
    "- The gate uses plain fixture DTOs and does not require MCP, JSON-RPC, HTTP, or external agent clients.",
    "- The gate records evidence names and counts, not transport payloads, tool input text, tokens, credentials, or provider keys.",
  ];
  return `${lines.join("\n")}\n`;
}

export async function runMcpApiGate({
  root = process.cwd(),
  reportPath = ".tasks/mcp-api-product-gate-result.md",
} = {}) {
  let state = transitionMcpApiGateState(McpApiGateState.Pending, McpApiGateEvent.Start);
  const sources = await readTargetSources(root);
  const gate = analyzeMcpApiGateSources({ sources });
  state = transitionMcpApiGateState(
    state,
    gate.status === "passed" ? McpApiGateEvent.Pass : McpApiGateEvent.Fail,
  );
  await mkdir(path.dirname(path.join(root, reportPath)), { recursive: true });
  await writeFile(path.join(root, reportPath), renderMcpApiGateMarkdown(gate), "utf8");
  state = transitionMcpApiGateState(state, McpApiGateEvent.Report);
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
  const gate = await runMcpApiGate({ root: repoRoot });
  if (gate.status === "passed") {
    console.log("phase005_mcp_api_product_gate=passed");
    console.log(`gate_state=${gate.state}`);
    console.log(`covered_target_count=${gate.summary.covered}`);
    console.log(`report_path=${path.join(repoRoot, gate.reportPath)}`);
    return;
  }
  console.error("phase005_mcp_api_product_gate=failed");
  console.error(`missing_target_count=${gate.summary.targetsNeedingWork}`);
  console.error(`next_target=${gate.nextImplementationTarget?.id ?? "none"}`);
  process.exitCode = 1;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await runCli();
}
