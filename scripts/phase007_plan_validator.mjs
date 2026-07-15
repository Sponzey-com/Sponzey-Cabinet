import { readdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const Phase007PlanState = Object.freeze({
  NotStarted: "NotStarted",
  ReadingArchive: "ReadingArchive",
  ValidatingPlan: "ValidatingPlan",
  ValidatingReadme: "ValidatingReadme",
  WritingResult: "WritingResult",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase007PlanEvent = Object.freeze({
  Start: "Start",
  ArchiveChecked: "ArchiveChecked",
  PlanChecked: "PlanChecked",
  ReadmeChecked: "ReadmeChecked",
  ResultWritten: "ResultWritten",
  Fail: "Fail",
});

export const Phase007PlanErrorCode = Object.freeze({
  ActivePhase006ArtifactFound: "PHASE007_ACTIVE_PHASE006_ARTIFACT_FOUND",
  Phase006ArchiveMissing: "PHASE006_ARCHIVE_MISSING",
  Phase006ReleaseMarkerMissing: "PHASE006_RELEASE_MARKER_MISSING",
  Phase007ActivePhaseMismatch: "PHASE007_ACTIVE_PHASE_MISMATCH",
  Phase007ForbiddenActiveScope: "PHASE007_FORBIDDEN_ACTIVE_SCOPE",
  Phase007PlanSectionMissing: "PHASE007_PLAN_SECTION_MISSING",
  Phase007PlanTermMissing: "PHASE007_PLAN_TERM_MISSING",
  Phase007ReadmeMissing: "PHASE007_README_MISSING",
  Phase007ReadmeTermMissing: "PHASE007_README_TERM_MISSING",
  IoFailed: "PHASE007_PLAN_IO_FAILED",
  InvalidTransition: "PHASE007_PLAN_INVALID_TRANSITION",
});

const requiredPhase006ArchiveDirectories = [
  ".tasks/phase006",
  ".tasks/phase006/release",
];

const requiredPhase006ArchiveFiles = [
  ".tasks/phase006/plan.md",
  ".tasks/phase006/phase006-plan-validation-result.md",
  ".tasks/phase006/phase006-local-runtime-gate-result.md",
  ".tasks/phase006/phase006-workspace-shell-gate-result.md",
  ".tasks/phase006/phase006-document-ux-gate-result.md",
  ".tasks/phase006/phase006-search-graph-asset-gate-result.md",
  ".tasks/phase006/phase006-ai-ux-gate-result.md",
  ".tasks/phase006/phase006-backup-package-gate-result.md",
  ".tasks/phase006/phase006-product-smoke-gate-result.md",
  ".tasks/phase006/phase006-release-gate-result.md",
  ...Array.from(
    { length: 19 },
    (_, index) => `.tasks/phase006/task${String(index + 1).padStart(3, "0")}.md`,
  ),
  ".tasks/phase006/release/performance-budget-phase006.md",
  ".tasks/phase006/release/local-desktop-runbook.md",
  ".tasks/phase006/release/product-log-event-matrix.md",
  ".tasks/phase006/release/runbook-validation-manifest.json",
  ".tasks/phase006/release/security-log-policy-manifest.json",
  ".tasks/phase006/release/data-ownership-verification.md",
];

const requiredSections = [
  "## 1. Project Goal",
  "## 2. Source Documents and Baseline",
  "## 3. Current Plan Assessment",
  "## 4. Architecture Direction",
  "## 5. Development Principles",
  "## 6. Implementation Phases",
  "## 7. TDD Strategy",
  "## 8. Configuration and Runtime Environment Policy",
  "## 9. Logging Strategy",
  "## 10. State Machine Strategy",
  "## 11. Performance Strategy",
  "## 12. Dependency and Boundary Rules",
  "## 13. Risk and Mitigation",
  "## 14. Review Checklist",
  "## 15. Definition of Done",
  "## 16. Prohibited Implementation Patterns",
  "## 17. Required Task Format",
  "## 18. Next Actions",
];

const requiredTerms = [
  "현재 단계: Phase 007",
  "Daily Local Knowledge Workspace and Desktop App Usability",
  "personal_local_desktop",
  "Daily Local Knowledge Workspace",
  "phase007_plan_validation=passed",
  "phase007_release_gate=passed",
  "CodeMirror",
  "Markdown preview",
  "current document",
  "history read",
  "p95 300ms",
  "Product Log",
  "Field Debug Log",
  "Development Log",
  "Layered Architecture",
  "Clean Architecture",
  "Tidy First",
  "TDD",
  "Windows",
  "macOS",
  "Linux",
];

const requiredTermGroups = [
  ["server/SaaS/multi-user", "서버/SaaS/멀티 사용자"],
];

const requiredReadmeTerms = [
  "Active phase: Phase 007",
  "Active plan: `.tasks/plan.md`",
  "Current product scope: `personal_local_desktop`",
  "Phase 007 root tasks restart at `.tasks/task001.md`.",
  "Archive phases: Phase 001, Phase 002, Phase 003, Phase 004, Phase 005, Phase 006",
];

const forbiddenActiveScopeTerms = [
  "phase 007 active implementation: server hosting runtime",
  "phase 007 active implementation: saas runtime",
  "phase 007 active implementation: multi-user runtime",
  "phase 007 builds server hosting runtime",
  "phase 007 builds saas runtime",
  "phase 007 builds multi-user runtime",
  "현재 개발 범위: 서버 호스팅",
  "현재 개발 범위: saas",
  "현재 개발 범위: 멀티 사용자",
];

export function transitionPhase007PlanState(currentState, event, detail = {}) {
  if (currentState === Phase007PlanState.NotStarted && event === Phase007PlanEvent.Start) {
    return { state: Phase007PlanState.ReadingArchive };
  }
  if (
    currentState === Phase007PlanState.ReadingArchive &&
    event === Phase007PlanEvent.ArchiveChecked
  ) {
    return { state: Phase007PlanState.ValidatingPlan };
  }
  if (
    currentState === Phase007PlanState.ValidatingPlan &&
    event === Phase007PlanEvent.PlanChecked
  ) {
    return { state: Phase007PlanState.ValidatingReadme };
  }
  if (
    currentState === Phase007PlanState.ValidatingReadme &&
    event === Phase007PlanEvent.ReadmeChecked
  ) {
    return { state: Phase007PlanState.WritingResult };
  }
  if (
    currentState === Phase007PlanState.WritingResult &&
    event === Phase007PlanEvent.ResultWritten
  ) {
    return { state: Phase007PlanState.Passed };
  }
  if (
    [
      Phase007PlanState.ReadingArchive,
      Phase007PlanState.ValidatingPlan,
      Phase007PlanState.ValidatingReadme,
      Phase007PlanState.WritingResult,
    ].includes(currentState) &&
    event === Phase007PlanEvent.Fail
  ) {
    return {
      state: Phase007PlanState.Failed,
      errorCode: detail.errorCode ?? Phase007PlanErrorCode.IoFailed,
      findingId: detail.findingId,
    };
  }
  return {
    state: Phase007PlanState.Failed,
    errorCode: Phase007PlanErrorCode.InvalidTransition,
  };
}

export function validatePhase007PlanText(text) {
  if (!text.includes("# Phase 007") || !text.includes("현재 단계: Phase 007")) {
    return [
      {
        errorCode: Phase007PlanErrorCode.Phase007ActivePhaseMismatch,
        findingId: "current_phase",
      },
    ];
  }
  if (text.includes("현재 단계: Phase 006 -")) {
    return [
      {
        errorCode: Phase007PlanErrorCode.Phase007ActivePhaseMismatch,
        findingId: "phase006_active_state",
      },
    ];
  }

  const lowerText = text.toLowerCase();
  for (const term of forbiddenActiveScopeTerms) {
    if (lowerText.includes(term.toLowerCase())) {
      return [
        {
          errorCode: Phase007PlanErrorCode.Phase007ForbiddenActiveScope,
          findingId: term.replace(/^phase 007 active implementation: /i, ""),
        },
      ];
    }
  }

  for (const section of requiredSections) {
    if (!text.includes(section)) {
      return [
        {
          errorCode: Phase007PlanErrorCode.Phase007PlanSectionMissing,
          findingId: section,
        },
      ];
    }
  }
  for (const term of requiredTerms) {
    if (!text.includes(term)) {
      return [
        {
          errorCode: Phase007PlanErrorCode.Phase007PlanTermMissing,
          findingId: term,
        },
      ];
    }
  }
  for (const group of requiredTermGroups) {
    if (!group.some((term) => text.includes(term))) {
      return [
        {
          errorCode: Phase007PlanErrorCode.Phase007PlanTermMissing,
          findingId: group.join(" | "),
        },
      ];
    }
  }
  return [];
}

export function validatePhase007ReadmeText(text) {
  for (const term of requiredReadmeTerms) {
    if (!text.includes(term)) {
      return [
        {
          errorCode: Phase007PlanErrorCode.Phase007ReadmeTermMissing,
          findingId: term,
        },
      ];
    }
  }
  return [];
}

export async function runPhase007PlanValidation({
  root = process.cwd(),
  planPath = ".tasks/plan.md",
  readmePath = ".tasks/readme.md",
} = {}) {
  let state = transitionPhase007PlanState(
    Phase007PlanState.NotStarted,
    Phase007PlanEvent.Start,
  );

  const archiveFindings = await validatePhase006Archive({ root });
  if (archiveFindings.length > 0) {
    state = transitionPhase007PlanState(
      state.state,
      Phase007PlanEvent.Fail,
      archiveFindings[0],
    );
    return failedResult(state, archiveFindings);
  }
  state = transitionPhase007PlanState(state.state, Phase007PlanEvent.ArchiveChecked);

  try {
    const text = await readFile(join(root, planPath), "utf8");
    const planFindings = validatePhase007PlanText(text);
    if (planFindings.length > 0) {
      state = transitionPhase007PlanState(
        state.state,
        Phase007PlanEvent.Fail,
        planFindings[0],
      );
      return failedResult(state, planFindings);
    }
    state = transitionPhase007PlanState(state.state, Phase007PlanEvent.PlanChecked);
  } catch {
    state = transitionPhase007PlanState(state.state, Phase007PlanEvent.Fail, {
      errorCode: Phase007PlanErrorCode.IoFailed,
      findingId: planPath,
    });
    return failedResult(state, []);
  }

  try {
    const readmeText = await readFile(join(root, readmePath), "utf8");
    const readmeFindings = validatePhase007ReadmeText(readmeText);
    if (readmeFindings.length > 0) {
      state = transitionPhase007PlanState(
        state.state,
        Phase007PlanEvent.Fail,
        readmeFindings[0],
      );
      return failedResult(state, readmeFindings);
    }
    state = transitionPhase007PlanState(state.state, Phase007PlanEvent.ReadmeChecked);
    state = transitionPhase007PlanState(state.state, Phase007PlanEvent.ResultWritten);
  } catch {
    state = transitionPhase007PlanState(state.state, Phase007PlanEvent.Fail, {
      errorCode: Phase007PlanErrorCode.Phase007ReadmeMissing,
      findingId: readmePath,
    });
    return failedResult(state, []);
  }

  return {
    passed: true,
    state: state.state,
    archiveFileCount:
      requiredPhase006ArchiveDirectories.length + requiredPhase006ArchiveFiles.length,
    requiredSectionCount: requiredSections.length,
    requiredTermCount: requiredTerms.length,
    requiredReadmeTermCount: requiredReadmeTerms.length,
    findings: [],
  };
}

export function renderPhase007PlanValidationResult(result) {
  if (result.passed) {
    return [
      "phase007_plan_validation=passed",
      `validation_state=${result.state}`,
      `archive_file_count=${result.archiveFileCount}`,
      `required_section_count=${result.requiredSectionCount}`,
      `required_term_count=${result.requiredTermCount}`,
      `required_readme_term_count=${result.requiredReadmeTermCount}`,
    ].join("\n");
  }

  const lines = [
    "phase007_plan_validation=failed",
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

export function renderPhase007PlanValidationArtifact(result) {
  const rendered = renderPhase007PlanValidationResult(result);
  const status = result.passed ? "passed" : "failed";
  return [
    "# Phase 007 Plan Validation Result",
    "",
    rendered,
    "",
    "- phase: `Phase 007`",
    "- gate: `Plan Validation`",
    `- status: \`${status}\``,
    `- state: \`${result.state}\``,
    "- commands:",
    "  - `npm run run:phase007-plan-validator-tests`",
    "  - `npm run run:phase007-plan-validator`",
    "- source evidence:",
    "  - `.tasks/phase006/phase006-release-gate-result.md`",
    "  - `.tasks/plan.md`",
    "  - `.tasks/readme.md`",
    "- scope lock: `personal_local_desktop` active scope only; server hosting, SaaS, and multi-user runtime remain future-compatible architecture, not active implementation.",
    "- sensitive data exclusion: this artifact records markers, counts, state, and stable error codes only. It does not record document body, asset content, AI prompt, AI answer, provider key, token, credential, or personal absolute path.",
    "- follow-up limitation: Phase 007.1 workspace home, app frame, and navigation contract remains incomplete.",
    "",
  ].join("\n");
}

async function validatePhase006Archive({ root }) {
  for (const relativePath of requiredPhase006ArchiveDirectories) {
    try {
      await readdir(join(root, relativePath));
    } catch {
      return [
        {
          errorCode: Phase007PlanErrorCode.Phase006ArchiveMissing,
          findingId: relativePath,
        },
      ];
    }
  }
  for (const relativePath of requiredPhase006ArchiveFiles) {
    try {
      await readFile(join(root, relativePath), "utf8");
    } catch {
      return [
        {
          errorCode: Phase007PlanErrorCode.Phase006ArchiveMissing,
          findingId: relativePath,
        },
      ];
    }
  }

  const finalReleasePath = ".tasks/phase006/phase006-release-gate-result.md";
  const finalReleaseText = await readFile(join(root, finalReleasePath), "utf8");
  if (!finalReleaseText.includes("phase006_release_gate=passed")) {
    return [
      {
        errorCode: Phase007PlanErrorCode.Phase006ReleaseMarkerMissing,
        findingId: finalReleasePath,
      },
    ];
  }

  return validateActiveRootArtifacts({ root });
}

async function validateActiveRootArtifacts({ root }) {
  let entries;
  try {
    entries = await readdir(join(root, ".tasks"));
  } catch {
    return [
      {
        errorCode: Phase007PlanErrorCode.IoFailed,
        findingId: ".tasks",
      },
    ];
  }

  for (const entry of entries) {
    if (/^phase006-.*\.md$/.test(entry)) {
      return [
        {
          errorCode: Phase007PlanErrorCode.ActivePhase006ArtifactFound,
          findingId: `.tasks/${entry}`,
        },
      ];
    }
  }

  const activeTaskFiles = entries
    .filter((entry) => /^task\d{3}\.md$/.test(entry))
    .sort();
  for (const filename of activeTaskFiles) {
    const relativePath = `.tasks/${filename}`;
    const text = await readFile(join(root, relativePath), "utf8");
    if (/^# Task \d{3}\. Phase 006\b/m.test(text)) {
      return [
        {
          errorCode: Phase007PlanErrorCode.ActivePhase006ArtifactFound,
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
  const result = await runPhase007PlanValidation();
  const artifact = renderPhase007PlanValidationArtifact(result);
  await writeFile(".tasks/phase007-plan-validation-result.md", artifact);
  const rendered = renderPhase007PlanValidationResult(result);
  if (result.passed) {
    console.log(rendered);
    return;
  }
  console.error(rendered);
  process.exit(1);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runCli();
}
