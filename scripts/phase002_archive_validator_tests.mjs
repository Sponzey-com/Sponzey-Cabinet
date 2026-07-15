import assert from "node:assert/strict";
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  ArchiveValidationErrorCode,
  ArchiveValidationEvent,
  ArchiveValidationState,
  renderArchiveValidationResult,
  runPhase002ArchiveValidation,
  transitionArchiveValidationState,
  validateArchiveManifest,
} from "./phase002_archive_validator.mjs";

test("archive validator reports missing archived task with stable error code", async () => {
  const root = await createArchiveFixture({ missingTask: 12 });

  const result = await runPhase002ArchiveValidation({ root });
  const rendered = renderArchiveValidationResult(result);

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, ArchiveValidationErrorCode.TaskNumberingGap);
  assert.equal(result.findingId, "task012.md");
  assert.match(rendered, /error_code=PHASE002_ARCHIVE_TASK_NUMBERING_GAP/);
});

test("archive validator rejects active archived task conflict", async () => {
  const root = await createArchiveFixture({ activeTaskConflict: true });

  const result = await runPhase002ArchiveValidation({ root });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, ArchiveValidationErrorCode.ActiveArchiveConflict);
  assert.equal(result.findingId, "task001.md");
});

test("archive validator rejects next phase plan without required planning terminology", async () => {
  const root = await createArchiveFixture({ activePlanText: "# Phase 003 Plan\n\ncurrent phase only\n" });

  const result = await runPhase002ArchiveValidation({ root });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, ArchiveValidationErrorCode.NextPhaseTerminologyMissing);
  assert.equal(result.findingId, "contract complete");
});

test("archive validator passes complete archive fixture", async () => {
  const root = await createArchiveFixture();

  const result = await runPhase002ArchiveValidation({ root });

  assert.equal(result.passed, true);
  assert.equal(result.state, ArchiveValidationState.Archived);
  assert.equal(result.archivedTaskCount, 35);
});

test("archive manifest validates archived path checksum or size summary", () => {
  assert.throws(
    () =>
      validateArchiveManifest({
        schemaVersion: 1,
        phase: "Phase 002",
        entries: [{ sourcePath: ".tasks/plan.md", archivePath: ".tasks/phase002/plan.md" }],
      }),
    /PHASE002_ARCHIVE_MALFORMED_MANIFEST/,
  );
});

test("archive validation state machine exposes terminal archive states", () => {
  const validating = transitionArchiveValidationState(
    ArchiveValidationState.Planned,
    ArchiveValidationEvent.StartValidation,
  );
  const archiving = transitionArchiveValidationState(
    validating.state,
    ArchiveValidationEvent.ValidationPassed,
  );
  const archived = transitionArchiveValidationState(
    archiving.state,
    ArchiveValidationEvent.WriteManifest,
  );
  const failed = transitionArchiveValidationState(
    validating.state,
    ArchiveValidationEvent.Fail,
    {
      errorCode: ArchiveValidationErrorCode.MissingArchiveFile,
      findingId: "plan.md",
    },
  );

  assert.equal(validating.state, ArchiveValidationState.Validating);
  assert.equal(archiving.state, ArchiveValidationState.Archiving);
  assert.equal(archived.state, ArchiveValidationState.Archived);
  assert.equal(failed.state, ArchiveValidationState.Failed);
  assert.equal(failed.findingId, "plan.md");
});

async function createArchiveFixture({
  missingTask,
  activeTaskConflict = false,
  activePlanText = nextPhasePlanText(),
} = {}) {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase002-archive-"));
  await mkdir(join(root, ".tasks", "phase002", "decisions"), { recursive: true });
  await mkdir(join(root, ".tasks", "phase002", "contracts"), { recursive: true });
  await mkdir(join(root, ".tasks", "phase002", "release"), { recursive: true });
  await writeFile(join(root, ".tasks", "plan.md"), activePlanText);
  await writeFile(join(root, ".tasks", "README.md"), "# Phase 003 Task Index\n");
  await writeFile(join(root, ".tasks", "phase-gates.md"), "# Phase 003 Gate Rules\n");
  await writeFile(join(root, ".tasks", "phase002", "plan.md"), "# Phase 002 Plan\n");
  await writeFile(join(root, ".tasks", "phase002", "README.md"), "# Phase 002 Task Index\n");
  await writeFile(join(root, ".tasks", "phase002", "phase-gates.md"), "# Phase 002 Gates\n");
  await writeFile(join(root, ".tasks", "phase002", "decisions", "README.md"), "# Decisions\n");
  await writeFile(join(root, ".tasks", "phase002", "contracts", "sample.md"), "# Contract\n");
  await writeFile(
    join(root, ".tasks", "phase002", "release", "phase002-release-report.md"),
    "# Release\n",
  );

  for (let index = 1; index <= 35; index += 1) {
    if (index === missingTask) {
      continue;
    }
    await writeFile(
      join(root, ".tasks", "phase002", `task${String(index).padStart(3, "0")}.md`),
      `# Task ${index}\n`,
    );
  }
  if (activeTaskConflict) {
    await writeFile(join(root, ".tasks", "task001.md"), "# Phase 002 Active duplicate\n");
  }
  await writeFile(
    join(root, ".tasks", "phase002", "archive-manifest.json"),
    JSON.stringify(
      {
        schemaVersion: 1,
        phase: "Phase 002",
        status: "archived",
        entries: [
          {
            sourcePath: ".tasks/plan.md",
            archivePath: ".tasks/phase002/plan.md",
            sizeBytes: Buffer.byteLength("# Phase 002 Plan\n"),
          },
        ],
      },
      null,
      2,
    ),
  );
  return root;
}

function nextPhasePlanText() {
  return [
    "# Phase 003 Active Plan",
    "",
    "현재 단계: Phase 003 - Self-host Runtime and Product Hardening",
    "",
    "- contract complete",
    "- runtime wired",
    "- product smoke passed",
    "- production hardening complete",
  ].join("\n");
}
