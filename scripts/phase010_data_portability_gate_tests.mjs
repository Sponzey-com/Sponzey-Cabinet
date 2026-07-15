import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  Phase010DataPortabilityErrorCode,
  Phase010DataPortabilityEvent,
  Phase010DataPortabilityState,
  buildPhase010DataPortabilityCommandPlan,
  buildPhase010DataPortabilityManifest,
  evaluatePhase010DataPortabilityGate,
  renderPhase010DataPortabilityArtifact,
  renderPhase010DataPortabilityManifest,
  runPhase010DataPortabilityGate,
  transitionPhase010DataPortabilityState,
} from "./phase010_data_portability_gate.mjs";

test("phase010 data portability gate rejects missing durable authoring prerequisite", async () => {
  const root = await createDataPortabilityFixture({
    durableAuthoringText: "phase010_durable_authoring_gate=failed\n",
  });

  const result = await runPhase010DataPortabilityGate({
    root,
    writeArtifacts: false,
    runner: passingRunner,
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010DataPortabilityErrorCode.DurableAuthoringMarkerMissing);
  assert.equal(result.findingId, ".tasks/phase010-durable-authoring-gate-result.md");
});

test("phase010 data portability gate rejects failed package command", () => {
  const result = evaluatePhase010DataPortabilityGate({
    durableAuthoringText: "phase010_durable_authoring_gate=passed",
    commandResults: {
      ...passingCommandResults(),
      backupUsecases: { passed: false, exitCode: 101, command: "cargo backup usecases" },
    },
    manifest: passingManifest(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010DataPortabilityErrorCode.PackageTestsFailed);
  assert.equal(result.failedStepId, "backupUsecases");
});

test("phase010 data portability gate rejects failed ui command", () => {
  const result = evaluatePhase010DataPortabilityGate({
    durableAuthoringText: "phase010_durable_authoring_gate=passed",
    commandResults: {
      ...passingCommandResults(),
      uiPortabilityModels: { passed: false, exitCode: 1, command: "node ui tests" },
    },
    manifest: passingManifest(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010DataPortabilityErrorCode.UiTestsFailed);
  assert.equal(result.failedStepId, "uiPortabilityModels");
});

test("phase010 data portability gate rejects failed security scan command", () => {
  const result = evaluatePhase010DataPortabilityGate({
    durableAuthoringText: "phase010_durable_authoring_gate=passed",
    commandResults: {
      ...passingCommandResults(),
      securityScan: { passed: false, exitCode: 1, command: "security scan" },
    },
    manifest: passingManifest(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010DataPortabilityErrorCode.SecurityScanFailed);
  assert.equal(result.failedStepId, "securityScan");
});

test("phase010 data portability gate rejects unsafe manifest content", () => {
  const result = evaluatePhase010DataPortabilityGate({
    durableAuthoringText: "phase010_durable_authoring_gate=passed",
    commandResults: passingCommandResults(),
    manifest: {
      ...passingManifest(),
      safeWarningIds: ["raw_document_body_fixture"],
    },
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010DataPortabilityErrorCode.ManifestUnsafeContent);
  assert.equal(result.findingId, "raw_document_body_fixture");
});

test("phase010 data portability command plan is personal local desktop only", () => {
  const steps = buildPhase010DataPortabilityCommandPlan();

  assert.deepEqual(
    steps.map((step) => step.id),
    [
      "exportMarkdown",
      "importMarkdownFolder",
      "backupUsecases",
      "localBackupStore",
      "uiPortabilityModels",
      "desktopPortabilitySmoke",
      "securityScan",
    ],
  );
  assert.ok(steps.every((step) => !step.command.join(" ").includes("self-host")));
  assert.ok(steps.every((step) => !step.command.join(" ").includes("mobile")));
  assert.ok(steps.every((step) => !step.command.join(" ").includes("server-base-url")));
});

test("phase010 data portability gate passes complete evidence and renders safe artifacts", () => {
  const manifest = passingManifest();
  const result = evaluatePhase010DataPortabilityGate({
    durableAuthoringText: "phase010_durable_authoring_gate=passed",
    commandResults: passingCommandResults(),
    manifest,
  });
  const artifact = renderPhase010DataPortabilityArtifact(result);
  const manifestJson = renderPhase010DataPortabilityManifest(
    buildPhase010DataPortabilityManifest(manifest),
  );

  assert.equal(result.passed, true);
  assert.equal(result.state, Phase010DataPortabilityState.Passed);
  assert.match(artifact, /phase010_data_portability_gate=passed/);
  assert.match(artifact, /import_preview_no_mutation=verified/);
  assert.match(manifestJson, /phase010_data_portability_manifest=passed/);
  assert.match(manifestJson, /personal_local_desktop/);
  assert.doesNotMatch(artifact, /raw_document_body_fixture/);
  assert.doesNotMatch(manifestJson, /provider_api_key_fixture/);
  assert.doesNotMatch(manifestJson, /personal_absolute_path_fixture/);
});

test("phase010 data portability gate writes marker and manifest artifacts", async () => {
  const root = await createDataPortabilityFixture();

  const result = await runPhase010DataPortabilityGate({
    root,
    writeArtifacts: true,
    runner: passingRunner,
  });
  const marker = await readFile(
    join(root, ".tasks", "phase010-data-portability-gate-result.md"),
    "utf8",
  );
  const manifest = await readFile(
    join(root, ".tasks", "release", "data-portability-manifest-phase010.json"),
    "utf8",
  );

  assert.equal(result.passed, true);
  assert.match(marker, /phase010_data_portability_gate=passed/);
  assert.match(manifest, /phase010_data_portability_manifest=passed/);
});

test("phase010 data portability state machine exposes success, failure, and invalid transition", () => {
  const reading = transitionPhase010DataPortabilityState(
    Phase010DataPortabilityState.Pending,
    Phase010DataPortabilityEvent.Start,
  );
  const runningPackage = transitionPhase010DataPortabilityState(
    reading.state,
    Phase010DataPortabilityEvent.PrerequisitesRead,
  );
  const runningUi = transitionPhase010DataPortabilityState(
    runningPackage.state,
    Phase010DataPortabilityEvent.PackageTestsPassed,
  );
  const runningSecurity = transitionPhase010DataPortabilityState(
    runningUi.state,
    Phase010DataPortabilityEvent.UiTestsPassed,
  );
  const writingManifest = transitionPhase010DataPortabilityState(
    runningSecurity.state,
    Phase010DataPortabilityEvent.SecurityScanPassed,
  );
  const writingResult = transitionPhase010DataPortabilityState(
    writingManifest.state,
    Phase010DataPortabilityEvent.ManifestWritten,
  );
  const passed = transitionPhase010DataPortabilityState(
    writingResult.state,
    Phase010DataPortabilityEvent.ResultWritten,
  );
  const failed = transitionPhase010DataPortabilityState(reading.state, Phase010DataPortabilityEvent.Fail, {
    errorCode: Phase010DataPortabilityErrorCode.DurableAuthoringMarkerMissing,
    findingId: ".tasks/phase010-durable-authoring-gate-result.md",
  });
  const invalid = transitionPhase010DataPortabilityState(
    Phase010DataPortabilityState.Pending,
    Phase010DataPortabilityEvent.UiTestsPassed,
  );

  assert.equal(reading.state, Phase010DataPortabilityState.ReadingPrerequisites);
  assert.equal(runningPackage.state, Phase010DataPortabilityState.RunningPackageTests);
  assert.equal(runningUi.state, Phase010DataPortabilityState.RunningUiTests);
  assert.equal(runningSecurity.state, Phase010DataPortabilityState.RunningSecurityScan);
  assert.equal(writingManifest.state, Phase010DataPortabilityState.WritingManifest);
  assert.equal(writingResult.state, Phase010DataPortabilityState.WritingResult);
  assert.equal(passed.state, Phase010DataPortabilityState.Passed);
  assert.equal(failed.state, Phase010DataPortabilityState.Failed);
  assert.equal(invalid.errorCode, Phase010DataPortabilityErrorCode.InvalidTransition);
});

async function createDataPortabilityFixture({
  durableAuthoringText = "phase010_durable_authoring_gate=passed\n",
} = {}) {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase010-data-portability-"));
  await mkdir(join(root, ".tasks"), { recursive: true });
  await writeFile(join(root, ".tasks", "phase010-durable-authoring-gate-result.md"), durableAuthoringText);
  return root;
}

function passingCommandResults() {
  return Object.fromEntries(
    buildPhase010DataPortabilityCommandPlan().map((step) => [
      step.id,
      { command: step.command.join(" "), passed: true, exitCode: 0, durationMs: 5 },
    ]),
  );
}

function passingManifest() {
  return {
    schemaVersion: "phase010.data_portability.v1",
    productScope: "personal_local_desktop",
    workspaceScope: "single_user_local_workspace",
    documentCount: 12,
    versionCount: 36,
    assetMetadataCount: 4,
    graphProjectionCount: 8,
    exportFormats: ["markdown_folder", "workspace_backup_package"],
    importSources: ["markdown_folder", "obsidian_vault"],
    capabilities: {
      exportPackage: true,
      importPreviewNoMutation: true,
      backupCreate: true,
      restoreRequiresValidation: true,
    },
    safeWarningIds: ["IMPORT_CONFLICT_DETECTED", "RESTORE_REQUIRES_CONFIRMATION"],
  };
}

async function passingRunner() {
  return { exitCode: 0, signal: null, durationMs: 5 };
}
