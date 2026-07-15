import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const Phase003PlanState = Object.freeze({
  NotStarted: "NotStarted",
  ReadingPlan: "ReadingPlan",
  Validating: "Validating",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase003PlanEvent = Object.freeze({
  Start: "Start",
  PlanLoaded: "PlanLoaded",
  Complete: "Complete",
  Fail: "Fail",
});

export const Phase003PlanErrorCode = Object.freeze({
  ActivePhaseMismatch: "PHASE003_PLAN_ACTIVE_PHASE_MISMATCH",
  RequiredSectionMissing: "PHASE003_PLAN_REQUIRED_SECTION_MISSING",
  RequiredTermMissing: "PHASE003_PLAN_REQUIRED_TERM_MISSING",
  PolicyPhraseMissing: "PHASE003_PLAN_POLICY_PHRASE_MISSING",
  IoFailed: "PHASE003_PLAN_IO_FAILED",
  InvalidTransition: "PHASE003_PLAN_INVALID_TRANSITION",
});

const requiredSections = [
  "## 1. Project Goal",
  "## 2. Current State Assessment",
  "## 3. Architecture Direction",
  "## 4. Development Principles",
  "## 5. Implementation Phases",
  "## 6. TDD Strategy",
  "## 7. Configuration and Runtime Environment Policy",
  "## 8. Logging Strategy",
  "## 9. State Machine Strategy",
  "## 10. Dependency and Boundary Rules",
  "## 11. Performance Strategy",
  "## 12. Release and Validation Gates",
  "## 13. Risk and Mitigation",
  "## 14. Review Checklist",
  "## 15. Definition of Done",
  "## 16. Prohibited Implementation Patterns",
  "## 17. Next Task Decision",
];

const requiredTerms = [
  "contract complete",
  "runtime wired",
  "product smoke passed",
  "production hardening complete",
];

const requiredPolicyPhrases = [
  "현재 단계: Phase 003",
  ".tasks/phase002/archive-manifest.json",
  "Layered Architecture",
  "Clean Architecture",
  "Tidy First",
  "TDD",
  "Runtime config is read once at bootstrap",
  "Product Log",
  "Field Debug Log",
  "Development Log",
  "state machine",
  "p95 300ms",
  "Web, iOS, Android, Windows, macOS, Linux",
];

export function transitionPhase003PlanState(currentState, event, detail = {}) {
  if (currentState === Phase003PlanState.NotStarted && event === Phase003PlanEvent.Start) {
    return { state: Phase003PlanState.ReadingPlan };
  }
  if (currentState === Phase003PlanState.ReadingPlan && event === Phase003PlanEvent.PlanLoaded) {
    return { state: Phase003PlanState.Validating };
  }
  if (currentState === Phase003PlanState.Validating && event === Phase003PlanEvent.Complete) {
    return { state: Phase003PlanState.Passed };
  }
  if (
    [Phase003PlanState.ReadingPlan, Phase003PlanState.Validating].includes(currentState) &&
    event === Phase003PlanEvent.Fail
  ) {
    return {
      state: Phase003PlanState.Failed,
      errorCode: detail.errorCode ?? Phase003PlanErrorCode.IoFailed,
      findingId: detail.findingId,
    };
  }
  return {
    state: Phase003PlanState.Failed,
    errorCode: Phase003PlanErrorCode.InvalidTransition,
  };
}

export function validatePhase003PlanText(text) {
  if (!text.includes("현재 단계: Phase 003")) {
    return [
      {
        errorCode: Phase003PlanErrorCode.ActivePhaseMismatch,
        findingId: "current_phase",
      },
    ];
  }
  if (text.includes("현재 단계: Phase 002")) {
    return [
      {
        errorCode: Phase003PlanErrorCode.ActivePhaseMismatch,
        findingId: "phase002_active_state",
      },
    ];
  }
  for (const section of requiredSections) {
    if (!text.includes(section)) {
      return [
        {
          errorCode: Phase003PlanErrorCode.RequiredSectionMissing,
          findingId: section,
        },
      ];
    }
  }
  for (const term of requiredTerms) {
    if (!text.includes(term)) {
      return [
        {
          errorCode: Phase003PlanErrorCode.RequiredTermMissing,
          findingId: term,
        },
      ];
    }
  }
  for (const phrase of requiredPolicyPhrases) {
    if (!text.includes(phrase)) {
      return [
        {
          errorCode: Phase003PlanErrorCode.PolicyPhraseMissing,
          findingId: phrase,
        },
      ];
    }
  }
  return [];
}

export async function runPhase003PlanValidation({
  root = process.cwd(),
  planPath = ".tasks/plan.md",
} = {}) {
  let state = transitionPhase003PlanState(Phase003PlanState.NotStarted, Phase003PlanEvent.Start);
  try {
    const text = await readFile(join(root, planPath), "utf8");
    state = transitionPhase003PlanState(state.state, Phase003PlanEvent.PlanLoaded);
    const findings = validatePhase003PlanText(text);
    if (findings.length > 0) {
      state = transitionPhase003PlanState(state.state, Phase003PlanEvent.Fail, findings[0]);
      return failedResult(state, findings);
    }
    state = transitionPhase003PlanState(state.state, Phase003PlanEvent.Complete);
    return {
      passed: true,
      state: state.state,
      requiredSectionCount: requiredSections.length,
      requiredTermCount: requiredTerms.length,
      findings: [],
    };
  } catch {
    state = transitionPhase003PlanState(state.state, Phase003PlanEvent.Fail, {
      errorCode: Phase003PlanErrorCode.IoFailed,
      findingId: planPath,
    });
    return failedResult(state, []);
  }
}

export function renderPhase003PlanValidationResult(result) {
  if (result.passed) {
    return [
      "phase003_plan_validation=passed",
      `validation_state=${result.state}`,
      `required_section_count=${result.requiredSectionCount}`,
      `required_term_count=${result.requiredTermCount}`,
    ].join("\n");
  }

  const lines = [
    "phase003_plan_validation=failed",
    `validation_state=${result.state}`,
    `error_code=${result.errorCode}`,
  ];
  if (result.findingId) {
    lines.push(`finding_id=${result.findingId}`);
  }
  if (result.findings?.length) {
    lines.push(`finding_count=${result.findings.length}`);
  }
  return lines.join("\n");
}

function failedResult(state, findings) {
  return {
    passed: false,
    state: state.state,
    errorCode: state.errorCode,
    findingId: state.findingId,
    findings,
  };
}

async function runCli() {
  const result = await runPhase003PlanValidation();
  const rendered = renderPhase003PlanValidationResult(result);
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
