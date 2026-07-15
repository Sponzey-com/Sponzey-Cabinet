import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  Phase010FirstRunWorkspaceErrorCode,
  Phase010FirstRunWorkspaceEvent,
  Phase010FirstRunWorkspaceState,
  buildPhase010FirstRunWorkspaceCommandPlan,
  evaluatePhase010FirstRunWorkspaceGate,
  renderPhase010FirstRunWorkspaceArtifact,
  runPhase010FirstRunWorkspaceGate,
  transitionPhase010FirstRunWorkspaceState,
} from "./phase010_first_run_workspace_gate.mjs";

test("phase010 first-run workspace gate rejects missing packaged launch marker", async () => {
  const root = await createFirstRunFixture({
    packagedLaunchText: "phase010_packaged_launch_gate=failed\n",
  });

  const result = await runPhase010FirstRunWorkspaceGate({
    root,
    writeArtifact: false,
    runner: passingRunner,
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010FirstRunWorkspaceErrorCode.PackagedLaunchMarkerMissing);
  assert.equal(result.findingId, ".tasks/phase010-packaged-launch-gate-result.md");
});

test("phase010 first-run workspace gate rejects failed first-run command", () => {
  const result = evaluatePhase010FirstRunWorkspaceGate({
    packagedLaunchText: "phase010_packaged_launch_gate=passed",
    commandResults: {
      ...passingCommandResults(),
      firstRunCore: { passed: false, exitCode: 101, command: "cargo test first-run" },
    },
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010FirstRunWorkspaceErrorCode.FirstRunTestsFailed);
  assert.equal(result.failedStepId, "firstRunCore");
});

test("phase010 first-run workspace gate rejects failed health command", () => {
  const result = evaluatePhase010FirstRunWorkspaceGate({
    packagedLaunchText: "phase010_packaged_launch_gate=passed",
    commandResults: {
      ...passingCommandResults(),
      setupHealth: { passed: false, exitCode: 101, command: "cargo test setup health" },
    },
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010FirstRunWorkspaceErrorCode.HealthTestsFailed);
  assert.equal(result.failedStepId, "setupHealth");
});

test("phase010 first-run workspace command plan is local desktop only", () => {
  const steps = buildPhase010FirstRunWorkspaceCommandPlan();

  assert.deepEqual(
    steps.map((step) => step.id),
    [
      "firstRunCore",
      "firstRunInitializer",
      "firstRunStore",
      "setupHealth",
      "nativeBootstrap",
      "startupRepair",
    ],
  );
  assert.ok(steps.every((step) => !step.command.join(" ").includes("self-host")));
  assert.ok(steps.every((step) => !step.command.join(" ").includes("mobile")));
});

test("phase010 first-run workspace gate passes complete evidence and renders safe marker", () => {
  const result = evaluatePhase010FirstRunWorkspaceGate({
    packagedLaunchText: "phase010_packaged_launch_gate=passed",
    commandResults: passingCommandResults(),
  });
  const artifact = renderPhase010FirstRunWorkspaceArtifact(result);

  assert.equal(result.passed, true);
  assert.equal(result.state, Phase010FirstRunWorkspaceState.Passed);
  assert.match(artifact, /phase010_first_run_workspace_gate=passed/);
  assert.match(artifact, /first_run_idempotent=true/);
  assert.match(artifact, /setup_health_status=healthy/);
  assert.match(artifact, /repair_evidence=verified/);
  assert.doesNotMatch(artifact, /raw_document_body_fixture/);
  assert.doesNotMatch(artifact, /provider_api_key_fixture/);
  assert.doesNotMatch(artifact, /personal_absolute_path_fixture/);
});

test("phase010 first-run workspace failed artifact does not claim healthy evidence", () => {
  const result = evaluatePhase010FirstRunWorkspaceGate({
    packagedLaunchText: "phase010_packaged_launch_gate=passed",
    commandResults: {
      ...passingCommandResults(),
      setupHealth: { passed: false, exitCode: 101, command: "cargo test setup health" },
    },
  });
  const artifact = renderPhase010FirstRunWorkspaceArtifact(result);

  assert.equal(result.passed, false);
  assert.match(artifact, /phase010_first_run_workspace_gate=failed/);
  assert.match(artifact, /setup_health_status=failed/);
  assert.doesNotMatch(artifact, /first_run_idempotent=true/);
  assert.doesNotMatch(artifact, /setup_health_status=healthy/);
  assert.doesNotMatch(artifact, /repair_evidence=verified/);
});

test("phase010 first-run workspace gate writes marker artifact to explicit root", async () => {
  const root = await createFirstRunFixture();

  const result = await runPhase010FirstRunWorkspaceGate({
    root,
    writeArtifact: true,
    runner: passingRunner,
  });
  const written = await readFile(
    join(root, ".tasks", "phase010-first-run-workspace-gate-result.md"),
    "utf8",
  );

  assert.equal(result.passed, true);
  assert.match(written, /phase010_first_run_workspace_gate=passed/);
  assert.match(written, /validation_state=Passed/);
});

test("phase010 first-run workspace state machine exposes success, failure, and invalid transition", () => {
  const reading = transitionPhase010FirstRunWorkspaceState(
    Phase010FirstRunWorkspaceState.Pending,
    Phase010FirstRunWorkspaceEvent.Start,
  );
  const runningFirstRun = transitionPhase010FirstRunWorkspaceState(
    reading.state,
    Phase010FirstRunWorkspaceEvent.PrerequisitesRead,
  );
  const runningHealth = transitionPhase010FirstRunWorkspaceState(
    runningFirstRun.state,
    Phase010FirstRunWorkspaceEvent.FirstRunTestsPassed,
  );
  const runningRepair = transitionPhase010FirstRunWorkspaceState(
    runningHealth.state,
    Phase010FirstRunWorkspaceEvent.HealthTestsPassed,
  );
  const writing = transitionPhase010FirstRunWorkspaceState(
    runningRepair.state,
    Phase010FirstRunWorkspaceEvent.RepairTestsPassed,
  );
  const passed = transitionPhase010FirstRunWorkspaceState(
    writing.state,
    Phase010FirstRunWorkspaceEvent.ResultWritten,
  );
  const failed = transitionPhase010FirstRunWorkspaceState(reading.state, Phase010FirstRunWorkspaceEvent.Fail, {
    errorCode: Phase010FirstRunWorkspaceErrorCode.PackagedLaunchMarkerMissing,
    findingId: ".tasks/phase010-packaged-launch-gate-result.md",
  });
  const invalid = transitionPhase010FirstRunWorkspaceState(
    Phase010FirstRunWorkspaceState.Pending,
    Phase010FirstRunWorkspaceEvent.HealthTestsPassed,
  );

  assert.equal(reading.state, Phase010FirstRunWorkspaceState.ReadingPrerequisites);
  assert.equal(runningFirstRun.state, Phase010FirstRunWorkspaceState.RunningFirstRunTests);
  assert.equal(runningHealth.state, Phase010FirstRunWorkspaceState.RunningHealthTests);
  assert.equal(runningRepair.state, Phase010FirstRunWorkspaceState.RunningRepairTests);
  assert.equal(writing.state, Phase010FirstRunWorkspaceState.WritingResult);
  assert.equal(passed.state, Phase010FirstRunWorkspaceState.Passed);
  assert.equal(failed.state, Phase010FirstRunWorkspaceState.Failed);
  assert.equal(invalid.errorCode, Phase010FirstRunWorkspaceErrorCode.InvalidTransition);
});

async function createFirstRunFixture({
  packagedLaunchText = "phase010_packaged_launch_gate=passed\n",
} = {}) {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase010-first-run-"));
  await mkdir(join(root, ".tasks"), { recursive: true });
  await writeFile(join(root, ".tasks", "phase010-packaged-launch-gate-result.md"), packagedLaunchText);
  return root;
}

function passingCommandResults() {
  return {
    firstRunCore: commandResult("cargo test -p cabinet-core --test first_run_tests"),
    firstRunInitializer: commandResult("cargo test -p cabinet-core --test first_run_initializer_tests"),
    firstRunStore: commandResult("cargo test -p cabinet-adapters --test local_first_run_store_tests"),
    setupHealth: commandResult("cargo test -p cabinet-adapters --test local_setup_health_checker_tests"),
    nativeBootstrap: commandResult("cargo test -p cabinet-platform --test local_desktop_bootstrap_state_tests"),
    startupRepair: commandResult("cargo test -p cabinet-platform --test startup_repair_smoke"),
  };
}

function commandResult(command) {
  return { command, passed: true, exitCode: 0 };
}

async function passingRunner() {
  return { exitCode: 0, signal: null, durationMs: 5 };
}
