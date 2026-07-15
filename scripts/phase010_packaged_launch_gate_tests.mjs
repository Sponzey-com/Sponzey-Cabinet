import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  Phase010PackagedLaunchErrorCode,
  Phase010PackagedLaunchEvent,
  Phase010PackagedLaunchState,
  buildPhase010PackagedLaunchCommandPlan,
  renderPhase010PackagedLaunchArtifact,
  renderPhase010PackagedRuntimeManifest,
  runPhase010PackagedLaunchGate,
  transitionPhase010PackagedLaunchState,
  validatePhase010PackagedLaunchSources,
} from "./phase010_packaged_launch_gate.mjs";

test("phase010 packaged launch gate rejects missing plan validation marker", async () => {
  const root = await createPackagedLaunchFixture({
    planMarkerText: "phase010_plan_validation=failed\n",
  });

  const result = await runPhase010PackagedLaunchGate({
    root,
    writeArtifact: false,
    runner: passingRunner,
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010PackagedLaunchErrorCode.PlanValidationMarkerMissing);
  assert.equal(result.findingId, ".tasks/phase010-plan-validation-result.md");
});

test("phase010 packaged launch source validation rejects dev server dependency", () => {
  const sources = completeSources({
    packageSmokeText: "node scripts/run_web_app.mjs 5173\n",
  });

  const result = validatePhase010PackagedLaunchSources(sources);

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010PackagedLaunchErrorCode.DevServerDependencyDetected);
  assert.equal(result.findingId, "scripts/run_desktop_package_smoke.sh");
});

test("phase010 packaged launch source validation rejects installed runtime Node requirement", () => {
  const sources = completeSources({
    packageSmokeText: "node scripts/run_web_app.mjs 5173\nnode runtime required for installed app\n",
  });

  const result = validatePhase010PackagedLaunchSources(sources);

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010PackagedLaunchErrorCode.DevServerDependencyDetected);
});

test("phase010 packaged launch command plan is active-scope desktop only", () => {
  const steps = buildPhase010PackagedLaunchCommandPlan();

  assert.deepEqual(
    steps.map((step) => step.id),
    ["desktop_package_smoke", "desktop_packaged_app_smoke"],
  );
  assert.ok(steps.every((step) => step.command.join(" ") !== "npm run run:web"));
  assert.ok(steps.every((step) => !step.command.join(" ").includes("self-host")));
});

test("phase010 packaged runtime manifest renders safe passed evidence", () => {
  const manifest = renderPhase010PackagedRuntimeManifest({
    passed: true,
    commandResults: [
      { id: "desktop_package_smoke", status: "passed", exitCode: 0 },
      { id: "desktop_packaged_app_smoke", status: "passed", exitCode: 0 },
    ],
    sourceValidation: validatePhase010PackagedLaunchSources(completeSources()),
  });

  assert.match(manifest, /"marker": "phase010_packaged_runtime_manifest=passed"/);
  assert.match(manifest, /"devServerRequired": false/);
  assert.match(manifest, /"installedNodeRuntimeRequired": false/);
  assert.doesNotMatch(manifest, /raw_document_body_fixture/);
  assert.doesNotMatch(manifest, /provider_api_key_fixture/);
  assert.doesNotMatch(manifest, /personal_absolute_path_fixture/);
});

test("phase010 packaged launch gate passes with fake command runner and writes artifacts", async () => {
  const root = await createPackagedLaunchFixture();

  const result = await runPhase010PackagedLaunchGate({
    root,
    writeArtifact: true,
    runner: passingRunner,
  });
  const gateArtifact = await readFile(
    join(root, ".tasks", "phase010-packaged-launch-gate-result.md"),
    "utf8",
  );
  const manifest = await readFile(
    join(root, ".tasks", "release", "packaged-runtime-manifest-phase010.json"),
    "utf8",
  );

  assert.equal(result.passed, true);
  assert.equal(result.state, Phase010PackagedLaunchState.Passed);
  assert.match(gateArtifact, /phase010_packaged_launch_gate=passed/);
  assert.match(manifest, /phase010_packaged_runtime_manifest=passed/);
});

