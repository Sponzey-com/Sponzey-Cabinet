import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  Phase010PlanErrorCode,
  Phase010PlanEvent,
  Phase010PlanState,
  renderPhase010PlanValidationArtifact,
  runPhase010PlanValidation,
  transitionPhase010PlanState,
  validatePhase010PlanText,
} from "./phase010_plan_validator.mjs";

test("phase010 plan validator rejects missing archive validation marker", async () => {
  const root = await createPlanFixtureRoot({
    archiveMarkerText: "phase010_archive_validation=failed\n",
  });

  const result = await runPhase010PlanValidation({ root, writeArtifact: false });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010PlanErrorCode.ArchiveMarkerMissing);
  assert.equal(result.findingId, ".tasks/phase010-archive-validation-result.md");
});

test("phase010 plan text validator rejects wrong active phase", () => {
  const findings = validatePhase010PlanText(
    completePhase010PlanText().replace("# Phase 010 Development Plan", "# Phase 009 Development Plan"),
  );

  assert.equal(findings[0].errorCode, Phase010PlanErrorCode.ActivePhaseMismatch);
  assert.equal(findings[0].findingId, "current_phase");
});

test("phase010 plan text validator rejects active SaaS implementation scope", () => {
  const findings = validatePhase010PlanText(
    `${completePhase010PlanText()}\nPhase 010 active implementation: SaaS runtime\n`,
  );

  assert.equal(findings[0].errorCode, Phase010PlanErrorCode.ForbiddenActiveScope);
  assert.equal(findings[0].findingId, "Phase 010 active implementation: SaaS runtime");
});

test("phase010 plan text validator rejects checkbox completion as release evidence", () => {
  const findings = validatePhase010PlanText(
    `${completePhase010PlanText()}\nPhase 010 release gate accepts task checkbox text as completion evidence.\n`,
  );

  assert.equal(findings[0].errorCode, Phase010PlanErrorCode.CheckboxEvidenceForbidden);
  assert.equal(findings[0].findingId, "task checkbox text as completion evidence");
});

test("phase010 plan validator rejects stale readme active phase", async () => {
  const root = await createPlanFixtureRoot({
    readmeText: "# Task Index\n\nActive phase: Phase 009\n",
  });

  const result = await runPhase010PlanValidation({ root, writeArtifact: false });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010PlanErrorCode.ReadmePhaseMismatch);
  assert.equal(result.findingId, ".tasks/readme.md");
});

test("phase010 plan validator passes fixture and renders safe artifact", async () => {
  const root = await createPlanFixtureRoot();

  const result = await runPhase010PlanValidation({ root, writeArtifact: false });
  const artifact = renderPhase010PlanValidationArtifact(result);

  assert.equal(result.passed, true);
  assert.equal(result.state, Phase010PlanState.Passed);
  assert.match(artifact, /phase010_plan_validation=passed/);
  assert.match(artifact, /phase010_archive_validation=passed/);
  assert.match(artifact, /scope lock: personal local desktop only/);
  assert.match(artifact, /completion evidence: marker artifacts only/);
  assert.doesNotMatch(artifact, /raw_document_body_fixture/);
  assert.doesNotMatch(artifact, /provider_api_key_fixture/);
  assert.doesNotMatch(artifact, /personal_absolute_path_fixture/);
});

test("phase010 plan validator writes marker artifact to explicit root", async () => {
  const root = await createPlanFixtureRoot();

  const result = await runPhase010PlanValidation({ root, writeArtifact: true });
  const written = await readFile(join(root, ".tasks", "phase010-plan-validation-result.md"), "utf8");

  assert.equal(result.passed, true);
  assert.match(written, /phase010_plan_validation=passed/);
  assert.match(written, /validation_state=Passed/);
});

test("phase010 plan state machine exposes success, failure, and invalid transition", () => {
  const reading = transitionPhase010PlanState(Phase010PlanState.Pending, Phase010PlanEvent.Start);
  const validating = transitionPhase010PlanState(reading.state, Phase010PlanEvent.PlanRead);
  const writing = transitionPhase010PlanState(validating.state, Phase010PlanEvent.ScopeValidated);
  const passed = transitionPhase010PlanState(writing.state, Phase010PlanEvent.ResultWritten);
  const failed = transitionPhase010PlanState(reading.state, Phase010PlanEvent.Fail, {
    errorCode: Phase010PlanErrorCode.ArchiveMarkerMissing,
    findingId: ".tasks/phase010-archive-validation-result.md",
  });
  const invalid = transitionPhase010PlanState(
    Phase010PlanState.Pending,
    Phase010PlanEvent.ScopeValidated,
  );

  assert.equal(reading.state, Phase010PlanState.ReadingPlan);
  assert.equal(validating.state, Phase010PlanState.ValidatingScope);
  assert.equal(writing.state, Phase010PlanState.WritingResult);
  assert.equal(passed.state, Phase010PlanState.Passed);
  assert.equal(failed.state, Phase010PlanState.Failed);
  assert.equal(failed.findingId, ".tasks/phase010-archive-validation-result.md");
  assert.equal(invalid.errorCode, Phase010PlanErrorCode.InvalidTransition);
});

