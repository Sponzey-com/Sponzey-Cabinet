import { existsSync, readdirSync, statSync } from "node:fs";
import { readFile, readFile as readFileAsync } from "node:fs/promises";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

export const ArchiveValidationState = Object.freeze({
  Planned: "Planned",
  Validating: "Validating",
  Archiving: "Archiving",
  Archived: "Archived",
  Failed: "Failed",
});

export const ArchiveValidationEvent = Object.freeze({
  StartValidation: "StartValidation",
  ValidationPassed: "ValidationPassed",
  MoveFiles: "MoveFiles",
  WriteManifest: "WriteManifest",
  Fail: "Fail",
});

export const ArchiveValidationErrorCode = Object.freeze({
  MissingArchiveFile: "PHASE002_ARCHIVE_MISSING_FILE",
  TaskNumberingGap: "PHASE002_ARCHIVE_TASK_NUMBERING_GAP",
  ActiveArchiveConflict: "PHASE002_ARCHIVE_ACTIVE_CONFLICT",
  NextPhaseTerminologyMissing: "PHASE002_ARCHIVE_NEXT_PHASE_TERMINOLOGY_MISSING",
  MalformedManifest: "PHASE002_ARCHIVE_MALFORMED_MANIFEST",
  IoFailed: "PHASE002_ARCHIVE_IO_FAILED",
  InvalidTransition: "PHASE002_ARCHIVE_INVALID_TRANSITION",
});

const requiredNextPhaseTerms = [
  "contract complete",
  "runtime wired",
  "product smoke passed",
  "production hardening complete",
];

export function transitionArchiveValidationState(currentState, event, detail = {}) {
  if (
    currentState === ArchiveValidationState.Planned &&
    event === ArchiveValidationEvent.StartValidation
  ) {
    return { state: ArchiveValidationState.Validating };
  }
  if (
    currentState === ArchiveValidationState.Validating &&
    event === ArchiveValidationEvent.ValidationPassed
  ) {
    return { state: ArchiveValidationState.Archiving };
  }
  if (
    [ArchiveValidationState.Archiving, ArchiveValidationState.Validating].includes(currentState) &&
    [ArchiveValidationEvent.MoveFiles, ArchiveValidationEvent.WriteManifest].includes(event)
  ) {
    return { state: ArchiveValidationState.Archived };
  }
  if (
    [ArchiveValidationState.Validating, ArchiveValidationState.Archiving].includes(currentState) &&
    event === ArchiveValidationEvent.Fail
  ) {
    return {
      state: ArchiveValidationState.Failed,
      errorCode: detail.errorCode ?? ArchiveValidationErrorCode.IoFailed,
      findingId: detail.findingId,
    };
  }
  return {
    state: ArchiveValidationState.Failed,
    errorCode: ArchiveValidationErrorCode.InvalidTransition,
  };
}

export function validateArchiveManifest(manifest) {
  if (!isRecord(manifest) || manifest.schemaVersion !== 1) {
    throw new ArchiveValidationError(ArchiveValidationErrorCode.MalformedManifest, "schemaVersion");
  }
  if (manifest.phase !== "Phase 002" || manifest.status !== "archived") {
    throw new ArchiveValidationError(ArchiveValidationErrorCode.MalformedManifest, "phase_status");
  }
  if (!Array.isArray(manifest.entries) || manifest.entries.length === 0) {
    throw new ArchiveValidationError(ArchiveValidationErrorCode.MalformedManifest, "entries");
  }
  for (const entry of manifest.entries) {
    if (
      !isRecord(entry) ||
      !isNonEmptyString(entry.sourcePath) ||
      !isNonEmptyString(entry.archivePath) ||
      !(Number.isInteger(entry.sizeBytes) || isNonEmptyString(entry.sha256))
    ) {
      throw new ArchiveValidationError(ArchiveValidationErrorCode.MalformedManifest, "entry_shape");
    }
  }
}

