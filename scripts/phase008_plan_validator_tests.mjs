import assert from "node:assert/strict";
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  Phase008PlanErrorCode,
  Phase008PlanEvent,
  Phase008PlanState,
  renderPhase008PlanValidationArtifact,
  renderPhase008PlanValidationResult,
  runPhase008PlanValidation,
  transitionPhase008PlanState,
  validatePhase008PlanText,
  validatePhase008ReadmeText,
} from "./phase008_plan_validator.mjs";

test("phase008 plan validator rejects missing phase007 final release marker", async () => {
  const root = await createFixtureRoot({
    phase007ReleaseText: "# Phase 007 Release\nphase007_release_gate=failed\n",
  });

  const result = await runPhase008PlanValidation({ root });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase008PlanErrorCode.Phase007ReleaseMarkerMissing);
  assert.equal(result.findingId, ".tasks/phase007/phase007-release-gate-result.md");
});

test("phase008 plan validator rejects missing phase007 product smoke marker", async () => {
  const root = await createFixtureRoot({
    phase007ProductSmokeText: "# Phase 007 Product Smoke\nphase007_product_smoke_gate=failed\n",
  });

  const result = await runPhase008PlanValidation({ root });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase008PlanErrorCode.Phase007ProductSmokeMarkerMissing);
  assert.equal(result.findingId, ".tasks/phase007/phase007-product-smoke-gate-result.md");
});

test("phase008 plan validator rejects wrong active phase", () => {
  const findings = validatePhase008PlanText(completePhase008PlanText().replace("Phase 008", "Phase 007"));

  assert.equal(findings[0].errorCode, Phase008PlanErrorCode.Phase008ActivePhaseMismatch);
  assert.equal(findings[0].findingId, "current_phase");
});

test("phase008 plan validator rejects stale phase007 root task", async () => {
  const root = await createFixtureRoot({
    activeTaskText: "# Task 001. Phase 007 stale root task\n",
  });

  const result = await runPhase008PlanValidation({ root });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase008PlanErrorCode.ActivePhase007ArtifactFound);
  assert.equal(result.findingId, ".tasks/task001.md");
});

test("phase008 plan validator rejects phase007 root result artifact", async () => {
  const root = await createFixtureRoot({
    activePhase007ResultText: "# Phase 007 Result\nphase007_release_gate=passed\n",
  });

  const result = await runPhase008PlanValidation({ root });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase008PlanErrorCode.ActivePhase007ArtifactFound);
  assert.equal(result.findingId, ".tasks/phase007-release-gate-result.md");
});

test("phase008 plan validator rejects missing evidence manifest", () => {
  const findings = validatePhase008PlanText(
    completePhase008PlanText().replace(
      "| `.tasks/phase008-release-gate-result.md` | `phase008_release_gate=passed` | Phase 008.7 | final completion |",
      "",
    ),
  );

  assert.equal(findings[0].errorCode, Phase008PlanErrorCode.Phase008PlanTermMissing);
  assert.equal(findings[0].findingId, "phase008_release_gate=passed");
});

test("phase008 readme validator rejects stale active phase pointer", () => {
  const findings = validatePhase008ReadmeText(
    completePhase008ReadmeText().replace("Active phase: Phase 008", "Active phase: Phase 007"),
  );

  assert.equal(findings[0].errorCode, Phase008PlanErrorCode.Phase008ReadmeTermMissing);
  assert.equal(findings[0].findingId, "Active phase: Phase 008");
});

test("phase008 plan validator passes complete fixture and renders safe artifact", async () => {
  const root = await createFixtureRoot({});

  const result = await runPhase008PlanValidation({ root });
  const rendered = renderPhase008PlanValidationResult(result);
  const artifact = renderPhase008PlanValidationArtifact(result);

  assert.equal(result.passed, true);
  assert.equal(result.state, Phase008PlanState.Passed);
  assert.equal(result.archiveFileCount >= 20, true);
  assert.match(rendered, /phase008_plan_validation=passed/);
  assert.match(rendered, /validation_state=Passed/);
  assert.match(artifact, /sensitive data exclusion/);
  assert.doesNotMatch(artifact, /raw_document_body_fixture/);
  assert.doesNotMatch(artifact, /provider_api_key_fixture/);
  assert.doesNotMatch(artifact, /personal_absolute_path_fixture/);
});

