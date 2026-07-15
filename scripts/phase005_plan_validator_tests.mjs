import assert from "node:assert/strict";
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  Phase005PlanErrorCode,
  Phase005PlanEvent,
  Phase005PlanState,
  renderPhase005PlanValidationResult,
  runPhase005PlanValidation,
  transitionPhase005PlanState,
  validatePhase005PlanText,
} from "./phase005_plan_validator.mjs";

test("phase005 plan validator rejects missing phase004 archived task", async () => {
  const root = await createFixtureRoot({
    planText: completePhase005PlanText(),
    omitArchiveTasks: ["task037.md"],
  });

  const result = await runPhase005PlanValidation({ root });
  const rendered = renderPhase005PlanValidationResult(result);

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase005PlanErrorCode.Phase004ArchiveMissing);
  assert.equal(result.findingId, ".tasks/phase004/task037.md");
  assert.match(rendered, /phase005_plan_validation=failed/);
  assert.match(rendered, /error_code=PHASE004_ARCHIVE_MISSING/);
});

test("phase005 plan validator rejects missing phase004 final release marker", async () => {
  const root = await createFixtureRoot({
    planText: completePhase005PlanText(),
    finalReleaseText: "# Final\nphase004_release_gate=failed\n",
  });

  const result = await runPhase005PlanValidation({ root });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase005PlanErrorCode.Phase004ReleaseMarkerMissing);
  assert.equal(result.findingId, ".tasks/phase004/phase004-final-release-gate-result.md");
});

test("phase005 plan validator rejects stale active phase004 root task", async () => {
  const root = await createFixtureRoot({
    planText: completePhase005PlanText(),
    activeTaskText: "# Task 001. Phase 004 stale active task\n",
  });

  const result = await runPhase005PlanValidation({ root });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase005PlanErrorCode.ActiveArchiveConflict);
  assert.equal(result.findingId, ".tasks/task001.md");
});

test("phase005 plan validator rejects phase004 manifest target outside archive path", async () => {
  const root = await createFixtureRoot({
    planText: completePhase005PlanText(),
    securityManifest: {
      schemaVersion: 1,
      scanTargets: [
        {
          id: "phase004_graph_coverage_audit",
          path: ".tasks/graph-coverage-audit.md",
          required: true,
        },
      ],
    },
  });

  const result = await runPhase005PlanValidation({ root });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase005PlanErrorCode.Phase004SecurityManifestPathInvalid);
  assert.equal(result.findingId, ".tasks/graph-coverage-audit.md");
});

test("phase005 plan validator rejects missing required phase005 planning term", () => {
  const findings = validatePhase005PlanText(
    completePhase005PlanText().replaceAll("provider boundary", "provider edge"),
  );

  assert.equal(findings[0].errorCode, Phase005PlanErrorCode.Phase005PlanTermMissing);
  assert.equal(findings[0].findingId, "provider boundary");
});

test("phase005 plan validator passes complete phase005 fixture with active phase005 task", async () => {
  const root = await createFixtureRoot({
    planText: completePhase005PlanText(),
    activeTaskText: "# Task 001. Phase 005 Planning and Archive Boundary Gate\n",
  });

  const result = await runPhase005PlanValidation({ root });
  const rendered = renderPhase005PlanValidationResult(result);

  assert.equal(result.passed, true);
  assert.equal(result.state, Phase005PlanState.Passed);
  assert.equal(result.archiveFileCount >= 45, true);
  assert.equal(result.requiredTermCount >= 20, true);
  assert.match(rendered, /phase005_plan_validation=passed/);
  assert.match(rendered, /validation_state=Passed/);
});

test("phase005 plan validator allows active task without literal phase marker when it is not stale phase004 work", async () => {
  const root = await createFixtureRoot({
    planText: completePhase005PlanText(),
    activeTaskText: [
      "# Task 002. Retrieval Domain Value Objects",
      "",
      "- This task contributes to `.tasks/plan.md` permission-aware retrieval.",
      "- It does not declare stale archived work.",
    ].join("\n"),
  });

  const result = await runPhase005PlanValidation({ root });

  assert.equal(result.passed, true);
  assert.equal(result.state, Phase005PlanState.Passed);
});

test("phase005 plan state machine exposes explicit failure and terminal states", () => {
  const checkingArchive = transitionPhase005PlanState(
    Phase005PlanState.Pending,
    Phase005PlanEvent.Start,
  );
  const checkingPlan = transitionPhase005PlanState(
    checkingArchive.state,
    Phase005PlanEvent.ArchiveChecked,
  );
  const passed = transitionPhase005PlanState(
    checkingPlan.state,
    Phase005PlanEvent.PlanChecked,
  );
  const failed = transitionPhase005PlanState(checkingPlan.state, Phase005PlanEvent.Fail, {
    errorCode: Phase005PlanErrorCode.Phase005PlanTermMissing,
    findingId: "provider boundary",
  });
  const invalid = transitionPhase005PlanState(
    Phase005PlanState.Pending,
    Phase005PlanEvent.PlanChecked,
  );

  assert.equal(checkingArchive.state, Phase005PlanState.CheckingArchive);
  assert.equal(checkingPlan.state, Phase005PlanState.CheckingPlan);
  assert.equal(passed.state, Phase005PlanState.Passed);
  assert.equal(failed.state, Phase005PlanState.Failed);
  assert.equal(failed.findingId, "provider boundary");
  assert.equal(invalid.errorCode, Phase005PlanErrorCode.InvalidTransition);
});

