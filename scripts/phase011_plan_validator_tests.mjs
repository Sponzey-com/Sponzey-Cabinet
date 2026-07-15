import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  Phase011PlanErrorCode,
  Phase011PlanEvent,
  Phase011PlanState,
  renderPhase011PlanValidationArtifact,
  runPhase011PlanValidation,
  transitionPhase011PlanState,
  validatePhase011PlanArtifactFreshness,
} from "./phase011_plan_validator.mjs";

const requirementIds = [
  "SCOPE-01", "BOOT-01", "HOME-01", "NAV-01", "DOC-01", "DOC-02", "DOC-03",
  "HIST-01", "HIST-02", "DISC-01", "DATA-01", "CFG-01", "CFG-02", "LOG-01",
  "STATE-01", "PERF-01", "SEC-01", "UX-01", "PLAT-01", "COMPAT-01",
];

test("phase011 plan validator rejects missing archive prerequisite", async () => {
  const root = await createFixture({ archiveMarker: "phase011_archive_validation=failed" });

  const result = await runPhase011PlanValidation({ root, writeArtifact: false });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase011PlanErrorCode.ArchivePrerequisiteMissing);
  assert.equal(result.findingId, ".tasks/phase011-archive-validation-result.md");
});

test("phase011 plan validator rejects stale task index", async () => {
  const root = await createFixture({ activeReadmePhase: "Phase 010" });

  const result = await runPhase011PlanValidation({ root, writeArtifact: false });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase011PlanErrorCode.ReadmeScopeInvalid);
  assert.equal(result.findingId, "Active phase: Phase 011");
});

test("phase011 plan validator rejects missing required section", async () => {
  const root = await createFixture({ omitSection: "## 11. Logging Strategy" });

  const result = await runPhase011PlanValidation({ root, writeArtifact: false });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase011PlanErrorCode.PlanStructureInvalid);
  assert.equal(result.findingId, "## 11. Logging Strategy");
});

test("phase011 plan validator rejects future platform in current active platform block", async () => {
  const root = await createFixture({ activePlatforms: ["Windows", "macOS", "Linux", "Web"] });

  const result = await runPhase011PlanValidation({ root, writeArtifact: false });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase011PlanErrorCode.FutureScopeActivated);
  assert.equal(result.findingId, "Current active platforms");
});

test("phase011 plan validator rejects a gate missing required execution subsection", async () => {
  const root = await createFixture({ omitPhaseSubsection: { phase: 4, subsection: "* TDD Requirements:" } });

  const result = await runPhase011PlanValidation({ root, writeArtifact: false });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase011PlanErrorCode.PhaseStructureInvalid);
  assert.equal(result.findingId, "Phase 011.4:* TDD Requirements:");
});

test("phase011 plan validator rejects stale plan fingerprint", () => {
  const findings = validatePhase011PlanArtifactFreshness(
    "plan_fingerprint=old-fingerprint\n",
    "current-fingerprint",
  );
  assert.deepEqual(findings, [
    {
      errorCode: Phase011PlanErrorCode.PlanFingerprintMismatch,
      findingId: "plan_fingerprint",
    },
  ]);
});

test("phase011 plan validator passes complete fixture and renders safe artifact", async () => {
  const root = await createFixture();

  const result = await runPhase011PlanValidation({ root, writeArtifact: false });
  const artifact = renderPhase011PlanValidationArtifact(result);

  assert.equal(result.passed, true);
  assert.equal(result.state, Phase011PlanState.Passed);
  assert.equal(result.requirementCount, 20);
  assert.equal(result.phaseCount, 9);
  assert.equal(result.topLevelSectionCount, 18);
  assert.match(result.planFingerprint, /^[a-f0-9]{64}$/);
  assert.match(artifact, /phase011_plan_validation=passed/);
  assert.match(artifact, /release_scope=personal_local_desktop/);
  assert.match(artifact, /phase011_archive_validation=passed/);
  assert.doesNotMatch(artifact, /raw_document_body_fixture/);
  assert.doesNotMatch(artifact, /provider_api_key_fixture/);
  assert.equal(artifact.includes(root), false);
});

