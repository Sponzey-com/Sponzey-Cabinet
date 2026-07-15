import { readFile, writeFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { pathToFileURL } from "node:url";

export const AssetLifecycleGateState = Object.freeze({
  NotStarted: "NotStarted",
  CheckingPrerequisites: "CheckingPrerequisites",
  RunningAssetTests: "RunningAssetTests",
  RunningUiTests: "RunningUiTests",
  WritingResult: "WritingResult",
  Passed: "Passed",
  Failed: "Failed",
});

export const AssetLifecycleGateEvent = Object.freeze({
  Start: "Start",
  PrerequisitesChecked: "PrerequisitesChecked",
  AssetTestsPassed: "AssetTestsPassed",
  UiTestsPassed: "UiTestsPassed",
  ResultWritten: "ResultWritten",
  Fail: "Fail",
});

export const AssetLifecycleGateErrorCode = Object.freeze({
  ProjectionGateMissing: "PHASE008_ASSET_LIFECYCLE_PROJECTION_GATE_MISSING",
  AssetTestsFailed: "PHASE008_ASSET_LIFECYCLE_ASSET_TESTS_FAILED",
  AdapterTestsFailed: "PHASE008_ASSET_LIFECYCLE_ADAPTER_TESTS_FAILED",
  UiTestsFailed: "PHASE008_ASSET_LIFECYCLE_UI_TESTS_FAILED",
  IoFailed: "PHASE008_ASSET_LIFECYCLE_IO_FAILED",
  InvalidTransition: "PHASE008_ASSET_LIFECYCLE_INVALID_TRANSITION",
});

const commandOrder = [
  {
    key: "asset",
    errorCode: AssetLifecycleGateErrorCode.AssetTestsFailed,
    command: "npm run run:phase008-asset-lifecycle-domain-usecase-tests",
    spawn: ["npm", ["run", "run:phase008-asset-lifecycle-domain-usecase-tests"]],
  },
  {
    key: "adapter",
    errorCode: AssetLifecycleGateErrorCode.AdapterTestsFailed,
    command: "npm run run:phase008-asset-lifecycle-adapter-tests",
    spawn: ["npm", ["run", "run:phase008-asset-lifecycle-adapter-tests"]],
  },
  {
    key: "ui",
    errorCode: AssetLifecycleGateErrorCode.UiTestsFailed,
    command: "npm run run:phase008-asset-lifecycle-ui-tests",
    spawn: ["npm", ["run", "run:phase008-asset-lifecycle-ui-tests"]],
  },
];

export function transitionAssetLifecycleGateState(currentState, event, detail = {}) {
  if (currentState === AssetLifecycleGateState.NotStarted && event === AssetLifecycleGateEvent.Start) {
    return { state: AssetLifecycleGateState.CheckingPrerequisites };
  }
  if (
    currentState === AssetLifecycleGateState.CheckingPrerequisites &&
    event === AssetLifecycleGateEvent.PrerequisitesChecked
  ) {
    return { state: AssetLifecycleGateState.RunningAssetTests };
  }
  if (
    currentState === AssetLifecycleGateState.RunningAssetTests &&
    event === AssetLifecycleGateEvent.AssetTestsPassed
  ) {
    return { state: AssetLifecycleGateState.RunningUiTests };
  }
  if (
    currentState === AssetLifecycleGateState.RunningUiTests &&
    event === AssetLifecycleGateEvent.UiTestsPassed
  ) {
    return { state: AssetLifecycleGateState.WritingResult };
  }
  if (
    currentState === AssetLifecycleGateState.WritingResult &&
    event === AssetLifecycleGateEvent.ResultWritten
  ) {
    return { state: AssetLifecycleGateState.Passed };
  }
  if (
    [
      AssetLifecycleGateState.CheckingPrerequisites,
      AssetLifecycleGateState.RunningAssetTests,
      AssetLifecycleGateState.RunningUiTests,
      AssetLifecycleGateState.WritingResult,
    ].includes(currentState) &&
    event === AssetLifecycleGateEvent.Fail
  ) {
    return {
      state: AssetLifecycleGateState.Failed,
      errorCode: detail.errorCode ?? AssetLifecycleGateErrorCode.IoFailed,
      findingId: detail.findingId,
      failedCommandExitCode: detail.failedCommandExitCode,
    };
  }
  return { state: AssetLifecycleGateState.Failed, errorCode: AssetLifecycleGateErrorCode.InvalidTransition };
}

export function evaluateAssetLifecycleGate({ projectionText, commandResults }) {
  let state = transitionAssetLifecycleGateState(
    AssetLifecycleGateState.NotStarted,
    AssetLifecycleGateEvent.Start,
  );
  if (!projectionText.includes("phase008_projection_index_gate=passed")) {
    state = transitionAssetLifecycleGateState(state.state, AssetLifecycleGateEvent.Fail, {
      errorCode: AssetLifecycleGateErrorCode.ProjectionGateMissing,
      findingId: ".tasks/phase008-projection-index-gate-result.md",
    });
    return failedResult(state, commandResults);
  }
  state = transitionAssetLifecycleGateState(state.state, AssetLifecycleGateEvent.PrerequisitesChecked);

  const assetCommandKeys = ["asset", "adapter"];
  for (const commandKey of assetCommandKeys) {
    const command = commandOrder.find((candidate) => candidate.key === commandKey);
    const result = commandResults[command.key];
    if (!result?.passed) {
      state = transitionAssetLifecycleGateState(state.state, AssetLifecycleGateEvent.Fail, {
        errorCode: command.errorCode,
        findingId: command.key,
        failedCommandExitCode: result?.exitCode,
      });
      return failedResult(state, commandResults);
    }
  }
  state = transitionAssetLifecycleGateState(state.state, AssetLifecycleGateEvent.AssetTestsPassed);

  const uiCommand = commandOrder.find((candidate) => candidate.key === "ui");
  const uiResult = commandResults[uiCommand.key];
  if (!uiResult?.passed) {
    state = transitionAssetLifecycleGateState(state.state, AssetLifecycleGateEvent.Fail, {
      errorCode: uiCommand.errorCode,
      findingId: uiCommand.key,
      failedCommandExitCode: uiResult?.exitCode,
    });
    return failedResult(state, commandResults);
  }
  state = transitionAssetLifecycleGateState(state.state, AssetLifecycleGateEvent.UiTestsPassed);
  state = transitionAssetLifecycleGateState(state.state, AssetLifecycleGateEvent.ResultWritten);
  return { passed: true, state: state.state, commandCount: commandOrder.length };
}

export function renderAssetLifecycleGateArtifact(result) {
  const status = result.passed ? "passed" : "failed";
  return [
    "# Phase 008 Asset Lifecycle Gate Result",
    "",
    result.passed
      ? [
          "phase008_asset_lifecycle_gate=passed",
          `validation_state=${result.state}`,
          `validation_command_count=${result.commandCount}`,
          "asset_metadata_without_object_bytes=true",
        ].join("\n")
      : [
          "phase008_asset_lifecycle_gate=failed",
          `validation_state=${result.state}`,
          `error_code=${result.errorCode}`,
          result.findingId ? `finding_id=${result.findingId}` : undefined,
        ].filter(Boolean).join("\n"),
    "",
    "- phase: `Phase 008.5`",
    "- gate: `Native Asset Lifecycle`",
    `- status: \`${status}\``,
    "- prerequisites:",
    "  - `.tasks/phase008-projection-index-gate-result.md` with `phase008_projection_index_gate=passed`",
    "- validation commands:",
    ...commandOrder.map(({ command }) => `  - \`${command}\``),
    "- evidence: asset lifecycle, attach file to document, list asset metadata, local asset store, object storage adapter contract, desktop asset UI safety.",
    "- sensitive-data exclusion: this artifact records markers, command ids, counts, states, and stable error codes only. It does not record raw asset content, raw local absolute path, document body, AI prompt, AI answer, provider key, token, credential, or secret.",
    "- follow-up limitation: Phase 008.6 recovery/backup gate remains incomplete.",
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

async function runAssetLifecycleGateCli() {
  let result;
  try {
    const projectionText = await readFile(".tasks/phase008-projection-index-gate-result.md", "utf8");
    const commandResults = projectionText.includes("phase008_projection_index_gate=passed")
      ? Object.fromEntries(commandOrder.map((commandSpec) => [commandSpec.key, runCommand(commandSpec)]))
      : {};
    result = evaluateAssetLifecycleGate({ projectionText, commandResults });
  } catch {
    const state = {
      state: AssetLifecycleGateState.Failed,
      errorCode: AssetLifecycleGateErrorCode.IoFailed,
      findingId: ".tasks/phase008-projection-index-gate-result.md",
    };
    result = failedResult(state, {});
  }
  await writeFile(
    ".tasks/phase008-asset-lifecycle-gate-result.md",
    renderAssetLifecycleGateArtifact(result),
  );
  if (result.passed) {
    console.log("phase008_asset_lifecycle_gate=passed");
    return;
  }
  console.error("phase008_asset_lifecycle_gate=failed");
  process.exit(1);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runAssetLifecycleGateCli();
}