async function createFixtureRoot({
  planText,
  activeTaskText,
  finalReleaseText = [
    "# Phase 004 Final Release Gate Result",
    "",
    "phase004_release_gate=passed",
    "",
    "- release conclusion: `knowledge graph and realtime collaboration UX expansion complete`",
  ].join("\n"),
  omitArchiveTasks = [],
  securityManifest = defaultSecurityManifest(),
}) {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase005-plan-"));
  await mkdir(join(root, ".tasks", "phase004"), { recursive: true });
  await mkdir(join(root, ".tasks", "release"), { recursive: true });
  await writeFile(join(root, ".tasks", "plan.md"), planText);
  await writeFile(
    join(root, ".tasks", "release", "security-log-policy-manifest.json"),
    `${JSON.stringify(securityManifest, null, 2)}\n`,
  );

  const phase004Files = [
    "README.md",
    "plan.md",
    "phase-gates.md",
    "graph-coverage-audit.md",
    "graph-product-gate-result.md",
    "realtime-collaboration-smoke-result.md",
    "collaboration-coverage-audit.md",
    "realtime-collaboration-product-gate-result.md",
    "canvas-coverage-audit.md",
    "canvas-product-smoke-result.md",
    "mobile-capability-audit.md",
    "phase004-product-smoke-gate-result.md",
  ];
  for (const filename of phase004Files) {
    await writeFile(join(root, ".tasks", "phase004", filename), `${filename}\n`);
  }
  await writeFile(
    join(root, ".tasks", "phase004", "phase004-final-release-gate-result.md"),
    finalReleaseText,
  );
  for (let index = 1; index <= 37; index += 1) {
    const filename = `task${String(index).padStart(3, "0")}.md`;
    if (omitArchiveTasks.includes(filename)) {
      continue;
    }
    await writeFile(
      join(root, ".tasks", "phase004", filename),
      `# Task ${String(index).padStart(3, "0")}. Archived Phase 004 task\n`,
    );
  }
  if (activeTaskText) {
    await writeFile(join(root, ".tasks", "task001.md"), activeTaskText);
  }
  return root;
}

function defaultSecurityManifest() {
  return {
    schemaVersion: 1,
    scanTargets: [
      {
        id: "phase004_graph_coverage_audit",
        path: ".tasks/phase004/graph-coverage-audit.md",
        required: true,
      },
      {
        id: "phase004_final_release_gate_result",
        path: ".tasks/phase004/phase004-final-release-gate-result.md",
        required: true,
      },
    ],
  };
}

function completePhase005PlanText() {
  return [
    "# Phase 005 Development Plan",
    "",
    "현재 단계: Phase 005 - AI and External Integration Platform",
    "",
    "## 1. Project Goal",
    "AI-ready Knowledge Base product.",
    "## 2. Source Documents and Baseline",
    ".tasks/phase004/phase004-final-release-gate-result.md",
    "## 3. Current Plan Assessment",
    "AGENTS.md 기준과의 충돌은 발견되지 않았다.",
    "## 4. Architecture Direction",
    "Layered Architecture and Clean Architecture.",
    "## 5. Development Principles",
    "Tidy First and TDD.",
    "retrieval contract complete, provider boundary complete, privacy safe, async by default, query p95 measured, platform consistent.",
    "## 6. Implementation Phases",
    "Phase 005.0. Archive Boundary and Phase 005 Planning Gate",
    "Phase 005.1. Permission-Aware Retrieval Domain and Source Contract",
    "Phase 005.2. Semantic Search and Vector Index Pipeline",
    "Phase 005.3. AI Answer, Citation, Summary, and Refusal Boundary",
    "Phase 005.4. MCP Server and Public Tool API Boundary",
    "Phase 005.5. Webhook, Event Stream, and Delivery Runtime",
    "Phase 005.6. External Connector Authorization and Baseline Integrations",
    "Phase 005.7. Cross-Platform AI Query UX and Product Smoke",
    "Phase 005.8. AI and Integration Observability, Runbooks, and Release Gate",
    "## 7. TDD Strategy",
    "Failing tests first.",
    "## 8. Tidy First Policy",
    "Separate cleanup from feature changes.",
    "## 9. Configuration and Runtime Environment Policy",
    "bootstrap/composition root에서 최초 1회 수신한다.",
    "## 10. Logging Strategy",
    "Product Log, Field Debug Log, Development Log.",
    "## 11. State Machine Strategy",
    "state machine.",
    "## 12. Performance Strategy",
    "p95 300ms.",
    "## 13. Dependency and Boundary Rules",
    "provider SDK, MCP library, HTTP framework do not enter domain/usecase.",
    "## 14. Release Gates",
    "phase005_plan_validation, phase005_release_gate.",
    "## 15. Risk and Mitigation",
    "Provider leakage risk.",
    "## 16. Review Checklist",
    "Boundary checks.",
    "## 16.1. Definition of Ready",
    "Ready before implementation.",
    "## 17. Definition of Done",
    "Complete only after release gate.",
    "## 18. Prohibited Implementation Patterns",
    "No hidden environment lookup.",
    "## 19. Next Actions",
    "Create task001.",
    "AI, semantic search, permission-aware retrieval, MCP, webhook, event stream, connector, citation, provider boundary, p95 300ms.",
    "answer generation, retrieval-only, embedding, vector index, AI provider, connector gateway, webhook delivery.",
    "Web, iOS, Android, Windows, macOS, Linux.",
    "Product Log, Field Debug Log, Development Log, state machine.",
  ].join("\n");
}
