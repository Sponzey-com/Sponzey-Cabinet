import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  ReleaseGateEvent,
  ReleaseGateState,
  createReleaseGateResultPath,
  renderReleaseGateResult,
  runReleaseGate,
  sanitizeReleaseGateArtifact,
  summarizeGitStatusText,
  summarizePerformanceReport,
  transitionReleaseGateState,
  writeReleaseGateResultArtifact,
} from "./run_phase002_release_gate.mjs";

test("release gate result serializer includes command status duration p95 scan and git summaries", () => {
  const rendered = renderReleaseGateResult({
    phase: "Phase 002",
    gate: "Release Boundary",
    status: "passed",
    state: "StepPassed",
    startedAt: "2026-06-28T00:00:00.000Z",
    completedAt: "2026-06-28T00:00:01.000Z",
    manifestPath: ".tasks/release/phase002-release-manifest.json",
    failureCategory: "none",
    failedStepId: undefined,
    failureMessage: "none",
    performanceSummary: "p95 300ms target documented; paths=permission-aware current document lookup",
    sensitiveScanSummary: "passed",
    gitStatusSummary: {
      modified: 2,
      added: 1,
      deleted: 0,
      renamed: 0,
      untracked: 3,
      other: 0,
    },
    commandResults: [
      {
        id: "example_step",
        command: ["npm", "run", "example"],
        status: "passed",
        exitCode: 0,
        signal: null,
        durationMs: 42,
      },
    ],
  });

  assert.match(rendered, /# Phase 002 Release Gate Result/);
  assert.match(rendered, /example_step/);
  assert.match(rendered, /`npm run example`/);
  assert.match(rendered, /p95 300ms target documented/);
  assert.match(rendered, /Sensitive Scan: passed/);
  assert.match(rendered, /modified=2 added=1 deleted=0 renamed=0 untracked=3 other=0/);
});

test("release gate writes partial artifact when a command fails", async () => {
  const root = await createReleaseFixtureRoot();
  const manifestPath = join(root, ".tasks", "release", "phase002-release-manifest.json");
  const result = await runReleaseGate({
    root,
    manifestPath,
    startedAt: new Date("2026-06-28T00:00:00.000Z"),
    commandRunner: async (command) => ({
      exitCode: command === "false-command" ? 7 : 0,
      signal: null,
    }),
    gitStatusProvider: async () => ({
      modified: 0,
      added: 0,
      deleted: 0,
      renamed: 0,
      untracked: 0,
      other: 0,
    }),
  });
  const artifactPath = await writeReleaseGateResultArtifact({
    root,
    result,
    securityManifestPath: join(root, ".tasks", "release", "security-log-policy-manifest.json"),
  });
  const artifact = await readFile(artifactPath, "utf8");

  assert.equal(result.status, "failed");
  assert.equal(result.failedStepId, "failing_step");
  assert.equal(result.failureCategory, "command_exit_nonzero");
  assert.match(artifact, /Runner State: ArtifactWritten/);
  assert.match(artifact, /Failed Step Id: failing_step/);
  assert.match(artifact, /failing_step/);
  assert.match(artifact, /Exit Code/);
});

test("release result sanitizer rejects denied fixture values before write", () => {
  assert.throws(
    () =>
      sanitizeReleaseGateArtifact("result contains fixture-secret-access-key", [
        {
          id: "s3_secret_fixture",
          value: "fixture-secret-access-key",
        },
      ]),
    /denied fixture/,
  );
});

test("release gate state machine exposes terminal artifact state", () => {
  const running = transitionReleaseGateState(ReleaseGateState.Pending, ReleaseGateEvent.Start);
  const stepRunning = transitionReleaseGateState(running.state, ReleaseGateEvent.StepStart, {
    stepId: "rust_format",
  });
  const stepPassed = transitionReleaseGateState(stepRunning.state, ReleaseGateEvent.StepExit, {
    stepId: "rust_format",
    status: "passed",
  });
  const sanitizing = transitionReleaseGateState(stepPassed.state, ReleaseGateEvent.Sanitize);
  const written = transitionReleaseGateState(sanitizing.state, ReleaseGateEvent.WriteArtifact);

  assert.equal(running.state, "Running");
  assert.equal(stepRunning.currentStepId, "rust_format");
  assert.equal(stepPassed.state, "StepPassed");
  assert.equal(sanitizing.state, "Sanitizing");
  assert.equal(written.state, "ArtifactWritten");
});

test("release gate summaries are stable and do not include file names from git status", () => {
  const performanceSummary = summarizePerformanceReport(`
    # Performance
    p95 300ms
    permission-aware current document lookup
    permission-aware document search
    audit event list
  `);
  const gitSummary = summarizeGitStatusText(" M secret-file-name.md\n?? token-output.txt\n D deleted.md\n");

  assert.match(performanceSummary, /p95 300ms target documented/);
  assert.deepEqual(gitSummary, {
    modified: 1,
    added: 0,
    deleted: 1,
    renamed: 0,
    untracked: 1,
    other: 0,
  });
});

test("release result path uses timestamped markdown location", () => {
  const artifactPath = createReleaseGateResultPath("/repo", new Date(2026, 5, 28, 1, 2, 3));

  assert.equal(
    artifactPath,
    "/repo/.tasks/release/results/phase002-release-gate-20260628-010203.md",
  );
});

async function createReleaseFixtureRoot() {
  const root = await mkdtemp(join(tmpdir(), "sponzey-cabinet-release-gate-test-"));
  await mkdir(join(root, ".tasks", "release"), { recursive: true });
  await writeFile(
    join(root, ".tasks", "release", "phase002-release-manifest.json"),
    JSON.stringify(
      {
        phase: "Phase 002",
        gate: "Release Boundary",
        requiredCompletedTaskCount: 0,
        requiredFiles: [],
        requiredReports: [
          {
            path: ".tasks/release/fixture-report.md",
            requiredText: ["release fixture"],
          },
        ],
        commands: [
          { id: "passing_step", command: ["true-command"] },
          { id: "failing_step", command: ["false-command"] },
          { id: "not_run_step", command: ["not-run-command"] },
        ],
      },
      null,
      2,
    ),
  );
  await writeFile(
    join(root, ".tasks", "release", "fixture-report.md"),
    "release fixture\n",
  );
  await writeFile(
    join(root, ".tasks", "release", "phase002-performance-report.md"),
    "p95 300ms\npermission-aware current document lookup\nasset metadata list\n",
  );
  await writeFile(
    join(root, ".tasks", "release", "security-log-policy-manifest.json"),
    JSON.stringify(
      {
        deniedFixtures: [
          {
            id: "secret_fixture",
            value: "fixture-secret-access-key",
          },
        ],
      },
      null,
      2,
    ),
  );
  return root;
}
