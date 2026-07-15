import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const Phase009DesktopLaunchState = Object.freeze({
  NotStarted: "NotStarted",
  PreparingAssets: "PreparingAssets",
  StartingUiServer: "StartingUiServer",
  WaitingForUiServer: "WaitingForUiServer",
  StartingTauriShell: "StartingTauriShell",
  HydratingReactApp: "HydratingReactApp",
  Ready: "Ready",
  Failed: "Failed",
});

export const Phase009DesktopLaunchEvent = Object.freeze({
  Start: "Start",
  AssetsPrepared: "AssetsPrepared",
  UiServerStarted: "UiServerStarted",
  UiServerReachable: "UiServerReachable",
  TauriShellStarted: "TauriShellStarted",
  HydrationCompleted: "HydrationCompleted",
  LaunchFailed: "LaunchFailed",
});

export const Phase009DesktopLaunchErrorCode = Object.freeze({
  CurrentInventoryMarkerMissing: "PHASE009_CURRENT_INVENTORY_MARKER_MISSING",
  PlanValidationMarkerMissing: "PHASE009_PLAN_VALIDATION_MARKER_MISSING",
  ProductLauncherInvalid: "DESKTOP_PRODUCT_LAUNCHER_INVALID",
  ShellRunnerAmbiguous: "DESKTOP_SHELL_RUNNER_AMBIGUOUS",
  PackageScriptAmbiguous: "DESKTOP_PACKAGE_SCRIPT_AMBIGUOUS",
  AssetBuildFailed: "DESKTOP_ASSET_BUILD_FAILED",
  UiServerUnavailable: "DESKTOP_UI_SERVER_UNAVAILABLE",
  DevUrlPortConflict: "DESKTOP_DEVURL_PORT_CONFLICT",
  TauriStartFailed: "DESKTOP_TAURI_START_FAILED",
  UiHydrationFailed: "DESKTOP_UI_HYDRATION_FAILED",
  BlankScreenDetected: "DESKTOP_BLANK_SCREEN_DETECTED",
  IoFailed: "PHASE009_DESKTOP_LAUNCH_IO_FAILED",
  InvalidTransition: "PHASE009_DESKTOP_LAUNCH_INVALID_TRANSITION",
});

const productLogEvents = Object.freeze([
  "desktop.launch.started",
  "desktop.launch.ready",
  "desktop.launch.failed",
]);

const failureCodes = Object.freeze([
  "DESKTOP_ASSET_BUILD_FAILED",
  "DESKTOP_UI_SERVER_UNAVAILABLE",
  "DESKTOP_DEVURL_PORT_CONFLICT",
  "DESKTOP_TAURI_START_FAILED",
  "DESKTOP_UI_HYDRATION_FAILED",
  "DESKTOP_BLANK_SCREEN_DETECTED",
]);

const requiredUiMarkers = Object.freeze([
  {
    id: "data-cabinet-app-root",
    value: 'data-cabinet-app-root="mounted"',
    errorCode: Phase009DesktopLaunchErrorCode.BlankScreenDetected,
  },
  {
    id: "data-cabinet-workspace-shell",
    value: 'data-cabinet-workspace-shell="ready"',
    errorCode: Phase009DesktopLaunchErrorCode.UiHydrationFailed,
  },
  {
    id: "data-cabinet-editor",
    value: "data-cabinet-editor",
    errorCode: Phase009DesktopLaunchErrorCode.UiHydrationFailed,
  },
  {
    id: "data-cabinet-bootstrap-state",
    value: 'data-cabinet-bootstrap-state="ready"',
    errorCode: Phase009DesktopLaunchErrorCode.UiHydrationFailed,
  },
]);

