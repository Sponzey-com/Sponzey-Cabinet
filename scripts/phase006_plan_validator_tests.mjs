import assert from "node:assert/strict";
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  Phase006PlanErrorCode,
  Phase006PlanEvent,
  Phase006PlanState,
  renderPhase006PlanValidationResult,
  runPhase006PlanValidation,
  transitionPhase006PlanState,
  validatePhase006PlanText,
  validatePhase006ReadmeText,
} from "./phase006_plan_validator.mjs";

test("phase006 plan validator rejects missing phase005 archived task", async () => {
  const root = await createFixtureRoot({
    planText: completePhase006PlanText(),
    readmeText: completePhase006ReadmeText(),
    omitArchiveTasks: ["task031.md"],
  });

  const result = await runPhase006PlanValidation({ root });
  const rendered = renderPhase006PlanValidationResult(result);

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase006PlanErrorCode.Phase005ArchiveMissing);
  assert.equal(result.findingId, ".tasks/phase005/task031.md");
  assert.match(rendered, /phase006_plan_validation=failed/);
  assert.match(rendered, /error_code=PHASE005_ARCHIVE_MISSING/);
});

test("phase006 plan validator rejects missing phase005 final release marker", async () => {
  const root = await createFixtureRoot({
    planText: completePhase006PlanText(),
    readmeText: completePhase006ReadmeText(),
    finalReleaseText: "# Phase 005 Final Release Gate Result\nphase005_release_gate=failed\n",
  });

  const result = await runPhase006PlanValidation({ root });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase006PlanErrorCode.Phase005ReleaseMarkerMissing);
  assert.equal(result.findingId, ".tasks/phase005/phase005-release-gate-result.md");
});

test("phase006 plan validator rejects stale active phase005 root task", async () => {
  const root = await createFixtureRoot({
    planText: completePhase006PlanText(),
    readmeText: completePhase006ReadmeText(),
    activeTaskText: "# Task 001. Phase 005 stale active task\n",
  });

  const result = await runPhase006PlanValidation({ root });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase006PlanErrorCode.ActiveArchiveConflict);
  assert.equal(result.findingId, ".tasks/task001.md");
});

test("phase006 plan validator rejects phase005 manifest target outside archive path", async () => {
  const root = await createFixtureRoot({
    planText: completePhase006PlanText(),
    readmeText: completePhase006ReadmeText(),
    securityManifest: {
      schemaVersion: 1,
      logClasses: defaultLogClasses(),
      deniedFixtures: defaultDeniedFixtures(),
      scanTargets: [
        {
          id: "phase005_ai_answer_product_gate_result",
          path: ".tasks/ai-answer-product-gate-result.md",
          required: true,
        },
      ],
    },
  });

  const result = await runPhase006PlanValidation({ root });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase006PlanErrorCode.Phase005SecurityManifestPathInvalid);
  assert.equal(result.findingId, ".tasks/ai-answer-product-gate-result.md");
});

test("phase006 plan validator rejects missing required phase006 planning term", () => {
  const findings = validatePhase006PlanText(
    completePhase006PlanText().replaceAll("personal_local_desktop", "personal-desktop"),
  );

  assert.equal(findings[0].errorCode, Phase006PlanErrorCode.Phase006PlanTermMissing);
  assert.equal(findings[0].findingId, "personal_local_desktop");
});

test("phase006 plan validator rejects stale or missing task readme pointer", () => {
  const findings = validatePhase006ReadmeText(
    completePhase006ReadmeText().replaceAll("Active phase: Phase 006", "Active phase: Phase 005"),
  );

  assert.equal(findings[0].errorCode, Phase006PlanErrorCode.Phase006ReadmeTermMissing);
  assert.equal(findings[0].findingId, "Active phase: Phase 006");
});

test("phase006 plan validator passes complete phase006 fixture before active task creation", async () => {
  const root = await createFixtureRoot({
    planText: completePhase006PlanText(),
    readmeText: completePhase006ReadmeText(),
  });

  const result = await runPhase006PlanValidation({ root });
  const rendered = renderPhase006PlanValidationResult(result);

  assert.equal(result.passed, true);
  assert.equal(result.state, Phase006PlanState.Passed);
  assert.equal(result.archiveFileCount >= 40, true);
  assert.equal(result.requiredTermCount >= 30, true);
  assert.match(rendered, /phase006_plan_validation=passed/);
  assert.match(rendered, /validation_state=Passed/);
});