export async function runPhase002ArchiveValidation({ root = process.cwd() } = {}) {
  let state = transitionArchiveValidationState(
    ArchiveValidationState.Planned,
    ArchiveValidationEvent.StartValidation,
  );

  try {
    const archiveRoot = join(root, ".tasks", "phase002");
    const manifestPath = join(archiveRoot, "archive-manifest.json");
    const requiredFiles = [
      "plan.md",
      "README.md",
      "phase-gates.md",
      "decisions/README.md",
      "release/phase002-release-report.md",
    ];

    for (const file of requiredFiles) {
      const absolutePath = join(archiveRoot, file);
      if (!existsSync(absolutePath)) {
        return fail(state, ArchiveValidationErrorCode.MissingArchiveFile, file);
      }
    }

    for (let index = 1; index <= 35; index += 1) {
      const taskName = `task${String(index).padStart(3, "0")}.md`;
      if (!existsSync(join(archiveRoot, taskName))) {
        return fail(state, ArchiveValidationErrorCode.TaskNumberingGap, taskName);
      }
      const activeTaskPath = join(root, ".tasks", taskName);
      if (existsSync(activeTaskPath)) {
        const activeTaskText = await readFileAsync(activeTaskPath, "utf8");
        if (
          activeTaskText.includes("현재 단계: Phase 002") ||
          activeTaskText.includes("# Phase 002") ||
          activeTaskText.includes("Plan 25.")
        ) {
          return fail(state, ArchiveValidationErrorCode.ActiveArchiveConflict, taskName);
        }
      }
    }

    const archiveTasks = listTaskFiles(archiveRoot);
    if (archiveTasks.length !== 35) {
      return fail(state, ArchiveValidationErrorCode.TaskNumberingGap, "task_count");
    }

    const activePlan = await readFile(join(root, ".tasks", "plan.md"), "utf8");
    if (activePlan.includes("현재 단계: Phase 002")) {
      return fail(state, ArchiveValidationErrorCode.ActiveArchiveConflict, "active_plan_phase002");
    }
    for (const term of requiredNextPhaseTerms) {
      if (!activePlan.includes(term)) {
        return fail(state, ArchiveValidationErrorCode.NextPhaseTerminologyMissing, term);
      }
    }

    const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
    validateArchiveManifest(manifest);
    for (const entry of manifest.entries) {
      const archivePath = resolve(root, entry.archivePath);
      if (!existsSync(archivePath)) {
        return fail(state, ArchiveValidationErrorCode.MissingArchiveFile, entry.archivePath);
      }
      const sizeBytes = statSync(archivePath).size;
      if (Number.isInteger(entry.sizeBytes) && entry.sizeBytes !== sizeBytes) {
        return fail(state, ArchiveValidationErrorCode.MalformedManifest, entry.archivePath);
      }
    }

    state = transitionArchiveValidationState(
      state.state,
      ArchiveValidationEvent.ValidationPassed,
    );
    state = transitionArchiveValidationState(state.state, ArchiveValidationEvent.WriteManifest);
    return {
      passed: true,
      state: state.state,
      archivedTaskCount: archiveTasks.length,
      manifestEntryCount: manifest.entries.length,
    };
  } catch (error) {
    if (error instanceof ArchiveValidationError) {
      return fail(state, error.code, "archive_manifest");
    }
    return fail(state, ArchiveValidationErrorCode.IoFailed, "io");
  }
}

export function renderArchiveValidationResult(result) {
  if (result.passed) {
    return [
      "phase002_archive_validation=passed",
      `validation_state=${result.state}`,
      `archived_task_count=${result.archivedTaskCount}`,
      `manifest_entry_count=${result.manifestEntryCount}`,
    ].join("\n");
  }

  const lines = [
    "phase002_archive_validation=failed",
    `validation_state=${result.state}`,
    `error_code=${result.errorCode}`,
  ];
  if (result.findingId) {
    lines.push(`finding_id=${result.findingId}`);
  }
  return lines.join("\n");
}

function fail(currentState, errorCode, findingId) {
  const state = transitionArchiveValidationState(currentState.state, ArchiveValidationEvent.Fail, {
    errorCode,
    findingId,
  });
  return {
    passed: false,
    state: state.state,
    errorCode: state.errorCode,
    findingId: state.findingId,
  };
}

function listTaskFiles(directory) {
  return readdirSync(directory)
    .filter((fileName) => /^task\d{3}\.md$/.test(fileName))
    .sort();
}

class ArchiveValidationError extends Error {
  constructor(code, detail) {
    super(`${code}:${detail}`);
    this.code = code;
  }
}

function isRecord(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isNonEmptyString(value) {
  return typeof value === "string" && value.trim().length > 0;
}

async function runCli() {
  const result = await runPhase002ArchiveValidation();
  const rendered = renderArchiveValidationResult(result);
  if (result.passed) {
    console.log(rendered);
    return;
  }
  console.error(rendered);
  process.exit(1);
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  runCli();
}
