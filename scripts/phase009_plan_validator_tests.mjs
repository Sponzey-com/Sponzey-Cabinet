import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  Phase009PlanErrorCode,
  Phase009PlanEvent,
  Phase009PlanState,
  renderPhase009PlanValidationArtifact,
  runPhase009PlanValidation,
  transitionPhase009PlanState,
  validatePhase009PlanText,
} from "./phase009_plan_validator.mjs";

test("phase009 plan validator rejects missing current inventory marker", async () => {
  const root = await createPlanFixtureRoot({
    inventoryText: "phase009_current_inventory=failed\n",
  });

  const result = await runPhase009PlanValidation({ root, writeArtifact: false });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase009PlanErrorCode.CurrentInventoryMarkerMissing);
  assert.equal(result.findingId, ".tasks/phase009-current-implementation-inventory.md");
});

test("phase009 plan validator rejects missing phase008 release marker", async () => {
  const root = await createPlanFixtureRoot({
    phase008ReleaseText: "phase008_release_gate=failed\n",
  });

  const result = await runPhase009PlanValidation({ root, writeArtifact: false });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase009PlanErrorCode.Phase008ReleaseMarkerMissing);
  assert.equal(result.findingId, ".tasks/phase008/phase008-release-gate-result.md");
});

test("phase009 plan text validator rejects wrong active phase", () => {
  const findings = validatePhase009PlanText(
    completePhase009PlanText().replace("# Phase 009 Development Plan", "# Phase 008 Development Plan"),
  );

  assert.equal(findings[0].errorCode, Phase009PlanErrorCode.ActivePhaseMismatch);
  assert.equal(findings[0].findingId, "current_phase");
});

test("phase009 plan text validator rejects active SaaS implementation scope", () => {
  const findings = validatePhase009PlanText(
    `${completePhase009PlanText()}\nPhase 009 active implementation: SaaS runtime\n`,
  );

  assert.equal(findings[0].errorCode, Phase009PlanErrorCode.ForbiddenActiveScope);
  assert.equal(findings[0].findingId, "Phase 009 active implementation: SaaS runtime");
});

test("phase009 plan text validator rejects checkbox completion as release evidence", () => {
  const findings = validatePhase009PlanText(
    `${completePhase009PlanText()}\nPhase 009 gates use task checkbox text as completion evidence.\n`,
  );

  assert.equal(findings[0].errorCode, Phase009PlanErrorCode.CheckboxEvidenceForbidden);
  assert.equal(findings[0].findingId, "task checkbox text as completion evidence");
});

test("phase009 plan validator passes fixture and renders safe artifact", async () => {
  const root = await createPlanFixtureRoot();

  const result = await runPhase009PlanValidation({ root, writeArtifact: false });
  const artifact = renderPhase009PlanValidationArtifact(result);

  assert.equal(result.passed, true);
  assert.equal(result.state, Phase009PlanState.Passed);
  assert.match(artifact, /phase009_plan_validation=passed/);
  assert.match(artifact, /phase009_current_inventory=passed/);
  assert.match(artifact, /phase008_release_gate=passed/);
  assert.match(artifact, /sensitive data exclusion/);
  assert.doesNotMatch(artifact, /raw_document_body_fixture/);
  assert.doesNotMatch(artifact, /asset_content_fixture/);
  assert.doesNotMatch(artifact, /provider_api_key_fixture/);
  assert.doesNotMatch(artifact, /personal_absolute_path_fixture/);
});

test("phase009 plan validator writes marker artifact to explicit root", async () => {
  const root = await createPlanFixtureRoot();

  const result = await runPhase009PlanValidation({ root, writeArtifact: true });
  const written = await readFile(join(root, ".tasks", "phase009-plan-validation-result.md"), "utf8");

  assert.equal(result.passed, true);
  assert.match(written, /phase009_plan_validation=passed/);
  assert.match(written, /validation_state=Passed/);
});

