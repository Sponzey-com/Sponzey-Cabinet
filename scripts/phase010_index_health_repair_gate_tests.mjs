import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  Phase010IndexHealthRepairErrorCode,
  Phase010IndexHealthRepairEvent,
  Phase010IndexHealthRepairState,
  buildPhase010IndexHealthRepairCommandPlan,
  buildPhase010IndexHealthRepairPerformanceBudget,
  evaluatePhase010IndexHealthRepairGate,
  renderPhase010IndexHealthRepairArtifact,
  renderPhase010IndexHealthRepairPerformanceBudget,
  runPhase010IndexHealthRepairGate,
  transitionPhase010IndexHealthRepairState,
} from "./phase010_index_health_repair_gate.mjs";

test("phase010 index health gate rejects missing durable authoring prerequisite", async () => {
  const root = await createIndexHealthFixture({
    durableAuthoringText: "phase010_durable_authoring_gate=failed\n",
  });

  const result = await runPhase010IndexHealthRepairGate({
    root,
    writeArtifacts: false,
    runner: passingRunner,
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010IndexHealthRepairErrorCode.DurableAuthoringMarkerMissing);
});

test("phase010 index health gate rejects missing data portability prerequisite", async () => {
  const root = await createIndexHealthFixture({
    dataPortabilityText: "phase010_data_portability_gate=failed\n",
  });

  const result = await runPhase010IndexHealthRepairGate({
    root,
    writeArtifacts: false,
    runner: passingRunner,
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010IndexHealthRepairErrorCode.DataPortabilityMarkerMissing);
});

test("phase010 index health gate rejects failed projection command", () => {
  const result = evaluatePhase010IndexHealthRepairGate({
    durableAuthoringText: "phase010_durable_authoring_gate=passed",
    dataPortabilityText: "phase010_data_portability_gate=passed",
    commandResults: {
      ...passingCommandResults(),
      searchAdapter: { passed: false, exitCode: 101, command: "cargo search adapter" },
    },
    performanceBudgetRows: passingBudgetRows(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010IndexHealthRepairErrorCode.ProjectionTestsFailed);
  assert.equal(result.failedStepId, "searchAdapter");
});

test("phase010 index health gate rejects failed performance command", () => {
  const result = evaluatePhase010IndexHealthRepairGate({
    durableAuthoringText: "phase010_durable_authoring_gate=passed",
    dataPortabilityText: "phase010_data_portability_gate=passed",
    commandResults: {
      ...passingCommandResults(),
      queryPerformance: { passed: false, exitCode: 101, command: "cargo performance" },
    },
    performanceBudgetRows: passingBudgetRows(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010IndexHealthRepairErrorCode.PerformanceTestsFailed);
  assert.equal(result.failedStepId, "queryPerformance");
});

test("phase010 index health gate rejects failed ui command and budget failure", () => {
  const failedUi = evaluatePhase010IndexHealthRepairGate({
    durableAuthoringText: "phase010_durable_authoring_gate=passed",
    dataPortabilityText: "phase010_data_portability_gate=passed",
    commandResults: {
      ...passingCommandResults(),
      discoveryUiModels: { passed: false, exitCode: 1, command: "node ui" },
    },
    performanceBudgetRows: passingBudgetRows(),
  });
  const failedBudget = evaluatePhase010IndexHealthRepairGate({
    durableAuthoringText: "phase010_durable_authoring_gate=passed",
    dataPortabilityText: "phase010_data_portability_gate=passed",
    commandResults: passingCommandResults(),
    performanceBudgetRows: [
      ...passingBudgetRows(),
      { path: "search", p95Ms: 301, budgetMs: 300, validatedBy: "fixture" },
    ],
  });

  assert.equal(failedUi.errorCode, Phase010IndexHealthRepairErrorCode.UiTestsFailed);
  assert.equal(failedUi.failedStepId, "discoveryUiModels");
  assert.equal(failedBudget.errorCode, Phase010IndexHealthRepairErrorCode.PerformanceBudgetFailed);
  assert.equal(failedBudget.findingId, "search");
});

test("phase010 index health command plan is personal local desktop only", () => {
  const steps = buildPhase010IndexHealthRepairCommandPlan();

  assert.deepEqual(
    steps.map((step) => step.id),
    [
      "searchAdapter",
      "linkAdapter",
      "graphProjectionAdapter",
      "assetMetadataAdapter",
      "assetStore",
      "searchUsecase",
      "graphLiteUsecase",
      "permissionGraphUsecase",
      "assetMetadataUsecase",
      "queryPerformance",
      "discoveryUiModels",
      "desktopDiscoverySmoke",
    ],
  );
  assert.ok(steps.every((step) => !step.command.join(" ").includes("self-host")));
  assert.ok(steps.every((step) => !step.command.join(" ").includes("mobile")));
  assert.ok(steps.every((step) => !step.command.join(" ").includes("server-base-url")));
});

test("phase010 index health gate passes evidence and renders safe artifacts", () => {
  const result = evaluatePhase010IndexHealthRepairGate({
    durableAuthoringText: "phase010_durable_authoring_gate=passed",
    dataPortabilityText: "phase010_data_portability_gate=passed",
    commandResults: passingCommandResults(),
    performanceBudgetRows: passingBudgetRows(),
  });
  const artifact = renderPhase010IndexHealthRepairArtifact(result);
  const budget = renderPhase010IndexHealthRepairPerformanceBudget(
    buildPhase010IndexHealthRepairPerformanceBudget(passingBudgetRows()),
  );

  assert.equal(result.passed, true);
  assert.equal(result.state, Phase010IndexHealthRepairState.Passed);
  assert.match(artifact, /phase010_index_health_repair_gate=passed/);
  assert.match(artifact, /projection_repair=verified/);
  assert.match(budget, /phase010_performance_budget=passed/);
  assert.match(budget, /asset metadata/);
  assert.doesNotMatch(artifact, /raw_document_body_fixture/);
  assert.doesNotMatch(budget, /provider_api_key_fixture/);
});

test("phase010 index health gate writes marker and performance budget artifacts", async () => {
  const root = await createIndexHealthFixture();

  const result = await runPhase010IndexHealthRepairGate({
    root,
    writeArtifacts: true,
    runner: passingRunner,
  });
  const marker = await readFile(
    join(root, ".tasks", "phase010-index-health-repair-gate-result.md"),
    "utf8",
  );
  const budget = await readFile(
    join(root, ".tasks", "release", "performance-budget-phase010.md"),
    "utf8",
  );

  assert.equal(result.passed, true);
  assert.match(marker, /phase010_index_health_repair_gate=passed/);
  assert.match(budget, /phase010_performance_budget=passed/);
});

test("phase010 index health state machine exposes success, failure, and invalid transition", () => {
  const reading = transitionPhase010IndexHealthRepairState(
    Phase010IndexHealthRepairState.Pending,
    Phase010IndexHealthRepairEvent.Start,
  );
  const runningProjection = transitionPhase010IndexHealthRepairState(
    reading.state,
    Phase010IndexHealthRepairEvent.PrerequisitesRead,
  );
  const runningPerformance = transitionPhase010IndexHealthRepairState(
    runningProjection.state,
    Phase010IndexHealthRepairEvent.ProjectionTestsPassed,
  );
  const runningUi = transitionPhase010IndexHealthRepairState(
    runningPerformance.state,
    Phase010IndexHealthRepairEvent.PerformanceTestsPassed,
  );
  const writingBudget = transitionPhase010IndexHealthRepairState(
    runningUi.state,
    Phase010IndexHealthRepairEvent.UiTestsPassed,
  );
  const writingResult = transitionPhase010IndexHealthRepairState(
    writingBudget.state,
    Phase010IndexHealthRepairEvent.PerformanceBudgetWritten,
  );
  const passed = transitionPhase010IndexHealthRepairState(
    writingResult.state,
    Phase010IndexHealthRepairEvent.ResultWritten,
  );
  const invalid = transitionPhase010IndexHealthRepairState(
    Phase010IndexHealthRepairState.Pending,
    Phase010IndexHealthRepairEvent.UiTestsPassed,
  );

  assert.equal(reading.state, Phase010IndexHealthRepairState.ReadingPrerequisites);
  assert.equal(runningProjection.state, Phase010IndexHealthRepairState.RunningProjectionTests);
  assert.equal(runningPerformance.state, Phase010IndexHealthRepairState.RunningPerformanceTests);
  assert.equal(runningUi.state, Phase010IndexHealthRepairState.RunningUiTests);
  assert.equal(writingBudget.state, Phase010IndexHealthRepairState.WritingPerformanceBudget);
  assert.equal(writingResult.state, Phase010IndexHealthRepairState.WritingResult);
  assert.equal(passed.state, Phase010IndexHealthRepairState.Passed);
  assert.equal(invalid.errorCode, Phase010IndexHealthRepairErrorCode.InvalidTransition);
});

async function createIndexHealthFixture({
  durableAuthoringText = "phase010_durable_authoring_gate=passed\n",
  dataPortabilityText = "phase010_data_portability_gate=passed\n",
} = {}) {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase010-index-health-"));
  await mkdir(join(root, ".tasks"), { recursive: true });
  await writeFile(join(root, ".tasks", "phase010-durable-authoring-gate-result.md"), durableAuthoringText);
  await writeFile(join(root, ".tasks", "phase010-data-portability-gate-result.md"), dataPortabilityText);
  return root;
}

function passingCommandResults() {
  return Object.fromEntries(
    buildPhase010IndexHealthRepairCommandPlan().map((step) => [
      step.id,
      { command: step.command.join(" "), passed: true, exitCode: 0, durationMs: 5 },
    ]),
  );
}

function passingBudgetRows() {
  return [
    { path: "current document read", p95Ms: 300, budgetMs: 300, validatedBy: "durable authoring" },
    { path: "history list", p95Ms: 300, budgetMs: 300, validatedBy: "durable authoring" },
    { path: "specific version read", p95Ms: 300, budgetMs: 300, validatedBy: "durable authoring" },
    { path: "restore preview status", p95Ms: 300, budgetMs: 300, validatedBy: "durable authoring" },
    { path: "search", p95Ms: 300, budgetMs: 300, validatedBy: "query_performance_benchmarks" },
    { path: "backlink", p95Ms: 300, budgetMs: 300, validatedBy: "query_performance_benchmarks" },
    { path: "graph projection", p95Ms: 300, budgetMs: 300, validatedBy: "query_performance_benchmarks" },
    { path: "asset metadata", p95Ms: 300, budgetMs: 300, validatedBy: "query_performance_benchmarks" },
    { path: "index health status", p95Ms: 300, budgetMs: 300, validatedBy: "local discovery model" },
  ];
}

async function passingRunner() {
  return { exitCode: 0, signal: null, durationMs: 5 };
}
