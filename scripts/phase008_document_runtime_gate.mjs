import { readFile, writeFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { pathToFileURL } from "node:url";

export const DocumentRuntimeGateState = Object.freeze({
  NotStarted: "NotStarted",
  CheckingPrerequisites: "CheckingPrerequisites",
  RunningUsecaseTests: "RunningUsecaseTests",
  RunningAdapterTests: "RunningAdapterTests",
  RunningDesktopTests: "RunningDesktopTests",
  WritingResult: "WritingResult",
  Passed: "Passed",
  Failed: "Failed",
});

export const DocumentRuntimeGateEvent = Object.freeze({
  Start: "Start",
  PrerequisitesChecked: "PrerequisitesChecked",
  UsecaseTestsPassed: "UsecaseTestsPassed",
  AdapterTestsPassed: "AdapterTestsPassed",
  DesktopTestsPassed: "DesktopTestsPassed",
  ResultWritten: "ResultWritten",
  Fail: "Fail",
});

export const DocumentRuntimeGateErrorCode = Object.freeze({
  CommandBridgeMissing: "PHASE008_DOCUMENT_RUNTIME_COMMAND_BRIDGE_MISSING",
  UsecaseTestsFailed: "PHASE008_DOCUMENT_RUNTIME_USECASE_TESTS_FAILED",
  AdapterTestsFailed: "PHASE008_DOCUMENT_RUNTIME_ADAPTER_TESTS_FAILED",
  DesktopTestsFailed: "PHASE008_DOCUMENT_RUNTIME_DESKTOP_TESTS_FAILED",
  IoFailed: "PHASE008_DOCUMENT_RUNTIME_IO_FAILED",
  InvalidTransition: "PHASE008_DOCUMENT_RUNTIME_INVALID_TRANSITION",
});

const commandOrder = [
  {
    key: "usecase",
    errorCode: DocumentRuntimeGateErrorCode.UsecaseTestsFailed,
    command: "npm run run:phase008-document-runtime-usecase-tests",
    spawn: ["npm", ["run", "run:phase008-document-runtime-usecase-tests"]],
  },
  {
    key: "adapter",
    errorCode: DocumentRuntimeGateErrorCode.AdapterTestsFailed,
    command: "npm run run:phase008-document-runtime-adapter-tests",
    spawn: ["npm", ["run", "run:phase008-document-runtime-adapter-tests"]],
  },
  {
    key: "desktop",
    errorCode: DocumentRuntimeGateErrorCode.DesktopTestsFailed,
    command: "node --test apps/desktop/tests/desktop_local_persistence_flow_tests.ts",
    spawn: ["node", ["--test", "apps/desktop/tests/desktop_local_persistence_flow_tests.ts"]],
  },
];

export function transitionDocumentRuntimeGateState(currentState, event, detail = {}) {
  if (currentState === DocumentRuntimeGateState.NotStarted && event === DocumentRuntimeGateEvent.Start) {
    return { state: DocumentRuntimeGateState.CheckingPrerequisites };
  }
  if (
    currentState === DocumentRuntimeGateState.CheckingPrerequisites &&
    event === DocumentRuntimeGateEvent.PrerequisitesChecked
  ) {
    return { state: DocumentRuntimeGateState.RunningUsecaseTests };
  }
  if (
    currentState === DocumentRuntimeGateState.RunningUsecaseTests &&
    event === DocumentRuntimeGateEvent.UsecaseTestsPassed
  ) {
    return { state: DocumentRuntimeGateState.RunningAdapterTests };
  }
  if (
    currentState === DocumentRuntimeGateState.RunningAdapterTests &&
    event === DocumentRuntimeGateEvent.AdapterTestsPassed
  ) {
    return { state: DocumentRuntimeGateState.RunningDesktopTests };
  }
  if (
    currentState === DocumentRuntimeGateState.RunningDesktopTests &&
    event === DocumentRuntimeGateEvent.DesktopTestsPassed
  ) {
    return { state: DocumentRuntimeGateState.WritingResult };
  }
  if (
    currentState === DocumentRuntimeGateState.WritingResult &&
    event === DocumentRuntimeGateEvent.ResultWritten
  ) {
    return { state: DocumentRuntimeGateState.Passed };
  }
  if (
    [
      DocumentRuntimeGateState.CheckingPrerequisites,
      DocumentRuntimeGateState.RunningUsecaseTests,
      DocumentRuntimeGateState.RunningAdapterTests,
      DocumentRuntimeGateState.RunningDesktopTests,
      DocumentRuntimeGateState.WritingResult,
    ].includes(currentState) &&
    event === DocumentRuntimeGateEvent.Fail
  ) {
    return {
      state: DocumentRuntimeGateState.Failed,
      errorCode: detail.errorCode ?? DocumentRuntimeGateErrorCode.IoFailed,
      findingId: detail.findingId,
      failedCommandExitCode: detail.failedCommandExitCode,
    };
  }
  return {
    state: DocumentRuntimeGateState.Failed,
    errorCode: DocumentRuntimeGateErrorCode.InvalidTransition,
  };
}

export function evaluateDocumentRuntimeGate({ commandBridgeText, commandResults }) {
  let state = transitionDocumentRuntimeGateState(
    DocumentRuntimeGateState.NotStarted,
    DocumentRuntimeGateEvent.Start,
  );
  if (!commandBridgeText.includes("phase008_command_bridge_gate=passed")) {
    state = transitionDocumentRuntimeGateState(state.state, DocumentRuntimeGateEvent.Fail, {
      errorCode: DocumentRuntimeGateErrorCode.CommandBridgeMissing,
      findingId: ".tasks/phase008-command-bridge-gate-result.md",
    });
    return failedResult(state, commandResults);
  }
  state = transitionDocumentRuntimeGateState(
    state.state,
    DocumentRuntimeGateEvent.PrerequisitesChecked,
  );

  for (const command of commandOrder) {
    const result = commandResults[command.key];
    if (!result?.passed) {
      state = transitionDocumentRuntimeGateState(state.state, DocumentRuntimeGateEvent.Fail, {
        errorCode: command.errorCode,
        findingId: command.key,
        failedCommandExitCode: result?.exitCode,
      });
      return failedResult(state, commandResults);
    }
    state = transitionDocumentRuntimeGateState(
      state.state,
      command.key === "usecase"
        ? DocumentRuntimeGateEvent.UsecaseTestsPassed
        : command.key === "adapter"
          ? DocumentRuntimeGateEvent.AdapterTestsPassed
          : DocumentRuntimeGateEvent.DesktopTestsPassed,
    );
  }
  state = transitionDocumentRuntimeGateState(state.state, DocumentRuntimeGateEvent.ResultWritten);
  return {
    passed: true,
    state: state.state,
    commandCount: commandOrder.length,
    commandResults,
  };
}

export function renderDocumentRuntimeGateResult(result) {
  if (result.passed) {
    return [
      "phase008_document_runtime_gate=passed",
      `validation_state=${result.state}`,
      `validation_command_count=${result.commandCount}`,
      "current_history_separated=true",
      "restore_path_tested=true",
    ].join("\n");
  }
  const lines = [
    "phase008_document_runtime_gate=failed",
    `validation_state=${result.state}`,
    `error_code=${result.errorCode}`,
  ];
  if (result.findingId) {
    lines.push(`finding_id=${result.findingId}`);
  }
  if (result.failedCommandExitCode !== undefined) {
    lines.push(`failed_command_exit_code=${result.failedCommandExitCode}`);
  }
  return lines.join("\n");
}

export function renderDocumentRuntimeGateArtifact(result) {
  const status = result.passed ? "passed" : "failed";
  return [
    "# Phase 008 Document Runtime Gate Result",
    "",
    renderDocumentRuntimeGateResult(result),
    "",
    "- phase: `Phase 008.3`",
    "- gate: `Durable Document Runtime`",
    `- status: \`${status}\``,
    `- state: \`${result.state}\``,
    "- prerequisites:",
    "  - `.tasks/phase008-command-bridge-gate-result.md` with `phase008_command_bridge_gate=passed`",
    "- validation commands:",
    ...commandOrder.map(({ command }) => `  - \`${command}\``),
    "- changed layers:",
    "  - `scripts`: document runtime gate and wrappers",
    "  - `.tasks`: document runtime gate artifact",
    "- Product Log candidates: `document.saved`, `document.restore.completed`, `document.version_conflict`",
    "- Field Debug metadata candidates: `document_id`, `version_id`, `operation`, `stable_error_code`, `retryable`",
    "- current/history separation: current document read uses current repository path; history list uses paginated version metadata path.",
    "- restore evidence: restore preview/apply usecase tests remain in the usecase validation set.",
    "- sensitive-data exclusion: this artifact records markers, command ids, counts, states, and stable error codes only. It does not record raw document body, asset content, AI prompt, AI answer, provider key, token, credential, secret, or personal absolute path.",
    "- follow-up limitation: Phase 008 performance budget, projection refresh, search/backlink/graph, and asset reference indexing remain incomplete.",
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

async function runDocumentRuntimeGateCli() {
  let result;
  try {
    const commandBridgeText = await readFile(
      ".tasks/phase008-command-bridge-gate-result.md",
      "utf8",
    );
    const commandResults = Object.fromEntries(
      commandOrder.map((commandSpec) => [commandSpec.key, runCommand(commandSpec)]),
    );
    result = evaluateDocumentRuntimeGate({ commandBridgeText, commandResults });
  } catch {
    const state = {
      state: DocumentRuntimeGateState.Failed,
      errorCode: DocumentRuntimeGateErrorCode.IoFailed,
      findingId: ".tasks/phase008-command-bridge-gate-result.md",
    };
    result = failedResult(state, {});
  }
  await writeFile(
    ".tasks/phase008-document-runtime-gate-result.md",
    renderDocumentRuntimeGateArtifact(result),
  );
  const rendered = renderDocumentRuntimeGateResult(result);
  if (result.passed) {
    console.log(rendered);
    return;
  }
  console.error(rendered);
  process.exit(1);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runDocumentRuntimeGateCli();
}
