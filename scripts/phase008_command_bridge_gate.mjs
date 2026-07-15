import { readFile, writeFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { pathToFileURL } from "node:url";

export const CommandBridgeGateState = Object.freeze({
  NotStarted: "NotStarted",
  CheckingPrerequisites: "CheckingPrerequisites",
  RunningContractTests: "RunningContractTests",
  RunningRustBoundaryTests: "RunningRustBoundaryTests",
  ScanningBoundary: "ScanningBoundary",
  WritingResult: "WritingResult",
  Passed: "Passed",
  Failed: "Failed",
});

export const CommandBridgeGateEvent = Object.freeze({
  Start: "Start",
  PrerequisitesChecked: "PrerequisitesChecked",
  ContractTestsPassed: "ContractTestsPassed",
  RustBoundaryTestsPassed: "RustBoundaryTestsPassed",
  BoundaryScanPassed: "BoundaryScanPassed",
  ResultWritten: "ResultWritten",
  Fail: "Fail",
});

export const CommandBridgeGateErrorCode = Object.freeze({
  PlanValidationMissing: "PHASE008_COMMAND_BRIDGE_PLAN_VALIDATION_MISSING",
  NativeBootstrapMissing: "PHASE008_COMMAND_BRIDGE_NATIVE_BOOTSTRAP_MISSING",
  ContractTestsFailed: "PHASE008_COMMAND_BRIDGE_CONTRACT_TESTS_FAILED",
  RustBoundaryTestsFailed: "PHASE008_COMMAND_BRIDGE_RUST_BOUNDARY_TESTS_FAILED",
  ForbiddenBoundaryAccess: "PHASE008_COMMAND_BRIDGE_FORBIDDEN_BOUNDARY_ACCESS",
  IoFailed: "PHASE008_COMMAND_BRIDGE_IO_FAILED",
  InvalidTransition: "PHASE008_COMMAND_BRIDGE_INVALID_TRANSITION",
});

const allowedCommandNames = [
  "open_default_workspace",
  "get_current_document",
  "save_current_document",
  "list_document_history",
  "search_documents",
  "get_link_overview",
  "get_asset_metadata",
];

const commandOrder = [
  {
    key: "contract",
    errorCode: CommandBridgeGateErrorCode.ContractTestsFailed,
    command: "npm run run:phase008-command-bridge-contract-tests",
    spawn: ["npm", ["run", "run:phase008-command-bridge-contract-tests"]],
  },
  {
    key: "rustBoundary",
    errorCode: CommandBridgeGateErrorCode.RustBoundaryTestsFailed,
    command: "cargo test -p cabinet-desktop-shell local_desktop_command",
    spawn: ["cargo", ["test", "-p", "cabinet-desktop-shell", "local_desktop_command"]],
  },
];

const forbiddenBoundaryTerms = [
  "std::env::var",
  "std::fs",
  "LocalDocumentRepository",
  "LocalVersionStore",
  "LocalSearchIndex",
  "SPONZEY_CABINET_APP_DATA_DIR",
  "metadata_dir",
  "version_store",
  "asset_store",
  "search_index",
];

export function transitionCommandBridgeGateState(currentState, event, detail = {}) {
  if (currentState === CommandBridgeGateState.NotStarted && event === CommandBridgeGateEvent.Start) {
    return { state: CommandBridgeGateState.CheckingPrerequisites };
  }
  if (
    currentState === CommandBridgeGateState.CheckingPrerequisites &&
    event === CommandBridgeGateEvent.PrerequisitesChecked
  ) {
    return { state: CommandBridgeGateState.RunningContractTests };
  }
  if (
    currentState === CommandBridgeGateState.RunningContractTests &&
    event === CommandBridgeGateEvent.ContractTestsPassed
  ) {
    return { state: CommandBridgeGateState.RunningRustBoundaryTests };
  }
  if (
    currentState === CommandBridgeGateState.RunningRustBoundaryTests &&
    event === CommandBridgeGateEvent.RustBoundaryTestsPassed
  ) {
    return { state: CommandBridgeGateState.ScanningBoundary };
  }
  if (
    currentState === CommandBridgeGateState.ScanningBoundary &&
    event === CommandBridgeGateEvent.BoundaryScanPassed
  ) {
    return { state: CommandBridgeGateState.WritingResult };
  }
  if (
    currentState === CommandBridgeGateState.WritingResult &&
    event === CommandBridgeGateEvent.ResultWritten
  ) {
    return { state: CommandBridgeGateState.Passed };
  }
  if (
    [
      CommandBridgeGateState.CheckingPrerequisites,
      CommandBridgeGateState.RunningContractTests,
      CommandBridgeGateState.RunningRustBoundaryTests,
      CommandBridgeGateState.ScanningBoundary,
      CommandBridgeGateState.WritingResult,
    ].includes(currentState) &&
    event === CommandBridgeGateEvent.Fail
  ) {
    return {
      state: CommandBridgeGateState.Failed,
      errorCode: detail.errorCode ?? CommandBridgeGateErrorCode.IoFailed,
      findingId: detail.findingId,
      failedCommandExitCode: detail.failedCommandExitCode,
    };
  }
  return {
    state: CommandBridgeGateState.Failed,
    errorCode: CommandBridgeGateErrorCode.InvalidTransition,
  };
}

export function evaluateCommandBridgeGate({
  planValidationText,
  nativeBootstrapText,
  commandResults,
  routeSourceText,
}) {
  let state = transitionCommandBridgeGateState(
    CommandBridgeGateState.NotStarted,
    CommandBridgeGateEvent.Start,
  );

  if (!planValidationText.includes("phase008_plan_validation=passed")) {
    state = transitionCommandBridgeGateState(state.state, CommandBridgeGateEvent.Fail, {
      errorCode: CommandBridgeGateErrorCode.PlanValidationMissing,
      findingId: ".tasks/phase008-plan-validation-result.md",
    });
    return failedResult(state, commandResults);
  }
  if (!nativeBootstrapText.includes("phase008_native_bootstrap_gate=passed")) {
    state = transitionCommandBridgeGateState(state.state, CommandBridgeGateEvent.Fail, {
      errorCode: CommandBridgeGateErrorCode.NativeBootstrapMissing,
      findingId: ".tasks/phase008-native-bootstrap-gate-result.md",
    });
    return failedResult(state, commandResults);
  }
  state = transitionCommandBridgeGateState(
    state.state,
    CommandBridgeGateEvent.PrerequisitesChecked,
  );

  const contractResult = commandResults.contract;
  if (!contractResult?.passed) {
    state = transitionCommandBridgeGateState(state.state, CommandBridgeGateEvent.Fail, {
      errorCode: CommandBridgeGateErrorCode.ContractTestsFailed,
      findingId: "contract",
      failedCommandExitCode: contractResult?.exitCode,
    });
    return failedResult(state, commandResults);
  }
  state = transitionCommandBridgeGateState(
    state.state,
    CommandBridgeGateEvent.ContractTestsPassed,
  );

  const rustResult = commandResults.rustBoundary;
  if (!rustResult?.passed) {
    state = transitionCommandBridgeGateState(state.state, CommandBridgeGateEvent.Fail, {
      errorCode: CommandBridgeGateErrorCode.RustBoundaryTestsFailed,
      findingId: "rustBoundary",
      failedCommandExitCode: rustResult?.exitCode,
    });
    return failedResult(state, commandResults);
  }
  state = transitionCommandBridgeGateState(
    state.state,
    CommandBridgeGateEvent.RustBoundaryTestsPassed,
  );

  for (const term of forbiddenBoundaryTerms) {
    if (routeSourceText.includes(term)) {
      state = transitionCommandBridgeGateState(state.state, CommandBridgeGateEvent.Fail, {
        errorCode: CommandBridgeGateErrorCode.ForbiddenBoundaryAccess,
        findingId: term,
      });
      return failedResult(state, commandResults);
    }
  }
  state = transitionCommandBridgeGateState(state.state, CommandBridgeGateEvent.BoundaryScanPassed);
  state = transitionCommandBridgeGateState(state.state, CommandBridgeGateEvent.ResultWritten);

  return {
    passed: true,
    state: state.state,
    allowedCommandCount: allowedCommandNames.length,
    commandCount: commandOrder.length,
    commandResults,
  };
}

export function renderCommandBridgeGateResult(result) {
  if (result.passed) {
    return [
      "phase008_command_bridge_gate=passed",
      `validation_state=${result.state}`,
      `allowed_command_count=${result.allowedCommandCount}`,
      `validation_command_count=${result.commandCount}`,
      "boundary_scan=passed",
    ].join("\n");
  }
  const lines = [
    "phase008_command_bridge_gate=failed",
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

export function renderCommandBridgeGateArtifact(result) {
  const status = result.passed ? "passed" : "failed";
  return [
    "# Phase 008 Command Bridge Gate Result",
    "",
    renderCommandBridgeGateResult(result),
    "",
    "- phase: `Phase 008.2`",
    "- gate: `Tauri Command Bridge`",
    `- status: \`${status}\``,
    `- state: \`${result.state}\``,
    "- prerequisites:",
    "  - `.tasks/phase008-plan-validation-result.md` with `phase008_plan_validation=passed`",
    "  - `.tasks/phase008-native-bootstrap-gate-result.md` with `phase008_native_bootstrap_gate=passed`",
    "- validation commands:",
    ...commandOrder.map(({ command }) => `  - \`${command}\``),
    "- allowed commands:",
    ...allowedCommandNames.map((name) => `  - \`${name}\``),
    "- changed layers:",
    "  - `packages/client-core`: local desktop command client contract",
    "  - `apps/desktop`: desktop local facade adapter",
    "  - `apps/desktop/src-tauri`: Rust/Tauri local command boundary",
    "  - `scripts`: command bridge gate and wrappers",
    "- Product Log candidates: `command.bridge.failed`, `command.bridge.dispatched`",
    "- Field Debug metadata candidates: `command_name`, `stable_error_code`, `retryable`, `duration_bucket`",
    "- boundary scan: local command route does not directly read environment variables, filesystem storage, repositories, version store, search index, or asset store.",
    "- sensitive-data exclusion: this artifact records markers, command ids, counts, states, and stable error codes only. It does not record raw command payload, document body, asset content, AI prompt, AI answer, provider key, token, credential, secret, or personal absolute path.",
    "- follow-up limitation: Phase 008.3 durable document runtime remains incomplete.",
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
  return {
    key,
    command,
    passed: completed.status === 0,
    exitCode: completed.status ?? 1,
  };
}

function extractRouteSource(sourceText) {
  const start = sourceText.indexOf("pub fn route_local_desktop_command");
  if (start < 0) {
    return "";
  }
  const end = sourceText.indexOf("/// Returns the expected bundled desktop asset directory.", start);
  return sourceText.slice(start, end < 0 ? undefined : end);
}

async function runCommandBridgeGateCli() {
  let result;
  try {
    const [planValidationText, nativeBootstrapText, libSourceText] = await Promise.all([
      readFile(".tasks/phase008-plan-validation-result.md", "utf8"),
      readFile(".tasks/phase008-native-bootstrap-gate-result.md", "utf8"),
      readFile("apps/desktop/src-tauri/src/lib.rs", "utf8"),
    ]);
    const commandResults = Object.fromEntries(
      commandOrder.map((commandSpec) => [commandSpec.key, runCommand(commandSpec)]),
    );
    result = evaluateCommandBridgeGate({
      planValidationText,
      nativeBootstrapText,
      commandResults,
      routeSourceText: extractRouteSource(libSourceText),
    });
  } catch {
    const state = {
      state: CommandBridgeGateState.Failed,
      errorCode: CommandBridgeGateErrorCode.IoFailed,
      findingId: "command_bridge_sources",
    };
    result = failedResult(state, {});
  }

  await writeFile(
    ".tasks/phase008-command-bridge-gate-result.md",
    renderCommandBridgeGateArtifact(result),
  );
  const rendered = renderCommandBridgeGateResult(result);
  if (result.passed) {
    console.log(rendered);
    return;
  }
  console.error(rendered);
  process.exit(1);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runCommandBridgeGateCli();
}
