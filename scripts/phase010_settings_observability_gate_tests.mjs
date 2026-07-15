import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  Phase010SettingsObservabilityErrorCode,
  Phase010SettingsObservabilityEvent,
  Phase010SettingsObservabilityState,
  buildPhase010ProductLogMatrix,
  buildPhase010SecurityLogManifest,
  buildPhase010SettingsObservabilityCommandPlan,
  evaluatePhase010SettingsObservabilityGate,
  renderPhase010LocalDesktopRunbook,
  renderPhase010ProductLogMatrix,
  renderPhase010SecurityLogManifest,
  renderPhase010SettingsObservabilityArtifact,
  runPhase010SettingsObservabilityGate,
  transitionPhase010SettingsObservabilityState,
} from "./phase010_settings_observability_gate.mjs";

test("phase010 settings observability gate rejects missing index health prerequisite", async () => {
  const root = await createSettingsObservabilityFixture({
    indexHealthText: "phase010_index_health_repair_gate=failed\n",
  });

  const result = await runPhase010SettingsObservabilityGate({
    root,
    writeArtifacts: false,
    runner: passingRunner,
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010SettingsObservabilityErrorCode.IndexHealthMarkerMissing);
});

test("phase010 settings observability gate rejects failed settings command", () => {
  const result = evaluatePhase010SettingsObservabilityGate({
    indexHealthText: "phase010_index_health_repair_gate=passed",
    commandResults: {
      ...passingCommandResults(),
      settingsUiModels: { passed: false, exitCode: 1, command: "node settings" },
    },
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010SettingsObservabilityErrorCode.SettingsTestsFailed);
  assert.equal(result.failedStepId, "settingsUiModels");
});

test("phase010 settings observability gate rejects failed AI and Field Debug commands", () => {
  const failedAi = evaluatePhase010SettingsObservabilityGate({
    indexHealthText: "phase010_index_health_repair_gate=passed",
    commandResults: {
      ...passingCommandResults(),
      aiProviderModels: { passed: false, exitCode: 1, command: "node ai" },
    },
  });
  const failedFieldDebug = evaluatePhase010SettingsObservabilityGate({
    indexHealthText: "phase010_index_health_repair_gate=passed",
    commandResults: {
      ...passingCommandResults(),
      fieldDebugUsecases: { passed: false, exitCode: 101, command: "cargo field debug" },
    },
  });

  assert.equal(failedAi.errorCode, Phase010SettingsObservabilityErrorCode.AiTestsFailed);
  assert.equal(failedAi.failedStepId, "aiProviderModels");
  assert.equal(
    failedFieldDebug.errorCode,
    Phase010SettingsObservabilityErrorCode.FieldDebugTestsFailed,
  );
  assert.equal(failedFieldDebug.failedStepId, "fieldDebugUsecases");
});

test("phase010 settings observability gate rejects failed self-check and unsafe artifacts", () => {
  const failedSelfCheck = evaluatePhase010SettingsObservabilityGate({
    indexHealthText: "phase010_index_health_repair_gate=passed",
    commandResults: {
      ...passingCommandResults(),
      settingsObservabilitySelfCheck: { passed: false, exitCode: 1, command: "node self check" },
    },
  });
  const unsafe = evaluatePhase010SettingsObservabilityGate({
    indexHealthText: "phase010_index_health_repair_gate=passed",
    commandResults: passingCommandResults(),
    artifacts: {
      productLogMatrixText: "provider_api_key_fixture",
      securityManifestText: renderPhase010SecurityLogManifest(buildPhase010SecurityLogManifest()),
      runbookText: renderPhase010LocalDesktopRunbook(),
    },
  });

  assert.equal(
    failedSelfCheck.errorCode,
    Phase010SettingsObservabilityErrorCode.SelfCheckFailed,
  );
  assert.equal(unsafe.errorCode, Phase010SettingsObservabilityErrorCode.UnsafeArtifactContent);
  assert.equal(unsafe.findingId, "provider_api_key_fixture");
});

test("phase010 settings observability command plan is current-scope local desktop only", () => {
  const steps = buildPhase010SettingsObservabilityCommandPlan();

  assert.deepEqual(
    steps.map((step) => step.id),
    [
      "settingsUiModels",
      "desktopSettingsSmoke",
      "aiProviderModels",
      "aiPromptBuilder",
      "aiSummaryUsecases",
      "aiUsecases",
      "fieldDebugUsecases",
      "settingsObservabilitySelfCheck",
    ],
  );
  assert.ok(steps.every((step) => !step.command.join(" ").includes("self-host")));
  assert.ok(steps.every((step) => !step.command.join(" ").includes("mobile")));
  assert.ok(steps.every((step) => !step.command.join(" ").includes("server-base-url")));
  assert.ok(steps.every((step) => !step.command.join(" ").includes("admin")));
});

test("phase010 settings observability artifacts render required markers and safe sections", () => {
  const result = evaluatePhase010SettingsObservabilityGate({
    indexHealthText: "phase010_index_health_repair_gate=passed",
    commandResults: passingCommandResults(),
  });
  const artifact = renderPhase010SettingsObservabilityArtifact(result);
  const matrix = renderPhase010ProductLogMatrix(buildPhase010ProductLogMatrix());
  const manifest = renderPhase010SecurityLogManifest(buildPhase010SecurityLogManifest());
  const runbook = renderPhase010LocalDesktopRunbook();

  assert.equal(result.passed, true);
  assert.equal(result.state, Phase010SettingsObservabilityState.Passed);
  assert.match(artifact, /phase010_settings_observability_gate=passed/);
  assert.match(matrix, /phase010_product_log_matrix=passed/);
  assert.match(manifest, /phase010_security_log_manifest=passed/);
  assert.match(runbook, /phase010_runbook=passed/);
  assert.match(runbook, /Clean Install/);
  assert.match(runbook, /Packaged Launch/);
  assert.match(runbook, /Reinstall Preservation/);
  assert.match(runbook, /Blank Screen Recovery/);
  assert.match(runbook, /Index Repair/);
  assert.match(runbook, /Export Import/);
  assert.match(runbook, /Backup Restore/);
  assert.match(runbook, /Field Debug/);
  assert.match(runbook, /Data Export/);
  for (const text of [artifact, matrix, manifest, runbook]) {
    assert.doesNotMatch(text, /provider_api_key_fixture/);
    assert.doesNotMatch(text, /raw_document_body_fixture/);
    assert.doesNotMatch(text, /personal_absolute_path_fixture/);
    assert.doesNotMatch(text, /C:\\Users\\/);
  }
});

test("phase010 settings observability gate writes marker and release artifacts", async () => {
  const root = await createSettingsObservabilityFixture();

  const result = await runPhase010SettingsObservabilityGate({
    root,
    writeArtifacts: true,
    runner: passingRunner,
  });
  const marker = await readFile(
    join(root, ".tasks", "phase010-settings-observability-gate-result.md"),
    "utf8",
  );
  const matrix = await readFile(
    join(root, ".tasks", "release", "product-log-event-matrix-phase010.md"),
    "utf8",
  );
  const manifest = await readFile(
    join(root, ".tasks", "release", "security-log-policy-manifest-phase010.json"),
    "utf8",
  );
  const runbook = await readFile(
    join(root, ".tasks", "release", "local-desktop-runbook-phase010.md"),
    "utf8",
  );

  assert.equal(result.passed, true);
  assert.match(marker, /phase010_settings_observability_gate=passed/);
  assert.match(matrix, /phase010_product_log_matrix=passed/);
  assert.match(manifest, /phase010_security_log_manifest=passed/);
  assert.match(runbook, /phase010_runbook=passed/);
});

test("phase010 settings observability state machine exposes success, failure, and invalid transition", () => {
  const reading = transitionPhase010SettingsObservabilityState(
    Phase010SettingsObservabilityState.Pending,
    Phase010SettingsObservabilityEvent.Start,
  );
  const runningSettings = transitionPhase010SettingsObservabilityState(
    reading.state,
    Phase010SettingsObservabilityEvent.PrerequisitesRead,
  );
  const runningAi = transitionPhase010SettingsObservabilityState(
    runningSettings.state,
    Phase010SettingsObservabilityEvent.SettingsTestsPassed,
  );
  const runningFieldDebug = transitionPhase010SettingsObservabilityState(
    runningAi.state,
    Phase010SettingsObservabilityEvent.AiTestsPassed,
  );
  const writingArtifacts = transitionPhase010SettingsObservabilityState(
    runningFieldDebug.state,
    Phase010SettingsObservabilityEvent.FieldDebugTestsPassed,
  );
  const runningSelfCheck = transitionPhase010SettingsObservabilityState(
    writingArtifacts.state,
    Phase010SettingsObservabilityEvent.ArtifactsValidated,
  );
  const writingResult = transitionPhase010SettingsObservabilityState(
    runningSelfCheck.state,
    Phase010SettingsObservabilityEvent.SelfCheckPassed,
  );
  const passed = transitionPhase010SettingsObservabilityState(
    writingResult.state,
    Phase010SettingsObservabilityEvent.ResultWritten,
  );
  const invalid = transitionPhase010SettingsObservabilityState(
    Phase010SettingsObservabilityState.Pending,
    Phase010SettingsObservabilityEvent.AiTestsPassed,
  );

  assert.equal(reading.state, Phase010SettingsObservabilityState.ReadingPrerequisites);
  assert.equal(runningSettings.state, Phase010SettingsObservabilityState.RunningSettingsTests);
  assert.equal(runningAi.state, Phase010SettingsObservabilityState.RunningAiTests);
  assert.equal(runningFieldDebug.state, Phase010SettingsObservabilityState.RunningFieldDebugTests);
  assert.equal(writingArtifacts.state, Phase010SettingsObservabilityState.ValidatingArtifacts);
  assert.equal(runningSelfCheck.state, Phase010SettingsObservabilityState.RunningSelfCheck);
  assert.equal(writingResult.state, Phase010SettingsObservabilityState.WritingResult);
  assert.equal(passed.state, Phase010SettingsObservabilityState.Passed);
  assert.equal(invalid.errorCode, Phase010SettingsObservabilityErrorCode.InvalidTransition);
});

async function createSettingsObservabilityFixture({
  indexHealthText = "phase010_index_health_repair_gate=passed\n",
} = {}) {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase010-settings-observability-"));
  await mkdir(join(root, ".tasks"), { recursive: true });
  await writeFile(join(root, ".tasks", "phase010-index-health-repair-gate-result.md"), indexHealthText);
  return root;
}

function passingCommandResults() {
  return Object.fromEntries(
    buildPhase010SettingsObservabilityCommandPlan().map((step) => [
      step.id,
      { command: step.command.join(" "), passed: true, exitCode: 0, durationMs: 5 },
    ]),
  );
}

async function passingRunner() {
  return { exitCode: 0, signal: null, durationMs: 5 };
}
