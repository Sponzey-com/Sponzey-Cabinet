import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";

import {
  Phase011ProductSmokeErrorCode,
  Phase011ProductSmokeEvent,
  Phase011ProductSmokeState,
  analyzePhase011ProductSmokeEvidence,
  buildNativePlatformMatrix,
  buildPhase011ProductSmokeCommandPlan,
  renderPhase011ProductSmokeGateMarkdown,
  runPhase011ProductSmokeGate,
  transitionPhase011ProductSmokeState,
} from "./phase011_product_smoke_gate.mjs";

test("phase011 product smoke rejects missing lower marker", () => {
  const sources = completeSources();
  sources[".tasks/phase011-discovery-gate-result.md"] = "phase011_discovery_gate=failed";

  const result = analyzePhase011ProductSmokeEvidence({
    sources,
    commandResults: passingCommandResults(),
    currentPlatform: "macos",
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase011ProductSmokeErrorCode.RequiredEvidenceMissing);
  assert.equal(result.findingId, "phase011_discovery_gate");
});

test("phase011 product smoke defers non-current platforms without copying current host evidence", () => {
  const sources = completeSources({ includeExternalPlatformEvidence: false });

  const result = analyzePhase011ProductSmokeEvidence({
    sources,
    commandResults: passingCommandResults(),
    currentPlatform: "macos",
  });

  assert.equal(result.passed, true);
  assert.equal(result.platformMatrix.rows.find((row) => row.platform === "macos").status, "passed");
  assert.equal(result.platformMatrix.rows.find((row) => row.platform === "windows").status, "deferred_future");
  assert.equal(result.platformMatrix.rows.find((row) => row.platform === "linux").status, "deferred_future");
  assert.equal(result.platformMatrix.rows.find((row) => row.platform === "windows").evidence, "deferred_by_phase011_scope_decision");
});

test("phase011 product smoke passes complete independent platform evidence", () => {
  const result = analyzePhase011ProductSmokeEvidence({
    sources: completeSources(),
    commandResults: passingCommandResults(),
    currentPlatform: "macos",
  });
  const artifact = renderPhase011ProductSmokeGateMarkdown(result);

  assert.equal(result.passed, true);
  assert.equal(result.marker, "phase011_product_smoke_gate=passed");
  assert.equal(result.platformMatrix.marker, "phase011_native_platform_matrix=passed");
  assert.equal(result.visualReport.marker, "phase011_visual_accessibility=passed");
  assert.match(artifact, /future_scope_exclusion=server,SaaS,multi-user,mobile,admin,SSO,billing/);
  assert.doesNotMatch(artifact, /RAW_DOC_BODY_SAMPLE/);
  assert.doesNotMatch(artifact, /PERSONAL_PATH_SAMPLE/);
});

test("phase011 product smoke rejects failed visual accessibility evidence", () => {
  const sources = completeSources();
  const visual = JSON.parse(sources[".tasks/release/workspace-home-visual-phase011.json"]);
  visual.runs[0].overlapCount = 1;
  sources[".tasks/release/workspace-home-visual-phase011.json"] = JSON.stringify(visual);

  const result = analyzePhase011ProductSmokeEvidence({
    sources,
    commandResults: passingCommandResults(),
    currentPlatform: "macos",
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase011ProductSmokeErrorCode.VisualAccessibilityFailed);
  assert.equal(result.findingId, "overlap");
});

test("phase011 native platform matrix rejects stale external evidence", () => {
  const matrix = buildNativePlatformMatrix({
    sources: completeSources({ staleExternalEvidence: true }),
    commandResults: passingCommandResults(),
    currentPlatform: "macos",
    sourceFingerprint: fingerprint(),
    deferOtherPlatforms: false,
  });

  assert.equal(matrix.passed, false);
  assert.equal(matrix.rows.find((row) => row.platform === "windows").status, "not_verified");
});

test("phase011 product smoke command plan is current local desktop only", () => {
  const commands = buildPhase011ProductSmokeCommandPlan().map((entry) => entry.command.join(" "));

  assert.deepEqual(
    buildPhase011ProductSmokeCommandPlan().map((entry) => entry.id),
    [
      "workspaceHomeGate",
      "documentAuthoringGate",
      "historyRestoreGate",
      "discoveryGate",
      "dataSettingsGate",
      "recoveryObservabilityGate",
      "desktopPackageSmoke",
    ],
  );
  assert.ok(commands.every((command) => !command.includes("self-host")));
  assert.ok(commands.every((command) => !command.includes("mobile")));
  assert.ok(commands.every((command) => !command.includes("admin")));
});

test("phase011 product smoke writes reports on complete evidence", async () => {
  const root = await createFixture();

  const result = await runPhase011ProductSmokeGate({
    root,
    runner: passingRunner,
    currentPlatform: "macos",
  });
  const marker = await readFile(join(root, ".tasks", "phase011-product-smoke-gate-result.md"), "utf8");
  const visual = await readFile(join(root, ".tasks", "release", "visual-accessibility-report-phase011.md"), "utf8");
  const platforms = await readFile(join(root, ".tasks", "release", "native-platform-matrix-phase011.md"), "utf8");

  assert.equal(result.passed, true);
  assert.match(marker, /phase011_product_smoke_gate=passed/);
  assert.match(visual, /phase011_visual_accessibility=passed/);
  assert.match(platforms, /phase011_native_platform_matrix=passed/);
});

test("phase011 product smoke writes deferred platform matrix when current host is sufficient", async () => {
  const root = await createFixture({ includeExternalPlatformEvidence: false });

  const result = await runPhase011ProductSmokeGate({
    root,
    runner: passingRunner,
    currentPlatform: "macos",
  });
  const marker = await readFile(join(root, ".tasks", "phase011-product-smoke-gate-result.md"), "utf8");
  const platforms = await readFile(join(root, ".tasks", "release", "native-platform-matrix-phase011.md"), "utf8");

  assert.equal(result.passed, true);
  assert.match(marker, /phase011_product_smoke_gate=passed/);
  assert.match(marker, /deferred_platforms=windows,linux/);
  assert.match(platforms, /phase011_native_platform_matrix=passed/);
  assert.match(platforms, /\| `windows` \| `deferred_future` \| `deferred_by_phase011_scope_decision` \|/);
});

test("phase011 product smoke state machine reaches terminal states", () => {
  const running = transitionPhase011ProductSmokeState(
    Phase011ProductSmokeState.Pending,
    Phase011ProductSmokeEvent.Start,
  );
  const reading = transitionPhase011ProductSmokeState(
    running.state,
    Phase011ProductSmokeEvent.CommandsPassed,
  );
  const validating = transitionPhase011ProductSmokeState(
    reading.state,
    Phase011ProductSmokeEvent.EvidenceRead,
  );
  const writing = transitionPhase011ProductSmokeState(
    validating.state,
    Phase011ProductSmokeEvent.EvidenceValidated,
  );
  const passed = transitionPhase011ProductSmokeState(
    writing.state,
    Phase011ProductSmokeEvent.ArtifactsWritten,
  );
  const invalid = transitionPhase011ProductSmokeState(
    Phase011ProductSmokeState.Pending,
    Phase011ProductSmokeEvent.ArtifactsWritten,
  );

  assert.equal(running.state, Phase011ProductSmokeState.RunningCommands);
  assert.equal(reading.state, Phase011ProductSmokeState.ReadingEvidence);
  assert.equal(validating.state, Phase011ProductSmokeState.ValidatingEvidence);
  assert.equal(writing.state, Phase011ProductSmokeState.WritingArtifacts);
  assert.equal(passed.state, Phase011ProductSmokeState.Passed);
  assert.equal(invalid.errorCode, Phase011ProductSmokeErrorCode.InvalidTransition);
});

async function createFixture(options = {}) {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase011-product-smoke-"));
  for (const [path, text] of Object.entries(completeSources(options))) {
    await mkdir(join(root, dirname(path)), { recursive: true });
    await writeFile(join(root, path), text);
  }
  return root;
}

function completeSources({
  includeExternalPlatformEvidence = true,
  staleExternalEvidence = false,
} = {}) {
  const fp = fingerprint();
  const platformFingerprint = staleExternalEvidence ? "b".repeat(64) : fp;
  const sources = {
    ".tasks/phase011-current-implementation-inventory.md": `phase011_current_inventory=passed\nsource_fingerprint=${fp}\n`,
    ".tasks/phase011-workspace-home-gate-result.md": [
      "phase011_workspace_home_gate=passed",
      "release_scope=personal_local_desktop",
    ].join("\n"),
    ".tasks/phase011-document-authoring-gate-result.md": [
      "phase011_document_authoring_gate=passed",
      "raw_body_excluded=true",
      "raw_path_excluded=true",
    ].join("\n"),
    ".tasks/phase011-history-restore-gate-result.md": [
      "phase011_history_restore_gate=passed",
      "current_history_query_separation=true",
      "git_terms_excluded=true",
    ].join("\n"),
    ".tasks/phase011-discovery-gate-result.md": [
      "phase011_discovery_gate=passed",
      "graph_neighborhood_bounded=true",
      "raw_query_excluded=true",
    ].join("\n"),
    ".tasks/phase011-data-settings-gate-result.md": [
      "phase011_data_settings_gate=passed",
      "future_server_admin_settings_excluded=true",
    ].join("\n"),
    ".tasks/phase011-recovery-observability-gate-result.md": [
      "phase011_recovery_observability_gate=passed",
      "product_log_classes_separated=true",
    ].join("\n"),
    ".tasks/release/performance-budget-phase011.md": [
      "phase011_performance_budget=passed",
      "300ms",
    ].join("\n"),
    ".tasks/release/security-log-policy-manifest-phase011.json": JSON.stringify({
      marker: "phase011_security_log_manifest=passed",
      logClasses: ["Product Log", "Field Debug Log", "Development Log"],
    }),
    ".tasks/release/local-desktop-runbook-phase011.md": [
      "phase011_runbook=passed",
      "Do not require external DB, external search, Git CLI, Node.js runtime, manual environment variables, or direct config file editing as a user recovery path.",
    ].join("\n"),
    ".tasks/release/workspace-home-visual-phase011.json": JSON.stringify({
      marker: "phase011_workspace_home_visual=passed",
      runs: [visualRun(1280, 800), visualRun(1440, 900)],
    }),
    ".tasks/release/authoring-browser-phase011.json": JSON.stringify({
      marker: "phase011_authoring_browser=passed",
      interactions: {
        codeMirrorMounted: true,
        previewTableRendered: true,
      },
      runs: [visualRun(1024, 700), visualRun(1280, 800)],
    }),
  };
  if (includeExternalPlatformEvidence) {
    for (const platform of ["windows", "macos", "linux"]) {
      sources[`.tasks/release/native-platform-evidence-${platform}-phase011.md`] = [
        "phase011_native_platform_evidence=passed",
        `native_platform=${platform}`,
        `source_fingerprint=${platformFingerprint}`,
      ].join("\n");
    }
  }
  return sources;
}

function visualRun(width, height) {
  return {
    width,
    height,
    readyState: true,
    overlapCount: 0,
    horizontalOverflow: false,
    focusVisible: true,
    nonBlankPixelCount: 1000,
  };
}

function passingCommandResults() {
  return Object.fromEntries(
    buildPhase011ProductSmokeCommandPlan().map((entry) => [
      entry.id,
      { command: entry.command.join(" "), passed: true, exitCode: 0, durationMs: 5 },
    ]),
  );
}

async function passingRunner() {
  return { exitCode: 0, signal: null, durationMs: 5 };
}

function fingerprint() {
  return "a".repeat(64);
}
