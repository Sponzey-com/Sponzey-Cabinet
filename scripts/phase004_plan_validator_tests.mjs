import assert from "node:assert/strict";
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  Phase004PlanErrorCode,
  Phase004PlanEvent,
  Phase004PlanState,
  renderPhase004PlanValidationResult,
  runPhase004PlanValidation,
  transitionPhase004PlanState,
  validatePhase004PlanText,
} from "./phase004_plan_validator.mjs";

test("phase004 plan validator rejects missing phase003 archived task", async () => {
  const root = await createFixtureRoot({
    planText: completePhase004PlanText(),
    omitArchiveTasks: ["task059.md"],
  });

  const result = await runPhase004PlanValidation({ root });
  const rendered = renderPhase004PlanValidationResult(result);

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase004PlanErrorCode.Phase003ArchiveMissing);
  assert.equal(result.findingId, ".tasks/phase003/task059.md");
  assert.match(rendered, /phase004_plan_validation=failed/);
  assert.match(rendered, /error_code=PHASE003_ARCHIVE_MISSING/);
});

test("phase004 plan validator rejects missing phase003 final release marker", async () => {
  const root = await createFixtureRoot({
    planText: completePhase004PlanText(),
    finalReleaseText: "# Final\nphase003_release_gate=failed\n",
  });

  const result = await runPhase004PlanValidation({ root });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase004PlanErrorCode.Phase003ReleaseMarkerMissing);
  assert.equal(result.findingId, ".tasks/phase003/final-release-gate-result.md");
});

test("phase004 plan validator rejects stale active phase003 root task", async () => {
  const root = await createFixtureRoot({
    planText: completePhase004PlanText(),
    activeTaskText: "# Task 001. Phase 003 stale active task\n",
  });

  const result = await runPhase004PlanValidation({ root });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase004PlanErrorCode.ActiveArchiveConflict);
  assert.equal(result.findingId, ".tasks/task001.md");
});

test("phase004 plan validator rejects missing required phase004 planning term", () => {
  const findings = validatePhase004PlanText(
    completePhase004PlanText().replace("Canvas/Edgeless", "Board"),
  );

  assert.equal(findings[0].errorCode, Phase004PlanErrorCode.Phase004PlanTermMissing);
  assert.equal(findings[0].findingId, "Canvas/Edgeless");
});

test("phase004 plan validator passes complete phase004 fixture with active phase004 task", async () => {
  const root = await createFixtureRoot({
    planText: completePhase004PlanText(),
    activeTaskText: "# Task 001. Phase 004 Planning and Archive Boundary Gate\n",
  });

  const result = await runPhase004PlanValidation({ root });
  const rendered = renderPhase004PlanValidationResult(result);

  assert.equal(result.passed, true);
  assert.equal(result.state, Phase004PlanState.Passed);
  assert.equal(result.archiveFileCount >= 70, true);
  assert.equal(result.requiredTermCount >= 20, true);
  assert.match(rendered, /phase004_plan_validation=passed/);
});

test("phase004 plan validator allows active task without literal phase marker when it is not stale phase003 work", async () => {
  const root = await createFixtureRoot({
    planText: completePhase004PlanText(),
    activeTaskText: [
      "# Task 012. Local Realtime Adapter Baseline",
      "",
      "- This task contributes to `.tasks/plan.md` realtime gateway baseline.",
      "- It does not declare stale archived work.",
    ].join("\n"),
  });

  const result = await runPhase004PlanValidation({ root });

  assert.equal(result.passed, true);
  assert.equal(result.state, Phase004PlanState.Passed);
});

test("phase004 plan state machine exposes explicit failure and terminal states", () => {
  const checkingArchive = transitionPhase004PlanState(
    Phase004PlanState.Pending,
    Phase004PlanEvent.Start,
  );
  const checkingPlan = transitionPhase004PlanState(
    checkingArchive.state,
    Phase004PlanEvent.ArchiveChecked,
  );
  const passed = transitionPhase004PlanState(
    checkingPlan.state,
    Phase004PlanEvent.PlanChecked,
  );
  const failed = transitionPhase004PlanState(checkingPlan.state, Phase004PlanEvent.Fail, {
    errorCode: Phase004PlanErrorCode.Phase004PlanTermMissing,
    findingId: "Canvas/Edgeless",
  });
  const invalid = transitionPhase004PlanState(
    Phase004PlanState.Pending,
    Phase004PlanEvent.PlanChecked,
  );

  assert.equal(checkingArchive.state, Phase004PlanState.CheckingArchive);
  assert.equal(checkingPlan.state, Phase004PlanState.CheckingPlan);
  assert.equal(passed.state, Phase004PlanState.Passed);
  assert.equal(failed.state, Phase004PlanState.Failed);
  assert.equal(failed.findingId, "Canvas/Edgeless");
  assert.equal(invalid.errorCode, Phase004PlanErrorCode.InvalidTransition);
});

