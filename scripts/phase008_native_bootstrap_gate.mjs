import { readFile, writeFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { pathToFileURL } from "node:url";

export const NativeBootstrapGateState = Object.freeze({
  NotStarted: "NotStarted",
  CheckingPrerequisites: "CheckingPrerequisites",
  RunningContractTests: "RunningContractTests",
  RunningFirstRunTests: "RunningFirstRunTests",
  WritingResult: "WritingResult",
  Passed: "Passed",
  Failed: "Failed",
});

export const NativeBootstrapGateEvent = Object.freeze({
  Start: "Start",
  PrerequisitesChecked: "PrerequisitesChecked",
  ContractTestsPassed: "ContractTestsPassed",
  FirstRunTestsPassed: "FirstRunTestsPassed",
  ResultWritten: "ResultWritten",
  Fail: "Fail",
});

export const NativeBootstrapGateErrorCode = Object.freeze({
  PlanValidationMissing: "PHASE008_NATIVE_BOOTSTRAP_PLAN_VALIDATION_MISSING",
  ContractTestsFailed: "PHASE008_NATIVE_BOOTSTRAP_CONTRACT_TESTS_FAILED",
  FirstRunTestsFailed: "PHASE008_NATIVE_BOOTSTRAP_FIRST_RUN_TESTS_FAILED",
  SetupHealthTestsFailed: "PHASE008_NATIVE_BOOTSTRAP_SETUP_HEALTH_TESTS_FAILED",
  PlatformTestsFailed: "PHASE008_NATIVE_BOOTSTRAP_PLATFORM_TESTS_FAILED",
  IoFailed: "PHASE008_NATIVE_BOOTSTRAP_IO_FAILED",
  InvalidTransition: "PHASE008_NATIVE_BOOTSTRAP_INVALID_TRANSITION",
});

const commandOrder = [
  {
    key: "contract",
    errorCode: NativeBootstrapGateErrorCode.ContractTestsFailed,
    command: "npm run run:phase008-native-bootstrap-contract-tests",
    spawn: ["npm", ["run", "run:phase008-native-bootstrap-contract-tests"]],
  },
  {
    key: "firstRunStore",
    errorCode: NativeBootstrapGateErrorCode.FirstRunTestsFailed,
    command: "cargo test -p cabinet-adapters --test local_first_run_store_tests",
    spawn: ["cargo", ["test", "-p", "cabinet-adapters", "--test", "local_first_run_store_tests"]],
  },
  {
    key: "setupHealth",
    errorCode: NativeBootstrapGateErrorCode.SetupHealthTestsFailed,
    command: "cargo test -p cabinet-adapters --test local_setup_health_checker_tests",
    spawn: [
      "cargo",
      ["test", "-p", "cabinet-adapters", "--test", "local_setup_health_checker_tests"],
    ],
  },
  {
    key: "platform",
    errorCode: NativeBootstrapGateErrorCode.PlatformTestsFailed,
    command: "cargo test -p cabinet-platform",
    spawn: ["cargo", ["test", "-p", "cabinet-platform"]],
  },
];

export function transitionNativeBootstrapGateState(currentState, event, detail = {}) {
  if (currentState === NativeBootstrapGateState.NotStarted && event === NativeBootstrapGateEvent.Start) {
    return { state: NativeBootstrapGateState.CheckingPrerequisites };
  }
  if (
    currentState === NativeBootstrapGateState.CheckingPrerequisites &&
    event === NativeBootstrapGateEvent.PrerequisitesChecked
  ) {
    return { state: NativeBootstrapGateState.RunningContractTests };
  }
  if (
    currentState === NativeBootstrapGateState.RunningContractTests &&
    event === NativeBootstrapGateEvent.ContractTestsPassed
  ) {
    return { state: NativeBootstrapGateState.RunningFirstRunTests };
  }
  if (
    currentState === NativeBootstrapGateState.RunningFirstRunTests &&
    event === NativeBootstrapGateEvent.FirstRunTestsPassed
  ) {
    return { state: NativeBootstrapGateState.WritingResult };
  }
  if (
    currentState === NativeBootstrapGateState.WritingResult &&
    event === NativeBootstrapGateEvent.ResultWritten
  ) {
    return { state: NativeBootstrapGateState.Passed };
  }
  if (
    [
      NativeBootstrapGateState.CheckingPrerequisites,
      NativeBootstrapGateState.RunningContractTests,
      NativeBootstrapGateState.RunningFirstRunTests,
      NativeBootstrapGateState.WritingResult,
    ].includes(currentState) &&
    event === NativeBootstrapGateEvent.Fail
  ) {
    return {
      state: NativeBootstrapGateState.Failed,
      errorCode: detail.errorCode ?? NativeBootstrapGateErrorCode.IoFailed,
      findingId: detail.findingId,
      failedCommandExitCode: detail.failedCommandExitCode,
    };
  }
  return {
    state: NativeBootstrapGateState.Failed,
    errorCode: NativeBootstrapGateErrorCode.InvalidTransition,
  };
}

export function evaluateNativeBootstrapGate({ planValidationText, commandResults }) {
  let state = transitionNativeBootstrapGateState(
    NativeBootstrapGateState.NotStarted,
    NativeBootstrapGateEvent.Start,
  );

  if (!planValidationText.includes("phase008_plan_validation=passed")) {
    state = transitionNativeBootstrapGateState(state.state, NativeBootstrapGateEvent.Fail, {
      errorCode: NativeBootstrapGateErrorCode.PlanValidationMissing,
      findingId: ".tasks/phase008-plan-validation-result.md",
    });
    return failedResult(state, []);
  }
  state = transitionNativeBootstrapGateState(
    state.state,
    NativeBootstrapGateEvent.PrerequisitesChecked,
  );

  const contractResult = commandResults.contract;
  if (!contractResult?.passed) {
    state = transitionNativeBootstrapGateState(state.state, NativeBootstrapGateEvent.Fail, {
      errorCode: NativeBootstrapGateErrorCode.ContractTestsFailed,
      findingId: "contract",
      failedCommandExitCode: contractResult?.exitCode,
    });
    return failedResult(state, commandResults);
  }
  state = transitionNativeBootstrapGateState(
    state.state,
    NativeBootstrapGateEvent.ContractTestsPassed,
  );

  for (const { key, errorCode } of commandOrder.filter(({ key }) => key !== "contract")) {
    const commandResult = commandResults[key];
    if (!commandResult?.passed) {
      state = transitionNativeBootstrapGateState(state.state, NativeBootstrapGateEvent.Fail, {
        errorCode,
        findingId: key,
        failedCommandExitCode: commandResult?.exitCode,
      });
      return failedResult(state, commandResults);
    }
  }

  state = transitionNativeBootstrapGateState(
    state.state,
    NativeBootstrapGateEvent.FirstRunTestsPassed,
  );
  state = transitionNativeBootstrapGateState(state.state, NativeBootstrapGateEvent.ResultWritten);

  return {
    passed: true,
    state: state.state,
    commandCount: commandOrder.length,
    commandResults,
  };
}

export function renderNativeBootstrapGateResult(result) {
  if (result.passed) {
    return [
      "phase008_native_bootstrap_gate=passed",
      `validation_state=${result.state}`,
      `validation_command_count=${result.commandCount}`,
      "setup_health_status=healthy",
      "first_run_idempotent=true",
    ].join("\n");
  }

  const lines = [
    "phase008_native_bootstrap_gate=failed",
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

export function renderNativeBootstrapGateArtifact(result) {
  const status = result.passed ? "passed" : "failed";
  return [
    "# Phase 008 Native Bootstrap Gate Result",
    "",
    renderNativeBootstrapGateResult(result),
    "",
    "- phase: `Phase 008.1`",
    "- gate: `Native Desktop Bootstrap`",
    `- status: \`${status}\``,
    `- state: \`${result.state}\``,
    "- prerequisites:",
    "  - `.tasks/phase008-plan-validation-result.md` with `phase008_plan_validation=passed`",
    "- validation commands:",
    ...commandOrder.map(({ command }) => `  - \`${command}\``),
    "- changed layers:",
    "  - `cabinet-core`: LocalDesktopConfig contract and read-once bootstrap helper",
    "  - `cabinet-platform`: native bootstrap state machine",
    "  - `cabinet-adapters`: existing local first-run and setup health adapters verified",
    "  - `scripts`: native bootstrap gate and wrappers",
    "- Product Log candidates: `app.start.completed`, `workspace.default.ready`, `workspace.bootstrap.failed`",
    "- Field Debug metadata candidates: `setup_role`, `setup_status`, `stable_error_code`, `retryable`",
    "- setup health evidence: clean profile healthy, missing required directory detected, file-instead-of-directory detected, missing first-run marker detected.",
    "- first-run evidence: clean profile creates metadata, version-store, asset, search-index, and workspace directories; second run is idempotent.",
    "- sensitive-data exclusion: this artifact records markers, command ids, counts, states, and stable error codes only. It does not record document body, asset content, AI prompt, AI answer, provider key, token, credential, secret, or personal absolute path.",
    "- follow-up limitation: Phase 008.2 Tauri command bridge remains incomplete.",
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

async function runNativeBootstrapGateCli() {
  let result;
  try {
    const planValidationText = await readFile(
      ".tasks/phase008-plan-validation-result.md",
      "utf8",
    );
    const commandResults = Object.fromEntries(
      commandOrder.map((commandSpec) => [commandSpec.key, runCommand(commandSpec)]),
    );
    result = evaluateNativeBootstrapGate({ planValidationText, commandResults });
  } catch {
    const state = {
      state: NativeBootstrapGateState.Failed,
      errorCode: NativeBootstrapGateErrorCode.IoFailed,
      findingId: ".tasks/phase008-plan-validation-result.md",
    };
    result = failedResult(state, {});
  }

  await writeFile(
    ".tasks/phase008-native-bootstrap-gate-result.md",
    renderNativeBootstrapGateArtifact(result),
  );
  const rendered = renderNativeBootstrapGateResult(result);
  if (result.passed) {
    console.log(rendered);
    return;
  }
  console.error(rendered);
  process.exit(1);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runNativeBootstrapGateCli();
}
