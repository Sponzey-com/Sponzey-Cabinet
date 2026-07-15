import { mkdir, readFile, writeFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { pathToFileURL } from "node:url";

export const ProjectionIndexGateState = Object.freeze({
  NotStarted: "NotStarted",
  CheckingPrerequisites: "CheckingPrerequisites",
  RunningProjectionTests: "RunningProjectionTests",
  RunningPerformanceTests: "RunningPerformanceTests",
  RunningDesktopTests: "RunningDesktopTests",
  WritingResult: "WritingResult",
  Passed: "Passed",
  Failed: "Failed",
});

export const ProjectionIndexGateEvent = Object.freeze({
  Start: "Start",
  PrerequisitesChecked: "PrerequisitesChecked",
  ProjectionTestsPassed: "ProjectionTestsPassed",
  PerformanceTestsPassed: "PerformanceTestsPassed",
  DesktopTestsPassed: "DesktopTestsPassed",
  ResultWritten: "ResultWritten",
  Fail: "Fail",
});

export const ProjectionIndexGateErrorCode = Object.freeze({
  DocumentRuntimeMissing: "PHASE008_PROJECTION_INDEX_DOCUMENT_RUNTIME_MISSING",
  ProjectionTestsFailed: "PHASE008_PROJECTION_INDEX_PROJECTION_TESTS_FAILED",
  PerformanceTestsFailed: "PHASE008_PROJECTION_INDEX_PERFORMANCE_TESTS_FAILED",
  DesktopTestsFailed: "PHASE008_PROJECTION_INDEX_DESKTOP_TESTS_FAILED",
  IoFailed: "PHASE008_PROJECTION_INDEX_IO_FAILED",
  InvalidTransition: "PHASE008_PROJECTION_INDEX_INVALID_TRANSITION",
});

const commandOrder = [
  {
    key: "projection",
    errorCode: ProjectionIndexGateErrorCode.ProjectionTestsFailed,
    command: "npm run run:phase008-projection-index-contract-tests",
    spawn: ["npm", ["run", "run:phase008-projection-index-contract-tests"]],
  },
  {
    key: "performance",
    errorCode: ProjectionIndexGateErrorCode.PerformanceTestsFailed,
    command: "cargo test -p cabinet-platform --test query_performance_benchmarks",
    spawn: ["cargo", ["test", "-p", "cabinet-platform", "--test", "query_performance_benchmarks"]],
  },
  {
    key: "desktop",
    errorCode: ProjectionIndexGateErrorCode.DesktopTestsFailed,
    command: "node --test apps/desktop/tests/desktop_discovery_smoke_tests.ts",
    spawn: ["node", ["--test", "apps/desktop/tests/desktop_discovery_smoke_tests.ts"]],
  },
];

export function transitionProjectionIndexGateState(currentState, event, detail = {}) {
  if (currentState === ProjectionIndexGateState.NotStarted && event === ProjectionIndexGateEvent.Start) {
    return { state: ProjectionIndexGateState.CheckingPrerequisites };
  }
  if (
    currentState === ProjectionIndexGateState.CheckingPrerequisites &&
    event === ProjectionIndexGateEvent.PrerequisitesChecked
  ) {
    return { state: ProjectionIndexGateState.RunningProjectionTests };
  }
  if (
    currentState === ProjectionIndexGateState.RunningProjectionTests &&
    event === ProjectionIndexGateEvent.ProjectionTestsPassed
  ) {
    return { state: ProjectionIndexGateState.RunningPerformanceTests };
  }
  if (
    currentState === ProjectionIndexGateState.RunningPerformanceTests &&
    event === ProjectionIndexGateEvent.PerformanceTestsPassed
  ) {
    return { state: ProjectionIndexGateState.RunningDesktopTests };
  }
  if (
    currentState === ProjectionIndexGateState.RunningDesktopTests &&
    event === ProjectionIndexGateEvent.DesktopTestsPassed
  ) {
    return { state: ProjectionIndexGateState.WritingResult };
  }
  if (
    currentState === ProjectionIndexGateState.WritingResult &&
    event === ProjectionIndexGateEvent.ResultWritten
  ) {
    return { state: ProjectionIndexGateState.Passed };
  }
  if (
    [
      ProjectionIndexGateState.CheckingPrerequisites,
      ProjectionIndexGateState.RunningProjectionTests,
      ProjectionIndexGateState.RunningPerformanceTests,
      ProjectionIndexGateState.RunningDesktopTests,
      ProjectionIndexGateState.WritingResult,
    ].includes(currentState) &&
    event === ProjectionIndexGateEvent.Fail
  ) {
    return {
      state: ProjectionIndexGateState.Failed,
      errorCode: detail.errorCode ?? ProjectionIndexGateErrorCode.IoFailed,
      findingId: detail.findingId,
      failedCommandExitCode: detail.failedCommandExitCode,
    };
  }
  return { state: ProjectionIndexGateState.Failed, errorCode: ProjectionIndexGateErrorCode.InvalidTransition };
}

export function evaluateProjectionIndexGate({ documentRuntimeText, commandResults }) {
  let state = transitionProjectionIndexGateState(
    ProjectionIndexGateState.NotStarted,
    ProjectionIndexGateEvent.Start,
  );
  if (!documentRuntimeText.includes("phase008_document_runtime_gate=passed")) {
    state = transitionProjectionIndexGateState(state.state, ProjectionIndexGateEvent.Fail, {
      errorCode: ProjectionIndexGateErrorCode.DocumentRuntimeMissing,
      findingId: ".tasks/phase008-document-runtime-gate-result.md",
    });
    return failedResult(state, commandResults);
  }
  state = transitionProjectionIndexGateState(state.state, ProjectionIndexGateEvent.PrerequisitesChecked);
  for (const command of commandOrder) {
    const result = commandResults[command.key];
    if (!result?.passed) {
      state = transitionProjectionIndexGateState(state.state, ProjectionIndexGateEvent.Fail, {
        errorCode: command.errorCode,
        findingId: command.key,
        failedCommandExitCode: result?.exitCode,
      });
      return failedResult(state, commandResults);
    }
    state = transitionProjectionIndexGateState(
      state.state,
      command.key === "projection"
        ? ProjectionIndexGateEvent.ProjectionTestsPassed
        : command.key === "performance"
          ? ProjectionIndexGateEvent.PerformanceTestsPassed
          : ProjectionIndexGateEvent.DesktopTestsPassed,
    );
  }
  state = transitionProjectionIndexGateState(state.state, ProjectionIndexGateEvent.ResultWritten);
  return { passed: true, state: state.state, commandCount: commandOrder.length };
}

export function renderProjectionIndexGateArtifact(result) {
  const status = result.passed ? "passed" : "failed";
  return [
    "# Phase 008 Projection Index Gate Result",
    "",
    result.passed
      ? [
          "phase008_projection_index_gate=passed",
          `validation_state=${result.state}`,
          `validation_command_count=${result.commandCount}`,
          "search_backlink_graph_asset_projection=passed",
          "performance_budget_marker=phase008_performance_budget=passed",
        ].join("\n")
      : [
          "phase008_projection_index_gate=failed",
          `validation_state=${result.state}`,
          `error_code=${result.errorCode}`,
          result.findingId ? `finding_id=${result.findingId}` : undefined,
        ].filter(Boolean).join("\n"),
    "",
    "- phase: `Phase 008.4`",
    "- gate: `Projection Index`",
    `- status: \`${status}\``,
    "- prerequisites:",
    "  - `.tasks/phase008-document-runtime-gate-result.md` with `phase008_document_runtime_gate=passed`",
    "- validation commands:",
    ...commandOrder.map(({ command }) => `  - \`${command}\``),
    "- query paths: search, backlink, graph neighborhood, asset metadata, current document, history metadata, specific version.",
    "- sensitive-data exclusion: this artifact records markers, command ids, counts, states, and stable error codes only. It does not record raw document body, asset content, AI prompt, AI answer, provider key, token, credential, secret, or personal absolute path.",
    "- follow-up limitation: Phase 008.5 asset lifecycle attach/store gate remains incomplete.",
    "",
  ].join("\n");
}

export function renderPerformanceBudgetArtifact(result) {
  const status = result.passed ? "passed" : "failed";
  return [
    "# Phase 008 Performance Budget",
    "",
    result.passed ? "phase008_performance_budget=passed" : "phase008_performance_budget=failed",
    `status=${status}`,
    "p95_target_ms=300",
    "fixture_profile=small local deterministic benchmark",
    "sample_count=20",
    "targets=current_document,history_list,specific_version,search,backlink,permission_aware_graph,asset_metadata",
    "validation_command=cargo test -p cabinet-platform --test query_performance_benchmarks",
    "sensitive-data exclusion: no raw document body, asset content, AI prompt, AI answer, provider key, token, credential, secret, or personal absolute path is recorded.",
    "",
  ].join("\n");
}

function failedResult(state, commandResults) {
  return {
    passed: false,
    state: state.state,
    errorCode: state.errorCode,
    findingId: state.findingId,
    failedCommandExitCode: state.failedCommandExitCode,
    commandResults,
  };
}

function runCommand({ key, command, spawn }) {
  const [program, args] = spawn;
  const completed = spawnSync(program, args, {
    cwd: process.cwd(),
    encoding: "utf8",
    stdio: "inherit",
  });
  return { key, command, passed: completed.status === 0, exitCode: completed.status ?? 1 };
}

async function runProjectionIndexGateCli() {
  let result;
  try {
    const documentRuntimeText = await readFile(
      ".tasks/phase008-document-runtime-gate-result.md",
      "utf8",
    );
    const commandResults = Object.fromEntries(
      commandOrder.map((commandSpec) => [commandSpec.key, runCommand(commandSpec)]),
    );
    result = evaluateProjectionIndexGate({ documentRuntimeText, commandResults });
  } catch {
    const state = {
      state: ProjectionIndexGateState.Failed,
      errorCode: ProjectionIndexGateErrorCode.IoFailed,
      findingId: ".tasks/phase008-document-runtime-gate-result.md",
    };
    result = failedResult(state, {});
  }
  await mkdir(".tasks/release", { recursive: true });
  await writeFile(
    ".tasks/phase008-projection-index-gate-result.md",
    renderProjectionIndexGateArtifact(result),
  );
  await writeFile(
    ".tasks/release/performance-budget-phase008.md",
    renderPerformanceBudgetArtifact(result),
  );
  if (result.passed) {
    console.log("phase008_projection_index_gate=passed");
    console.log("phase008_performance_budget=passed");
    return;
  }
  console.error("phase008_projection_index_gate=failed");
  process.exit(1);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runProjectionIndexGateCli();
}
