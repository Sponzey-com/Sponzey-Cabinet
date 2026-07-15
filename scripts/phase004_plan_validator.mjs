import { readdir, readFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const Phase004PlanState = Object.freeze({
  Pending: "Pending",
  CheckingArchive: "CheckingArchive",
  CheckingPlan: "CheckingPlan",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase004PlanEvent = Object.freeze({
  Start: "Start",
  ArchiveChecked: "ArchiveChecked",
  PlanChecked: "PlanChecked",
  Fail: "Fail",
});

export const Phase004PlanErrorCode = Object.freeze({
  ActiveArchiveConflict: "PHASE004_ACTIVE_ARCHIVE_CONFLICT",
  Phase003ArchiveMissing: "PHASE003_ARCHIVE_MISSING",
  Phase003ReleaseMarkerMissing: "PHASE003_RELEASE_MARKER_MISSING",
  Phase004ActivePhaseMismatch: "PHASE004_ACTIVE_PHASE_MISMATCH",
  Phase004PlanSectionMissing: "PHASE004_PLAN_SECTION_MISSING",
  Phase004PlanTermMissing: "PHASE004_PLAN_TERM_MISSING",
  IoFailed: "PHASE004_PLAN_IO_FAILED",
  InvalidTransition: "PHASE004_PLAN_INVALID_TRANSITION",
});

const requiredPhase003ArchiveFiles = [
  ".tasks/phase003/README.md",
  ".tasks/phase003/plan.md",
  ".tasks/phase003/phase-gates.md",
  ".tasks/phase003/runtime-wiring-audit.md",
  ".tasks/phase003/persistence-gap-audit.md",
  ".tasks/phase003/durable-dependency-manifest-audit.md",
  ".tasks/phase003/recovery-coverage-audit.md",
  ".tasks/phase003/product-smoke-coverage-audit.md",
  ".tasks/phase003/product-smoke-gate-result.md",
  ".tasks/phase003/packaging-coverage-audit.md",
  ".tasks/phase003/packaging-gate-result.md",
  ".tasks/phase003/hardening-coverage-audit.md",
  ".tasks/phase003/phase003-gate-result.md",
  ".tasks/phase003/final-release-gate-result.md",
  ...Array.from(
    { length: 59 },
    (_, index) => `.tasks/phase003/task${String(index + 1).padStart(3, "0")}.md`,
  ),
];

const requiredSections = [
  "## 1. Project Goal",
  "## 2. Source Documents and Baseline",
  "## 3. Current Plan Assessment",
  "## 4. Architecture Direction",
  "## 5. Development Principles",
  "## 6. Implementation Phases",
  "## 7. TDD Strategy",
  "## 8. Tidy First Strategy",
  "## 9. Configuration and Runtime Environment Policy",
  "## 10. Logging Strategy",
  "## 11. State Machine Strategy",
  "## 12. Dependency and Boundary Rules",
  "## 13. Performance Strategy",
  "## 14. Release and Validation Gates",
  "## 15. Risk and Mitigation",
  "## 16. Review Checklist",
  "## 17. Required Verification Criteria",
  "## 18. Definition of Done",
  "## 19. Prohibited Implementation Patterns",
  "## 20. Next Task Decision",
];

const requiredTerms = [
  "현재 단계: Phase 004",
  "Knowledge Graph",
  "Realtime Collaboration",
  "Canvas/Edgeless",
  "mobile baseline",
  "graph",
  "collaboration",
  "Canvas",
  "realtime gateway",
  "platform capability matrix",
  "projection contract complete",
  "runtime wired",
  "product smoke passed",
  "performance measured",
  "collaboration safe",
  "Layered Architecture",
  "Clean Architecture",
  "Tidy First",
  "TDD",
  "bootstrap 또는 composition root에서 1회만 읽는다",
  "Product Log",
  "Field Debug Log",
  "Development Log",
  "state machine",
  "p95 300ms",
  "phase004_release_gate",
  "Web, iOS, Android, Windows, macOS, Linux",
];

export function transitionPhase004PlanState(currentState, event, detail = {}) {
  if (currentState === Phase004PlanState.Pending && event === Phase004PlanEvent.Start) {
    return { state: Phase004PlanState.CheckingArchive };
  }
  if (
    currentState === Phase004PlanState.CheckingArchive &&
    event === Phase004PlanEvent.ArchiveChecked
  ) {
    return { state: Phase004PlanState.CheckingPlan };
  }
  if (
    currentState === Phase004PlanState.CheckingPlan &&
    event === Phase004PlanEvent.PlanChecked
  ) {
    return { state: Phase004PlanState.Passed };
  }
  if (
    [Phase004PlanState.CheckingArchive, Phase004PlanState.CheckingPlan].includes(
      currentState,
    ) &&
    event === Phase004PlanEvent.Fail
  ) {
    return {
      state: Phase004PlanState.Failed,
      errorCode: detail.errorCode ?? Phase004PlanErrorCode.IoFailed,
      findingId: detail.findingId,
    };
  }
  return {
    state: Phase004PlanState.Failed,
    errorCode: Phase004PlanErrorCode.InvalidTransition,
  };
}

export function validatePhase004PlanText(text) {
  if (!text.includes("현재 단계: Phase 004")) {
    return [
      {
        errorCode: Phase004PlanErrorCode.Phase004ActivePhaseMismatch,
        findingId: "current_phase",
      },
    ];
  }
  if (text.includes("현재 단계: Phase 003 -")) {
    return [
      {
        errorCode: Phase004PlanErrorCode.Phase004ActivePhaseMismatch,
        findingId: "phase003_active_state",
      },
    ];
  }
  for (const section of requiredSections) {
    if (!text.includes(section)) {
      return [
        {
          errorCode: Phase004PlanErrorCode.Phase004PlanSectionMissing,
          findingId: section,
        },
      ];
    }
  }
  for (const term of requiredTerms) {
    if (!text.includes(term)) {
      return [
        {
          errorCode: Phase004PlanErrorCode.Phase004PlanTermMissing,
          findingId: term,
        },
      ];
    }
  }
  return [];
}

export async function runPhase004PlanValidation({
  root = process.cwd(),
  planPath = ".tasks/plan.md",
} = {}) {
  let state = transitionPhase004PlanState(
    Phase004PlanState.Pending,
    Phase004PlanEvent.Start,
  );

  const archiveFindings = await validatePhase003Archive({ root });
  if (archiveFindings.length > 0) {
    state = transitionPhase004PlanState(
      state.state,
      Phase004PlanEvent.Fail,
      archiveFindings[0],
    );
    return failedResult(state, archiveFindings);
  }
  state = transitionPhase004PlanState(state.state, Phase004PlanEvent.ArchiveChecked);

  try {
    const text = await readFile(join(root, planPath), "utf8");
    const planFindings = validatePhase004PlanText(text);
    if (planFindings.length > 0) {
      state = transitionPhase004PlanState(
        state.state,
        Phase004PlanEvent.Fail,
        planFindings[0],
      );
      return failedResult(state, planFindings);
    }
    state = transitionPhase004PlanState(state.state, Phase004PlanEvent.PlanChecked);
    return {
      passed: true,
      state: state.state,
      archiveFileCount: requiredPhase003ArchiveFiles.length,
      requiredSectionCount: requiredSections.length,
      requiredTermCount: requiredTerms.length,
      findings: [],
    };
  } catch {
    state = transitionPhase004PlanState(state.state, Phase004PlanEvent.Fail, {
      errorCode: Phase004PlanErrorCode.IoFailed,
      findingId: planPath,
    });
    return failedResult(state, []);
  }
}

export function renderPhase004PlanValidationResult(result) {
  if (result.passed) {
    return [
      "phase004_plan_validation=passed",
      `validation_state=${result.state}`,
      `archive_file_count=${result.archiveFileCount}`,
      `required_section_count=${result.requiredSectionCount}`,
      `required_term_count=${result.requiredTermCount}`,
    ].join("\n");
  }

  const lines = [
    "phase004_plan_validation=failed",
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

async function validatePhase003Archive({ root }) {
  const findings = [];
  for (const relativePath of requiredPhase003ArchiveFiles) {
    try {
      await readFile(join(root, relativePath), "utf8");
    } catch {
      findings.push({
        errorCode: Phase004PlanErrorCode.Phase003ArchiveMissing,
        findingId: relativePath,
      });
      return findings;
    }
  }

  const finalReleasePath = ".tasks/phase003/final-release-gate-result.md";
  const finalReleaseText = await readFile(join(root, finalReleasePath), "utf8");
  if (
    !finalReleaseText.includes("phase003_release_gate=passed") ||
    !finalReleaseText.includes("production hardening complete")
  ) {
    return [
      {
        errorCode: Phase004PlanErrorCode.Phase003ReleaseMarkerMissing,
        findingId: finalReleasePath,
      },
    ];
  }

  return validateActiveRootTasks({ root });
}

async function validateActiveRootTasks({ root }) {
  let entries;
  try {
    entries = await readdir(join(root, ".tasks"));
  } catch {
    return [
      {
        errorCode: Phase004PlanErrorCode.IoFailed,
        findingId: ".tasks",
      },
    ];
  }
  const activeTaskFiles = entries
    .filter((entry) => /^task\d{3}\.md$/.test(entry))
    .sort();
  for (const filename of activeTaskFiles) {
    const relativePath = `.tasks/${filename}`;
    const text = await readFile(join(root, relativePath), "utf8");
    if (/^# Task \d{3}\. Phase 003\b/m.test(text)) {
      return [
        {
          errorCode: Phase004PlanErrorCode.ActiveArchiveConflict,
          findingId: relativePath,
        },
      ];
    }
  }
  return [];
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
  const result = await runPhase004PlanValidation();
  const rendered = renderPhase004PlanValidationResult(result);
  if (result.passed) {
    console.log(rendered);
    return;
  }
  console.error(rendered);
  process.exit(1);
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runCli();
}
