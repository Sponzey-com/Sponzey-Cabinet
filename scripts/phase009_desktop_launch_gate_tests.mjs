import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  Phase009DesktopLaunchErrorCode,
  Phase009DesktopLaunchEvent,
  Phase009DesktopLaunchState,
  renderPhase009DesktopLaunchGateArtifact,
  runPhase009DesktopLaunchGate,
  transitionPhase009DesktopLaunchState,
  validateDesktopLaunchEvidence,
  validateDesktopUiMarkers,
} from "./phase009_desktop_launch_gate.mjs";

test("desktop launch gate rejects shell smoke script when it is still the default product UI launcher", () => {
  const result = validateDesktopLaunchEvidence({
    desktopAppLauncherText: validDesktopAppLauncherText(),
    desktopShellText: [
      "#!/usr/bin/env sh",
      "set -eu",
      'mode="${1:-app}"',
      "case \"$mode\" in",
      "  app)",
      "    exec scripts/run_desktop_app.sh",
      "    ;;",
      "esac",
      "",
    ].join("\n"),
    packageJsonText: packageJsonText({ runDesktop: "sh scripts/run_desktop_shell.sh" }),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase009DesktopLaunchErrorCode.ShellRunnerAmbiguous);
  assert.equal(result.findingId, "scripts/run_desktop_shell.sh");
});

test("desktop launch gate validates product launcher, internal shell smoke runner, and package scripts", () => {
  const result = validateDesktopLaunchEvidence({
    desktopAppLauncherText: validDesktopAppLauncherText(),
    desktopShellText: validDesktopShellText(),
    packageJsonText: packageJsonText(),
  });

  assert.equal(result.passed, true);
  assert.equal(result.productLauncher, "scripts/run_desktop_app.sh");
  assert.equal(result.internalShellRunner, "scripts/run_desktop_shell.sh");
});

test("desktop UI marker validation rejects blank app root and missing hydration markers", () => {
  const result = validateDesktopUiMarkers({
    indexHtmlText: "<!doctype html><div id=\"app\"></div>",
    appBundleText: "function render(){ app.innerHTML = ``; }",
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase009DesktopLaunchErrorCode.BlankScreenDetected);
  assert.equal(result.findingId, "data-cabinet-app-root");
});

test("desktop UI marker validation passes only with app, workspace, editor, and bootstrap markers", () => {
  const result = validateDesktopUiMarkers({
    indexHtmlText: "<!doctype html><div id=\"app\" data-cabinet-bootstrap-state=\"loading\"></div>",
    appBundleText: [
      '<main data-cabinet-app-root="mounted" data-cabinet-bootstrap-state="ready">',
      '<section data-cabinet-workspace-shell="ready">',
      '<div id="body-editor" data-cabinet-editor="mounting"></div>',
      'parent.dataset.cabinetEditor = "mounted";',
    ].join("\n"),
  });

  assert.equal(result.passed, true);
  assert.equal(result.markerCount, 4);
});

test("desktop launch state machine exposes success, failure, and invalid transition", () => {
  const preparing = transitionPhase009DesktopLaunchState(
    Phase009DesktopLaunchState.NotStarted,
    Phase009DesktopLaunchEvent.Start,
  );
  const startingServer = transitionPhase009DesktopLaunchState(
    preparing.state,
    Phase009DesktopLaunchEvent.AssetsPrepared,
  );
  const waitingServer = transitionPhase009DesktopLaunchState(
    startingServer.state,
    Phase009DesktopLaunchEvent.UiServerStarted,
  );
  const startingTauri = transitionPhase009DesktopLaunchState(
    waitingServer.state,
    Phase009DesktopLaunchEvent.UiServerReachable,
  );
  const hydrating = transitionPhase009DesktopLaunchState(
    startingTauri.state,
    Phase009DesktopLaunchEvent.TauriShellStarted,
  );
  const ready = transitionPhase009DesktopLaunchState(
    hydrating.state,
    Phase009DesktopLaunchEvent.HydrationCompleted,
  );
  const failed = transitionPhase009DesktopLaunchState(
    Phase009DesktopLaunchState.WaitingForUiServer,
    Phase009DesktopLaunchEvent.LaunchFailed,
    { errorCode: Phase009DesktopLaunchErrorCode.UiServerUnavailable },
  );
  const invalid = transitionPhase009DesktopLaunchState(
    Phase009DesktopLaunchState.NotStarted,
    Phase009DesktopLaunchEvent.HydrationCompleted,
  );

  assert.equal(preparing.state, Phase009DesktopLaunchState.PreparingAssets);
  assert.equal(startingServer.state, Phase009DesktopLaunchState.StartingUiServer);
  assert.equal(waitingServer.state, Phase009DesktopLaunchState.WaitingForUiServer);
  assert.equal(startingTauri.state, Phase009DesktopLaunchState.StartingTauriShell);
  assert.equal(hydrating.state, Phase009DesktopLaunchState.HydratingReactApp);
  assert.equal(ready.state, Phase009DesktopLaunchState.Ready);
  assert.equal(failed.state, Phase009DesktopLaunchState.Failed);
  assert.equal(failed.errorCode, Phase009DesktopLaunchErrorCode.UiServerUnavailable);
  assert.equal(invalid.errorCode, Phase009DesktopLaunchErrorCode.InvalidTransition);
});

test("desktop launch gate passes fixture and renders safe artifact", async () => {
  const root = await createDesktopLaunchFixtureRoot();

  const result = await runPhase009DesktopLaunchGate({ root, writeArtifact: false });
  const artifact = renderPhase009DesktopLaunchGateArtifact(result);

  assert.equal(result.passed, true);
  assert.equal(result.state, Phase009DesktopLaunchState.Ready);
  assert.match(artifact, /phase009_desktop_launch_gate=passed/);
  assert.match(artifact, /desktop.launch.started/);
  assert.match(artifact, /desktop.launch.ready/);
  assert.match(artifact, /DESKTOP_BLANK_SCREEN_DETECTED/);
  assert.match(artifact, /sensitive data exclusion/);
  assert.doesNotMatch(artifact, /raw_document_body_fixture/);
  assert.doesNotMatch(artifact, /provider_api_key_fixture/);
  assert.doesNotMatch(artifact, /personal_absolute_path_fixture/);
});

test("desktop launch gate writes marker artifact to explicit root", async () => {
  const root = await createDesktopLaunchFixtureRoot();

  const result = await runPhase009DesktopLaunchGate({ root, writeArtifact: true });
  const written = await readFile(join(root, ".tasks", "phase009-desktop-launch-gate-result.md"), "utf8");

  assert.equal(result.passed, true);
  assert.match(written, /phase009_desktop_launch_gate=passed/);
  assert.match(written, /validation_state=Ready/);
});

async function createDesktopLaunchFixtureRoot() {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase009-desktop-launch-"));
  await mkdir(join(root, ".tasks"), { recursive: true });
  await mkdir(join(root, "scripts"), { recursive: true });
  await mkdir(join(root, "apps", "desktop", "dist"), { recursive: true });

  await writeFile(
    join(root, ".tasks", "phase009-current-implementation-inventory.md"),
    "phase009_current_inventory=passed\n",
  );
  await writeFile(
    join(root, ".tasks", "phase009-plan-validation-result.md"),
    "phase009_plan_validation=passed\n",
  );
  await writeFile(join(root, "scripts", "run_desktop_app.sh"), validDesktopAppLauncherText());
  await writeFile(join(root, "scripts", "run_desktop_shell.sh"), validDesktopShellText());
  await writeFile(join(root, "package.json"), packageJsonText());
  await writeFile(
    join(root, "apps", "desktop", "dist", "index.html"),
    "<!doctype html><div id=\"app\" data-cabinet-bootstrap-state=\"loading\"></div>",
  );
  await writeFile(
    join(root, "apps", "desktop", "dist", "app.bundle.js"),
    [
      '<main data-cabinet-app-root="mounted" data-cabinet-bootstrap-state="ready">',
      '<section data-cabinet-workspace-shell="ready">',
      '<div id="body-editor" data-cabinet-editor="mounting"></div>',
      'parent.dataset.cabinetEditor = "mounted";',
    ].join("\n"),
  );

  return root;
}

function validDesktopAppLauncherText() {
  return [
    "#!/usr/bin/env sh",
    "set -eu",
    'dev_port="5173"',
    "node scripts/build_desktop_assets.mjs",
    "SPONZEY_CABINET_WEB_PUBLIC_DIR=apps/desktop/dist \\",
    "SPONZEY_CABINET_RUNNER_ANNOUNCED=1 \\",
    "SPONZEY_CABINET_REQUIRE_EXACT_PORT=1 \\",
    'node scripts/run_web_app.mjs "$dev_port" &',
    'web_pid="$!"',
    "cleanup() {",
    '  kill "$web_pid" 2>/dev/null || true',
    "}",
    "trap cleanup EXIT INT TERM",
    "waitForServer",
    "cargo run -p cabinet-desktop-shell",
    "",
  ].join("\n");
}

function validDesktopShellText() {
  return [
    "#!/usr/bin/env sh",
    "set -eu",
    'mode="${1:-smoke}"',
    "case \"$mode\" in",
    "  smoke)",
    "    echo \"Running Sponzey Cabinet internal desktop shell smoke...\"",
    '    command="${2:-open_workspace}"',
    '    cargo run --quiet -p cabinet-desktop-shell -- --shell-smoke "$command"',
    "    ;;",
    "  web)",
    "    echo \"Starting Sponzey Cabinet browser preview...\"",
    "    exec scripts/run_web_app.sh \"${SPONZEY_CABINET_DESKTOP_PORT:-5174}\"",
    "    ;;",
    "esac",
    "",
  ].join("\n");
}

function packageJsonText({ runDesktop = "sh scripts/run_desktop_app.sh" } = {}) {
  return JSON.stringify(
    {
      scripts: {
        "run:desktop": runDesktop,
        "run:desktop-app": "sh scripts/run_desktop_app.sh",
        "run:desktop-shell-smoke": "sh scripts/run_desktop_shell.sh",
        "run:desktop-dist-browser-smoke": "sh scripts/run_desktop_dist_browser_smoke.sh",
      },
    },
    null,
    2,
  );
}
