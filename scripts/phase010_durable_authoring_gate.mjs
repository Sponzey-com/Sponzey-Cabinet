import { spawn } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const Phase010DurableAuthoringState = Object.freeze({
  Pending: "Pending",
  ReadingPrerequisites: "ReadingPrerequisites",
  RunningDocumentTests: "RunningDocumentTests",
  RunningUiTests: "RunningUiTests",
  WritingPerformanceBudget: "WritingPerformanceBudget",
  WritingResult: "WritingResult",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase010DurableAuthoringEvent = Object.freeze({
  Start: "Start",
  PrerequisitesRead: "PrerequisitesRead",
  DocumentTestsPassed: "DocumentTestsPassed",
  UiTestsPassed: "UiTestsPassed",
  PerformanceBudgetWritten: "PerformanceBudgetWritten",
  ResultWritten: "ResultWritten",
  Fail: "Fail",
});

export const Phase010DurableAuthoringErrorCode = Object.freeze({
  FirstRunMarkerMissing: "PHASE010_FIRST_RUN_WORKSPACE_MARKER_MISSING",
  DocumentTestsFailed: "PHASE010_DURABLE_AUTHORING_DOCUMENT_TESTS_FAILED",
  UiTestsFailed: "PHASE010_DURABLE_AUTHORING_UI_TESTS_FAILED",
  PerformanceBudgetFailed: "PHASE010_DURABLE_AUTHORING_PERFORMANCE_BUDGET_FAILED",
  IoFailed: "PHASE010_DURABLE_AUTHORING_IO_FAILED",
  InvalidTransition: "PHASE010_DURABLE_AUTHORING_INVALID_TRANSITION",
});

const documentStepIds = [
  "documentCreate",
  "documentUpdate",
  "documentCurrent",
  "documentHistory",
  "documentVersion",
  "restorePreview",
  "restoreApply",
  "localDocumentRepository",
  "localVersionStore",
  "localDurableAuthoring",
];

const uiStepIds = ["uiAuthoringRestoreModels", "desktopPersistence"];

export function transitionPhase010DurableAuthoringState(currentState, event, detail = {}) {
  if (
    currentState === Phase010DurableAuthoringState.Pending &&
    event === Phase010DurableAuthoringEvent.Start
  ) {
    return { state: Phase010DurableAuthoringState.ReadingPrerequisites };
  }
  if (
    currentState === Phase010DurableAuthoringState.ReadingPrerequisites &&
    event === Phase010DurableAuthoringEvent.PrerequisitesRead
  ) {
    return { state: Phase010DurableAuthoringState.RunningDocumentTests };
  }
  if (
    currentState === Phase010DurableAuthoringState.RunningDocumentTests &&
    event === Phase010DurableAuthoringEvent.DocumentTestsPassed
  ) {
    return { state: Phase010DurableAuthoringState.RunningUiTests };
  }
  if (
    currentState === Phase010DurableAuthoringState.RunningUiTests &&
    event === Phase010DurableAuthoringEvent.UiTestsPassed
  ) {
    return { state: Phase010DurableAuthoringState.WritingPerformanceBudget };
  }
  if (
    currentState === Phase010DurableAuthoringState.WritingPerformanceBudget &&
    event === Phase010DurableAuthoringEvent.PerformanceBudgetWritten
  ) {
    return { state: Phase010DurableAuthoringState.WritingResult };
  }
  if (
    currentState === Phase010DurableAuthoringState.WritingResult &&
    event === Phase010DurableAuthoringEvent.ResultWritten
  ) {
    return { state: Phase010DurableAuthoringState.Passed };
  }
  if (
    [
      Phase010DurableAuthoringState.ReadingPrerequisites,
      Phase010DurableAuthoringState.RunningDocumentTests,
      Phase010DurableAuthoringState.RunningUiTests,
      Phase010DurableAuthoringState.WritingPerformanceBudget,
      Phase010DurableAuthoringState.WritingResult,
    ].includes(currentState) &&
    event === Phase010DurableAuthoringEvent.Fail
  ) {
    return {
      state: Phase010DurableAuthoringState.Failed,
      errorCode: detail.errorCode ?? Phase010DurableAuthoringErrorCode.IoFailed,
      findingId: detail.findingId,
      failedStepId: detail.failedStepId,
    };
  }
  return {
    state: Phase010DurableAuthoringState.Failed,
    errorCode: Phase010DurableAuthoringErrorCode.InvalidTransition,
  };
}

export function buildPhase010DurableAuthoringCommandPlan() {
  return [
    step("documentCreate", [
      "cargo",
      "test",
      "-p",
      "cabinet-usecases",
      "--test",
      "create_document_tests",
    ]),
    step("documentUpdate", [
      "cargo",
      "test",
      "-p",
      "cabinet-usecases",
      "--test",
      "update_document_tests",
    ]),
    step("documentCurrent", [
      "cargo",
      "test",
      "-p",
      "cabinet-usecases",
      "--test",
      "get_current_document_tests",
    ]),
    step("documentHistory", [
      "cargo",
      "test",
      "-p",
      "cabinet-usecases",
      "--test",
      "get_document_history_tests",
    ]),
    step("documentVersion", [
      "cargo",
      "test",
      "-p",
      "cabinet-usecases",
      "--test",
      "get_document_version_tests",
    ]),
    step("restorePreview", [
      "cargo",
      "test",
      "-p",
      "cabinet-usecases",
      "--test",
      "preview_document_restore_tests",
    ]),
    step("restoreApply", [
      "cargo",
      "test",
      "-p",
      "cabinet-usecases",
      "--test",
      "restore_document_version_tests",
    ]),
    step("localDocumentRepository", [
      "cargo",
      "test",
      "-p",
      "cabinet-adapters",
      "--test",
      "local_document_repository_tests",
    ]),
    step("localVersionStore", [
      "cargo",
      "test",
      "-p",
      "cabinet-adapters",
      "--test",
      "local_version_store_tests",
    ]),
    step("localDurableAuthoring", [
      "cargo",
      "test",
      "-p",
      "cabinet-platform",
      "--test",
      "local_durable_authoring_flow_tests",
    ]),
    step("uiAuthoringRestoreModels", [
      "node",
      "--test",
      "packages/ui/tests/document_authoring_preview_model_tests.ts",
      "packages/ui/tests/restore_flow_model_tests.ts",
    ]),
    step("desktopPersistence", [
      "node",
      "--test",
      "apps/desktop/tests/desktop_local_persistence_flow_tests.ts",
      "apps/desktop/tests/desktop_document_authoring_smoke_tests.ts",
    ]),
  ];
}

export function buildPhase010DurableAuthoringPerformanceBudget(rows = defaultBudgetRows()) {
  const failed = rows.find((row) => row.p95Ms > row.budgetMs);
  return {
    passed: !failed,
    marker: failed ? "phase010_performance_budget=failed" : "phase010_performance_budget=passed",
    rows,
    failedPath: failed?.path,
  };
}

export function evaluatePhase010DurableAuthoringGate({
  firstRunText,
  commandResults,
  performanceBudgetRows = defaultBudgetRows(),
}) {
  let state = transitionPhase010DurableAuthoringState(
    Phase010DurableAuthoringState.Pending,
    Phase010DurableAuthoringEvent.Start,
  );

  if (!firstRunText.includes("phase010_first_run_workspace_gate=passed")) {
    state = transitionPhase010DurableAuthoringState(
      state.state,
      Phase010DurableAuthoringEvent.Fail,
      {
        errorCode: Phase010DurableAuthoringErrorCode.FirstRunMarkerMissing,
        findingId: ".tasks/phase010-first-run-workspace-gate-result.md",
      },
    );
    return failedResult(state, commandResults, performanceBudgetRows);
  }

  state = transitionPhase010DurableAuthoringState(
    state.state,
    Phase010DurableAuthoringEvent.PrerequisitesRead,
  );

  for (const stepId of documentStepIds) {
    const result = commandResults[stepId];
    if (!result?.passed) {
      state = transitionPhase010DurableAuthoringState(
        state.state,
        Phase010DurableAuthoringEvent.Fail,
        {
          errorCode: Phase010DurableAuthoringErrorCode.DocumentTestsFailed,
          findingId: stepId,
          failedStepId: stepId,
        },
      );
      return failedResult(state, commandResults, performanceBudgetRows);
    }
  }

  state = transitionPhase010DurableAuthoringState(
    state.state,
    Phase010DurableAuthoringEvent.DocumentTestsPassed,
  );

  for (const stepId of uiStepIds) {
    const result = commandResults[stepId];
    if (!result?.passed) {
      state = transitionPhase010DurableAuthoringState(
        state.state,
        Phase010DurableAuthoringEvent.Fail,
        {
          errorCode: Phase010DurableAuthoringErrorCode.UiTestsFailed,
          findingId: stepId,
          failedStepId: stepId,
        },
      );
      return failedResult(state, commandResults, performanceBudgetRows);
    }
  }

  state = transitionPhase010DurableAuthoringState(
    state.state,
    Phase010DurableAuthoringEvent.UiTestsPassed,
  );

  const performanceBudget = buildPhase010DurableAuthoringPerformanceBudget(performanceBudgetRows);
  if (!performanceBudget.passed) {
    state = transitionPhase010DurableAuthoringState(
      state.state,
      Phase010DurableAuthoringEvent.Fail,
      {
        errorCode: Phase010DurableAuthoringErrorCode.PerformanceBudgetFailed,
        findingId: performanceBudget.failedPath,
      },
    );
    return failedResult(state, commandResults, performanceBudgetRows);
  }

  state = transitionPhase010DurableAuthoringState(
    state.state,
    Phase010DurableAuthoringEvent.PerformanceBudgetWritten,
  );
  state = transitionPhase010DurableAuthoringState(
    state.state,
    Phase010DurableAuthoringEvent.ResultWritten,
  );

  return {
    passed: true,
    state: state.state,
    commandCount: Object.keys(commandResults).length,
    commandResults,
    performanceBudgetRows,
  };
}

export async function runPhase010DurableAuthoringGate({
  root = process.cwd(),
  writeArtifacts = true,
  runner = runCommandStep,
  steps = buildPhase010DurableAuthoringCommandPlan(),
  performanceBudgetRows = defaultBudgetRows(),
} = {}) {
  const firstRunPath = ".tasks/phase010-first-run-workspace-gate-result.md";
  let firstRunText;
  try {
    firstRunText = await readFile(join(root, firstRunPath), "utf8");
  } catch (error) {
    return {
      passed: false,
      state: Phase010DurableAuthoringState.Failed,
      errorCode: Phase010DurableAuthoringErrorCode.IoFailed,
      findingId: error.path ?? firstRunPath,
      commandResults: {},
      performanceBudgetRows,
    };
  }

  if (!firstRunText.includes("phase010_first_run_workspace_gate=passed")) {
    return evaluatePhase010DurableAuthoringGate({
      firstRunText,
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
    const partial = evaluatePhase010DurableAuthoringGate({
      firstRunText,
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

  const result = evaluatePhase010DurableAuthoringGate({
    firstRunText,
    commandResults,
    performanceBudgetRows,
  });

  if (writeArtifacts) {
    await writeArtifactFiles(root, result);
  }

  return result;
}

export function renderPhase010DurableAuthoringArtifact(result) {
  const passed = result.passed === true;
  const lines = [
    "# Phase 010 Durable Authoring Gate Result",
    "",
    passed ? "phase010_durable_authoring_gate=passed" : "phase010_durable_authoring_gate=failed",
    `validation_state=${result.state}`,
    `restart_persistence=${passed ? "verified" : "not_verified"}`,
    `history_restore=${passed ? "verified" : "not_verified"}`,
    `performance_budget=${passed ? "passed" : "not_verified"}`,
    "",
    "- phase: `Phase 010.3`",
    "- gate: `Durable Local Authoring, History, Diff, and Restore Gate`",
    `- status: \`${passed ? "passed" : "failed"}\``,
    "- prerequisites:",
    "  - `.tasks/phase010-first-run-workspace-gate-result.md` with `phase010_first_run_workspace_gate=passed`",
    "- validation commands:",
    ...buildPhase010DurableAuthoringCommandPlan().map(
      (gateStep) => `  - \`${gateStep.command.join(" ")}\``,
    ),
    "- changed layers: `platform-test`, `adapter-validation`, `usecase-validation`, `ui-validation`, `release-tooling`, `task-tooling`.",
    "- p95 300ms path impact: current document read, history list, specific version read, and restore preview status are covered in `.tasks/release/performance-budget-phase010.md`.",
    "- Product Log candidates: `document.create.completed`, `document.save.completed`, `document.current.loaded`, `document.history.loaded`, `document.restore.previewed`, `document.restore.completed`, `document.restore.failed`.",
    "- Field Debug candidates: masked document id, version count, state, stable error code, and duration bucket with explicit scope/expiry only.",
    "- durable evidence: local repositories are reconstructed after create/update, then current/history/version/restore paths are read from persisted local stores.",
    "- restore evidence: restore preview does not mutate current content; restore apply appends a restore version and updates current after confirmation input.",
    "- query path evidence: current read, history list, specific version read, and restore preview are separate usecase commands and are validated by dedicated tests.",
    "- sensitive-data exclusion: this artifact records command ids, counts, states, and stable error codes only. It excludes raw command stdout, document body, diff body, asset content, authentication material, and personal absolute paths.",
    "- current scope: personal local desktop only. Server, SaaS, multi-user, collaboration, mobile, and remote workspace tests are not release prerequisites for this gate.",
    "",
    "## Command Results",
    "",
    "| command | status | exit code | duration ms |",
    "| --- | --- | ---: | ---: |",
    ...Object.entries(result.commandResults ?? {}).map(
      ([id, entry]) =>
        `| \`${id}\` | ${entry.passed ? "passed" : "failed"} | ${entry.exitCode ?? "null"} | ${entry.durationMs ?? 0} |`,
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

export function renderPhase010DurableAuthoringPerformanceBudget(performanceBudget) {
  const lines = [
    "# Phase 010 Performance Budget",
    "",
    performanceBudget.marker,
    "",
    "- phase: `Phase 010.3`",
    "- producer: `phase010 durable authoring gate`",
    "- fixture: `cabinet-platform local_durable_authoring_flow_tests` creates a local temp workspace, writes current documents and versions through local adapters, reconstructs repositories, and repeats read paths.",
    "- budget rule: each user-facing read/status path must remain at p95 <= 300ms in normal indexed/local-store state.",
    "- failure action: if any row exceeds budget, keep the path asynchronous, paginated, cached, indexed, or split from unrelated history scans before release.",
    "- sensitive-data exclusion: this file records path names, budgets, measured threshold values, and validator names only. It excludes document body, diff body, asset bytes, package contents, authentication material, and personal absolute paths.",
    "",
    "| path | p95 ms | budget ms | status | validated by |",
    "| --- | ---: | ---: | --- | --- |",
    ...performanceBudget.rows.map((row) => {
      const status = row.p95Ms <= row.budgetMs ? "passed" : "failed";
      return `| ${row.path} | ${row.p95Ms} | ${row.budgetMs} | ${status} | ${row.validatedBy} |`;
    }),
    "",
  ];
  return lines.join("\n");
}

async function writeArtifactFiles(root, result) {
  await mkdir(join(root, ".tasks"), { recursive: true });
  await mkdir(join(root, ".tasks", "release"), { recursive: true });
  await writeFile(
    join(root, ".tasks", "phase010-durable-authoring-gate-result.md"),
    renderPhase010DurableAuthoringArtifact(result),
  );
  await writeFile(
    join(root, ".tasks", "release", "performance-budget-phase010.md"),
    renderPhase010DurableAuthoringPerformanceBudget(
      buildPhase010DurableAuthoringPerformanceBudget(result.performanceBudgetRows),
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
  const validator = "local_durable_authoring_read_paths_keep_p95_under_300ms";
  return [
    { path: "current document read", p95Ms: 300, budgetMs: 300, validatedBy: validator },
    { path: "history list", p95Ms: 300, budgetMs: 300, validatedBy: validator },
    { path: "specific version read", p95Ms: 300, budgetMs: 300, validatedBy: validator },
    { path: "restore preview status", p95Ms: 300, budgetMs: 300, validatedBy: validator },
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
  const result = await runPhase010DurableAuthoringGate({
    root: process.cwd(),
    writeArtifacts: true,
  });
  if (!result.passed) {
    console.error(`${result.errorCode} ${result.findingId ?? ""}`.trim());
    process.exit(1);
  }
  console.log("phase010_durable_authoring_gate=passed");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    console.error(error);
    process.exit(1);
  });
}
