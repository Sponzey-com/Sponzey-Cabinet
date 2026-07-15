import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const Phase010ArchiveState = Object.freeze({
  Pending: "Pending",
  ReadingArchive: "ReadingArchive",
  ValidatingMarkers: "ValidatingMarkers",
  WritingResult: "WritingResult",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase010ArchiveEvent = Object.freeze({
  Start: "Start",
  ArchiveRead: "ArchiveRead",
  MarkersValidated: "MarkersValidated",
  ResultWritten: "ResultWritten",
  Fail: "Fail",
});

export const Phase010ArchiveErrorCode = Object.freeze({
  PlanMissing: "PHASE010_ARCHIVE_PLAN_MISSING",
  Phase009FinalMarkerMissing: "PHASE010_ARCHIVE_PHASE009_FINAL_MARKER_MISSING",
  TaskNumberingGap: "PHASE010_ARCHIVE_TASK_NUMBERING_GAP",
  ReleaseEvidenceMissing: "PHASE010_ARCHIVE_RELEASE_EVIDENCE_MISSING",
  ReleaseEvidenceMarkerMissing: "PHASE010_ARCHIVE_RELEASE_EVIDENCE_MARKER_MISSING",
  IoFailed: "PHASE010_ARCHIVE_IO_FAILED",
  InvalidTransition: "PHASE010_ARCHIVE_INVALID_TRANSITION",
});

const requiredReleaseEvidence = Object.freeze([
  {
    path: ".tasks/phase009/release/local-desktop-runbook-phase009.md",
    marker: "phase009_runbook=passed",
  },
  {
    path: ".tasks/phase009/release/performance-budget-phase009.md",
    marker: "phase009_performance_budget=passed",
  },
  {
    path: ".tasks/phase009/release/product-log-event-matrix-phase009.md",
    marker: "phase009_product_log_matrix=passed",
  },
  {
    path: ".tasks/phase009/release/runbook-validation-manifest-phase009.json",
    marker: "phase009_local_desktop",
  },
  {
    path: ".tasks/phase009/release/security-log-policy-manifest-phase009.json",
    marker: "phase009_security_log_manifest=passed",
  },
]);

export function transitionPhase010ArchiveState(currentState, event, detail = {}) {
  if (currentState === Phase010ArchiveState.Pending && event === Phase010ArchiveEvent.Start) {
    return { state: Phase010ArchiveState.ReadingArchive };
  }
  if (
    currentState === Phase010ArchiveState.ReadingArchive &&
    event === Phase010ArchiveEvent.ArchiveRead
  ) {
    return { state: Phase010ArchiveState.ValidatingMarkers };
  }
  if (
    currentState === Phase010ArchiveState.ValidatingMarkers &&
    event === Phase010ArchiveEvent.MarkersValidated
  ) {
    return { state: Phase010ArchiveState.WritingResult };
  }
  if (
    currentState === Phase010ArchiveState.WritingResult &&
    event === Phase010ArchiveEvent.ResultWritten
  ) {
    return { state: Phase010ArchiveState.Passed };
  }
  if (
    [
      Phase010ArchiveState.ReadingArchive,
      Phase010ArchiveState.ValidatingMarkers,
      Phase010ArchiveState.WritingResult,
    ].includes(currentState) &&
    event === Phase010ArchiveEvent.Fail
  ) {
    return {
      state: Phase010ArchiveState.Failed,
      errorCode: detail.errorCode ?? Phase010ArchiveErrorCode.IoFailed,
      findingId: detail.findingId,
    };
  }
  return {
    state: Phase010ArchiveState.Failed,
    errorCode: Phase010ArchiveErrorCode.InvalidTransition,
  };
}

export async function runPhase010ArchiveValidation({
  root = process.cwd(),
  writeArtifact = true,
} = {}) {
  let state = transitionPhase010ArchiveState(
    Phase010ArchiveState.Pending,
    Phase010ArchiveEvent.Start,
  ).state;

  try {
    const archiveRoot = join(root, ".tasks", "phase009");
    const planPath = join(archiveRoot, "plan.md");
    const planText = await readFile(planPath, "utf8");
    if (!planText.startsWith("# Phase 009 Development Plan")) {
      return toFailedResult(
        transitionPhase010ArchiveState(state, Phase010ArchiveEvent.Fail, {
          errorCode: Phase010ArchiveErrorCode.PlanMissing,
          findingId: ".tasks/phase009/plan.md",
        }),
      );
    }

    const finalMarkerPath = ".tasks/phase009/phase009-ux-release-gate-result.md";
    const finalMarkerText = await readFile(join(root, finalMarkerPath), "utf8");
    if (!finalMarkerText.includes("phase009_ux_release_gate=passed")) {
      return toFailedResult(
        transitionPhase010ArchiveState(state, Phase010ArchiveEvent.Fail, {
          errorCode: Phase010ArchiveErrorCode.Phase009FinalMarkerMissing,
          findingId: finalMarkerPath,
        }),
      );
    }

    const archivedTasks = [];
    for (let index = 1; index <= 13; index += 1) {
      const taskName = `task${String(index).padStart(3, "0")}.md`;
      try {
        await readFile(join(archiveRoot, taskName), "utf8");
        archivedTasks.push(taskName);
      } catch {
        return toFailedResult(
          transitionPhase010ArchiveState(state, Phase010ArchiveEvent.Fail, {
            errorCode: Phase010ArchiveErrorCode.TaskNumberingGap,
            findingId: taskName,
          }),
        );
      }
    }

    state = transitionPhase010ArchiveState(state, Phase010ArchiveEvent.ArchiveRead).state;

    for (const evidence of requiredReleaseEvidence) {
      let text;
      try {
        text = await readFile(join(root, evidence.path), "utf8");
      } catch {
        return toFailedResult(
          transitionPhase010ArchiveState(state, Phase010ArchiveEvent.Fail, {
            errorCode: Phase010ArchiveErrorCode.ReleaseEvidenceMissing,
            findingId: evidence.path,
          }),
        );
      }
      if (!text.includes(evidence.marker)) {
        return toFailedResult(
          transitionPhase010ArchiveState(state, Phase010ArchiveEvent.Fail, {
            errorCode: Phase010ArchiveErrorCode.ReleaseEvidenceMarkerMissing,
            findingId: evidence.path,
          }),
        );
      }
    }

    state = transitionPhase010ArchiveState(state, Phase010ArchiveEvent.MarkersValidated).state;

    const result = {
      passed: true,
      state: Phase010ArchiveState.Passed,
      archivedTaskCount: archivedTasks.length,
      releaseEvidenceCount: requiredReleaseEvidence.length,
      prerequisiteMarkers: ["phase009_ux_release_gate=passed"],
    };
    const artifact = renderPhase010ArchiveValidationArtifact(result);

    if (writeArtifact) {
      await mkdir(join(root, ".tasks"), { recursive: true });
      await writeFile(join(root, ".tasks", "phase010-archive-validation-result.md"), artifact);
    }

    state = transitionPhase010ArchiveState(state, Phase010ArchiveEvent.ResultWritten).state;
    return { ...result, state };
  } catch (error) {
    const failed = transitionPhase010ArchiveState(state, Phase010ArchiveEvent.Fail, {
      errorCode: Phase010ArchiveErrorCode.IoFailed,
      findingId: error.path ?? error.message,
    });
    return toFailedResult(failed);
  }
}

export function renderPhase010ArchiveValidationArtifact(result) {
  const marker = result.passed
    ? "phase010_archive_validation=passed"
    : "phase010_archive_validation=failed";

  const lines = [
    "# Phase 010 Archive Validation Result",
    "",
    marker,
    `validation_state=${result.state}`,
    "",
    "- phase: `Phase 010.0`",
    "- gate: `Archive Integrity Validation`",
    `- status: \`${result.passed ? "passed" : "failed"}\``,
    "- prerequisites:",
    "  - `.tasks/phase009/phase009-ux-release-gate-result.md` with `phase009_ux_release_gate=passed`",
    "- validation commands:",
    "  - `npm run run:phase010-archive-validator-tests`",
    "  - `npm run run:phase010-archive-validator`",
    `- archived task count: ${result.archivedTaskCount ?? 0}`,
    `- release evidence count: ${result.releaseEvidenceCount ?? 0}`,
    "- changed layers: `task-tooling`, `release-tooling`.",
    "- p95 300ms path impact: none. This validator reads marker artifacts only.",
    "- scope lock: validates Phase 009 archive before Phase 010 personal local desktop work continues.",
    "- completion evidence: marker artifacts only. Task checkbox text is not release evidence.",
    "- sensitive data exclusion: this artifact records marker names, counts, paths, scopes, and stable error codes only. It does not record raw document body, asset content, AI prompt, AI answer, provider key, token, credential, secret, or personal absolute path.",
  ];

  if (!result.passed) {
    lines.push(`- error_code: \`${result.errorCode}\``);
    lines.push(`- finding_id: \`${result.findingId ?? "unknown"}\``);
  }

  lines.push("");
  return lines.join("\n");
}

function toFailedResult(failedTransition) {
  return {
    passed: false,
    state: Phase010ArchiveState.Failed,
    errorCode: failedTransition.errorCode,
    findingId: failedTransition.findingId,
    archivedTaskCount: 0,
    releaseEvidenceCount: requiredReleaseEvidence.length,
  };
}

async function main() {
  const result = await runPhase010ArchiveValidation({ root: process.cwd(), writeArtifact: true });
  if (!result.passed) {
    console.error(`${result.errorCode} ${result.findingId ?? ""}`.trim());
    process.exit(1);
  }
  console.log("phase010_archive_validation=passed");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    console.error(error);
    process.exit(1);
  });
}