test("phase006 plan validator allows active phase006 task001 while rejecting archived phase markers", async () => {
  const root = await createFixtureRoot({
    planText: completePhase006PlanText(),
    readmeText: completePhase006ReadmeText(),
    activeTaskText: "# Task 001. Phase 006 Archive Boundary and Planning Gate\n",
  });

  const result = await runPhase006PlanValidation({ root });

  assert.equal(result.passed, true);
  assert.equal(result.state, Phase006PlanState.Passed);
});

test("phase006 plan state machine exposes explicit failure and terminal states", () => {
  const readingArchive = transitionPhase006PlanState(
    Phase006PlanState.NotStarted,
    Phase006PlanEvent.Start,
  );
  const validatingPlan = transitionPhase006PlanState(
    readingArchive.state,
    Phase006PlanEvent.ArchiveChecked,
  );
  const validatingReadme = transitionPhase006PlanState(
    validatingPlan.state,
    Phase006PlanEvent.PlanChecked,
  );
  const passed = transitionPhase006PlanState(
    validatingReadme.state,
    Phase006PlanEvent.ReadmeChecked,
  );
  const failed = transitionPhase006PlanState(validatingPlan.state, Phase006PlanEvent.Fail, {
    errorCode: Phase006PlanErrorCode.Phase006PlanTermMissing,
    findingId: "personal_local_desktop",
  });
  const invalid = transitionPhase006PlanState(
    Phase006PlanState.NotStarted,
    Phase006PlanEvent.PlanChecked,
  );

  assert.equal(readingArchive.state, Phase006PlanState.ReadingArchive);
  assert.equal(validatingPlan.state, Phase006PlanState.ValidatingPlan);
  assert.equal(validatingReadme.state, Phase006PlanState.ValidatingReadme);
  assert.equal(passed.state, Phase006PlanState.Passed);
  assert.equal(failed.state, Phase006PlanState.Failed);
  assert.equal(failed.findingId, "personal_local_desktop");
  assert.equal(invalid.errorCode, Phase006PlanErrorCode.InvalidTransition);
});