test("phase009 plan state machine exposes success, failure, and invalid transition", () => {
  const reading = transitionPhase009PlanState(Phase009PlanState.NotStarted, Phase009PlanEvent.Start);
  const validating = transitionPhase009PlanState(reading.state, Phase009PlanEvent.PrerequisitesRead);
  const writing = transitionPhase009PlanState(validating.state, Phase009PlanEvent.PlanValidated);
  const passed = transitionPhase009PlanState(writing.state, Phase009PlanEvent.ResultWritten);
  const failed = transitionPhase009PlanState(reading.state, Phase009PlanEvent.Fail, {
    errorCode: Phase009PlanErrorCode.CurrentInventoryMarkerMissing,
    findingId: ".tasks/phase009-current-implementation-inventory.md",
  });
  const invalid = transitionPhase009PlanState(
    Phase009PlanState.NotStarted,
    Phase009PlanEvent.PlanValidated,
  );

  assert.equal(reading.state, Phase009PlanState.ReadingPrerequisites);
  assert.equal(validating.state, Phase009PlanState.ValidatingPlan);
  assert.equal(writing.state, Phase009PlanState.WritingResult);
  assert.equal(passed.state, Phase009PlanState.Passed);
  assert.equal(failed.state, Phase009PlanState.Failed);
  assert.equal(failed.findingId, ".tasks/phase009-current-implementation-inventory.md");
  assert.equal(invalid.errorCode, Phase009PlanErrorCode.InvalidTransition);
});

async function createPlanFixtureRoot({
  planText = completePhase009PlanText(),
  inventoryText = "phase009_current_inventory=passed\n",
  phase008ReleaseText = "phase008_release_gate=passed\n",
} = {}) {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase009-plan-"));
  await mkdir(join(root, ".tasks", "phase008"), { recursive: true });
  await writeFile(join(root, ".tasks", "plan.md"), planText);
  await writeFile(join(root, ".tasks", "phase009-current-implementation-inventory.md"), inventoryText);
  await writeFile(
    join(root, ".tasks", "phase008", "phase008-release-gate-result.md"),
    phase008ReleaseText,
  );
  return root;
}

function completePhase009PlanText() {
  return [
    "# Phase 009 Development Plan",
    "",
    "현재 단계: Phase 009 - User-Visible Local Desktop Product UX and Native Runtime Reliability",
    "",
    "Current product scope marker: `personal_local_desktop`.",
    "Phase 009는 서버/SaaS/멀티 사용자 기능을 추가하지 않는다.",
    "Product Log, Field Debug Log, Development Log are separated.",
    "Environment values are read once at bootstrap and passed explicitly.",
    "p95 300ms query budget applies to user-facing reads.",
    "current document read and history read stay separate.",
    "Tidy First and feature work are separated.",
    "Task files must use 2-3 functional changes per task.",
    "Blank screen is a gate failure.",
    "Layered Architecture, Clean Architecture, TDD, state machine policy.",
    "State machines are required for complex flows.",
    "Release gate reads marker files and does not rely on checkbox text.",
    "Mandatory Validation Matrix.",
    "",
    "| Evidence | Required Marker | Producer | Consumer |",
    "| --- | --- | --- | --- |",
    "| `.tasks/phase009-current-implementation-inventory.md` | `phase009_current_inventory=passed` | Phase 009.0 | plan validation, all later gates |",
    "| `.tasks/phase009-plan-validation-result.md` | `phase009_plan_validation=passed` | Phase 009.0 | all later gates |",
    "| `.tasks/phase009-desktop-launch-gate-result.md` | `phase009_desktop_launch_gate=passed` | Phase 009.1 | release gate |",
    "| `.tasks/phase009-command-runtime-gate-result.md` | `phase009_command_runtime_gate=passed` | Phase 009.2 | authoring, discovery, backup, release gate |",
    "| `.tasks/phase009-document-authoring-gate-result.md` | `phase009_document_authoring_gate=passed` | Phase 009.3 | release gate |",
    "| `.tasks/phase009-discovery-assets-gate-result.md` | `phase009_discovery_assets_gate=passed` | Phase 009.4 | release gate |",
    "| `.tasks/phase009-recovery-backup-ux-gate-result.md` | `phase009_recovery_backup_ux_gate=passed` | Phase 009.5 | release gate |",
    "| `.tasks/phase009-ux-release-gate-result.md` | `phase009_ux_release_gate=passed` | Phase 009.6 | final completion |",
    "| `.tasks/release/performance-budget-phase009.md` | `phase009_performance_budget=passed` | Phase 009.3-009.6 | release gate |",
    "",
  ].join("\n");
}
