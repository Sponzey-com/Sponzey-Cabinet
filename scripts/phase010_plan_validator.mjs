import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const Phase010PlanState = Object.freeze({
  Pending: "Pending",
  ReadingPlan: "ReadingPlan",
  ValidatingScope: "ValidatingScope",
  WritingResult: "WritingResult",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase010PlanEvent = Object.freeze({
  Start: "Start",
  PlanRead: "PlanRead",
  ScopeValidated: "ScopeValidated",
  ResultWritten: "ResultWritten",
  Fail: "Fail",
});

export const Phase010PlanErrorCode = Object.freeze({
  ArchiveMarkerMissing: "PHASE010_ARCHIVE_MARKER_MISSING",
  ActivePhaseMismatch: "PHASE010_ACTIVE_PHASE_MISMATCH",
  ForbiddenActiveScope: "PHASE010_FORBIDDEN_ACTIVE_SCOPE",
  CheckboxEvidenceForbidden: "PHASE010_CHECKBOX_EVIDENCE_FORBIDDEN",
  RequiredTermMissing: "PHASE010_PLAN_REQUIRED_TERM_MISSING",
  RequiredMarkerMissing: "PHASE010_PLAN_REQUIRED_MARKER_MISSING",
  ReadmePhaseMismatch: "PHASE010_README_PHASE_MISMATCH",
  IoFailed: "PHASE010_PLAN_IO_FAILED",
  InvalidTransition: "PHASE010_PLAN_INVALID_TRANSITION",
});

const requiredTerms = Object.freeze([
  "# Phase 010 Development Plan",
  "현재 단계: Phase 010",
  "Installable Local Desktop Release Candidate",
  "Current product scope marker: `personal_local_desktop`",
  "Product Log",
  "Field Debug Log",
  "Development Log",
  "bootstrap",
  "p95 300ms",
  "current document",
  "history",
  "Tidy First",
  "Layered Architecture",
  "Clean Architecture",
  "TDD",
  "State Machine Strategy",
  "Phase 010 Task Execution Controls",
  "Phase 010 Active-Scope Validation Command Matrix",
  "Script Hygiene Rules",
  "Canonical Phase 010 state machine contracts",
  "Task 001. Phase 010 Archive and Plan Validation Tooling",
  "server/SaaS/multi-user/mobile",
]);

const requiredMarkers = Object.freeze([
  "phase010_archive_validation=passed",
  "phase010_plan_validation=passed",
  "phase010_packaged_launch_gate=passed",
  "phase010_first_run_workspace_gate=passed",
  "phase010_durable_authoring_gate=passed",
  "phase010_data_portability_gate=passed",
  "phase010_index_health_repair_gate=passed",
  "phase010_settings_observability_gate=passed",
  "phase010_release_gate=passed",
  "phase010_performance_budget=passed",
  "phase010_packaged_runtime_manifest=passed",
  "phase010_data_portability_manifest=passed",
  "phase010_product_log_matrix=passed",
  "phase010_security_log_manifest=passed",
  "phase010_runbook=passed",
]);

const forbiddenActiveScopeTerms = Object.freeze([
  "Phase 010 active implementation: SaaS runtime",
  "Phase 010 active implementation: server hosting runtime",
  "Phase 010 active implementation: multi-user runtime",
  "Phase 010 active implementation: mobile product implementation",
  "Phase 010 builds SaaS runtime",
  "Phase 010 builds server hosting runtime",
  "Phase 010 builds multi-user runtime",
  "current release target: SaaS",
  "current release target: server hosting",
  "current release target: multi-user",
  "현재 개발 범위: SaaS",
  "현재 개발 범위: 서버 호스팅",
  "현재 개발 범위: 멀티 사용자",
]);

const forbiddenCheckboxEvidenceTerms = Object.freeze([
  "task checkbox text as completion evidence",
  "task checkbox text as release evidence",
]);

export function transitionPhase010PlanState(currentState, event, detail = {}) {
  if (currentState === Phase010PlanState.Pending && event === Phase010PlanEvent.Start) {
    return { state: Phase010PlanState.ReadingPlan };
  }
  if (
    currentState === Phase010PlanState.ReadingPlan &&
    event === Phase010PlanEvent.PlanRead
  ) {
    return { state: Phase010PlanState.ValidatingScope };
  }
  if (
    currentState === Phase010PlanState.ValidatingScope &&
    event === Phase010PlanEvent.ScopeValidated
  ) {
    return { state: Phase010PlanState.WritingResult };
  }
  if (
    currentState === Phase010PlanState.WritingResult &&
    event === Phase010PlanEvent.ResultWritten
  ) {
    return { state: Phase010PlanState.Passed };
  }
  if (
    [
      Phase010PlanState.ReadingPlan,
      Phase010PlanState.ValidatingScope,
      Phase010PlanState.WritingResult,
    ].includes(currentState) &&
    event === Phase010PlanEvent.Fail
  ) {
    return {
      state: Phase010PlanState.Failed,
      errorCode: detail.errorCode ?? Phase010PlanErrorCode.IoFailed,
      findingId: detail.findingId,
    };
  }
  return {
    state: Phase010PlanState.Failed,
    errorCode: Phase010PlanErrorCode.InvalidTransition,
  };
}

export function validatePhase010PlanText(text) {
  if (!text.includes("# Phase 010 Development Plan") || !text.includes("현재 단계: Phase 010")) {
    return [
      {
        errorCode: Phase010PlanErrorCode.ActivePhaseMismatch,
        findingId: "current_phase",
      },
    ];
  }

  for (const term of forbiddenActiveScopeTerms) {
    if (text.includes(term)) {
      return [
        {
          errorCode: Phase010PlanErrorCode.ForbiddenActiveScope,
          findingId: term,
        },
      ];
    }
  }

  for (const line of text.split(/\r?\n/)) {
    const lowerLine = line.toLowerCase();
    const hasForbiddenTerm = forbiddenCheckboxEvidenceTerms.some((term) =>
      lowerLine.includes(term.toLowerCase()),
    );
    const isNegativeOrTestRule =
      lowerLine.includes("do not") ||
      lowerLine.includes("must not") ||
      lowerLine.includes("not ") ||
      lowerLine.includes("not release evidence") ||
      lowerLine.includes("reject") ||
      lowerLine.includes("fails if") ||
      lowerLine.includes("marker artifacts only");
    if (hasForbiddenTerm && !isNegativeOrTestRule) {
      return [
        {
          errorCode: Phase010PlanErrorCode.CheckboxEvidenceForbidden,
          findingId: "task checkbox text as completion evidence",
        },
      ];
    }
  }

  for (const term of requiredTerms) {
    if (!text.includes(term)) {
      return [
        {
          errorCode: Phase010PlanErrorCode.RequiredTermMissing,
          findingId: term,
        },
      ];
    }
  }

  for (const marker of requiredMarkers) {
    if (!text.includes(marker)) {
      return [
        {
          errorCode: Phase010PlanErrorCode.RequiredMarkerMissing,
          findingId: marker,
        },
      ];
    }
  }

  return [];
}

export async function runPhase010PlanValidation({
  root = process.cwd(),
  writeArtifact = true,
} = {}) {
  let state = transitionPhase010PlanState(
    Phase010PlanState.Pending,
    Phase010PlanEvent.Start,
  ).state;

  try {
    const archiveMarkerPath = ".tasks/phase010-archive-validation-result.md";
    const archiveMarkerText = await readFile(join(root, archiveMarkerPath), "utf8");
    if (!archiveMarkerText.includes("phase010_archive_validation=passed")) {
      return toFailedResult(
        transitionPhase010PlanState(state, Phase010PlanEvent.Fail, {
          errorCode: Phase010PlanErrorCode.ArchiveMarkerMissing,
          findingId: archiveMarkerPath,
        }),
      );
    }

    const planText = await readFile(join(root, ".tasks", "plan.md"), "utf8");
    const readmeText = await readFile(join(root, ".tasks", "readme.md"), "utf8");

    state = transitionPhase010PlanState(state, Phase010PlanEvent.PlanRead).state;

    if (
      !readmeText.includes("Active phase: Phase 010") ||
      !readmeText.includes("Current product scope: `personal_local_desktop`")
    ) {
      return toFailedResult(
        transitionPhase010PlanState(state, Phase010PlanEvent.Fail, {
          errorCode: Phase010PlanErrorCode.ReadmePhaseMismatch,
          findingId: ".tasks/readme.md",
        }),
      );
    }

    const findings = validatePhase010PlanText(planText);
    if (findings.length > 0) {
      return toFailedResult(
        transitionPhase010PlanState(state, Phase010PlanEvent.Fail, {
          errorCode: findings[0].errorCode,
          findingId: findings[0].findingId,
        }),
      );
    }

    state = transitionPhase010PlanState(state, Phase010PlanEvent.ScopeValidated).state;

    const result = {
      passed: true,
      state: Phase010PlanState.Passed,
      requiredTermCount: requiredTerms.length,
      requiredMarkerCount: requiredMarkers.length,
      prerequisiteMarkers: ["phase010_archive_validation=passed"],
    };
    const artifact = renderPhase010PlanValidationArtifact(result);

    if (writeArtifact) {
      await mkdir(join(root, ".tasks"), { recursive: true });
      await writeFile(join(root, ".tasks", "phase010-plan-validation-result.md"), artifact);
    }

    state = transitionPhase010PlanState(state, Phase010PlanEvent.ResultWritten).state;
    return { ...result, state };
  } catch (error) {
    const failed = transitionPhase010PlanState(state, Phase010PlanEvent.Fail, {
      errorCode: Phase010PlanErrorCode.IoFailed,
      findingId: error.path ?? error.message,
    });
    return toFailedResult(failed);
  }
}

export function renderPhase010PlanValidationArtifact(result) {
  const marker = result.passed
    ? "phase010_plan_validation=passed"
    : "phase010_plan_validation=failed";

  const lines = [
    "# Phase 010 Plan Validation Result",
    "",
    marker,
    `validation_state=${result.state}`,
    "",
    "- phase: `Phase 010.0`",
    "- gate: `Plan Validation and Scope Lock`",
    `- status: \`${result.passed ? "passed" : "failed"}\``,
    "- prerequisites:",
    "  - `.tasks/phase010-archive-validation-result.md` with `phase010_archive_validation=passed`",
    "- validation commands:",
    "  - `npm run run:phase010-plan-validator-tests`",
    "  - `npm run run:phase010-plan-validator`",
    `- required term count: ${result.requiredTermCount ?? 0}`,
    `- required marker count: ${result.requiredMarkerCount ?? 0}`,
    "- changed layers: `task-tooling`, `release-tooling`.",
    "- p95 300ms path impact: none. This validator reads planning and marker artifacts only.",
    "- scope lock: personal local desktop only. Server hosting, SaaS, multi-user, mobile implementation, SSO, billing, admin console, and collaboration are future/out-of-scope for Phase 010.",
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
    state: Phase010PlanState.Failed,
    errorCode: failedTransition.errorCode,
    findingId: failedTransition.findingId,
    requiredTermCount: requiredTerms.length,
    requiredMarkerCount: requiredMarkers.length,
  };
}

async function main() {
  const result = await runPhase010PlanValidation({ root: process.cwd(), writeArtifact: true });
  if (!result.passed) {
    console.error(`${result.errorCode} ${result.findingId ?? ""}`.trim());
    process.exit(1);
  }
  console.log("phase010_plan_validation=passed");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    console.error(error);
    process.exit(1);
  });
}