test("phase010 packaged launch gate fails when package smoke command fails", async () => {
  const root = await createPackagedLaunchFixture();

  const result = await runPhase010PackagedLaunchGate({
    root,
    writeArtifact: false,
    runner: async (step) => ({
      exitCode: step.id === "desktop_package_smoke" ? 1 : 0,
      signal: null,
      durationMs: 5,
    }),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010PackagedLaunchErrorCode.PackageSmokeFailed);
  assert.equal(result.failedStepId, "desktop_package_smoke");
});

test("phase010 packaged launch artifact excludes raw command output", () => {
  const artifact = renderPhase010PackagedLaunchArtifact({
    passed: true,
    state: Phase010PackagedLaunchState.Passed,
    sourceValidation: validatePhase010PackagedLaunchSources(completeSources()),
    commandResults: [
      {
        id: "desktop_package_smoke",
        status: "passed",
        exitCode: 0,
        rawOutput: "raw_document_body_fixture provider_api_key_fixture personal_absolute_path_fixture",
      },
    ],
  });

  assert.match(artifact, /phase010_packaged_launch_gate=passed/);
  assert.doesNotMatch(artifact, /raw_document_body_fixture/);
  assert.doesNotMatch(artifact, /provider_api_key_fixture/);
  assert.doesNotMatch(artifact, /personal_absolute_path_fixture/);
});

test("phase010 packaged launch state machine exposes success, failure, and invalid transition", () => {
  const reading = transitionPhase010PackagedLaunchState(
    Phase010PackagedLaunchState.Pending,
    Phase010PackagedLaunchEvent.Start,
  );
  const validating = transitionPhase010PackagedLaunchState(
    reading.state,
    Phase010PackagedLaunchEvent.PrerequisitesRead,
  );
  const runningPackage = transitionPhase010PackagedLaunchState(
    validating.state,
    Phase010PackagedLaunchEvent.EvidenceValidated,
  );
  const runningPackagedApp = transitionPhase010PackagedLaunchState(
    runningPackage.state,
    Phase010PackagedLaunchEvent.PackageSmokePassed,
  );
  const writing = transitionPhase010PackagedLaunchState(
    runningPackagedApp.state,
    Phase010PackagedLaunchEvent.PackagedAppSmokePassed,
  );
  const passed = transitionPhase010PackagedLaunchState(
    writing.state,
    Phase010PackagedLaunchEvent.ResultWritten,
  );
  const failed = transitionPhase010PackagedLaunchState(reading.state, Phase010PackagedLaunchEvent.Fail, {
    errorCode: Phase010PackagedLaunchErrorCode.PlanValidationMarkerMissing,
    findingId: ".tasks/phase010-plan-validation-result.md",
  });
  const invalid = transitionPhase010PackagedLaunchState(
    Phase010PackagedLaunchState.Pending,
    Phase010PackagedLaunchEvent.EvidenceValidated,
  );

  assert.equal(reading.state, Phase010PackagedLaunchState.ReadingPrerequisites);
  assert.equal(validating.state, Phase010PackagedLaunchState.ValidatingEvidence);
  assert.equal(runningPackage.state, Phase010PackagedLaunchState.RunningPackageSmoke);
  assert.equal(runningPackagedApp.state, Phase010PackagedLaunchState.RunningPackagedAppSmoke);
  assert.equal(writing.state, Phase010PackagedLaunchState.WritingResult);
  assert.equal(passed.state, Phase010PackagedLaunchState.Passed);
  assert.equal(failed.state, Phase010PackagedLaunchState.Failed);
  assert.equal(invalid.errorCode, Phase010PackagedLaunchErrorCode.InvalidTransition);
});

async function createPackagedLaunchFixture({
  planMarkerText = "phase010_plan_validation=passed\n",
} = {}) {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase010-packaged-launch-"));
  await mkdir(join(root, ".tasks"), { recursive: true });
  await mkdir(join(root, "scripts"), { recursive: true });
  await writeFile(join(root, ".tasks", "phase010-plan-validation-result.md"), planMarkerText);
  const sources = completeSources();
  for (const [filePath, text] of Object.entries(sources)) {
    await writeFile(join(root, filePath), text);
  }
  return root;
}

function completeSources({
  packageSmokeText = [
    "#!/usr/bin/env sh",
    "set -eu",
    "node scripts/build_desktop_assets.mjs",
    "cargo build -p cabinet-desktop-shell",
    "target/debug/cabinet-desktop-shell --packaged-smoke",
  ].join("\n"),
  packagedAppSmokeText = [
    "#!/usr/bin/env sh",
    "set -eu",
    "scripts/run_desktop_tauri_build.sh",
    "app_binary=$(find target/debug/bundle/macos apps/desktop/src-tauri/target/debug/bundle/macos -type f | head -n 1)",
    "\"$app_binary\" --packaged-smoke",
  ].join("\n"),
  tauriBuildText = [
    "#!/usr/bin/env sh",
    "set -eu",
    "node scripts/build_desktop_assets.mjs",
    "cd apps/desktop",
    "../../node_modules/.bin/tauri build --debug --bundles app --no-sign --ci",
  ].join("\n"),
  packageJsonText = JSON.stringify({
    scripts: {
      "run:desktop-package-smoke": "sh scripts/run_desktop_package_smoke.sh",
      "run:desktop-packaged-app-smoke": "sh scripts/run_desktop_packaged_app_smoke.sh",
    },
  }),
} = {}) {
  return {
    "scripts/run_desktop_package_smoke.sh": packageSmokeText,
    "scripts/run_desktop_packaged_app_smoke.sh": packagedAppSmokeText,
    "scripts/run_desktop_tauri_build.sh": tauriBuildText,
    "package.json": packageJsonText,
  };
}

async function passingRunner() {
  return { exitCode: 0, signal: null, durationMs: 5 };
}