export function transitionPhase009DesktopLaunchState(currentState, event, detail = {}) {
  if (
    currentState === Phase009DesktopLaunchState.NotStarted &&
    event === Phase009DesktopLaunchEvent.Start
  ) {
    return { state: Phase009DesktopLaunchState.PreparingAssets };
  }
  if (
    currentState === Phase009DesktopLaunchState.PreparingAssets &&
    event === Phase009DesktopLaunchEvent.AssetsPrepared
  ) {
    return { state: Phase009DesktopLaunchState.StartingUiServer };
  }
  if (
    currentState === Phase009DesktopLaunchState.StartingUiServer &&
    event === Phase009DesktopLaunchEvent.UiServerStarted
  ) {
    return { state: Phase009DesktopLaunchState.WaitingForUiServer };
  }
  if (
    currentState === Phase009DesktopLaunchState.WaitingForUiServer &&
    event === Phase009DesktopLaunchEvent.UiServerReachable
  ) {
    return { state: Phase009DesktopLaunchState.StartingTauriShell };
  }
  if (
    currentState === Phase009DesktopLaunchState.StartingTauriShell &&
    event === Phase009DesktopLaunchEvent.TauriShellStarted
  ) {
    return { state: Phase009DesktopLaunchState.HydratingReactApp };
  }
  if (
    currentState === Phase009DesktopLaunchState.HydratingReactApp &&
    event === Phase009DesktopLaunchEvent.HydrationCompleted
  ) {
    return { state: Phase009DesktopLaunchState.Ready };
  }
  if (
    [
      Phase009DesktopLaunchState.PreparingAssets,
      Phase009DesktopLaunchState.StartingUiServer,
      Phase009DesktopLaunchState.WaitingForUiServer,
      Phase009DesktopLaunchState.StartingTauriShell,
      Phase009DesktopLaunchState.HydratingReactApp,
    ].includes(currentState) &&
    event === Phase009DesktopLaunchEvent.LaunchFailed
  ) {
    return {
      state: Phase009DesktopLaunchState.Failed,
      errorCode: detail.errorCode ?? Phase009DesktopLaunchErrorCode.UiHydrationFailed,
      findingId: detail.findingId,
    };
  }
  return {
    state: Phase009DesktopLaunchState.Failed,
    errorCode: Phase009DesktopLaunchErrorCode.InvalidTransition,
  };
}

export function validateDesktopLaunchEvidence({
  desktopAppLauncherText,
  desktopShellText,
  packageJsonText,
}) {
  const appLauncherChecks = [
    ["node scripts/build_desktop_assets.mjs", "build_desktop_assets"],
    ['dev_port="5173"', "dev_port_5173"],
    ["SPONZEY_CABINET_WEB_PUBLIC_DIR=apps/desktop/dist", "desktop_dist_public_dir"],
    ["SPONZEY_CABINET_REQUIRE_EXACT_PORT=1", "exact_port"],
    ['node scripts/run_web_app.mjs "$dev_port" &', "ui_server_before_cargo"],
    ["cargo run -p cabinet-desktop-shell", "tauri_shell"],
    ["trap cleanup EXIT INT TERM", "cleanup_trap"],
  ];

  for (const [needle, findingId] of appLauncherChecks) {
    if (!desktopAppLauncherText.includes(needle)) {
      return failed(Phase009DesktopLaunchErrorCode.ProductLauncherInvalid, findingId);
    }
  }

  if (
    desktopAppLauncherText.indexOf("node scripts/build_desktop_assets.mjs") >
      desktopAppLauncherText.indexOf("node scripts/run_web_app.mjs") ||
    desktopAppLauncherText.indexOf("node scripts/run_web_app.mjs") >
      desktopAppLauncherText.indexOf("cargo run -p cabinet-desktop-shell")
  ) {
    return failed(Phase009DesktopLaunchErrorCode.ProductLauncherInvalid, "launcher_order");
  }

  if (
    !desktopAppLauncherText.includes("Timed out waiting for") &&
    !desktopAppLauncherText.includes("waitForServer")
  ) {
    return failed(Phase009DesktopLaunchErrorCode.UiServerUnavailable, "ui_server_reachability");
  }

  if (
    desktopShellText.includes('mode="${1:-app}"') ||
    desktopShellText.includes("Launching Sponzey Cabinet desktop app") ||
    desktopShellText.includes("exec scripts/run_desktop_app.sh") ||
    !desktopShellText.includes('mode="${1:-smoke}"') ||
    !desktopShellText.includes("--shell-smoke")
  ) {
    return failed(
      Phase009DesktopLaunchErrorCode.ShellRunnerAmbiguous,
      "scripts/run_desktop_shell.sh",
    );
  }

  let packageJson;
  try {
    packageJson = JSON.parse(packageJsonText);
  } catch {
    return failed(Phase009DesktopLaunchErrorCode.PackageScriptAmbiguous, "package.json");
  }

  const scripts = packageJson.scripts ?? {};
  if (
    scripts["run:desktop"] !== "sh scripts/run_desktop_app.sh" ||
    scripts["run:desktop-app"] !== "sh scripts/run_desktop_app.sh" ||
    scripts["run:desktop-shell-smoke"] !== "sh scripts/run_desktop_shell.sh"
  ) {
    return failed(Phase009DesktopLaunchErrorCode.PackageScriptAmbiguous, "package.json scripts");
  }

  return {
    passed: true,
    productLauncher: "scripts/run_desktop_app.sh",
    internalShellRunner: "scripts/run_desktop_shell.sh",
  };
}

