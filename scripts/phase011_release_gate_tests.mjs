import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";

import {
  Phase011ReleaseGateErrorCode,
  Phase011ReleaseGateEvent,
  Phase011ReleaseGateState,
  analyzePhase011ReleaseEvidence,
  buildPhase011ReleaseCommandPlan,
  renderPhase011ReleaseGateMarkdown,
  runPhase011ReleaseGate,
  transitionPhase011ReleaseGateState,
} from "./phase011_release_gate.mjs";

test("phase011 release gate rejects missing product smoke marker", () => {
  const sources = completeSources();
  sources[".tasks/phase011-product-smoke-gate-result.md"] = "phase011_product_smoke_gate=failed";

  const result = analyzePhase011ReleaseEvidence({
    sources,
    commandResults: passingCommandResults(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase011ReleaseGateErrorCode.ProductSmokeFailed);
  assert.equal(result.findingId, "phase011_product_smoke_gate");
});

test("phase011 release gate accepts deferred non-current desktop OS rows", () => {
  const result = analyzePhase011ReleaseEvidence({
    sources: completeSources(),
    commandResults: passingCommandResults(),
  });
  const artifact = renderPhase011ReleaseGateMarkdown(result);

  assert.equal(result.passed, true);
  assert.match(artifact, /phase011_release_gate=passed/);
  assert.match(artifact, /non_current_desktop_os=deferred_future/);
});

test("phase011 release gate rejects not verified native platform rows", () => {
  const sources = completeSources();
  sources[".tasks/release/native-platform-matrix-phase011.md"] =
    sources[".tasks/release/native-platform-matrix-phase011.md"].replace("deferred_future", "not_verified");

  const result = analyzePhase011ReleaseEvidence({
    sources,
    commandResults: passingCommandResults(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase011ReleaseGateErrorCode.NativePlatformFailed);
});

test("phase011 release command plan stays local desktop scoped", () => {
  const commands = buildPhase011ReleaseCommandPlan().map((entry) => entry.command.join(" "));

  assert.deepEqual(
    buildPhase011ReleaseCommandPlan().map((entry) => entry.id),
    [
      "phase011ArchiveValidator",
      "phase011ProductSmokeGate",
      "desktopPackageSmoke",
      "securityScan",
      "runbookValidation",
      "repositoryIntegrityRust",
    ],
  );
  assert.ok(commands.every((command) => !command.includes("self-host")));
  assert.ok(commands.every((command) => !command.includes("mobile")));
  assert.ok(commands.every((command) => !command.includes("admin")));
});

test("phase011 release gate rejects failed product smoke command", async () => {
  const root = await createFixture();
  const result = await runPhase011ReleaseGate({
    root,
    runner: async (command) => ({
      exitCode: command.join(" ").includes("run_phase011_product_smoke_gate") ? 1 : 0,
      durationMs: 1,
    }),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase011ReleaseGateErrorCode.ProductSmokeFailed);
  assert.equal(result.findingId, "phase011ProductSmokeGate");
});

test("phase011 release gate writes final artifacts", async () => {
  const root = await createFixture();
  const result = await runPhase011ReleaseGate({ root, runner: passingRunner });
  const marker = await readFile(join(root, ".tasks", "phase011-release-gate-result.md"), "utf8");
  const productLogMatrix = await readFile(join(root, ".tasks", "release", "product-log-event-matrix-phase011.md"), "utf8");
  const requirementMatrix = await readFile(join(root, ".tasks", "release", "requirement-evidence-matrix-phase011.md"), "utf8");
  const compatibility = await readFile(join(root, ".tasks", "release", "phase010-compatibility-report-phase011.md"), "utf8");

  assert.equal(result.passed, true);
  assert.match(marker, /phase011_release_gate=passed/);
  assert.match(productLogMatrix, /phase011_product_log_matrix=passed/);
  assert.match(requirementMatrix, /phase011_requirement_evidence=passed/);
  assert.match(compatibility, /phase011_phase010_compatibility=passed/);
  assert.doesNotMatch(marker + productLogMatrix + requirementMatrix + compatibility, /RAW_DOC_BODY_SAMPLE/);
});

test("phase011 release state machine reaches terminal states", () => {
  const running = transitionPhase011ReleaseGateState(
    Phase011ReleaseGateState.Pending,
    Phase011ReleaseGateEvent.Start,
  );
  const reading = transitionPhase011ReleaseGateState(
    running.state,
    Phase011ReleaseGateEvent.CommandsPassed,
  );
  const validating = transitionPhase011ReleaseGateState(
    reading.state,
    Phase011ReleaseGateEvent.MarkersRead,
  );
  const writing = transitionPhase011ReleaseGateState(
    validating.state,
    Phase011ReleaseGateEvent.ArtifactsValidated,
  );
  const passed = transitionPhase011ReleaseGateState(
    writing.state,
    Phase011ReleaseGateEvent.ResultWritten,
  );
  const invalid = transitionPhase011ReleaseGateState(
    Phase011ReleaseGateState.Pending,
    Phase011ReleaseGateEvent.ResultWritten,
  );

  assert.equal(running.state, Phase011ReleaseGateState.RunningCommands);
  assert.equal(reading.state, Phase011ReleaseGateState.ReadingMarkers);
  assert.equal(validating.state, Phase011ReleaseGateState.ValidatingArtifacts);
  assert.equal(writing.state, Phase011ReleaseGateState.WritingResult);
  assert.equal(passed.state, Phase011ReleaseGateState.Passed);
  assert.equal(invalid.errorCode, Phase011ReleaseGateErrorCode.InvalidTransition);
});

async function createFixture() {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase011-release-"));
  for (const [path, text] of Object.entries(completeSources())) {
    await mkdir(join(root, dirname(path)), { recursive: true });
    await writeFile(join(root, path), text);
  }
  return root;
}

function completeSources() {
  const fp = "a".repeat(64);
  return {
    ".tasks/phase011-current-implementation-inventory.md": `phase011_current_inventory=passed\nsource_fingerprint=${fp}`,
    ".tasks/phase011-archive-validation-result.md": `phase011_archive_validation=passed\nsource_fingerprint=${fp}`,
    ".tasks/phase011-plan-validation-result.md": `phase011_plan_validation=passed\nsource_fingerprint=${fp}`,
    ".tasks/phase011-workspace-home-gate-result.md": `phase011_workspace_home_gate=passed\nsource_fingerprint=${fp}`,
    ".tasks/phase011-document-authoring-gate-result.md": `phase011_document_authoring_gate=passed\nsource_fingerprint=${fp}`,
    ".tasks/phase011-history-restore-gate-result.md": `phase011_history_restore_gate=passed\ngit_terms_excluded=true\nsource_fingerprint=${fp}`,
    ".tasks/phase011-discovery-gate-result.md": `phase011_discovery_gate=passed\nsource_fingerprint=${fp}`,
    ".tasks/phase011-data-settings-gate-result.md": `phase011_data_settings_gate=passed\nsource_fingerprint=${fp}`,
    ".tasks/phase011-recovery-observability-gate-result.md": `phase011_recovery_observability_gate=passed\nsource_fingerprint=${fp}`,
    ".tasks/phase011-product-smoke-gate-result.md": `phase011_product_smoke_gate=passed\nsource_fingerprint=${fp}`,
    ".tasks/release/performance-budget-phase011.md": `phase011_performance_budget=passed\nsource_fingerprint=${fp}\n300ms`,
    ".tasks/release/security-log-policy-manifest-phase011.json": `{"marker":"phase011_security_log_manifest=passed","logClasses":["Product Log","Field Debug Log","Development Log"]}`,
    ".tasks/release/local-desktop-runbook-phase011.md": `phase011_runbook=passed\nexternal DB\nGit CLI\nNode.js runtime\nsource_fingerprint=${fp}`,
    ".tasks/release/visual-accessibility-report-phase011.md": `phase011_visual_accessibility=passed\ncodemirror_mounted=true\nsource_fingerprint=${fp}`,
    ".tasks/release/native-platform-matrix-phase011.md": [
      "phase011_native_platform_matrix=passed",
      `source_fingerprint=${fp}`,
      "| `windows` | `deferred_future` | `deferred_by_phase011_scope_decision` |",
      "| `macos` | `passed` | `external_native_runner_evidence` |",
      "| `linux` | `deferred_future` | `deferred_by_phase011_scope_decision` |",
    ].join("\n"),
    ".tasks/phase010/phase010-release-gate-result.md": "phase010_release_gate=passed\nrelease_scope=personal_local_desktop",
  };
}

function passingCommandResults() {
  return Object.fromEntries(
    buildPhase011ReleaseCommandPlan().map((entry) => [
      entry.id,
      { command: entry.command.join(" "), passed: true, exitCode: 0, durationMs: 1 },
    ]),
  );
}

async function passingRunner() {
  return { exitCode: 0, durationMs: 1 };
}