test("phase008 plan validator allows active phase008 task001", async () => {
  const root = await createFixtureRoot({
    activeTaskText: "# Task 001. Phase 008 Archive Boundary, Plan Validator, and Scope Lock\n",
  });

  const result = await runPhase008PlanValidation({ root });

  assert.equal(result.passed, true);
  assert.equal(result.state, Phase008PlanState.Passed);
});

test("phase008 plan state machine exposes explicit failure and terminal states", () => {
  const readingArchive = transitionPhase008PlanState(
    Phase008PlanState.NotStarted,
    Phase008PlanEvent.Start,
  );
  const validatingPlan = transitionPhase008PlanState(
    readingArchive.state,
    Phase008PlanEvent.ArchiveChecked,
  );
  const validatingReadme = transitionPhase008PlanState(
    validatingPlan.state,
    Phase008PlanEvent.PlanChecked,
  );
  const writingResult = transitionPhase008PlanState(
    validatingReadme.state,
    Phase008PlanEvent.ReadmeChecked,
  );
  const passed = transitionPhase008PlanState(
    writingResult.state,
    Phase008PlanEvent.ResultWritten,
  );
  const failed = transitionPhase008PlanState(validatingPlan.state, Phase008PlanEvent.Fail, {
    errorCode: Phase008PlanErrorCode.Phase008PlanTermMissing,
    findingId: "LocalDesktopConfig",
  });
  const invalid = transitionPhase008PlanState(
    Phase008PlanState.NotStarted,
    Phase008PlanEvent.PlanChecked,
  );

  assert.equal(readingArchive.state, Phase008PlanState.ReadingArchive);
  assert.equal(validatingPlan.state, Phase008PlanState.ValidatingPlan);
  assert.equal(validatingReadme.state, Phase008PlanState.ValidatingReadme);
  assert.equal(writingResult.state, Phase008PlanState.WritingResult);
  assert.equal(passed.state, Phase008PlanState.Passed);
  assert.equal(failed.state, Phase008PlanState.Failed);
  assert.equal(failed.findingId, "LocalDesktopConfig");
  assert.equal(invalid.errorCode, Phase008PlanErrorCode.InvalidTransition);
});

async function createFixtureRoot({
  phase007ReleaseText = "# Phase 007 Release Gate Result\n\nphase007_release_gate=passed\n",
  phase007ProductSmokeText = "# Phase 007 Product Smoke Gate Result\n\nphase007_product_smoke_gate=passed\n",
  activeTaskText,
  activePhase007ResultText,
} = {}) {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase008-plan-"));
  await mkdir(join(root, ".tasks", "phase007", "release"), { recursive: true });
  await writeFile(join(root, ".tasks", "plan.md"), completePhase008PlanText());
  await writeFile(join(root, ".tasks", "readme.md"), completePhase008ReadmeText());

  const phase007Files = [
    "plan.md",
    "phase007-plan-validation-result.md",
    "phase007-workspace-home-gate-result.md",
    "phase007-document-authoring-gate-result.md",
    "phase007-local-persistence-gate-result.md",
    "phase007-discovery-gate-result.md",
    "phase007-ai-assistant-gate-result.md",
    "phase007-data-ownership-gate-result.md",
  ];
  for (const fileName of phase007Files) {
    await writeFile(join(root, ".tasks", "phase007", fileName), `${fileName}\n`);
  }
  await writeFile(
    join(root, ".tasks", "phase007", "phase007-product-smoke-gate-result.md"),
    phase007ProductSmokeText,
  );
  await writeFile(
    join(root, ".tasks", "phase007", "phase007-release-gate-result.md"),
    phase007ReleaseText,
  );
  for (let index = 1; index <= 8; index += 1) {
    await writeFile(
      join(root, ".tasks", "phase007", `task${String(index).padStart(3, "0")}.md`),
      `# Task ${String(index).padStart(3, "0")}. Archived Phase 007 task\n`,
    );
  }
  for (const fileName of [
    "performance-budget-phase007.md",
    "ai-status-result-budget-phase007.md",
    "local-desktop-runbook.md",
    "product-log-event-matrix.md",
    "security-log-policy-manifest.json",
  ]) {
    await writeFile(join(root, ".tasks", "phase007", "release", fileName), `${fileName}\n`);
  }

  if (activeTaskText) {
    await writeFile(join(root, ".tasks", "task001.md"), activeTaskText);
  }
  if (activePhase007ResultText) {
    await writeFile(
      join(root, ".tasks", "phase007-release-gate-result.md"),
      activePhase007ResultText,
    );
  }
  return root;
}