export function validateDesktopUiMarkers({ indexHtmlText, appBundleText }) {
  if (!indexHtmlText.includes('id="app"')) {
    return failed(Phase009DesktopLaunchErrorCode.BlankScreenDetected, "#app");
  }
  if (appBundleText.includes('data-cabinet-bootstrap-state="failed"')) {
    return failed(
      Phase009DesktopLaunchErrorCode.UiHydrationFailed,
      'data-cabinet-bootstrap-state="failed"',
    );
  }

  for (const marker of requiredUiMarkers) {
    if (!appBundleText.includes(marker.value)) {
      return failed(marker.errorCode, marker.id);
    }
  }

  if (!appBundleText.includes('cabinetEditor = "mounted"')) {
    return failed(Phase009DesktopLaunchErrorCode.UiHydrationFailed, "editor mounted state");
  }

  return {
    passed: true,
    markerCount: requiredUiMarkers.length,
    markers: requiredUiMarkers.map((marker) => marker.id),
  };
}

export async function runPhase009DesktopLaunchGate({
  root = process.cwd(),
  writeArtifact = true,
} = {}) {
  let state = transitionPhase009DesktopLaunchState(
    Phase009DesktopLaunchState.NotStarted,
    Phase009DesktopLaunchEvent.Start,
  ).state;

  try {
    const inventoryText = await readFile(
      join(root, ".tasks", "phase009-current-implementation-inventory.md"),
      "utf8",
    );
    if (!inventoryText.includes("phase009_current_inventory=passed")) {
      return toFailedResult(state, {
        errorCode: Phase009DesktopLaunchErrorCode.CurrentInventoryMarkerMissing,
        findingId: ".tasks/phase009-current-implementation-inventory.md",
      });
    }

    const planValidationText = await readFile(
      join(root, ".tasks", "phase009-plan-validation-result.md"),
      "utf8",
    );
    if (!planValidationText.includes("phase009_plan_validation=passed")) {
      return toFailedResult(state, {
        errorCode: Phase009DesktopLaunchErrorCode.PlanValidationMarkerMissing,
        findingId: ".tasks/phase009-plan-validation-result.md",
      });
    }

    const desktopAppLauncherText = await readFile(
      join(root, "scripts", "run_desktop_app.sh"),
      "utf8",
    );
    const desktopShellText = await readFile(
      join(root, "scripts", "run_desktop_shell.sh"),
      "utf8",
    );
    const packageJsonText = await readFile(join(root, "package.json"), "utf8");
    const indexHtmlText = await readFile(
      join(root, "apps", "desktop", "dist", "index.html"),
      "utf8",
    );
    const appBundleText = await readFile(
      join(root, "apps", "desktop", "dist", "app.bundle.js"),
      "utf8",
    );

    const launchEvidence = validateDesktopLaunchEvidence({
      desktopAppLauncherText,
      desktopShellText,
      packageJsonText,
    });
    if (!launchEvidence.passed) {
      return toFailedResult(state, launchEvidence);
    }

    state = transitionPhase009DesktopLaunchState(
      state,
      Phase009DesktopLaunchEvent.AssetsPrepared,
    ).state;
    state = transitionPhase009DesktopLaunchState(
      state,
      Phase009DesktopLaunchEvent.UiServerStarted,
    ).state;
    state = transitionPhase009DesktopLaunchState(
      state,
      Phase009DesktopLaunchEvent.UiServerReachable,
    ).state;
    state = transitionPhase009DesktopLaunchState(
      state,
      Phase009DesktopLaunchEvent.TauriShellStarted,
    ).state;

    const uiMarkers = validateDesktopUiMarkers({ indexHtmlText, appBundleText });
    if (!uiMarkers.passed) {
      return toFailedResult(state, uiMarkers);
    }

    state = transitionPhase009DesktopLaunchState(
      state,
      Phase009DesktopLaunchEvent.HydrationCompleted,
    ).state;

    const result = {
      passed: true,
      state,
      productLauncher: launchEvidence.productLauncher,
      internalShellRunner: launchEvidence.internalShellRunner,
      markerCount: uiMarkers.markerCount,
      productLogEvents,
      failureCodes,
    };

    if (writeArtifact) {
      await mkdir(join(root, ".tasks"), { recursive: true });
      await writeFile(
        join(root, ".tasks", "phase009-desktop-launch-gate-result.md"),
        renderPhase009DesktopLaunchGateArtifact(result),
      );
    }

    return result;
  } catch (error) {
    return toFailedResult(state, {
      errorCode: Phase009DesktopLaunchErrorCode.IoFailed,
      findingId: error.path ?? error.message,
    });
  }
}