test("phase011 plan validator writes marker artifact to explicit root", async () => {
  const root = await createFixture();

  const result = await runPhase011PlanValidation({ root, writeArtifact: true });
  const artifact = await readFile(join(root, ".tasks/phase011-plan-validation-result.md"), "utf8");

  assert.equal(result.passed, true);
  assert.match(artifact, /validation_state=Passed/);
  assert.match(artifact, new RegExp(`plan_fingerprint=${result.planFingerprint}`));
});

test("phase011 plan state machine exposes success failure and invalid transition", () => {
  const reading = transitionPhase011PlanState(Phase011PlanState.Pending, Phase011PlanEvent.Start);
  const plan = transitionPhase011PlanState(reading.state, Phase011PlanEvent.PrerequisitesRead);
  const readme = transitionPhase011PlanState(plan.state, Phase011PlanEvent.PlanValidated);
  const writing = transitionPhase011PlanState(readme.state, Phase011PlanEvent.ReadmeValidated);
  const passed = transitionPhase011PlanState(writing.state, Phase011PlanEvent.ResultWritten);
  const failed = transitionPhase011PlanState(reading.state, Phase011PlanEvent.Fail, {
    errorCode: Phase011PlanErrorCode.ArchivePrerequisiteMissing,
    findingId: ".tasks/phase011-archive-validation-result.md",
  });
  const invalid = transitionPhase011PlanState(
    Phase011PlanState.Pending,
    Phase011PlanEvent.PlanValidated,
  );

  assert.equal(reading.state, Phase011PlanState.ReadingPrerequisites);
  assert.equal(plan.state, Phase011PlanState.ValidatingPlan);
  assert.equal(readme.state, Phase011PlanState.ValidatingReadme);
  assert.equal(writing.state, Phase011PlanState.WritingResult);
  assert.equal(passed.state, Phase011PlanState.Passed);
  assert.equal(failed.state, Phase011PlanState.Failed);
  assert.equal(invalid.errorCode, Phase011PlanErrorCode.InvalidTransition);
});

async function createFixture({
  archiveMarker = "phase011_archive_validation=passed",
  activeReadmePhase = "Phase 011",
  omitSection,
  activePlatforms = ["Windows", "macOS", "Linux"],
  omitPhaseSubsection,
} = {}) {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase011-plan-"));
  await mkdir(join(root, ".tasks", "release"), { recursive: true });

  await writeFile(
    join(root, ".tasks/phase011-archive-validation-result.md"),
    `${archiveMarker}\nrelease_scope=personal_local_desktop\nsource_fingerprint=${"a".repeat(64)}\n`,
  );
  await writeFile(
    join(root, ".tasks/phase011-current-implementation-inventory.md"),
    `phase011_current_inventory=passed\nproduct_scope=personal_local_desktop\nsource_fingerprint=${"a".repeat(64)}\n`,
  );
  await writeFile(
    join(root, ".tasks/release/requirement-evidence-matrix-phase011.md"),
    `phase011_requirement_evidence=pending\nrequirement_count=20\nsource_fingerprint=${"a".repeat(64)}\n`,
  );
  await writeFile(
    join(root, ".tasks/readme.md"),
    [
      "# Sponzey Cabinet Task Index",
      `Active phase: ${activeReadmePhase}`,
      "Current product scope: `personal_local_desktop`",
      "Phase 011 root tasks restart at `.tasks/task001.md`.",
      "Current target: personal PC installable local-first desktop app for Windows, macOS, and Linux",
      "Current exclusions: server hosting, SaaS, multi-user, admin console, mobile implementation",
      "Archive phases: Phase 001 through Phase 010",
      "Release evidence must use marker files and command results, not task checkbox text.",
    ].join("\n"),
  );
  await writeFile(
    join(root, ".tasks/plan.md"),
    createPlan({ omitSection, activePlatforms, omitPhaseSubsection }),
  );
  return root;
}

