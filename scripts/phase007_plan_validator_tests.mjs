import assert from "node:assert/strict";
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  Phase007PlanErrorCode,
  Phase007PlanEvent,
  Phase007PlanState,
  renderPhase007PlanValidationArtifact,
  renderPhase007PlanValidationResult,
  runPhase007PlanValidation,
  transitionPhase007PlanState,
  validatePhase007PlanText,
  validatePhase007ReadmeText,
} from "./phase007_plan_validator.mjs";

test("phase007 plan validator rejects missing phase006 final release marker", async () => {
  const root = await createFixtureRoot({
    phase006ReleaseText: "# Phase 006 Release\nphase006_release_gate=failed\n",
  });

  const result = await runPhase007PlanValidation({ root });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase007PlanErrorCode.Phase006ReleaseMarkerMissing);
  assert.equal(result.findingId, ".tasks/phase006/phase006-release-gate-result.md");
});

test("phase007 plan validator rejects wrong active phase", () => {
  const findings = validatePhase007PlanText(completePhase007PlanText().replace("Phase 007", "Phase 006"));

  assert.equal(findings[0].errorCode, Phase007PlanErrorCode.Phase007ActivePhaseMismatch);
  assert.equal(findings[0].findingId, "current_phase");
});

test("phase007 plan validator rejects forbidden active server scope", () => {
  const findings = validatePhase007PlanText(
    `${completePhase007PlanText()}\n\nPhase 007 active implementation: server hosting runtime\n`,
  );

  assert.equal(findings[0].errorCode, Phase007PlanErrorCode.Phase007ForbiddenActiveScope);
  assert.equal(findings[0].findingId, "server hosting runtime");
});

test("phase007 plan validator rejects stale phase006 root task", async () => {
  const root = await createFixtureRoot({
    activeTaskText: "# Task 001. Phase 006 stale root task\n",
  });

  const result = await runPhase007PlanValidation({ root });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase007PlanErrorCode.ActivePhase006ArtifactFound);
  assert.equal(result.findingId, ".tasks/task001.md");
});

test("phase007 plan validator rejects phase006 root result artifact", async () => {
  const root = await createFixtureRoot({
    activePhase006ResultText: "# Phase 006 Result\nphase006_release_gate=passed\n",
  });

  const result = await runPhase007PlanValidation({ root });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase007PlanErrorCode.ActivePhase006ArtifactFound);
  assert.equal(result.findingId, ".tasks/phase006-release-gate-result.md");
});

test("phase007 readme validator rejects stale active phase pointer", () => {
  const findings = validatePhase007ReadmeText(
    completePhase007ReadmeText().replace("Active phase: Phase 007", "Active phase: Phase 006"),
  );

  assert.equal(findings[0].errorCode, Phase007PlanErrorCode.Phase007ReadmeTermMissing);
  assert.equal(findings[0].findingId, "Active phase: Phase 007");
});

test("phase007 plan validator passes complete fixture and renders safe artifact", async () => {
  const root = await createFixtureRoot({});

  const result = await runPhase007PlanValidation({ root });
  const rendered = renderPhase007PlanValidationResult(result);
  const artifact = renderPhase007PlanValidationArtifact(result);

  assert.equal(result.passed, true);
  assert.equal(result.state, Phase007PlanState.Passed);
  assert.equal(result.archiveFileCount >= 36, true);
  assert.match(rendered, /phase007_plan_validation=passed/);
  assert.match(rendered, /validation_state=Passed/);
  assert.match(artifact, /sensitive data exclusion/);
  assert.doesNotMatch(artifact, /document_body_secret_fixture/);
});

test("phase007 plan validator allows active phase007 task001", async () => {
  const root = await createFixtureRoot({
    activeTaskText: "# Task 001. Phase 007 Archive Boundary and Plan Validation Gate\n",
  });

  const result = await runPhase007PlanValidation({ root });

  assert.equal(result.passed, true);
  assert.equal(result.state, Phase007PlanState.Passed);
});