async function createFixtureRoot({
  planText,
  activeTaskText,
  finalReleaseText = "# Final\nphase003_release_gate=passed\nproduction hardening complete\n",
  omitArchiveTasks = [],
}) {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase004-plan-"));
  await mkdir(join(root, ".tasks", "phase003"), { recursive: true });
  await writeFile(join(root, ".tasks", "plan.md"), planText);

  const phase003Files = [
    "README.md",
    "plan.md",
    "phase-gates.md",
    "runtime-wiring-audit.md",
    "persistence-gap-audit.md",
    "durable-dependency-manifest-audit.md",
    "recovery-coverage-audit.md",
    "product-smoke-coverage-audit.md",
    "product-smoke-gate-result.md",
    "packaging-coverage-audit.md",
    "packaging-gate-result.md",
    "hardening-coverage-audit.md",
    "phase003-gate-result.md",
  ];
  for (const filename of phase003Files) {
    await writeFile(join(root, ".tasks", "phase003", filename), `${filename}\n`);
  }
  await writeFile(
    join(root, ".tasks", "phase003", "final-release-gate-result.md"),
    finalReleaseText,
  );
  for (let index = 1; index <= 59; index += 1) {
    const filename = `task${String(index).padStart(3, "0")}.md`;
    if (omitArchiveTasks.includes(filename)) {
      continue;
    }
    await writeFile(
      join(root, ".tasks", "phase003", filename),
      `# Task ${String(index).padStart(3, "0")}. Archived Phase 003 task\n`,
    );
  }
  if (activeTaskText) {
    await writeFile(join(root, ".tasks", "task001.md"), activeTaskText);
  }
  return root;
}

function completePhase004PlanText() {
  return [
    "# Phase 004 Development Plan",
    "",
    "현재 단계: Phase 004 - Knowledge Graph and Realtime Collaboration UX Expansion",
    "",
    "## 1. Project Goal",
    "Knowledge Graph and Realtime Collaboration UX Expansion with Canvas/Edgeless.",
    "## 2. Source Documents and Baseline",
    ".tasks/phase003/final-release-gate-result.md",
    "## 3. Current Plan Assessment",
    "AGENTS.md 기준과의 충돌은 발견되지 않았다.",
    "## 4. Architecture Direction",
    "Layered Architecture and Clean Architecture.",
    "## 5. Development Principles",
    "Tidy First and TDD.",
    "projection contract complete, runtime wired, product smoke passed, performance measured, collaboration safe.",
    "## 6. Implementation Phases",
    "Phase 004.0. Archive Boundary and Phase 004 Planning Gate",
    "Phase 004.1. Permission-Aware Knowledge Graph Domain and Projection Contract",
    "Phase 004.2. Graph Runtime API, Client Model, and Product Smoke",
    "Phase 004.3. Collaborative Edit Operation Model and Session State Machine",
    "Phase 004.4. Realtime Collaboration Gateway Runtime and Product Smoke",
    "Phase 004.5. Canvas and Edgeless Baseline",
    "Phase 004.6. Mobile Collaboration Baseline and Platform Capability Matrix",
    "Phase 004.7. Graph and Collaboration Performance, Observability, and Release Gate",
    "## 7. TDD Strategy",
    "Failing tests first.",
    "## 8. Tidy First Strategy",
    "Separate cleanup from feature changes.",
    "## 9. Configuration and Runtime Environment Policy",
    "bootstrap 또는 composition root에서 1회만 읽는다.",
    "## 10. Logging Strategy",
    "Product Log, Field Debug Log, Development Log.",
    "## 11. State Machine Strategy",
    "state machine.",
    "## 12. Dependency and Boundary Rules",
    "CodeMirror, Tauri, mobile SDK do not enter domain/usecase.",
    "## 13. Performance Strategy",
    "p95 300ms.",
    "## 14. Release and Validation Gates",
    "phase004_release_gate.",
    "## 15. Risk and Mitigation",
    "Unauthorized graph exposure.",
    "## 16. Review Checklist",
    "Boundary checks.",
    "## 17. Required Verification Criteria",
    "도메인 계층이 외부 프레임워크에 의존하지 않는지 확인한다.",
    "## 18. Definition of Done",
    "Complete only after release gate.",
    "## 19. Prohibited Implementation Patterns",
    "No hidden environment lookup.",
    "## 20. Next Task Decision",
    "Create task001.",
    "Web, iOS, Android, Windows, macOS, Linux.",
    "graph, collaboration, Canvas, mobile baseline, realtime gateway, platform capability matrix.",
    "Product Log, Field Debug Log, Development Log, p95 300ms.",
  ].join("\n");
}