export function renderPhase009DesktopLaunchGateArtifact(result) {
  const marker = result.passed
    ? "phase009_desktop_launch_gate=passed"
    : "phase009_desktop_launch_gate=failed";

  const lines = [
    "# Phase 009 Desktop Launch Gate Result",
    "",
    marker,
    `validation_state=${result.state}`,
    "",
    "- phase: `Phase 009.1`",
    "- gate: `Desktop Launch and Non-Blank Render Gate`",
    `- status: \`${result.passed ? "passed" : "failed"}\``,
  ];

  if (!result.passed) {
    lines.push(`- error code: \`${result.errorCode}\``, `- finding id: \`${result.findingId}\``);
  }

  lines.push(
    `- product app command: \`npm run run:desktop-app\``,
    `- product launcher: \`${result.productLauncher ?? "scripts/run_desktop_app.sh"}\``,
    `- internal command smoke: \`npm run run:desktop-shell-smoke\``,
    `- internal shell runner: \`${result.internalShellRunner ?? "scripts/run_desktop_shell.sh"}\``,
    `- UI marker count: \`${result.markerCount ?? 0}\``,
    "- prerequisites:",
    "  - `.tasks/phase009-current-implementation-inventory.md` with `phase009_current_inventory=passed`",
    "  - `.tasks/phase009-plan-validation-result.md` with `phase009_plan_validation=passed`",
    "- validation commands:",
    "  - `sh -n scripts/run_desktop_app.sh scripts/run_web_app.sh scripts/run_desktop_shell.sh`",
    "  - `node --check scripts/run_web_app.mjs scripts/phase009_desktop_launch_gate.mjs scripts/phase009_desktop_launch_gate_tests.mjs`",
    "  - `node --test scripts/desktop_app_launcher_tests.mjs`",
    "  - `npm run run:desktop-dist-browser-smoke`",
    "  - `npm run run:phase009-desktop-launch-gate-tests`",
    "  - `npm run run:phase009-desktop-launch-gate`",
    "- state machine:",
    "  - `PreparingAssets -> StartingUiServer -> WaitingForUiServer -> StartingTauriShell -> HydratingReactApp -> Ready`",
    "- Product Log events:",
    ...productLogEvents.map((eventName) => `  - \`${eventName}\``),
    "- failure codes:",
    ...failureCodes.map((errorCode) => `  - \`${errorCode}\``),
    "- Field Debug summary: devUrl reachability, sanitized port, bootstrap marker, and asset presence may be recorded only in bounded local diagnostics.",
    "- Development Log summary: launcher and smoke failure details remain test/development diagnostics only.",
    "- sensitive data exclusion: this artifact records marker names, command names, state names, counts, and stable error codes only. It does not record raw document body, asset content, AI prompt, AI answer, provider key, token, credential, secret, or personal absolute path.",
    "",
  );

  return lines.join("\n");
}

function toFailedResult(state, detail) {
  const failed = transitionPhase009DesktopLaunchState(
    state,
    Phase009DesktopLaunchEvent.LaunchFailed,
    detail,
  );
  return {
    passed: false,
    state: failed.state,
    errorCode: failed.errorCode,
    findingId: failed.findingId,
    productLogEvents,
    failureCodes,
  };
}

function failed(errorCode, findingId) {
  return {
    passed: false,
    errorCode,
    findingId,
  };
}

async function main() {
  const result = await runPhase009DesktopLaunchGate({ root: process.cwd(), writeArtifact: true });
  if (!result.passed) {
    console.error(`${result.errorCode} ${result.findingId ?? ""}`.trim());
    process.exit(1);
  }
  console.log("phase009_desktop_launch_gate=passed");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    console.error(error);
    process.exit(1);
  });
}
