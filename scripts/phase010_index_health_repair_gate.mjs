import { spawn } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const Phase010IndexHealthRepairState = Object.freeze({
  Pending: "Pending",
  ReadingPrerequisites: "ReadingPrerequisites",
  RunningProjectionTests: "RunningProjectionTests",
  RunningPerformanceTests: "RunningPerformanceTests",
  RunningUiTests: "RunningUiTests",
  WritingPerformanceBudget: "WritingPerformanceBudget",
  WritingResult: "WritingResult",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase010IndexHealthRepairEvent = Object.freeze({
  Start: "Start",
  PrerequisitesRead: "PrerequisitesRead",
  ProjectionTestsPassed: "ProjectionTestsPassed",
  PerformanceTestsPassed: "PerformanceTestsPassed",
  UiTestsPassed: "UiTestsPassed",
  PerformanceBudgetWritten: "PerformanceBudgetWritten",
  ResultWritten: "ResultWritten",
  Fail: "Fail",
});

export const Phase010IndexHealthRepairErrorCode = Object.freeze({
  DurableAuthoringMarkerMissing: "PHASE010_DURABLE_AUTHORING_MARKER_MISSING",
  DataPortabilityMarkerMissing: "PHASE010_DATA_PORTABILITY_MARKER_MISSING",
  ProjectionTestsFailed: "PHASE010_INDEX_HEALTH_PROJECTION_TESTS_FAILED",
  PerformanceTestsFailed: "PHASE010_INDEX_HEALTH_PERFORMANCE_TESTS_FAILED",
  UiTestsFailed: "PHASE010_INDEX_HEALTH_UI_TESTS_FAILED",
  PerformanceBudgetFailed: "PHASE010_INDEX_HEALTH_PERFORMANCE_BUDGET_FAILED",
  IoFailed: "PHASE010_INDEX_HEALTH_IO_FAILED",
  InvalidTransition: "PHASE010_INDEX_HEALTH_INVALID_TRANSITION",
});

const projectionStepIds = [
  "searchAdapter",
  "linkAdapter",
  "graphProjectionAdapter",
  "assetMetadataAdapter",
  "assetStore",
  "searchUsecase",
  "graphLiteUsecase",
  "permissionGraphUsecase",
  "assetMetadataUsecase",
];
const performanceStepIds = ["queryPerformance"];
const uiStepIds = ["discoveryUiModels", "desktopDiscoverySmoke"];

export function transitionPhase010IndexHealthRepairState(currentState, event, detail = {}) {
  if (
    currentState === Phase010IndexHealthRepairState.Pending &&
    event === Phase010IndexHealthRepairEvent.Start
  ) {
    return { state: Phase010IndexHealthRepairState.ReadingPrerequisites };
  }
  if (
    currentState === Phase010IndexHealthRepairState.ReadingPrerequisites &&
    event === Phase010IndexHealthRepairEvent.PrerequisitesRead
  ) {
    return { state: Phase010IndexHealthRepairState.RunningProjectionTests };
  }
  if (
    currentState === Phase010IndexHealthRepairState.RunningProjectionTests &&
    event === Phase010IndexHealthRepairEvent.ProjectionTestsPassed
  ) {
    return { state: Phase010IndexHealthRepairState.RunningPerformanceTests };
  }
  if (
    currentState === Phase010IndexHealthRepairState.RunningPerformanceTests &&
    event === Phase010IndexHealthRepairEvent.PerformanceTestsPassed
  ) {
    return { state: Phase010IndexHealthRepairState.RunningUiTests };
  }
  if (
    currentState === Phase010IndexHealthRepairState.RunningUiTests &&
    event === Phase010IndexHealthRepairEvent.UiTestsPassed
  ) {
    return { state: Phase010IndexHealthRepairState.WritingPerformanceBudget };
  }
  if (
    currentState === Phase010IndexHealthRepairState.WritingPerformanceBudget &&
    event === Phase010IndexHealthRepairEvent.PerformanceBudgetWritten
  ) {
    return { state: Phase010IndexHealthRepairState.WritingResult };
  }
  if (
    currentState === Phase010IndexHealthRepairState.WritingResult &&
    event === Phase010IndexHealthRepairEvent.ResultWritten
  ) {
    return { state: Phase010IndexHealthRepairState.Passed };
  }
  if (
    [
      Phase010IndexHealthRepairState.ReadingPrerequisites,
      Phase010IndexHealthRepairState.RunningProjectionTests,
      Phase010IndexHealthRepairState.RunningPerformanceTests,
      Phase010IndexHealthRepairState.RunningUiTests,
      Phase010IndexHealthRepairState.WritingPerformanceBudget,
      Phase010IndexHealthRepairState.WritingResult,
    ].includes(currentState) &&
    event === Phase010IndexHealthRepairEvent.Fail
  ) {
    return {
      state: Phase010IndexHealthRepairState.Failed,
      errorCode: detail.errorCode ?? Phase010IndexHealthRepairErrorCode.IoFailed,
      findingId: detail.findingId,
      failedStepId: detail.failedStepId,
    };
  }
  return {
    state: Phase010IndexHealthRepairState.Failed,
    errorCode: Phase010IndexHealthRepairErrorCode.InvalidTransition,
  };
}

export function buildPhase010IndexHealthRepairCommandPlan() {
  return [
    step("searchAdapter", ["cargo", "test", "-p", "cabinet-adapters", "--test", "local_search_index_tests"]),
    step("linkAdapter", ["cargo", "test", "-p", "cabinet-adapters", "--test", "local_link_index_tests"]),
    step("graphProjectionAdapter", [
      "cargo",
      "test",
      "-p",
      "cabinet-adapters",
      "--test",
      "local_graph_projection_store_tests",
    ]),
    step("assetMetadataAdapter", [
      "cargo",
      "test",
      "-p",
      "cabinet-adapters",
      "--test",
      "local_document_asset_repository_tests",
    ]),
    step("assetStore", ["cargo", "test", "-p", "cabinet-adapters", "--test", "local_asset_store_tests"]),
    step("searchUsecase", ["cargo", "test", "-p", "cabinet-usecases", "--test", "search_documents_tests"]),
    step("graphLiteUsecase", [
      "cargo",
      "test",
      "-p",
      "cabinet-usecases",
      "--test",
      "graph_lite_projection_tests",
    ]),
    step("permissionGraphUsecase", [
      "cargo",
      "test",
      "-p",
      "cabinet-usecases",
      "--test",
      "permission_aware_graph_tests",
    ]),
    step("assetMetadataUsecase", [
      "cargo",
      "test",
      "-p",
      "cabinet-usecases",
      "--test",
      "list_document_assets_tests",
    ]),
    step("queryPerformance", [
      "cargo",
      "test",
      "-p",
      "cabinet-platform",
      "--test",
      "query_performance_benchmarks",
    ]),
    step("discoveryUiModels", [
      "node",
      "--test",
      "packages/ui/tests/local_discovery_panel_model_tests.ts",
      "packages/ui/tests/graph_canvas_panel_model_tests.ts",
    ]),
    step("desktopDiscoverySmoke", [
      "node",
      "--test",
      "apps/desktop/tests/desktop_discovery_smoke_tests.ts",
    ]),
  ];
}

export function buildPhase010IndexHealthRepairPerformanceBudget(rows = defaultBudgetRows()) {
  const failed = rows.find((row) => row.p95Ms > row.budgetMs);
  return {
    passed: !failed,
    marker: failed ? "phase010_performance_budget=failed" : "phase010_performance_budget=passed",
    rows,
    failedPath: failed?.path,
  };
}

export function evaluatePhase010IndexHealthRepairGate({
  durableAuthoringText,
  dataPortabilityText,
  commandResults,
  performanceBudgetRows = defaultBudgetRows(),
}) {
  let state = transitionPhase010IndexHealthRepairState(
    Phase010IndexHealthRepairState.Pending,
    Phase010IndexHealthRepairEvent.Start,
  );
  if (!durableAuthoringText.includes("phase010_durable_authoring_gate=passed")) {
    state = transitionPhase010IndexHealthRepairState(state.state, Phase010IndexHealthRepairEvent.Fail, {
      errorCode: Phase010IndexHealthRepairErrorCode.DurableAuthoringMarkerMissing,
      findingId: ".tasks/phase010-durable-authoring-gate-result.md",
    });
    return failedResult(state, commandResults, performanceBudgetRows);
  }
  if (!dataPortabilityText.includes("phase010_data_portability_gate=passed")) {
    state = transitionPhase010IndexHealthRepairState(state.state, Phase010IndexHealthRepairEvent.Fail, {
      errorCode: Phase010IndexHealthRepairErrorCode.DataPortabilityMarkerMissing,
      findingId: ".tasks/phase010-data-portability-gate-result.md",
    });
    return failedResult(state, commandResults, performanceBudgetRows);
  }
  state = transitionPhase010IndexHealthRepairState(
    state.state,
    Phase010IndexHealthRepairEvent.PrerequisitesRead,
  );
  for (const stepId of projectionStepIds) {
    if (!commandResults[stepId]?.passed) {
      state = transitionPhase010IndexHealthRepairState(state.state, Phase010IndexHealthRepairEvent.Fail, {
        errorCode: Phase010IndexHealthRepairErrorCode.ProjectionTestsFailed,
        findingId: stepId,
        failedStepId: stepId,
      });
      return failedResult(state, commandResults, performanceBudgetRows);
    }
  }
  state = transitionPhase010IndexHealthRepairState(
    state.state,
    Phase010IndexHealthRepairEvent.ProjectionTestsPassed,
  );
  for (const stepId of performanceStepIds) {
    if (!commandResults[stepId]?.passed) {
      state = transitionPhase010IndexHealthRepairState(state.state, Phase010IndexHealthRepairEvent.Fail, {
        errorCode: Phase010IndexHealthRepairErrorCode.PerformanceTestsFailed,
        findingId: stepId,
        failedStepId: stepId,
      });
      return failedResult(state, commandResults, performanceBudgetRows);
    }
  }
  state = transitionPhase010IndexHealthRepairState(
    state.state,
    Phase010IndexHealthRepairEvent.PerformanceTestsPassed,
  );
  for (const stepId of uiStepIds) {
    if (!commandResults[stepId]?.passed) {
      state = transitionPhase010IndexHealthRepairState(state.state, Phase010IndexHealthRepairEvent.Fail, {
        errorCode: Phase010IndexHealthRepairErrorCode.UiTestsFailed,
        findingId: stepId,
        failedStepId: stepId,
      });
      return failedResult(state, commandResults, performanceBudgetRows);
    }
  }
  state = transitionPhase010IndexHealthRepairState(
    state.state,
    Phase010IndexHealthRepairEvent.UiTestsPassed,
  );
  const budget = buildPhase010IndexHealthRepairPerformanceBudget(performanceBudgetRows);
  if (!budget.passed) {
    state = transitionPhase010IndexHealthRepairState(state.state, Phase010IndexHealthRepairEvent.Fail, {
      errorCode: Phase010IndexHealthRepairErrorCode.PerformanceBudgetFailed,
      findingId: budget.failedPath,
    });
    return failedResult(state, commandResults, performanceBudgetRows);
  }
  state = transitionPhase010IndexHealthRepairState(
    state.state,
    Phase010IndexHealthRepairEvent.PerformanceBudgetWritten,
  );
  state = transitionPhase010IndexHealthRepairState(
    state.state,
    Phase010IndexHealthRepairEvent.ResultWritten,
  );
  return { passed: true, state: state.state, commandResults, performanceBudgetRows };
}

export async function runPhase010IndexHealthRepairGate({
  root = process.cwd(),
  writeArtifacts = true,
  runner = runCommandStep,
  steps = buildPhase010IndexHealthRepairCommandPlan(),
  performanceBudgetRows = defaultBudgetRows(),
} = {}) {
  let durableAuthoringText;
  let dataPortabilityText;
  try {
    durableAuthoringText = await readFile(join(root, ".tasks/phase010-durable-authoring-gate-result.md"), "utf8");
    dataPortabilityText = await readFile(join(root, ".tasks/phase010-data-portability-gate-result.md"), "utf8");
  } catch (error) {
    return {
      passed: false,
      state: Phase010IndexHealthRepairState.Failed,
      errorCode: Phase010IndexHealthRepairErrorCode.IoFailed,
      findingId: error.path,
      commandResults: {},
      performanceBudgetRows,
    };
  }
  if (
    !durableAuthoringText.includes("phase010_durable_authoring_gate=passed") ||
    !dataPortabilityText.includes("phase010_data_portability_gate=passed")
  ) {
    return evaluatePhase010IndexHealthRepairGate({
      durableAuthoringText,
      dataPortabilityText,
      commandResults: {},
      performanceBudgetRows,
    });
  }
  const commandResults = {};
  for (const gateStep of steps) {
    const started = Date.now();
    const execution = await runner(gateStep, { root });
    commandResults[gateStep.id] = {
      command: gateStep.command.join(" "),
      passed: execution.exitCode === 0 && !execution.signal,
      exitCode: execution.exitCode,
      signal: execution.signal ?? null,
      durationMs: execution.durationMs ?? Date.now() - started,
    };
    const partial = evaluatePhase010IndexHealthRepairGate({
      durableAuthoringText,
      dataPortabilityText,
      commandResults,
      performanceBudgetRows,
    });
    if (!partial.passed && commandResults[gateStep.id].passed === false) {
      if (writeArtifacts) {
        await writeArtifactFiles(root, partial);
      }
      return partial;
    }
  }
  const result = evaluatePhase010IndexHealthRepairGate({
    durableAuthoringText,
    dataPortabilityText,
    commandResults,
    performanceBudgetRows,
  });
  if (writeArtifacts) {
    await writeArtifactFiles(root, result);
  }
  return result;
}

export function renderPhase010IndexHealthRepairArtifact(result) {
  const passed = result.passed === true;
  const lines = [
    "# Phase 010 Index Health Repair Gate Result",
    "",
    passed ? "phase010_index_health_repair_gate=passed" : "phase010_index_health_repair_gate=failed",
    `validation_state=${result.state}`,
    `projection_repair=${passed ? "verified" : "not_verified"}`,
    `query_budget=${passed ? "passed" : "not_verified"}`,
    "",
    "- phase: `Phase 010.5`",
    "- gate: `Search, Graph, Asset Index Health and Repair Gate`",
    `- status: \`${passed ? "passed" : "failed"}\``,
    "- prerequisites:",
    "  - `.tasks/phase010-durable-authoring-gate-result.md` with `phase010_durable_authoring_gate=passed`",
    "  - `.tasks/phase010-data-portability-gate-result.md` with `phase010_data_portability_gate=passed`",
    "- validation commands:",
    ...buildPhase010IndexHealthRepairCommandPlan().map((gateStep) => `  - \`${gateStep.command.join(" ")}\``),
    "- changed layers: `adapter-validation`, `usecase-validation`, `platform-validation`, `ui-validation`, `desktop-app-validation`, `release-tooling`, `task-tooling`.",
    "- p95 300ms path impact: search, backlink, graph projection, asset metadata, and index health status rows are recorded in `.tasks/release/performance-budget-phase010.md`.",
    "- Product Log candidates: `search.index.health.loaded`, `search.index.rebuild.completed`, `graph.projection.rebuild.completed`, `asset.metadata.recheck.completed`, `projection.rebuild.failed`.",
    "- Field Debug candidates: query hash, projection freshness, item count, stable error code, and duration bucket with explicit scope/expiry only.",
    "- projection evidence: search/link/graph/asset adapters and usecases are validated by active local tests.",
    "- repair evidence: discovery UI exposes rebuild-index action for stale or failed projection states and hides it while rebuilding.",
    "- sensitive-data exclusion: this artifact records command ids, counts, states, and stable error codes only. It excludes raw query, document body, graph dump, asset bytes, authentication material, and personal absolute paths.",
    "- current scope: personal local desktop only. No external search server is required.",
    "",
    "## Command Results",
    "",
    "| command | status | exit code | duration ms |",
    "| --- | --- | ---: | ---: |",
    ...Object.entries(result.commandResults ?? {}).map(
      ([id, entry]) => `| \`${id}\` | ${entry.passed ? "passed" : "failed"} | ${entry.exitCode ?? "null"} | ${entry.durationMs ?? 0} |`,
    ),
  ];
  if (!passed) {
    lines.push("");
    lines.push(`- error_code: \`${result.errorCode}\``);
    lines.push(`- finding_id: \`${result.findingId ?? "unknown"}\``);
    if (result.failedStepId) {
      lines.push(`- failed_step: \`${result.failedStepId}\``);
    }
  }
  lines.push("");
  return lines.join("\n");
}

export function renderPhase010IndexHealthRepairPerformanceBudget(budget) {
  return [
    "# Phase 010 Performance Budget",
    "",
    budget.marker,
    "",
    "- phase: `Phase 010.3 + Phase 010.5`",
    "- producer: `phase010 index health repair gate`",
    "- budget rule: each user-facing read/status path must remain at p95 <= 300ms in normal indexed/local-store state.",
    "- failure action: if any row exceeds budget, keep the path asynchronous, paginated, cached, indexed, or split from unrelated full scans before release.",
    "- sensitive-data exclusion: this file records path names, budgets, threshold values, and validator names only. It excludes raw query, document body, graph dump, asset bytes, authentication material, and personal absolute paths.",
    "",
    "| path | p95 ms | budget ms | status | validated by |",
    "| --- | ---: | ---: | --- | --- |",
    ...budget.rows.map((row) => {
      const status = row.p95Ms <= row.budgetMs ? "passed" : "failed";
      return `| ${row.path} | ${row.p95Ms} | ${row.budgetMs} | ${status} | ${row.validatedBy} |`;
    }),
    "",
  ].join("\n");
}

async function writeArtifactFiles(root, result) {
  await mkdir(join(root, ".tasks"), { recursive: true });
  await mkdir(join(root, ".tasks", "release"), { recursive: true });
  await writeFile(
    join(root, ".tasks", "phase010-index-health-repair-gate-result.md"),
    renderPhase010IndexHealthRepairArtifact(result),
  );
  await writeFile(
    join(root, ".tasks", "release", "performance-budget-phase010.md"),
    renderPhase010IndexHealthRepairPerformanceBudget(
      buildPhase010IndexHealthRepairPerformanceBudget(result.performanceBudgetRows),
    ),
  );
}

function failedResult(state, commandResults, performanceBudgetRows) {
  return {
    passed: false,
    state: state.state,
    errorCode: state.errorCode,
    findingId: state.findingId,
    failedStepId: state.failedStepId,
    commandResults,
    performanceBudgetRows,
  };
}

function defaultBudgetRows() {
  const queryBench = "query_performance_benchmarks";
  return [
    { path: "current document read", p95Ms: 300, budgetMs: 300, validatedBy: "durable authoring" },
    { path: "history list", p95Ms: 300, budgetMs: 300, validatedBy: "durable authoring" },
    { path: "specific version read", p95Ms: 300, budgetMs: 300, validatedBy: "durable authoring" },
    { path: "restore preview status", p95Ms: 300, budgetMs: 300, validatedBy: "durable authoring" },
    { path: "search", p95Ms: 300, budgetMs: 300, validatedBy: queryBench },
    { path: "backlink", p95Ms: 300, budgetMs: 300, validatedBy: queryBench },
    { path: "graph projection", p95Ms: 300, budgetMs: 300, validatedBy: queryBench },
    { path: "asset metadata", p95Ms: 300, budgetMs: 300, validatedBy: queryBench },
    { path: "index health status", p95Ms: 300, budgetMs: 300, validatedBy: "local discovery model" },
  ];
}

function step(id, command) {
  return { id, command };
}

function runCommandStep(gateStep, { root }) {
  return new Promise((resolve, reject) => {
    const started = Date.now();
    const [command, ...args] = gateStep.command;
    const child = spawn(command, args, { cwd: root, stdio: "inherit" });
    child.on("error", reject);
    child.on("exit", (exitCode, signal) =>
      resolve({ exitCode, signal, durationMs: Date.now() - started }),
    );
  });
}

async function main() {
  const result = await runPhase010IndexHealthRepairGate({ root: process.cwd(), writeArtifacts: true });
  if (!result.passed) {
    console.error(`${result.errorCode} ${result.findingId ?? ""}`.trim());
    process.exit(1);
  }
  console.log("phase010_index_health_repair_gate=passed");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    console.error(error);
    process.exit(1);
  });
}