test("phase007 plan state machine exposes explicit failure and terminal states", () => {
  const readingArchive = transitionPhase007PlanState(
    Phase007PlanState.NotStarted,
    Phase007PlanEvent.Start,
  );
  const validatingPlan = transitionPhase007PlanState(
    readingArchive.state,
    Phase007PlanEvent.ArchiveChecked,
  );
  const validatingReadme = transitionPhase007PlanState(
    validatingPlan.state,
    Phase007PlanEvent.PlanChecked,
  );
  const writingResult = transitionPhase007PlanState(
    validatingReadme.state,
    Phase007PlanEvent.ReadmeChecked,
  );
  const passed = transitionPhase007PlanState(
    writingResult.state,
    Phase007PlanEvent.ResultWritten,
  );
  const failed = transitionPhase007PlanState(validatingPlan.state, Phase007PlanEvent.Fail, {
    errorCode: Phase007PlanErrorCode.Phase007PlanTermMissing,
    findingId: "personal_local_desktop",
  });
  const invalid = transitionPhase007PlanState(
    Phase007PlanState.NotStarted,
    Phase007PlanEvent.PlanChecked,
  );

  assert.equal(readingArchive.state, Phase007PlanState.ReadingArchive);
  assert.equal(validatingPlan.state, Phase007PlanState.ValidatingPlan);
  assert.equal(validatingReadme.state, Phase007PlanState.ValidatingReadme);
  assert.equal(writingResult.state, Phase007PlanState.WritingResult);
  assert.equal(passed.state, Phase007PlanState.Passed);
  assert.equal(failed.state, Phase007PlanState.Failed);
  assert.equal(failed.findingId, "personal_local_desktop");
  assert.equal(invalid.errorCode, Phase007PlanErrorCode.InvalidTransition);
});

async function createFixtureRoot({
  phase006ReleaseText = "# Phase 006 Release Gate Result\n\nphase006_release_gate=passed\n",
  activeTaskText,
  activePhase006ResultText,
} = {}) {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase007-plan-"));
  await mkdir(join(root, ".tasks", "phase006", "release"), { recursive: true });
  await writeFile(join(root, ".tasks", "plan.md"), completePhase007PlanText());
  await writeFile(join(root, ".tasks", "readme.md"), completePhase007ReadmeText());

  const phase006Files = [
    "plan.md",
    "phase006-plan-validation-result.md",
    "phase006-local-runtime-gate-result.md",
    "phase006-workspace-shell-gate-result.md",
    "phase006-document-ux-gate-result.md",
    "phase006-search-graph-asset-gate-result.md",
    "phase006-ai-ux-gate-result.md",
    "phase006-backup-package-gate-result.md",
    "phase006-product-smoke-gate-result.md",
  ];
  for (const fileName of phase006Files) {
    await writeFile(join(root, ".tasks", "phase006", fileName), `${fileName}\n`);
  }
  await writeFile(
    join(root, ".tasks", "phase006", "phase006-release-gate-result.md"),
    phase006ReleaseText,
  );
  for (let index = 1; index <= 19; index += 1) {
    await writeFile(
      join(root, ".tasks", "phase006", `task${String(index).padStart(3, "0")}.md`),
      `# Task ${String(index).padStart(3, "0")}. Archived Phase 006 task\n`,
    );
  }
  for (const fileName of [
    "performance-budget-phase006.md",
    "local-desktop-runbook.md",
    "product-log-event-matrix.md",
    "runbook-validation-manifest.json",
    "security-log-policy-manifest.json",
    "data-ownership-verification.md",
  ]) {
    await writeFile(join(root, ".tasks", "phase006", "release", fileName), `${fileName}\n`);
  }

  if (activeTaskText) {
    await writeFile(join(root, ".tasks", "task001.md"), activeTaskText);
  }
  if (activePhase006ResultText) {
    await writeFile(
      join(root, ".tasks", "phase006-release-gate-result.md"),
      activePhase006ResultText,
    );
  }
  return root;
}

function completePhase007ReadmeText() {
  return [
    "# Sponzey Cabinet Task Index",
    "",
    "Active phase: Phase 007",
    "",
    "Active plan: `.tasks/plan.md`",
    "",
    "Current product scope: `personal_local_desktop`",
    "",
    "Phase 007 root tasks restart at `.tasks/task001.md`.",
    "",
    "Archive phases: Phase 001, Phase 002, Phase 003, Phase 004, Phase 005, Phase 006",
  ].join("\n");
}

function completePhase007PlanText() {
  return [
    "# Phase 007 Development Plan",
    "",
    "현재 단계: Phase 007 - Daily Local Knowledge Workspace and Desktop App Usability",
    "",
    "## 1. Project Goal",
    "personal_local_desktop",
    "Daily Local Knowledge Workspace",
    "Windows",
    "macOS",
    "Linux",
    "CodeMirror",
    "Markdown preview",
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
    "phase007_plan_validation=passed",
    "phase007_release_gate=passed",
    "server/SaaS/multi-user excluded from active scope",
    "",
    "## 2. Source Documents and Baseline",
    "## 3. Current Plan Assessment",
    "## 4. Architecture Direction",
    "## 5. Development Principles",
    "## 6. Implementation Phases",
    "## 7. TDD Strategy",
    "## 8. Configuration and Runtime Environment Policy",
    "## 9. Logging Strategy",
    "## 10. State Machine Strategy",
    "## 11. Performance Strategy",
    "## 12. Dependency and Boundary Rules",
    "## 13. Risk and Mitigation",
    "## 14. Review Checklist",
    "## 15. Definition of Done",
    "## 16. Prohibited Implementation Patterns",
    "## 17. Required Task Format",
    "## 18. Next Actions",
  ].join("\n");
}