function completePhase008ReadmeText() {
  return [
    "# Sponzey Cabinet Task Index",
    "",
    "Active phase: Phase 008",
    "",
    "Active plan: `.tasks/plan.md`",
    "",
    "Current product scope: `personal_local_desktop`",
    "",
    "Phase 008 root tasks restart at `.tasks/task001.md`.",
    "",
    "Archive phases: Phase 001, Phase 002, Phase 003, Phase 004, Phase 005, Phase 006, Phase 007",
  ].join("\n");
}

function completePhase008PlanText() {
  return [
    "# Phase 008 Development Plan",
    "",
    "현재 단계: Phase 008 - Native Local Runtime, Durable Workspace, and Desktop Execution Hardening",
    "",
    "## 1. Project Goal",
    "personal_local_desktop",
    "Native Local Runtime",
    "Durable Workspace",
    "Desktop Execution Hardening",
    "Windows",
    "macOS",
    "Linux",
    "Tauri",
    "CodeMirror",
    "LocalDesktopConfig",
    "SelectedAssetDraft",
    "current document",
    "history read",
    "p95 300ms",
    "Product Log",
    "Field Debug Log",
    "Development Log",
    "Layered Architecture",
    "Clean Architecture",
    "Tidy First",
    "TDD",
    "phase008_plan_validation=passed",
    "phase008_release_gate=passed",
    "server/SaaS/multi-user excluded from active scope",
    "",
    "Phase 008 required evidence manifest:",
    "| Evidence | Required Marker | Producer Phase | Consumer Gate |",
    "| `.tasks/phase008-plan-validation-result.md` | `phase008_plan_validation=passed` | Phase 008.0 | all later phases |",
    "| `.tasks/phase008-release-gate-result.md` | `phase008_release_gate=passed` | Phase 008.7 | final completion |",
    "| `.tasks/release/performance-budget-phase008.md` | `phase008_performance_budget=passed` | Phase 008.3-008.7 | product smoke, release gate |",
    "Gate artifact contract",
    "Security and Privacy Gate Strategy",
    "Performance Budget Strategy",
    "",
    "## 2. Source Documents and Baseline",
    "## 3. Current Plan Assessment",
    "## 4. Architecture Direction",
    "## 5. Development Principles",
    "## 6. Implementation Phases",
    "## 7. TDD Strategy",
    "## 8. Configuration and Runtime Environment Policy",
    "## 9. Logging Strategy",
    "## 10. Performance Budget Strategy",
    "## 11. Security and Privacy Gate Strategy",
    "## 12. State Machine Strategy",
    "## 13. Dependency and Boundary Rules",
    "## 14. Risk and Mitigation",
    "## 15. Review Checklist",
    "## 16. Definition of Done",
    "## 17. Prohibited Implementation Patterns",
    "## 18. Required Task Format",
    "## 19. Expected Task Decomposition",
    "## 20. Next Actions",
  ].join("\n");
}
