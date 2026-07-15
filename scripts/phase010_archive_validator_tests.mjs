import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  Phase010ArchiveErrorCode,
  Phase010ArchiveEvent,
  Phase010ArchiveState,
  renderPhase010ArchiveValidationArtifact,
  runPhase010ArchiveValidation,
  transitionPhase010ArchiveState,
} from "./phase010_archive_validator.mjs";

test("phase010 archive validator rejects missing phase009 final marker", async () => {
  const root = await createArchiveFixture({
    finalMarkerText: "phase009_ux_release_gate=failed\n",
  });

  const result = await runPhase010ArchiveValidation({ root, writeArtifact: false });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010ArchiveErrorCode.Phase009FinalMarkerMissing);
  assert.equal(result.findingId, ".tasks/phase009/phase009-ux-release-gate-result.md");
});

test("phase010 archive validator rejects missing archived task", async () => {
  const root = await createArchiveFixture({ missingTask: 7 });

  const result = await runPhase010ArchiveValidation({ root, writeArtifact: false });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010ArchiveErrorCode.TaskNumberingGap);
  assert.equal(result.findingId, "task007.md");
});

test("phase010 archive validator rejects missing phase009 release evidence", async () => {
  const root = await createArchiveFixture({ omitReleaseEvidence: "product-log-event-matrix-phase009.md" });

  const result = await runPhase010ArchiveValidation({ root, writeArtifact: false });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase010ArchiveErrorCode.ReleaseEvidenceMissing);
  assert.equal(result.findingId, ".tasks/phase009/release/product-log-event-matrix-phase009.md");
});

test("phase010 archive validator passes complete fixture and renders safe artifact", async () => {
  const root = await createArchiveFixture();

  const result = await runPhase010ArchiveValidation({ root, writeArtifact: false });
  const artifact = renderPhase010ArchiveValidationArtifact(result);

  assert.equal(result.passed, true);
  assert.equal(result.state, Phase010ArchiveState.Passed);
  assert.equal(result.archivedTaskCount, 13);
  assert.match(artifact, /phase010_archive_validation=passed/);
  assert.match(artifact, /phase009_ux_release_gate=passed/);
  assert.doesNotMatch(artifact, /raw_document_body_fixture/);
  assert.doesNotMatch(artifact, /provider_api_key_fixture/);
  assert.doesNotMatch(artifact, /personal_absolute_path_fixture/);
});

test("phase010 archive validator writes marker artifact to explicit root", async () => {
  const root = await createArchiveFixture();

  const result = await runPhase010ArchiveValidation({ root, writeArtifact: true });
  const written = await readFile(join(root, ".tasks", "phase010-archive-validation-result.md"), "utf8");

  assert.equal(result.passed, true);
  assert.match(written, /phase010_archive_validation=passed/);
  assert.match(written, /validation_state=Passed/);
});

test("phase010 archive state machine exposes success, failure, and invalid transition", () => {
  const reading = transitionPhase010ArchiveState(Phase010ArchiveState.Pending, Phase010ArchiveEvent.Start);
  const validating = transitionPhase010ArchiveState(reading.state, Phase010ArchiveEvent.ArchiveRead);
  const writing = transitionPhase010ArchiveState(validating.state, Phase010ArchiveEvent.MarkersValidated);
  const passed = transitionPhase010ArchiveState(writing.state, Phase010ArchiveEvent.ResultWritten);
  const failed = transitionPhase010ArchiveState(reading.state, Phase010ArchiveEvent.Fail, {
    errorCode: Phase010ArchiveErrorCode.PlanMissing,
    findingId: ".tasks/phase009/plan.md",
  });
  const invalid = transitionPhase010ArchiveState(
    Phase010ArchiveState.Pending,
    Phase010ArchiveEvent.MarkersValidated,
  );

  assert.equal(reading.state, Phase010ArchiveState.ReadingArchive);
  assert.equal(validating.state, Phase010ArchiveState.ValidatingMarkers);
  assert.equal(writing.state, Phase010ArchiveState.WritingResult);
  assert.equal(passed.state, Phase010ArchiveState.Passed);
  assert.equal(failed.state, Phase010ArchiveState.Failed);
  assert.equal(failed.findingId, ".tasks/phase009/plan.md");
  assert.equal(invalid.errorCode, Phase010ArchiveErrorCode.InvalidTransition);
});

async function createArchiveFixture({
  missingTask,
  finalMarkerText = "phase009_ux_release_gate=passed\n",
  omitReleaseEvidence,
} = {}) {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase010-archive-"));
  const archiveRoot = join(root, ".tasks", "phase009");
  const releaseRoot = join(archiveRoot, "release");
  await mkdir(releaseRoot, { recursive: true });

  await writeFile(join(archiveRoot, "plan.md"), "# Phase 009 Development Plan\n");
  await writeFile(join(archiveRoot, "phase009-ux-release-gate-result.md"), finalMarkerText);
  await writeFile(join(archiveRoot, "phase009-plan-validation-result.md"), "phase009_plan_validation=passed\n");

  for (let index = 1; index <= 13; index += 1) {
    if (index === missingTask) {
      continue;
    }
    await writeFile(
      join(archiveRoot, `task${String(index).padStart(3, "0")}.md`),
      `# Task ${String(index).padStart(3, "0")}\n`,
    );
  }

  const releaseFiles = new Map([
    ["local-desktop-runbook-phase009.md", "phase009_runbook=passed\n"],
    ["performance-budget-phase009.md", "phase009_performance_budget=passed\n"],
    ["product-log-event-matrix-phase009.md", "phase009_product_log_matrix=passed\n"],
    ["runbook-validation-manifest-phase009.json", JSON.stringify({ phase009_local_desktop: true })],
    ["security-log-policy-manifest-phase009.json", JSON.stringify({ marker: "phase009_security_log_manifest=passed" })],
  ]);

  for (const [filename, contents] of releaseFiles) {
    if (filename === omitReleaseEvidence) {
      continue;
    }
    await writeFile(join(releaseRoot, filename), contents);
  }

  return root;
}