async function createFixtureRoot({
  planText,
  readmeText,
  activeTaskText,
  finalReleaseText = [
    "# Phase 005 Final Release Gate Result",
    "",
    "phase005_release_gate=passed",
    "",
    "- release conclusion: `AI and external integration platform complete`",
  ].join("\n"),
  omitArchiveTasks = [],
  securityManifest = defaultSecurityManifest(),
}) {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase006-plan-"));
  await mkdir(join(root, ".tasks", "phase005"), { recursive: true });
  await mkdir(join(root, ".tasks", "release"), { recursive: true });
  await writeFile(join(root, ".tasks", "plan.md"), planText);
  await writeFile(join(root, ".tasks", "readme.md"), readmeText);
  await writeFile(
    join(root, ".tasks", "release", "security-log-policy-manifest.json"),
    `${JSON.stringify(securityManifest, null, 2)}\n`,
  );

  const phase005Files = [
    "plan.md",
    "retrieval-coverage-audit.md",
    "semantic-search-gate-result.md",
    "ai-answer-product-gate-result.md",
    "mcp-api-product-gate-result.md",
    "webhook-connector-product-gate-result.md",
    "phase005-product-smoke-gate-result.md",
    "phase005-observability-matrix-gate-result.md",
  ];
  for (const filename of phase005Files) {
    await writeFile(join(root, ".tasks", "phase005", filename), `${filename}\n`);
  }
  await writeFile(
    join(root, ".tasks", "phase005", "phase005-release-gate-result.md"),
    finalReleaseText,
  );
  for (let index = 1; index <= 31; index += 1) {
    const filename = `task${String(index).padStart(3, "0")}.md`;
    if (omitArchiveTasks.includes(filename)) {
      continue;
    }
    await writeFile(
      join(root, ".tasks", "phase005", filename),
      `# Task ${String(index).padStart(3, "0")}. Archived Phase 005 task\n`,
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
    logClasses: defaultLogClasses(),
    deniedFixtures: defaultDeniedFixtures(),
    scanTargets: [
      {
        id: "phase005_ai_answer_product_gate_result",
        path: ".tasks/phase005/ai-answer-product-gate-result.md",
        required: true,
      },
      {
        id: "phase005_final_release_gate_result",
        path: ".tasks/phase005/phase005-release-gate-result.md",
        required: true,
      },
      {
        id: "phase006_plan_validation_result",
        path: ".tasks/phase006-plan-validation-result.md",
        required: false,
      },
    ],
  };
}

function defaultLogClasses() {
  return [
    {
      name: "Product Log",
      allowedFields: ["event_name", "correlation_id", "status", "error_code"],
      deniedFields: ["document_body", "raw_prompt", "ai_answer", "provider_api_key"],
    },
    {
      name: "Field Debug Log",
      allowedFields: ["scope", "ttl_seconds", "event_name", "state"],
      deniedFields: ["document_body", "raw_prompt", "ai_answer", "provider_api_key"],
    },
    {
      name: "Development Log",
      allowedFields: ["fixture_id", "state", "event"],
      deniedFields: ["production_default", "document_body", "raw_prompt", "provider_api_key"],
    },
  ];
}

function defaultDeniedFixtures() {
  return [
    {
      id: "phase006_raw_document_fixture",
      kind: "document_body",
      value: "phase006-raw-document-body-should-not-log",
    },
  ];
}

function completePhase006ReadmeText() {
  return [
    "# Sponzey Cabinet Task Index",
    "",
    "Active phase: Phase 006",
    "",
    "Active plan: `.tasks/plan.md`",
    "",
    "Archive phases: Phase 001, Phase 002, Phase 003, Phase 004, Phase 005",
    "",
    "Current product scope: personal_local_desktop",
    "",
    "Phase 006 root tasks restart at `.tasks/task001.md`.",
  ].join("\n");
}

function completePhase006PlanText() {
  return [
    "# Phase 006 Development Plan",
    "",
    "현재 단계: Phase 006 - Personal Desktop Productization and Local Workspace UX",
    "",
    "## 1. Project Goal",
    "personal_local_desktop install once no server required no SaaS no multi-user local workspace modern UI/UX CodeMirror preview p95 300ms.",
    "## 2. Source Documents and Baseline",
    ".tasks/phase005/phase005-release-gate-result.md and `.tasks/readme.md`.",
    "## 3. Current Plan Assessment",
    "Phase 005 archive complete and server/SaaS scope excluded.",
    "## 4. Architecture Direction",
    "Tauri Desktop Shell, React Workspace Shell, CodeMirror Editor, client-core local workspace facade, Rust usecases, domain, local adapters.",
    "## 5. Development Principles",
    "Layered Architecture, Clean Architecture, Tidy First, TDD, install once, no server required, current/history split, preview is product.",
    "## 6. Implementation Phases",
    "Phase 006.0. Archive Boundary and Phase 006 Planning Gate",
    "Phase 006.1. Local Desktop Workspace Runtime and First-Run Product Contract",
    "Phase 006.2. Personal Workspace Shell and Navigation UX",
    "Phase 006.3. Document Editor, Markdown Preview, History, and Restore UX",
    "Phase 006.4. Local Search, Graph, Canvas, and Asset Panels",
    "Phase 006.5. Local AI Query, Citation, and Tool Scope UX",
    "Phase 006.6. Backup, Import/Export, Packaging, and Final Desktop Release Gate",
    "Phase 006 required artifacts",
    ".tasks/phase006-plan-validation-result.md",
    "phase006_plan_validation=passed",
    "phase006_local_runtime_gate=passed",
    "phase006_workspace_shell_gate=passed",
    "phase006_document_ux_gate=passed",
    "phase006_search_graph_asset_gate=passed",
    "phase006_ai_ux_gate=passed",
    "phase006_backup_package_gate=passed",
    "phase006_product_smoke_gate=passed",
    "phase006_release_gate=passed",
    "## 7. Task Slicing Rules",
    "Archive/validator task, Local runtime task, UI model task.",
    "## 8. TDD Strategy",
    "Failing tests first.",
    "## 9. Configuration and Runtime Environment Policy",
    "bootstrap에서 최초 1회 수신하고 명시적으로 전달한다.",
    "## 10. Logging Strategy",
    "Product Log, Field Debug Log, Development Log, privacy safe logs.",
    "## 11. State Machine Strategy",
    "state machine NotStarted ReadingArchive ValidatingPlan ValidatingReadme Passed Failed.",
    "## 12. Performance Strategy",
    "p95 300ms performance budget phase006.",
    "## 13. Dependency and Boundary Rules",
    "Domain must not depend on framework Tauri browser API filesystem environment variable.",
    "## 14. Risk and Mitigation",
    "CodeMirror source editor and Markdown preview renderer responsibilities separated.",
    "## 15. Review Checklist",
    "No server URL tenant organization team invite SSO billing admin console.",
    "## 16. Definition of Done",
    "Phase 006 final release artifact must record phase006_release_gate=passed.",
    "## 17. Prohibited Implementation Patterns",
    "No hidden config, no raw document logging, no server/SaaS implementation.",
    "## 18. Next Actions",
    "Create .tasks/readme.md and Phase 006 plan validator tests.",
    "personal_local_desktop, Phase 006, Windows, macOS, Linux, desktop local, AI citation, backup, import/export.",
    "개인 PC 설치형 로컬 앱이며 server/SaaS 기능은 차후 호환 경계로만 유지한다.",
  ].join("\n");
}
