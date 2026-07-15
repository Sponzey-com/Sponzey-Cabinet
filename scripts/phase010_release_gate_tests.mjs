import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  Phase010ReleaseGateErrorCode,
  Phase010ReleaseGateEvent,
  Phase010ReleaseGateState,
  buildPhase010ReleaseCommandPlan,
  evaluatePhase010ReleaseGate,
  renderPhase010ReleaseGateArtifact,
  runPhase010ReleaseGate,
  transitionPhase010ReleaseGateState,
} from "./phase010_release_gate.mjs";

test("phase010 release gate rejects missing lower marker", () => {
  const sources = completeSources();
  sources[".tasks/phase010-packaged-launch-gate-result.md"] =
    "phase010_packaged_launch_gate=failed";

  const result = evaluatePhase010ReleaseGate({
    sources,
    commandResults: passingCommandResults(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010ReleaseGateErrorCode.MissingMarker);
  assert.equal(result.findingId, "phase010_packaged_launch_gate");
});

test("phase010 release gate rejects missing performance budget", () => {
  const sources = completeSources();
  sources[".tasks/release/performance-budget-phase010.md"] = "phase010_performance_budget=failed";

  const result = evaluatePhase010ReleaseGate({
    sources,
    commandResults: passingCommandResults(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010ReleaseGateErrorCode.PerformanceBudgetFailed);
  assert.equal(result.findingId, "phase010_performance_budget");
});

test("phase010 release gate rejects failed packaged, security, and runbook commands", () => {
  const packaged = evaluatePhase010ReleaseGate({
    sources: completeSources(),
    commandResults: {
      ...passingCommandResults(),
      desktopPackageSmoke: { command: "desktop package smoke", passed: false, exitCode: 1 },
    },
  });
  const security = evaluatePhase010ReleaseGate({
    sources: completeSources(),
    commandResults: {
      ...passingCommandResults(),
      securityScan: { command: "security scan", passed: false, exitCode: 1 },
    },
  });
  const runbook = evaluatePhase010ReleaseGate({
    sources: completeSources(),
    commandResults: {
      ...passingCommandResults(),
      runbookValidation: { command: "runbook validation", passed: false, exitCode: 1 },
    },
  });

  assert.equal(packaged.errorCode, Phase010ReleaseGateErrorCode.PackagedSmokeFailed);
  assert.equal(packaged.findingId, "desktopPackageSmoke");
  assert.equal(security.errorCode, Phase010ReleaseGateErrorCode.SecurityScanFailed);
  assert.equal(security.findingId, "securityScan");
  assert.equal(runbook.errorCode, Phase010ReleaseGateErrorCode.RunbookValidationFailed);
  assert.equal(runbook.findingId, "runbookValidation");
});

test("phase010 release gate rejects unsafe source content and future-scope command targets", () => {
  const unsafeSources = completeSources();
  unsafeSources[".tasks/release/local-desktop-runbook-phase010.md"] += "\nprovider_api_key_fixture";
  const unsafe = evaluatePhase010ReleaseGate({
    sources: unsafeSources,
    commandResults: passingCommandResults(),
  });
  const invalidScope = evaluatePhase010ReleaseGate({
    sources: completeSources(),
    commandResults: passingCommandResults(),
    commandPlan: [
      ...buildPhase010ReleaseCommandPlan(),
      { id: "futureScope", command: ["npm", "run", "run:self-host-e2e-smoke"] },
    ],
  });

  assert.equal(unsafe.errorCode, Phase010ReleaseGateErrorCode.UnsafeArtifactContent);
  assert.equal(unsafe.findingId, "provider_api_key_fixture");
  assert.equal(invalidScope.errorCode, Phase010ReleaseGateErrorCode.InvalidScope);
  assert.equal(invalidScope.findingId, "futureScope");
});

test("phase010 release command plan is current local desktop only", () => {
  const steps = buildPhase010ReleaseCommandPlan();
  const joined = steps.map((step) => step.command.join(" "));

  assert.deepEqual(
    steps.map((step) => step.id),
    [
      "phase010ArchiveValidator",
      "phase010PlanValidator",
      "phase010PackagedLaunchGate",
      "phase010FirstRunWorkspaceGate",
      "phase010DurableAuthoringGate",
      "phase010DataPortabilityGate",
      "phase010IndexHealthRepairGate",
      "phase010SettingsObservabilityGate",
      "rustWorkspace",
      "activeTypeScriptTests",
      "desktopPackageSmoke",
      "desktopPackagedAppSmoke",
      "securityScan",
      "runbookValidation",
    ],
  );
  assert.ok(joined.every((command) => !command.includes("self-host")));
  assert.ok(joined.every((command) => !command.includes("mobile")));
  assert.ok(joined.every((command) => !command.includes("remote")));
  assert.ok(joined.every((command) => !command.includes("admin")));
});

test("phase010 release gate passes complete evidence and renders safe marker", () => {
  const result = evaluatePhase010ReleaseGate({
    sources: completeSources(),
    commandResults: passingCommandResults(),
  });
  const artifact = renderPhase010ReleaseGateArtifact(result);

  assert.equal(result.passed, true);
  assert.equal(result.state, Phase010ReleaseGateState.Passed);
  assert.equal(result.evidenceCount, 15);
  assert.match(artifact, /phase010_release_gate=passed/);
  assert.match(artifact, /personal local desktop installable knowledge management app only/);
  assert.match(artifact, /future_only_targets=self-hosting,SaaS,multi-user,mobile/);
  assert.match(artifact, /p95_300ms_budget=passed/);
  assert.doesNotMatch(artifact, /raw_document_body_fixture/);
  assert.doesNotMatch(artifact, /provider_api_key_fixture/);
  assert.doesNotMatch(artifact, /\/Users\/example\/private/);
});

test("phase010 release gate writes final marker artifact", async () => {
  const root = await createReleaseFixture();

  const result = await runPhase010ReleaseGate({
    root,
    writeArtifacts: true,
    runner: passingRunner,
  });
  const marker = await readFile(join(root, ".tasks", "phase010-release-gate-result.md"), "utf8");

  assert.equal(result.passed, true);
  assert.match(marker, /phase010_release_gate=passed/);
});

test("phase010 release gate state machine reaches terminal states", () => {
  const reading = transitionPhase010ReleaseGateState(
    Phase010ReleaseGateState.Pending,
    Phase010ReleaseGateEvent.Start,
  );
  const validating = transitionPhase010ReleaseGateState(
    reading.state,
    Phase010ReleaseGateEvent.MarkersRead,
  );
  const runningCommands = transitionPhase010ReleaseGateState(
    validating.state,
    Phase010ReleaseGateEvent.ArtifactsValidated,
  );
  const writing = transitionPhase010ReleaseGateState(
    runningCommands.state,
    Phase010ReleaseGateEvent.CommandsPassed,
  );
  const passed = transitionPhase010ReleaseGateState(
    writing.state,
    Phase010ReleaseGateEvent.ResultWritten,
  );
  const invalid = transitionPhase010ReleaseGateState(
    Phase010ReleaseGateState.Pending,
    Phase010ReleaseGateEvent.ResultWritten,
  );

  assert.equal(reading.state, Phase010ReleaseGateState.ReadingMarkers);
  assert.equal(validating.state, Phase010ReleaseGateState.ValidatingArtifacts);
  assert.equal(runningCommands.state, Phase010ReleaseGateState.RunningReleaseCommands);
  assert.equal(writing.state, Phase010ReleaseGateState.WritingResult);
  assert.equal(passed.state, Phase010ReleaseGateState.Passed);
  assert.equal(invalid.errorCode, Phase010ReleaseGateErrorCode.InvalidTransition);
});

async function createReleaseFixture() {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase010-release-"));
  for (const [relativePath, text] of Object.entries(completeSources())) {
    const fullPath = join(root, relativePath);
    await mkdir(fullPath.slice(0, fullPath.lastIndexOf("/")), { recursive: true });
    await writeFile(fullPath, text);
  }
  return root;
}

function passingCommandResults() {
  return Object.fromEntries(
    buildPhase010ReleaseCommandPlan().map((step) => [
      step.id,
      { command: step.command.join(" "), passed: true, exitCode: 0, durationMs: 5 },
    ]),
  );
}

async function passingRunner() {
  return { exitCode: 0, signal: null, durationMs: 5 };
}

function completeSources() {
  return {
    ".tasks/phase010-archive-validation-result.md": "phase010_archive_validation=passed",
    ".tasks/phase010-plan-validation-result.md": "phase010_plan_validation=passed",
    ".tasks/phase010-packaged-launch-gate-result.md": "phase010_packaged_launch_gate=passed",
    ".tasks/phase010-first-run-workspace-gate-result.md": "phase010_first_run_workspace_gate=passed",
    ".tasks/phase010-durable-authoring-gate-result.md": "phase010_durable_authoring_gate=passed",
    ".tasks/phase010-data-portability-gate-result.md": "phase010_data_portability_gate=passed",
    ".tasks/phase010-index-health-repair-gate-result.md": "phase010_index_health_repair_gate=passed",
    ".tasks/phase010-settings-observability-gate-result.md": [
      "phase010_settings_observability_gate=passed",
      "settings_scope=personal_local_desktop",
      "ai_provider_optional=verified",
      "field_debug_guard=verified",
    ].join("\n"),
    ".tasks/release/performance-budget-phase010.md": [
      "phase010_performance_budget=passed",
      "current document read",
      "history list",
      "search",
      "asset metadata",
      "300",
    ].join("\n"),
    ".tasks/release/packaged-runtime-manifest-phase010.json": [
      "phase010_packaged_runtime_manifest=passed",
      '"devServerRequired": false',
      '"installedNodeRuntimeRequired": false',
      '"externalDbRequired": false',
      '"externalSearchRequired": false',
    ].join("\n"),
    ".tasks/release/data-portability-manifest-phase010.json": [
      "phase010_data_portability_manifest=passed",
      "personal_local_desktop",
      "single_user_local_workspace",
    ].join("\n"),
    ".tasks/release/product-log-event-matrix-phase010.md": [
      "phase010_product_log_matrix=passed",
      "Product Log",
      "Field Debug Log",
      "Development Log",
      "settings.opened",
      "field_debug.activation.created",
    ].join("\n"),
    ".tasks/release/security-log-policy-manifest-phase010.json": [
      "phase010_security_log_manifest=passed",
      "Product Log",
      "Field Debug Log",
      "Development Log",
      "AUTH_MATERIAL_SAMPLE",
      "RAW_DOC_BODY_SAMPLE",
    ].join("\n"),
    ".tasks/release/local-desktop-runbook-phase010.md": [
      "phase010_runbook=passed",
      "Clean Install",
      "Packaged Launch",
      "Reinstall Preservation",
      "Blank Screen Recovery",
      "Index Repair",
      "Export Import",
      "Backup Restore",
      "Field Debug",
      "Data Export",
    ].join("\n"),
    "package.json": [
      "run:phase010-release-gate-tests",
      "run:phase010-release-gate",
      "run:phase010-packaged-launch-gate",
      "run:phase010-settings-observability-gate",
    ].join("\n"),
  };
}
