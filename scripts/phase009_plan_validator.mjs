import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const Phase009PlanState = Object.freeze({
  NotStarted: "NotStarted",
  ReadingPrerequisites: "ReadingPrerequisites",
  ValidatingPlan: "ValidatingPlan",
  WritingResult: "WritingResult",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase009PlanEvent = Object.freeze({
  Start: "Start",
  PrerequisitesRead: "PrerequisitesRead",
  PlanValidated: "PlanValidated",
  ResultWritten: "ResultWritten",
  Fail: "Fail",
});

export const Phase009PlanErrorCode = Object.freeze({
  CurrentInventoryMarkerMissing: "PHASE009_CURRENT_INVENTORY_MARKER_MISSING",
  Phase008ReleaseMarkerMissing: "PHASE009_PHASE008_RELEASE_MARKER_MISSING",
  ActivePhaseMismatch: "PHASE009_ACTIVE_PHASE_MISMATCH",
  ForbiddenActiveScope: "PHASE009_FORBIDDEN_ACTIVE_SCOPE",
  CheckboxEvidenceForbidden: "PHASE009_CHECKBOX_EVIDENCE_FORBIDDEN",
  RequiredTermMissing: "PHASE009_PLAN_REQUIRED_TERM_MISSING",
  RequiredMarkerMissing: "PHASE009_PLAN_REQUIRED_MARKER_MISSING",
  IoFailed: "PHASE009_PLAN_IO_FAILED",
  InvalidTransition: "PHASE009_PLAN_INVALID_TRANSITION",
});

const requiredTerms = [
  "# Phase 009 Development Plan",
  "현재 단계: Phase 009",
  "Current product scope marker: `personal_local_desktop`",
  "Phase 009는 서버/SaaS/멀티 사용자 기능을 추가하지 않는다",
  "Product Log",
  "Field Debug Log",
  "Development Log",
  "bootstrap",
  "p95 300ms",
  "current document",
  "history read",
  "Tidy First",
  "Task files",
  "Blank screen",
  "Layered Architecture",
  "Clean Architecture",
  "TDD",
  "State machines",
  "Mandatory Validation Matrix",
];

const requiredMarkers = [
  "phase009_current_inventory=passed",
  "phase009_plan_validation=passed",
  "phase009_desktop_launch_gate=passed",
  "phase009_command_runtime_gate=passed",
  "phase009_document_authoring_gate=passed",
  "phase009_discovery_assets_gate=passed",
  "phase009_recovery_backup_ux_gate=passed",
  "phase009_ux_release_gate=passed",
  "phase009_performance_budget=passed",
];

const forbiddenActiveScopeTerms = [
  "Phase 009 active implementation: SaaS runtime",
  "Phase 009 active implementation: server hosting runtime",
  "Phase 009 active implementation: multi-user runtime",
  "Phase 009 builds SaaS runtime",
  "Phase 009 builds server hosting runtime",
  "Phase 009 builds multi-user runtime",
  "현재 개발 범위: SaaS",
  "현재 개발 범위: 서버 호스팅",
  "현재 개발 범위: 멀티 사용자",
];

const forbiddenCheckboxEvidenceTerms = [
  "task checkbox text as completion evidence",
  "checkbox text as release evidence",
];

export function transitionPhase009PlanState(currentState, event, detail = {}) {
  if (currentState === Phase009PlanState.NotStarted && event === Phase009PlanEvent.Start) {
    return { state: Phase009PlanState.ReadingPrerequisites };
  }
  if (
    currentState === Phase009PlanState.ReadingPrerequisites &&
    event === Phase009PlanEvent.PrerequisitesRead
  ) {
    return { state: Phase009PlanState.ValidatingPlan };
  }
  if (
    currentState === Phase009PlanState.ValidatingPlan &&
    event === Phase009PlanEvent.PlanValidated
  ) {
    return { state: Phase009PlanState.WritingResult };
  }
  if (
    currentState === Phase009PlanState.WritingResult &&
    event === Phase009PlanEvent.ResultWritten
  ) {
    return { state: Phase009PlanState.Passed };
  }
  if (
    [
      Phase009PlanState.ReadingPrerequisites,
      Phase009PlanState.ValidatingPlan,
      Phase009PlanState.WritingResult,
    ].includes(currentState) &&
    event === Phase009PlanEvent.Fail
  ) {
    return {
      state: Phase009PlanState.Failed,
      errorCode: detail.errorCode ?? Phase009PlanErrorCode.IoFailed,
      findingId: detail.findingId,
    };
  }
  return {
    state: Phase009PlanState.Failed,
    errorCode: Phase009PlanErrorCode.InvalidTransition,
  };
}

export function validatePhase009PlanText(text) {
  if (!text.includes("# Phase 009 Development Plan") || !text.includes("현재 단계: Phase 009")) {
    return [
      {
        errorCode: Phase009PlanErrorCode.ActivePhaseMismatch,
        findingId: "current_phase",
      },
    ];
  }

  for (const term of forbiddenActiveScopeTerms) {
    if (text.includes(term)) {
      return [
        {
          errorCode: Phase009PlanErrorCode.ForbiddenActiveScope,
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
      lowerLine.includes("fails if") ||
      lowerLine.includes("reject") ||
      lowerLine.includes("not release evidence") ||
      lowerLine.includes("does not rely");
    if (hasForbiddenTerm && !isNegativeOrTestRule) {
      return [
        {
          errorCode: Phase009PlanErrorCode.CheckboxEvidenceForbidden,
          findingId: "task checkbox text as completion evidence",
        },
      ];
    }
  }

  for (const term of requiredTerms) {
    if (!text.includes(term)) {
      return [
        {
          errorCode: Phase009PlanErrorCode.RequiredTermMissing,
          findingId: term,
        },
      ];
    }
  }

  for (const marker of requiredMarkers) {
    if (!text.includes(marker)) {
      return [
        {
          errorCode: Phase009PlanErrorCode.RequiredMarkerMissing,
          findingId: marker,
        },
      ];
    }
  }

  return [];
}

export async function runPhase009PlanValidation({ root = process.cwd(), writeArtifact = true } = {}) {
  let state = transitionPhase009PlanState(
    Phase009PlanState.NotStarted,
    Phase009PlanEvent.Start,
  ).state;

  try {
    const inventoryText = await readFile(
      join(root, ".tasks", "phase009-current-implementation-inventory.md"),
      "utf8",
    );
    if (!inventoryText.includes("phase009_current_inventory=passed")) {
      const failed = transitionPhase009PlanState(state, Phase009PlanEvent.Fail, {
        errorCode: Phase009PlanErrorCode.CurrentInventoryMarkerMissing,
        findingId: ".tasks/phase009-current-implementation-inventory.md",
      });
      return toFailedResult(failed);
    }

    const phase008Text = await readFile(
      join(root, ".tasks", "phase008", "phase008-release-gate-result.md"),
      "utf8",
    );
    if (!phase008Text.includes("phase008_release_gate=passed")) {
      const failed = transitionPhase009PlanState(state, Phase009PlanEvent.Fail, {
        errorCode: Phase009PlanErrorCode.Phase008ReleaseMarkerMissing,
        findingId: ".tasks/phase008/phase008-release-gate-result.md",
      });
      return toFailedResult(failed);
    }

    state = transitionPhase009PlanState(
      state,
      Phase009PlanEvent.PrerequisitesRead,
    ).state;

    const planText = await readFile(join(root, ".tasks", "plan.md"), "utf8");
    const findings = validatePhase009PlanText(planText);
    if (findings.length > 0) {
      const failed = transitionPhase009PlanState(state, Phase009PlanEvent.Fail, {
        errorCode: findings[0].errorCode,
        findingId: findings[0].findingId,
      });
      return toFailedResult(failed);
    }

    state = transitionPhase009PlanState(state, Phase009PlanEvent.PlanValidated).state;

    const result = {
      passed: true,
      state: Phase009PlanState.Passed,
      requiredTermCount: requiredTerms.length,
      requiredMarkerCount: requiredMarkers.length,
      prerequisiteMarkers: [
        "phase009_current_inventory=passed",
        "phase008_release_gate=passed",
      ],
    };

    const artifact = renderPhase009PlanValidationArtifact(result);

    if (writeArtifact) {
      await mkdir(join(root, ".tasks"), { recursive: true });
      await writeFile(join(root, ".tasks", "phase009-plan-validation-result.md"), artifact);
    }

    state = transitionPhase009PlanState(state, Phase009PlanEvent.ResultWritten).state;

    return { ...result, state };
  } catch (error) {
    const failed = transitionPhase009PlanState(state, Phase009PlanEvent.Fail, {
      errorCode: Phase009PlanErrorCode.IoFailed,
      findingId: error.path ?? error.message,
    });
    return toFailedResult(failed);
  }
}

export function renderPhase009PlanValidationArtifact(result) {
  const marker = result.passed
    ? "phase009_plan_validation=passed"
    : "phase009_plan_validation=failed";

  return [
    "# Phase 009 Plan Validation Result",
    "",
    marker,
    `validation_state=${result.state}`,
    "",
    "- phase: `Phase 009.0`",
    "- gate: `Plan Validation and Scope Lock`",
    `- status: \`${result.passed ? "passed" : "failed"}\``,
    "- prerequisites:",
    "  - `.tasks/phase009-current-implementation-inventory.md` with `phase009_current_inventory=passed`",
    "  - `.tasks/phase008/phase008-release-gate-result.md` with `phase008_release_gate=passed`",
    "- validation commands:",
    "  - `npm run run:phase009-plan-validator-tests`",
    "  - `npm run run:phase009-plan-validator`",
    `- required term count: ${result.requiredTermCount ?? 0}`,
    `- required marker count: ${result.requiredMarkerCount ?? 0}`,
    "- scope lock: personal local desktop only. Server hosting, SaaS, multi-user, SSO, billing, and collaboration are future/out-of-scope for Phase 009.",
    "- completion evidence: marker artifacts only. Task checkbox text is not release evidence.",
    "- sensitive data exclusion: this artifact records marker names, counts, paths, scopes, and stable error codes only. It does not record raw document body, asset content, AI prompt, AI answer, provider key, token, credential, secret, or personal absolute path.",
    "",
  ].join("\n");
}

function toFailedResult(failedTransition) {
  return {
    passed: false,
    state: Phase009PlanState.Failed,
    errorCode: failedTransition.errorCode,
    findingId: failedTransition.findingId,
    requiredTermCount: requiredTerms.length,
    requiredMarkerCount: requiredMarkers.length,
  };
}

async function main() {
  const result = await runPhase009PlanValidation({ root: process.cwd(), writeArtifact: true });
  if (!result.passed) {
    console.error(`${result.errorCode} ${result.findingId ?? ""}`.trim());
    process.exit(1);
  }
  console.log("phase009_plan_validation=passed");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    console.error(error);
    process.exit(1);
  });
}