function createPlan({ omitSection, activePlatforms, omitPhaseSubsection }) {
  const sections = [
    "## 1. Project Goal",
    "## 2. Current Plan Assessment",
    "## 3. Architecture Direction",
    "## 4. Development Principles",
    "## 5. Required Evidence Manifest",
    "## 6. Phase 011 Task Execution Controls",
    "## 7. Phase 011 Active-Scope Validation Command Matrix",
    "## 8. Implementation Phases",
    "## 9. TDD Strategy",
    "## 10. Configuration and Runtime Environment Policy",
    "## 11. Logging Strategy",
    "## 12. State Machine Strategy",
    "## 13. Dependency and Boundary Rules",
    "## 14. Risk and Mitigation",
    "## 15. Review Checklist",
    "## 16. Definition of Done",
    "## 17. Prohibited Implementation Patterns",
    "## 18. Next Actions",
  ].filter((section) => section !== omitSection);
  const phaseSubsections = [
    "* Goal:", "* Scope:", "* Required Changes:", "* Architecture Notes:",
    "* TDD Requirements:", "* Configuration Rules:", "* Logging Rules:",
    "* State Management:", "* Validation:", "* Done Criteria:", "* Risks:",
  ];
  const phases = Array.from({ length: 9 }, (_, phase) => [
    `### Phase 011.${phase}. Gate`,
    ...phaseSubsections
      .filter((subsection) => !(omitPhaseSubsection?.phase === phase && omitPhaseSubsection.subsection === subsection))
      .flatMap((subsection) => [subsection, "  - explicit validation rule"]),
  ].join("\n"));
  const markers = [
    "phase011_archive_validation=passed", "phase011_plan_validation=passed",
    "phase011_workspace_home_gate=passed", "phase011_document_authoring_gate=passed",
    "phase011_history_restore_gate=passed", "phase011_discovery_gate=passed",
    "phase011_data_settings_gate=passed", "phase011_recovery_observability_gate=passed",
    "phase011_product_smoke_gate=passed", "phase011_release_gate=passed",
    "phase011_performance_budget=passed", "phase011_product_log_matrix=passed",
    "phase011_security_log_manifest=passed", "phase011_runbook=passed",
    "phase011_requirement_evidence=passed", "phase011_visual_accessibility=passed",
    "phase011_native_platform_matrix=passed", "phase011_phase010_compatibility=passed",
  ];
  return [
    "# Phase 011 Development Plan",
    "Current product scope marker: `personal_local_desktop`.",
    "Current active platforms:",
    ...activePlatforms.map((platform) => `- ${platform} desktop installable app`),
    "Current active stack:",
    "- local desktop stack",
    ...requirementIds.map((id) => `| \`${id}\` | requirement | evidence |`),
    "Layered Architecture",
    "Clean Architecture",
    "Tidy First",
    "TDD",
    "Runtime environment is read only at bootstrap/composition root.",
    "Product Log",
    "Field Debug Log",
    "Development Log",
    "Performance Measurement Contract",
    "p95 300ms",
    "Desktop UI, Accessibility, And Visual Validation Contract",
    "State Machine Strategy",
    "Markdown/HTML rendering security",
    "Windows, macOS, and Linux native evidence",
    "Phase 010 compatibility",
    "Phase 011 explicitly does not build:",
    "- server hosting runtime",
    "- SaaS runtime",
    "- multi-user collaboration",
    "- iOS/Android product implementation",
    ...markers,
    ...sections.slice(0, 8),
    ...phases,
    ...sections.slice(8),
    "Do not use task checkbox text as release evidence.",
    "External settings files must remain minimal.",
    "Environment values are explicit constructor arguments after bootstrap.",
  ].join("\n\n");
}
