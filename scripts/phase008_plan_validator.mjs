import { readdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const Phase008PlanState = Object.freeze({
  NotStarted: "NotStarted",
  ReadingArchive: "ReadingArchive",
  ValidatingPlan: "ValidatingPlan",
  ValidatingReadme: "ValidatingReadme",
  WritingResult: "WritingResult",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase008PlanEvent = Object.freeze({
  Start: "Start",
  ArchiveChecked: "ArchiveChecked",
  PlanChecked: "PlanChecked",
  ReadmeChecked: "ReadmeChecked",
  ResultWritten: "ResultWritten",
  Fail: "Fail",
});

export const Phase008PlanErrorCode = Object.freeze({
  ActivePhase007ArtifactFound: "PHASE008_ACTIVE_PHASE007_ARTIFACT_FOUND",
  Phase007ArchiveMissing: "PHASE007_ARCHIVE_MISSING",
  Phase007ReleaseMarkerMissing: "PHASE007_RELEASE_MARKER_MISSING",
  Phase007ProductSmokeMarkerMissing: "PHASE007_PRODUCT_SMOKE_MARKER_MISSING",
  Phase008ActivePhaseMismatch: "PHASE008_ACTIVE_PHASE_MISMATCH",
  Phase008ForbiddenActiveScope: "PHASE008_FORBIDDEN_ACTIVE_SCOPE",
  Phase008PlanSectionMissing: "PHASE008_PLAN_SECTION_MISSING",
  Phase008PlanTermMissing: "PHASE008_PLAN_TERM_MISSING",
  Phase008ReadmeMissing: "PHASE008_README_MISSING",
  Phase008ReadmeTermMissing: "PHASE008_README_TERM_MISSING",
  IoFailed: "PHASE008_PLAN_IO_FAILED",
  InvalidTransition: "PHASE008_PLAN_INVALID_TRANSITION",
});

const requiredPhase007ArchiveDirectories = [
  ".tasks/phase007",
  ".tasks/phase007/release",
];

const requiredPhase007ArchiveFiles = [
  ".tasks/phase007/plan.md",
  ".tasks/phase007/phase007-plan-validation-result.md",
  ".tasks/phase007/phase007-workspace-home-gate-result.md",
  ".tasks/phase007/phase007-document-authoring-gate-result.md",
  ".tasks/phase007/phase007-local-persistence-gate-result.md",
  ".tasks/phase007/phase007-discovery-gate-result.md",
  ".tasks/phase007/phase007-ai-assistant-gate-result.md",
  ".tasks/phase007/phase007-data-ownership-gate-result.md",
  ".tasks/phase007/phase007-product-smoke-gate-result.md",
  ".tasks/phase007/phase007-release-gate-result.md",
  ...Array.from(
    { length: 8 },
    (_, index) => `.tasks/phase007/task${String(index + 1).padStart(3, "0")}.md`,
  ),
  ".tasks/phase007/release/performance-budget-phase007.md",
  ".tasks/phase007/release/ai-status-result-budget-phase007.md",
  ".tasks/phase007/release/local-desktop-runbook.md",
  ".tasks/phase007/release/product-log-event-matrix.md",
  ".tasks/phase007/release/security-log-policy-manifest.json",
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
  "## 10. Performance Budget Strategy",
  "## 11. Security and Privacy Gate Strategy",
  "## 12. State Machine Strategy",
  "## 13. Dependency and Boundary Rules",
  "## 14. Risk and Mitigation",
  "## 15. Review Checklist",
  "## 16. Definition of Done",
  "## 17. Prohibited Implementation Patterns",
  "## 18. Required Task Format",
  "## 19. Expected Task Decomposition",
  "## 20. Next Actions",
];

const requiredTerms = [
  "현재 단계: Phase 008",
  "Native Local Runtime, Durable Workspace, and Desktop Execution Hardening",
  "personal_local_desktop",
  "Native Local Runtime",
  "Durable Workspace",
  "Desktop Execution Hardening",
  "phase008_plan_validation=passed",
  "phase008_release_gate=passed",
  "LocalDesktopConfig",
  "SelectedAssetDraft",
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
  "Gate artifact contract",
  "Performance Budget Strategy",
  "Security and Privacy Gate Strategy",
];

const requiredTermGroups = [
  ["server/SaaS/multi-user", "서버/SaaS/멀티 사용자"],
];

const requiredEvidenceRows = [
  {
    filePath: ".tasks/phase008-plan-validation-result.md",
    marker: "phase008_plan_validation=passed",
  },
  {
    filePath: ".tasks/phase008-release-gate-result.md",
    marker: "phase008_release_gate=passed",
  },
  {
    filePath: ".tasks/release/performance-budget-phase008.md",
    marker: "phase008_performance_budget=passed",
  },
];

const requiredReadmeTerms = [
  "Active phase: Phase 008",
  "Active plan: `.tasks/plan.md`",
  "Current product scope: `personal_local_desktop`",
  "Phase 008 root tasks restart at `.tasks/task001.md`.",
  "Archive phases: Phase 001, Phase 002, Phase 003, Phase 004, Phase 005, Phase 006, Phase 007",
];

const forbiddenActiveScopeTerms = [
  "phase 008 active implementation: server hosting runtime",
  "phase 008 active implementation: saas runtime",
  "phase 008 active implementation: multi-user runtime",
  "phase 008 builds server hosting runtime",
  "phase 008 builds saas runtime",
  "phase 008 builds multi-user runtime",
  "현재 개발 범위: 서버 호스팅",
  "현재 개발 범위: saas",
  "현재 개발 범위: 멀티 사용자",
];

export function transitionPhase008PlanState(currentState, event, detail = {}) {
  if (currentState === Phase008PlanState.NotStarted && event === Phase008PlanEvent.Start) {
    return { state: Phase008PlanState.ReadingArchive };
  }
  if (
    currentState === Phase008PlanState.ReadingArchive &&
    event === Phase008PlanEvent.ArchiveChecked
  ) {
    return { state: Phase008PlanState.ValidatingPlan };
  }
  if (
    currentState === Phase008PlanState.ValidatingPlan &&
    event === Phase008PlanEvent.PlanChecked
  ) {
    return { state: Phase008PlanState.ValidatingReadme };
  }
  if (
    currentState === Phase008PlanState.ValidatingReadme &&
    event === Phase008PlanEvent.ReadmeChecked
  ) {
    return { state: Phase008PlanState.WritingResult };
  }
  if (
    currentState === Phase008PlanState.WritingResult &&
    event === Phase008PlanEvent.ResultWritten
  ) {
    return { state: Phase008PlanState.Passed };
  }
  if (
    [
      Phase008PlanState.ReadingArchive,
      Phase008PlanState.ValidatingPlan,
      Phase008PlanState.ValidatingReadme,
      Phase008PlanState.WritingResult,
    ].includes(currentState) &&
    event === Phase008PlanEvent.Fail
  ) {
    return {
      state: Phase008PlanState.Failed,
      errorCode: detail.errorCode ?? Phase008PlanErrorCode.IoFailed,
      findingId: detail.findingId,
    };
  }
  return {
    state: Phase008PlanState.Failed,
    errorCode: Phase008PlanErrorCode.InvalidTransition,
  };
}

export function validatePhase008PlanText(text) {
  if (!text.includes("# Phase 008") || !text.includes("현재 단계: Phase 008")) {
    return [
      {
        errorCode: Phase008PlanErrorCode.Phase008ActivePhaseMismatch,
        findingId: "current_phase",
      },
    ];
  }
  if (text.includes("현재 단계: Phase 007 -")) {
    return [
      {
        errorCode: Phase008PlanErrorCode.Phase008ActivePhaseMismatch,
        findingId: "phase007_active_state",
      },
    ];
  }

  const lowerText = text.toLowerCase();
  for (const term of forbiddenActiveScopeTerms) {
    if (lowerText.includes(term.toLowerCase())) {
      return [
        {
          errorCode: Phase008PlanErrorCode.Phase008ForbiddenActiveScope,
          findingId: term.replace(/^phase 008 active implementation: /i, ""),
        },
      ];
    }
  }

  for (const section of requiredSections) {
    if (!text.includes(section)) {
      return [
        {
          errorCode: Phase008PlanErrorCode.Phase008PlanSectionMissing,
          findingId: section,
        },
      ];
    }
  }
  for (const term of requiredTerms) {
    if (!text.includes(term)) {
      return [
        {
          errorCode: Phase008PlanErrorCode.Phase008PlanTermMissing,
          findingId: term,
        },
      ];
    }
  }
  for (const group of requiredTermGroups) {
    if (!group.some((term) => text.includes(term))) {
      return [
        {
          errorCode: Phase008PlanErrorCode.Phase008PlanTermMissing,
          findingId: group.join(" | "),
        },
      ];
    }
  }
  for (const row of requiredEvidenceRows) {
    if (!hasEvidenceRow(text, row)) {
      return [
        {
          errorCode: Phase008PlanErrorCode.Phase008PlanTermMissing,
          findingId: row.marker,
        },
      ];
    }
  }
  return [];
}

export function validatePhase008ReadmeText(text) {
  for (const term of requiredReadmeTerms) {
    if (!text.includes(term)) {
      return [
        {
          errorCode: Phase008PlanErrorCode.Phase008ReadmeTermMissing,
          findingId: term,
        },
      ];
    }
  }
  return [];
}

export async function runPhase008PlanValidation({
  root = process.cwd(),
  planPath = ".tasks/plan.md",
  readmePath = ".tasks/readme.md",
} = {}) {
  let state = transitionPhase008PlanState(
    Phase008PlanState.NotStarted,
    Phase008PlanEvent.Start,
  );

  const archiveFindings = await validatePhase007Archive({ root });
  if (archiveFindings.length > 0) {
    state = transitionPhase008PlanState(
      state.state,
      Phase008PlanEvent.Fail,
      archiveFindings[0],
    );
    return failedResult(state, archiveFindings);
  }
  state = transitionPhase008PlanState(state.state, Phase008PlanEvent.ArchiveChecked);

  try {
    const text = await readFile(join(root, planPath), "utf8");
    const planFindings = validatePhase008PlanText(text);
    if (planFindings.length > 0) {
      state = transitionPhase008PlanState(
        state.state,
        Phase008PlanEvent.Fail,
        planFindings[0],
      );
      return failedResult(state, planFindings);
    }
    state = transitionPhase008PlanState(state.state, Phase008PlanEvent.PlanChecked);
  } catch {
    state = transitionPhase008PlanState(state.state, Phase008PlanEvent.Fail, {
      errorCode: Phase008PlanErrorCode.IoFailed,
      findingId: planPath,
    });
    return failedResult(state, []);
  }

  try {
    const readmeText = await readFile(join(root, readmePath), "utf8");
    const readmeFindings = validatePhase008ReadmeText(readmeText);
    if (readmeFindings.length > 0) {
      state = transitionPhase008PlanState(
        state.state,
        Phase008PlanEvent.Fail,
        readmeFindings[0],
      );
      return failedResult(state, readmeFindings);
    }
    state = transitionPhase008PlanState(state.state, Phase008PlanEvent.ReadmeChecked);
    state = transitionPhase008PlanState(state.state, Phase008PlanEvent.ResultWritten);
  } catch {
    state = transitionPhase008PlanState(state.state, Phase008PlanEvent.Fail, {
      errorCode: Phase008PlanErrorCode.Phase008ReadmeMissing,
      findingId: readmePath,
    });
    return failedResult(state, []);
  }

  return {
    passed: true,
    state: state.state,
    archiveFileCount:
      requiredPhase007ArchiveDirectories.length + requiredPhase007ArchiveFiles.length,
    requiredSectionCount: requiredSections.length,
    requiredTermCount: requiredTerms.length,
    requiredReadmeTermCount: requiredReadmeTerms.length,
    findings: [],
  };
}

export function renderPhase008PlanValidationResult(result) {
  if (result.passed) {
    return [
      "phase008_plan_validation=passed",
      `validation_state=${result.state}`,
      `archive_file_count=${result.archiveFileCount}`,
      `required_section_count=${result.requiredSectionCount}`,
      `required_term_count=${result.requiredTermCount}`,
      `required_readme_term_count=${result.requiredReadmeTermCount}`,
    ].join("\n");
  }

  const lines = [
    "phase008_plan_validation=failed",
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

export function renderPhase008PlanValidationArtifact(result) {
  const rendered = renderPhase008PlanValidationResult(result);
  const status = result.passed ? "passed" : "failed";
  return [
    "# Phase 008 Plan Validation Result",
    "",
    rendered,
    "",
    "- phase: `Phase 008.0`",
    "- gate: `Plan Validation`",
    `- status: \`${status}\``,
    `- state: \`${result.state}\``,
    "- prerequisites:",
    "  - `.tasks/phase007/phase007-release-gate-result.md` with `phase007_release_gate=passed`",
    "  - `.tasks/phase007/phase007-product-smoke-gate-result.md` with `phase007_product_smoke_gate=passed`",
    "- validation commands:",
    "  - `npm run run:phase008-plan-validator-tests`",
    "  - `npm run run:phase008-plan-validator`",
    "- source evidence:",
    "  - `.tasks/phase007/phase007-release-gate-result.md`",
    "  - `.tasks/phase007/phase007-product-smoke-gate-result.md`",
    "  - `.tasks/plan.md`",
    "  - `.tasks/readme.md`",
    "- changed layers:",
    "  - `scripts`: Phase 008 plan validator and wrappers",
    "  - `.tasks`: Phase 008 task and validation result artifact",
    "- scope lock: `personal_local_desktop` active scope only; server hosting, SaaS, and multi-user runtime remain future-compatible architecture, not active implementation.",
    "- sensitive data exclusion: this artifact records markers, counts, state, and stable error codes only. It does not record document body, asset content, AI prompt, AI answer, provider key, token, credential, secret, or personal absolute path.",
    "- follow-up limitation: Phase 008.1 native bootstrap, local desktop config, and first-run runtime composition remain incomplete.",
    "",
  ].join("\n");
}

async function validatePhase007Archive({ root }) {
  for (const relativePath of requiredPhase007ArchiveDirectories) {
    try {
      await readdir(join(root, relativePath));
    } catch {
      return [
        {
          errorCode: Phase008PlanErrorCode.Phase007ArchiveMissing,
          findingId: relativePath,
        },
      ];
    }
  }
  for (const relativePath of requiredPhase007ArchiveFiles) {
    try {
      await readFile(join(root, relativePath), "utf8");
    } catch {
      return [
        {
          errorCode: Phase008PlanErrorCode.Phase007ArchiveMissing,
          findingId: relativePath,
        },
      ];
    }
  }

  const productSmokePath = ".tasks/phase007/phase007-product-smoke-gate-result.md";
  const productSmokeText = await readFile(join(root, productSmokePath), "utf8");
  if (!productSmokeText.includes("phase007_product_smoke_gate=passed")) {
    return [
      {
        errorCode: Phase008PlanErrorCode.Phase007ProductSmokeMarkerMissing,
        findingId: productSmokePath,
      },
    ];
  }

  const finalReleasePath = ".tasks/phase007/phase007-release-gate-result.md";
  const finalReleaseText = await readFile(join(root, finalReleasePath), "utf8");
  if (!finalReleaseText.includes("phase007_release_gate=passed")) {
    return [
      {
        errorCode: Phase008PlanErrorCode.Phase007ReleaseMarkerMissing,
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
        errorCode: Phase008PlanErrorCode.IoFailed,
        findingId: ".tasks",
      },
    ];
  }

  for (const entry of entries) {
    if (/^phase007-.*\.md$/.test(entry)) {
      return [
        {
          errorCode: Phase008PlanErrorCode.ActivePhase007ArtifactFound,
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
    if (/^# Task \d{3}\. Phase 007\b/m.test(text)) {
      return [
        {
          errorCode: Phase008PlanErrorCode.ActivePhase007ArtifactFound,
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

function hasEvidenceRow(text, { filePath, marker }) {
  return text
    .split(/\r?\n/)
    .some((line) => line.includes(filePath) && line.includes(marker));
}

async function runCli() {
  const result = await runPhase008PlanValidation();
  const artifact = renderPhase008PlanValidationArtifact(result);
  await writeFile(".tasks/phase008-plan-validation-result.md", artifact);
  const rendered = renderPhase008PlanValidationResult(result);
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
