import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  Phase010DurableAuthoringErrorCode,
  Phase010DurableAuthoringEvent,
  Phase010DurableAuthoringState,
  buildPhase010DurableAuthoringCommandPlan,
  buildPhase010DurableAuthoringPerformanceBudget,
  evaluatePhase010DurableAuthoringGate,
  renderPhase010DurableAuthoringArtifact,
  renderPhase010DurableAuthoringPerformanceBudget,
  runPhase010DurableAuthoringGate,
  transitionPhase010DurableAuthoringState,
} from "./phase010_durable_authoring_gate.mjs";

test("phase010 durable authoring gate rejects missing first-run prerequisite", async () => {
  const root = await createDurableAuthoringFixture({
    firstRunText: "phase010_first_run_workspace_gate=failed\n",
  });

  const result = await runPhase010DurableAuthoringGate({
    root,
    writeArtifacts: false,
    runner: passingRunner,
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010DurableAuthoringErrorCode.FirstRunMarkerMissing);
  assert.equal(result.findingId, ".tasks/phase010-first-run-workspace-gate-result.md");
});

test("phase010 durable authoring gate rejects failed document command", () => {
  const result = evaluatePhase010DurableAuthoringGate({
    firstRunText: "phase010_first_run_workspace_gate=passed",
    commandResults: {
      ...passingCommandResults(),
      documentCurrent: { passed: false, exitCode: 101, command: "cargo test current" },
    },
    performanceBudgetRows: passingBudgetRows(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010DurableAuthoringErrorCode.DocumentTestsFailed);
  assert.equal(result.failedStepId, "documentCurrent");
});

test("phase010 durable authoring gate rejects failed ui command", () => {
  const result = evaluatePhase010DurableAuthoringGate({
    firstRunText: "phase010_first_run_workspace_gate=passed",
    commandResults: {
      ...passingCommandResults(),
      uiAuthoringRestoreModels: { passed: false, exitCode: 1, command: "node ui tests" },
    },
    performanceBudgetRows: passingBudgetRows(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010DurableAuthoringErrorCode.UiTestsFailed);
  assert.equal(result.failedStepId, "uiAuthoringRestoreModels");
});

test("phase010 durable authoring gate rejects p95 budget failure", () => {
  const result = evaluatePhase010DurableAuthoringGate({
    firstRunText: "phase010_first_run_workspace_gate=passed",
    commandResults: passingCommandResults(),
    performanceBudgetRows: [
      ...passingBudgetRows(),
      { path: "restore preview", p95Ms: 301, budgetMs: 300, validatedBy: "fixture" },
    ],
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010DurableAuthoringErrorCode.PerformanceBudgetFailed);
  assert.equal(result.findingId, "restore preview");
});

test("phase010 durable authoring command plan is local desktop only", () => {
  const steps = buildPhase010DurableAuthoringCommandPlan();

  assert.deepEqual(
    steps.map((step) => step.id),
    [
      "documentCreate",
      "documentUpdate",
      "documentCurrent",
      "documentHistory",
      "documentVersion",
      "restorePreview",
      "restoreApply",
      "localDocumentRepository",
      "localVersionStore",
      "localDurableAuthoring",
      "uiAuthoringRestoreModels",
      "desktopPersistence",
    ],
  );
  assert.ok(steps.every((step) => !step.command.join(" ").includes("self-host")));
  assert.ok(steps.every((step) => !step.command.join(" ").includes("mobile")));
  assert.ok(steps.every((step) => !step.command.join(" ").includes("server-base-url")));
});

test("phase010 durable authoring gate passes complete evidence and renders safe artifacts", () => {
  const result = evaluatePhase010DurableAuthoringGate({
    firstRunText: "phase010_first_run_workspace_gate=passed",
    commandResults: passingCommandResults(),
    performanceBudgetRows: passingBudgetRows(),
  });
  const artifact = renderPhase010DurableAuthoringArtifact(result);
  const budgetArtifact = renderPhase010DurableAuthoringPerformanceBudget(
    buildPhase010DurableAuthoringPerformanceBudget(passingBudgetRows()),
  );

  assert.equal(result.passed, true);
  assert.equal(result.state, Phase010DurableAuthoringState.Passed);
  assert.match(artifact, /phase010_durable_authoring_gate=passed/);
  assert.match(artifact, /restart_persistence=verified/);
  assert.match(budgetArtifact, /phase010_performance_budget=passed/);
  assert.match(budgetArtifact, /current document read/);
  assert.doesNotMatch(artifact, /raw_document_body_fixture/);
  assert.doesNotMatch(artifact, /provider_api_key_fixture/);
  assert.doesNotMatch(artifact, /personal_absolute_path_fixture/);
});

test("phase010 durable authoring gate writes marker and performance artifacts", async () => {
  const root = await createDurableAuthoringFixture();

  const result = await runPhase010DurableAuthoringGate({
    root,
    writeArtifacts: true,
    runner: passingRunner,
  });
  const marker = await readFile(
    join(root, ".tasks", "phase010-durable-authoring-gate-result.md"),
    "utf8",
  );
  const performance = await readFile(
    join(root, ".tasks", "release", "performance-budget-phase010.md"),
    "utf8",
  );

  assert.equal(result.passed, true);
  assert.match(marker, /phase010_durable_authoring_gate=passed/);
  assert.match(performance, /phase010_performance_budget=passed/);
});

test("phase010 durable authoring state machine exposes success, failure, and invalid transition", () => {
  const reading = transitionPhase010DurableAuthoringState(
    Phase010DurableAuthoringState.Pending,
    Phase010DurableAuthoringEvent.Start,
  );
  const runningDocument = transitionPhase010DurableAuthoringState(
    reading.state,
    Phase010DurableAuthoringEvent.PrerequisitesRead,
  );
  const runningUi = transitionPhase010DurableAuthoringState(
    runningDocument.state,
    Phase010DurableAuthoringEvent.DocumentTestsPassed,
  );
  const writingPerformance = transitionPhase010DurableAuthoringState(
    runningUi.state,
    Phase010DurableAuthoringEvent.UiTestsPassed,
  );
  const writingResult = transitionPhase010DurableAuthoringState(
    writingPerformance.state,
    Phase010DurableAuthoringEvent.PerformanceBudgetWritten,
  );
  const passed = transitionPhase010DurableAuthoringState(
    writingResult.state,
    Phase010DurableAuthoringEvent.ResultWritten,
  );
  const failed = transitionPhase010DurableAuthoringState(reading.state, Phase010DurableAuthoringEvent.Fail, {
    errorCode: Phase010DurableAuthoringErrorCode.FirstRunMarkerMissing,
    findingId: ".tasks/phase010-first-run-workspace-gate-result.md",
  });
  const invalid = transitionPhase010DurableAuthoringState(
    Phase010DurableAuthoringState.Pending,
    Phase010DurableAuthoringEvent.UiTestsPassed,
  );

  assert.equal(reading.state, Phase010DurableAuthoringState.ReadingPrerequisites);
  assert.equal(runningDocument.state, Phase010DurableAuthoringState.RunningDocumentTests);
  assert.equal(runningUi.state, Phase010DurableAuthoringState.RunningUiTests);
  assert.equal(writingPerformance.state, Phase010DurableAuthoringState.WritingPerformanceBudget);
  assert.equal(writingResult.state, Phase010DurableAuthoringState.WritingResult);
  assert.equal(passed.state, Phase010DurableAuthoringState.Passed);
  assert.equal(failed.state, Phase010DurableAuthoringState.Failed);
  assert.equal(invalid.errorCode, Phase010DurableAuthoringErrorCode.InvalidTransition);
});

async function createDurableAuthoringFixture({
  firstRunText = "phase010_first_run_workspace_gate=passed\n",
} = {}) {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase010-durable-authoring-"));
  await mkdir(join(root, ".tasks"), { recursive: true });
  await writeFile(join(root, ".tasks", "phase010-first-run-workspace-gate-result.md"), firstRunText);
  return root;
}

function passingCommandResults() {
  return Object.fromEntries(
    buildPhase010DurableAuthoringCommandPlan().map((step) => [
      step.id,
      { command: step.command.join(" "), passed: true, exitCode: 0, durationMs: 5 },
    ]),
  );
}

function passingBudgetRows() {
  return [
    { path: "current document read", p95Ms: 300, budgetMs: 300, validatedBy: "local durable flow" },
    { path: "history list", p95Ms: 300, budgetMs: 300, validatedBy: "local durable flow" },
    { path: "specific version read", p95Ms: 300, budgetMs: 300, validatedBy: "local durable flow" },
    { path: "restore preview status", p95Ms: 300, budgetMs: 300, validatedBy: "local durable flow" },
  ];
}

async function passingRunner() {
  return { exitCode: 0, signal: null, durationMs: 5 };
}
