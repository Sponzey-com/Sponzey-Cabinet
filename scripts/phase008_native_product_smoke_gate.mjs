import { readFile, writeFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { pathToFileURL } from "node:url";

export const NativeProductSmokeGateState = Object.freeze({
  NotStarted: "NotStarted",
  ReadingEvidence: "ReadingEvidence",
  RunningNativeRuntimeSmoke: "RunningNativeRuntimeSmoke",
  RunningDesktopUiSmoke: "RunningDesktopUiSmoke",
  RunningBrowserSmoke: "RunningBrowserSmoke",
  WritingResult: "WritingResult",
  Passed: "Passed",
  Failed: "Failed",
});

export const NativeProductSmokeGateEvent = Object.freeze({
  Start: "Start",
  EvidenceReady: "EvidenceReady",
  NativeRuntimeSmokePassed: "NativeRuntimeSmokePassed",
  DesktopUiSmokePassed: "DesktopUiSmokePassed",
  BrowserSmokePassed: "BrowserSmokePassed",
  ResultWritten: "ResultWritten",
  Fail: "Fail",
});

export const NativeProductSmokeGateErrorCode = Object.freeze({
  LowerGateMissing: "PHASE008_NATIVE_PRODUCT_SMOKE_LOWER_GATE_MISSING",
  RuntimeSmokeFailed: "PHASE008_NATIVE_PRODUCT_SMOKE_RUNTIME_FAILED",
  DesktopUiSmokeFailed: "PHASE008_NATIVE_PRODUCT_SMOKE_DESKTOP_UI_FAILED",
  BrowserSmokeFailed: "PHASE008_NATIVE_PRODUCT_SMOKE_BROWSER_FAILED",
  IoFailed: "PHASE008_NATIVE_PRODUCT_SMOKE_IO_FAILED",
  InvalidTransition: "PHASE008_NATIVE_PRODUCT_SMOKE_INVALID_TRANSITION",
});

const requiredMarkers = [
  marker(".tasks/phase008-plan-validation-result.md", "phase008_plan_validation=passed"),
  marker(".tasks/phase008-native-bootstrap-gate-result.md", "phase008_native_bootstrap_gate=passed"),
  marker(".tasks/phase008-command-bridge-gate-result.md", "phase008_command_bridge_gate=passed"),
  marker(".tasks/phase008-document-runtime-gate-result.md", "phase008_document_runtime_gate=passed"),
  marker(".tasks/phase008-projection-index-gate-result.md", "phase008_projection_index_gate=passed"),
  marker(".tasks/phase008-asset-lifecycle-gate-result.md", "phase008_asset_lifecycle_gate=passed"),
  marker(".tasks/phase008-recovery-backup-gate-result.md", "phase008_recovery_backup_gate=passed"),
];

const commandOrder = [
  {
    key: "runtime",
    event: NativeProductSmokeGateEvent.NativeRuntimeSmokePassed,
    errorCode: NativeProductSmokeGateErrorCode.RuntimeSmokeFailed,
    command: "npm run run:phase008-native-product-smoke-runtime-tests",
    spawn: ["npm", ["run", "run:phase008-native-product-smoke-runtime-tests"]],
  },
  {
    key: "ui",
    event: NativeProductSmokeGateEvent.DesktopUiSmokePassed,
    errorCode: NativeProductSmokeGateErrorCode.DesktopUiSmokeFailed,
    command: "npm run run:phase008-native-product-smoke-ui-tests",
    spawn: ["npm", ["run", "run:phase008-native-product-smoke-ui-tests"]],
  },
  {
    key: "browser",
    event: NativeProductSmokeGateEvent.BrowserSmokePassed,
    errorCode: NativeProductSmokeGateErrorCode.BrowserSmokeFailed,
    command: "npm run run:phase008-native-product-smoke-browser",
    spawn: ["npm", ["run", "run:phase008-native-product-smoke-browser"]],
  },
];

export function transitionNativeProductSmokeGateState(currentState, event, detail = {}) {
  if (currentState === NativeProductSmokeGateState.NotStarted && event === NativeProductSmokeGateEvent.Start) {
    return { state: NativeProductSmokeGateState.ReadingEvidence };
  }
  if (
    currentState === NativeProductSmokeGateState.ReadingEvidence &&
    event === NativeProductSmokeGateEvent.EvidenceReady
  ) {
    return { state: NativeProductSmokeGateState.RunningNativeRuntimeSmoke };
  }
  if (
    currentState === NativeProductSmokeGateState.RunningNativeRuntimeSmoke &&
    event === NativeProductSmokeGateEvent.NativeRuntimeSmokePassed
  ) {
    return { state: NativeProductSmokeGateState.RunningDesktopUiSmoke };
  }
  if (
    currentState === NativeProductSmokeGateState.RunningDesktopUiSmoke &&
    event === NativeProductSmokeGateEvent.DesktopUiSmokePassed
  ) {
    return { state: NativeProductSmokeGateState.RunningBrowserSmoke };
  }
  if (
    currentState === NativeProductSmokeGateState.RunningBrowserSmoke &&
    event === NativeProductSmokeGateEvent.BrowserSmokePassed
  ) {
    return { state: NativeProductSmokeGateState.WritingResult };
  }
  if (
    currentState === NativeProductSmokeGateState.WritingResult &&
    event === NativeProductSmokeGateEvent.ResultWritten
  ) {
    return { state: NativeProductSmokeGateState.Passed };
  }
  if (
    [
      NativeProductSmokeGateState.ReadingEvidence,
      NativeProductSmokeGateState.RunningNativeRuntimeSmoke,
      NativeProductSmokeGateState.RunningDesktopUiSmoke,
      NativeProductSmokeGateState.RunningBrowserSmoke,
      NativeProductSmokeGateState.WritingResult,
    ].includes(currentState) &&
    event === NativeProductSmokeGateEvent.Fail
  ) {
    return {
      state: NativeProductSmokeGateState.Failed,
      errorCode: detail.errorCode ?? NativeProductSmokeGateErrorCode.IoFailed,
      findingId: detail.findingId,
      failedCommandExitCode: detail.failedCommandExitCode,
    };
  }
  return { state: NativeProductSmokeGateState.Failed, errorCode: NativeProductSmokeGateErrorCode.InvalidTransition };
}

export function evaluateNativeProductSmokeGate({ markerTexts, commandResults }) {
  let state = transitionNativeProductSmokeGateState(
    NativeProductSmokeGateState.NotStarted,
    NativeProductSmokeGateEvent.Start,
  );

  const missingMarker = requiredMarkers.find((entry) => !(markerTexts[entry.filePath] ?? "").includes(entry.marker));
  if (missingMarker) {
    state = transitionNativeProductSmokeGateState(state.state, NativeProductSmokeGateEvent.Fail, {
      errorCode: NativeProductSmokeGateErrorCode.LowerGateMissing,
      findingId: missingMarker.filePath,
    });
    return failedResult(state, commandResults);
  }

  state = transitionNativeProductSmokeGateState(state.state, NativeProductSmokeGateEvent.EvidenceReady);
  for (const command of commandOrder) {
    const result = commandResults[command.key];
    if (!result?.passed) {
      state = transitionNativeProductSmokeGateState(state.state, NativeProductSmokeGateEvent.Fail, {
        errorCode: command.errorCode,
        findingId: command.key,
        failedCommandExitCode: result?.exitCode,
      });
      return failedResult(state, commandResults);
    }
    state = transitionNativeProductSmokeGateState(state.state, command.event);
  }
  state = transitionNativeProductSmokeGateState(state.state, NativeProductSmokeGateEvent.ResultWritten);

  return {
    passed: true,
    state: state.state,
    commandCount: commandOrder.length,
    lowerGateCount: requiredMarkers.length,
  };
}

export function renderNativeProductSmokeGateArtifact(result) {
  const status = result.passed ? "passed" : "failed";
  return [
    "# Phase 008 Native Product Smoke Gate Result",
    "",
    result.passed
      ? [
          "phase008_native_product_smoke_gate=passed",
          `validation_state=${result.state}`,
          `validation_command_count=${result.commandCount}`,
          `validated_lower_gate_count=${result.lowerGateCount}`,
          "clean_install_smoke=true",
          "save_reopen_restore_search_asset_backup_smoke=true",
        ].join("\n")
      : [
          "phase008_native_product_smoke_gate=failed",
          `validation_state=${result.state}`,
          `error_code=${result.errorCode}`,
          result.findingId ? `finding_id=${result.findingId}` : undefined,
        ].filter(Boolean).join("\n"),
    "",
    "- phase: `Phase 008.7`",
    "- gate: `Native Desktop Product Smoke`",
    `- status: \`${status}\``,
    "- prerequisites:",
    ...requiredMarkers.map((entry) => `  - \`${entry.filePath}\` with \`${entry.marker}\``),
    "- validation commands:",
    ...commandOrder.map(({ command }) => `  - \`${command}\``),
    "- evidence: clean install, MVP end-to-end create/edit/link/search/asset/restore, data preservation after reopen, startup repair, desktop UI authoring/discovery/backup/import, and desktop dist browser render smoke.",
    "- sensitive-data exclusion: this artifact records markers, command ids, counts, states, and stable error codes only. It does not record raw document body, asset content, AI prompt, AI answer, provider key, token, credential, secret, raw local absolute path, browser stdout, or temp workspace path.",
    "- follow-up limitation: Phase 008 final release gate, security manifest, and runbook validation remain incomplete.",
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

function marker(filePath, expectedMarker) {
  return { filePath, marker: expectedMarker };
}

async function readRequiredMarkers() {
  const markerTexts = {};
  for (const entry of requiredMarkers) {
    markerTexts[entry.filePath] = await readFile(entry.filePath, "utf8");
  }
  return markerTexts;
}

async function runNativeProductSmokeGateCli() {
  let result;
  try {
    const markerTexts = await readRequiredMarkers();
    const markersReady = requiredMarkers.every((entry) => markerTexts[entry.filePath].includes(entry.marker));
    const commandResults = markersReady
      ? Object.fromEntries(commandOrder.map((commandSpec) => [commandSpec.key, runCommand(commandSpec)]))
      : {};
    result = evaluateNativeProductSmokeGate({ markerTexts, commandResults });
  } catch {
    const state = {
      state: NativeProductSmokeGateState.Failed,
      errorCode: NativeProductSmokeGateErrorCode.IoFailed,
      findingId: "phase008_lower_gate_marker_read",
    };
    result = failedResult(state, {});
  }
  await writeFile(
    ".tasks/phase008-native-product-smoke-gate-result.md",
    renderNativeProductSmokeGateArtifact(result),
  );
  if (result.passed) {
    console.log("phase008_native_product_smoke_gate=passed");
    return;
  }
  console.error("phase008_native_product_smoke_gate=failed");
  process.exit(1);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runNativeProductSmokeGateCli();
}