async function createPlanFixtureRoot({
  planText = completePhase010PlanText(),
  archiveMarkerText = "phase010_archive_validation=passed\n",
  readmeText = "# Sponzey Cabinet Task Index\n\nActive phase: Phase 010\n\nCurrent product scope: `personal_local_desktop`\n",
} = {}) {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase010-plan-"));
  await mkdir(join(root, ".tasks"), { recursive: true });
  await writeFile(join(root, ".tasks", "plan.md"), planText);
  await writeFile(join(root, ".tasks", "readme.md"), readmeText);
  await writeFile(join(root, ".tasks", "phase010-archive-validation-result.md"), archiveMarkerText);
  return root;
}

function completePhase010PlanText() {
  return [
    "# Phase 010 Development Plan",
    "",
    "현재 단계: Phase 010 - Installable Local Desktop Release Candidate and Durable Personal Workspace",
    "",
    "Current product scope marker: `personal_local_desktop`.",
    "Current active platforms include Windows desktop installable app, macOS desktop installable app, and Linux desktop installable app.",
    "Phase 010 explicitly does not build server hosting runtime, SaaS runtime, multi-user collaboration, iOS/Android product implementation, billing, SSO, or admin console.",
    "server/SaaS/multi-user/mobile work remains future-compatible architecture only.",
    "Product Log, Field Debug Log, Development Log are separated.",
    "Environment values are read once at bootstrap and passed explicitly.",
    "p95 300ms query budget applies to current document, history, search, backlink, graph, and asset metadata reads.",
    "Tidy First and feature work are separated.",
    "Layered Architecture, Clean Architecture, and TDD are mandatory.",
    "State Machine Strategy and Canonical Phase 010 state machine contracts are required.",
    "Phase 010 Task Execution Controls.",
    "Phase 010 Active-Scope Validation Command Matrix.",
    "Script Hygiene Rules.",
    "Task 001. Phase 010 Archive and Plan Validation Tooling.",
    "Release artifacts are based on marker files and command results, not task checkboxes.",
    "",
    "| Evidence | Required Marker | Producer | Consumer |",
    "| --- | --- | --- | --- |",
    "| `.tasks/phase010-archive-validation-result.md` | `phase010_archive_validation=passed` | Phase 010.0 | all later gates |",
    "| `.tasks/phase010-plan-validation-result.md` | `phase010_plan_validation=passed` | Phase 010.0 | all later gates |",
    "| `.tasks/phase010-packaged-launch-gate-result.md` | `phase010_packaged_launch_gate=passed` | Phase 010.1 | release gate |",
    "| `.tasks/phase010-first-run-workspace-gate-result.md` | `phase010_first_run_workspace_gate=passed` | Phase 010.2 | persistence, release gate |",
    "| `.tasks/phase010-durable-authoring-gate-result.md` | `phase010_durable_authoring_gate=passed` | Phase 010.3 | portability, release gate |",
    "| `.tasks/phase010-data-portability-gate-result.md` | `phase010_data_portability_gate=passed` | Phase 010.4 | release gate |",
    "| `.tasks/phase010-index-health-repair-gate-result.md` | `phase010_index_health_repair_gate=passed` | Phase 010.5 | release gate |",
    "| `.tasks/phase010-settings-observability-gate-result.md` | `phase010_settings_observability_gate=passed` | Phase 010.6 | release gate |",
    "| `.tasks/phase010-release-gate-result.md` | `phase010_release_gate=passed` | Phase 010.7 | final completion |",
    "| `.tasks/release/performance-budget-phase010.md` | `phase010_performance_budget=passed` | Phase 010.3-010.7 | release gate |",
    "| `.tasks/release/packaged-runtime-manifest-phase010.json` | `phase010_packaged_runtime_manifest=passed` | Phase 010.1 | release gate |",
    "| `.tasks/release/data-portability-manifest-phase010.json` | `phase010_data_portability_manifest=passed` | Phase 010.4 | release gate |",
    "| `.tasks/release/product-log-event-matrix-phase010.md` | `phase010_product_log_matrix=passed` | Phase 010.1-010.7 | release gate |",
    "| `.tasks/release/security-log-policy-manifest-phase010.json` | `phase010_security_log_manifest=passed` | Phase 010.6 | release gate |",
    "| `.tasks/release/local-desktop-runbook-phase010.md` | `phase010_runbook=passed` | Phase 010.6-010.7 | release gate |",
  ].join("\n");
}
