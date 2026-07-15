import { readdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const Phase006PlanState = Object.freeze({
  NotStarted: "NotStarted",
  ReadingArchive: "ReadingArchive",
  ValidatingPlan: "ValidatingPlan",
  ValidatingReadme: "ValidatingReadme",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase006PlanEvent = Object.freeze({
  Start: "Start",
  ArchiveChecked: "ArchiveChecked",
  PlanChecked: "PlanChecked",
  ReadmeChecked: "ReadmeChecked",
  Fail: "Fail",
});

export const Phase006PlanErrorCode = Object.freeze({
  ActiveArchiveConflict: "PHASE006_ACTIVE_ARCHIVE_CONFLICT",
  Phase005ArchiveMissing: "PHASE005_ARCHIVE_MISSING",
  Phase005ReleaseMarkerMissing: "PHASE005_RELEASE_MARKER_MISSING",
  Phase005SecurityManifestMissing: "PHASE005_SECURITY_MANIFEST_MISSING",
  Phase005SecurityManifestPathInvalid: "PHASE005_SECURITY_MANIFEST_PATH_INVALID",
  Phase006ActivePhaseMismatch: "PHASE006_ACTIVE_PHASE_MISMATCH",
  Phase006PlanSectionMissing: "PHASE006_PLAN_SECTION_MISSING",
  Phase006PlanTermMissing: "PHASE006_PLAN_TERM_MISSING",
  Phase006ReadmeMissing: "PHASE006_README_MISSING",
  Phase006ReadmeTermMissing: "PHASE006_README_TERM_MISSING",
  IoFailed: "PHASE006_PLAN_IO_FAILED",
  InvalidTransition: "PHASE006_PLAN_INVALID_TRANSITION",
});

const requiredPhase005ArchiveFiles = [
  ".tasks/phase005/plan.md",
  ".tasks/phase005/retrieval-coverage-audit.md",
  ".tasks/phase005/semantic-search-gate-result.md",
  ".tasks/phase005/ai-answer-product-gate-result.md",
  ".tasks/phase005/mcp-api-product-gate-result.md",
  ".tasks/phase005/webhook-connector-product-gate-result.md",
  ".tasks/phase005/phase005-product-smoke-gate-result.md",
  ".tasks/phase005/phase005-observability-matrix-gate-result.md",
  ".tasks/phase005/phase005-release-gate-result.md",
  ...Array.from(
    { length: 31 },
    (_, index) => `.tasks/phase005/task${String(index + 1).padStart(3, "0")}.md`,
  ),
];

const requiredSections = [
  "## 1. Project Goal",
  "## 2. Source Documents and Baseline",
  "## 3. Current Plan Assessment",
  "## 4. Architecture Direction",
  "## 5. Development Principles",
  "## 6. Implementation Phases",
  "## 7. Task Slicing Rules",
  "## 8. TDD Strategy",
  "## 9. Configuration and Runtime Environment Policy",
  "## 10. Logging Strategy",
  "## 11. State Machine Strategy",
  "## 12. Performance Strategy",
  "## 13. Dependency and Boundary Rules",
  "## 14. Risk and Mitigation",
  "## 15. Review Checklist",
  "## 16. Definition of Done",
  "## 17. Prohibited Implementation Patterns",
  "## 18. Next Actions",
];

const requiredTerms = [
  "현재 단계: Phase 006",
  "Personal Desktop Productization and Local Workspace UX",
  "personal_local_desktop",
  "개인 PC",
  "install once",
  "no server required",
  "no SaaS",
  "no multi-user",
  "local workspace",
  "modern UI/UX",
  "CodeMirror",
  "Markdown preview",
  "current/history split",
  "p95 300ms",
  "Product Log",
  "Field Debug Log",
  "Development Log",
  "state machine",
  "Layered Architecture",
  "Clean Architecture",
  "Tidy First",
  "TDD",
  "bootstrap",
  ".tasks/readme.md",
  ".tasks/phase006-plan-validation-result.md",
  "phase006_plan_validation=passed",
  "phase006_local_runtime_gate=passed",
  "phase006_workspace_shell_gate=passed",
  "phase006_document_ux_gate=passed",
  "phase006_search_graph_asset_gate=passed",
  "phase006_ai_ux_gate=passed",
  "phase006_backup_package_gate=passed",
  "phase006_product_smoke_gate=passed",
  "phase006_release_gate=passed",
  "Windows",
  "macOS",
  "Linux",
  "server/SaaS",
];

const requiredReadmeTerms = [
  "Active phase: Phase 006",
  "Active plan: `.tasks/plan.md`",
  "Archive phases: Phase 001, Phase 002, Phase 003, Phase 004, Phase 005",
  "Current product scope: personal_local_desktop",
  "Phase 006 root tasks restart at `.tasks/task001.md`.",
];

export function transitionPhase006PlanState(currentState, event, detail = {}) {
  if (currentState === Phase006PlanState.NotStarted && event === Phase006PlanEvent.Start) {
    return { state: Phase006PlanState.ReadingArchive };
  }
  if (
    currentState === Phase006PlanState.ReadingArchive &&
    event === Phase006PlanEvent.ArchiveChecked
  ) {
    return { state: Phase006PlanState.ValidatingPlan };
  }
  if (
    currentState === Phase006PlanState.ValidatingPlan &&
    event === Phase006PlanEvent.PlanChecked
  ) {
    return { state: Phase006PlanState.ValidatingReadme };
  }
  if (
    currentState === Phase006PlanState.ValidatingReadme &&
    event === Phase006PlanEvent.ReadmeChecked
  ) {
    return { state: Phase006PlanState.Passed };
  }
  if (
    [
      Phase006PlanState.ReadingArchive,
      Phase006PlanState.ValidatingPlan,
      Phase006PlanState.ValidatingReadme,
    ].includes(currentState) &&
    event === Phase006PlanEvent.Fail
  ) {
    return {
      state: Phase006PlanState.Failed,
      errorCode: detail.errorCode ?? Phase006PlanErrorCode.IoFailed,
      findingId: detail.findingId,
    };
  }
  return {
    state: Phase006PlanState.Failed,
    errorCode: Phase006PlanErrorCode.InvalidTransition,
  };
}

export function validatePhase006PlanText(text) {
  if (!text.includes("현재 단계: Phase 006")) {
    return [
      {
        errorCode: Phase006PlanErrorCode.Phase006ActivePhaseMismatch,
        findingId: "current_phase",
      },
    ];
  }
  if (text.includes("현재 단계: Phase 005 -")) {
    return [
      {
        errorCode: Phase006PlanErrorCode.Phase006ActivePhaseMismatch,
        findingId: "phase005_active_state",
      },
    ];
  }
  for (const section of requiredSections) {
    if (!text.includes(section)) {
      return [
        {
          errorCode: Phase006PlanErrorCode.Phase006PlanSectionMissing,
          findingId: section,
        },
      ];
    }
  }
  for (const term of requiredTerms) {
    if (!text.includes(term)) {
      return [
        {
          errorCode: Phase006PlanErrorCode.Phase006PlanTermMissing,
          findingId: term,
        },
      ];
    }
  }
  return [];
}

export function validatePhase006ReadmeText(text) {
  for (const term of requiredReadmeTerms) {
    if (!text.includes(term)) {
      return [
        {
          errorCode: Phase006PlanErrorCode.Phase006ReadmeTermMissing,
          findingId: term,
        },
      ];
    }
  }
  return [];
}

export async function runPhase006PlanValidation({
  root = process.cwd(),
  planPath = ".tasks/plan.md",
  readmePath = ".tasks/readme.md",
} = {}) {
  let state = transitionPhase006PlanState(
    Phase006PlanState.NotStarted,
    Phase006PlanEvent.Start,
  );

  const archiveFindings = await validatePhase005Archive({ root });
  if (archiveFindings.length > 0) {
    state = transitionPhase006PlanState(
      state.state,
      Phase006PlanEvent.Fail,
      archiveFindings[0],
    );
    return failedResult(state, archiveFindings);
  }
  state = transitionPhase006PlanState(state.state, Phase006PlanEvent.ArchiveChecked);

  try {
    const text = await readFile(join(root, planPath), "utf8");
    const planFindings = validatePhase006PlanText(text);
    if (planFindings.length > 0) {
      state = transitionPhase006PlanState(
        state.state,
        Phase006PlanEvent.Fail,
        planFindings[0],
      );
      return failedResult(state, planFindings);
    }
    state = transitionPhase006PlanState(state.state, Phase006PlanEvent.PlanChecked);
  } catch {
    state = transitionPhase006PlanState(state.state, Phase006PlanEvent.Fail, {
      errorCode: Phase006PlanErrorCode.IoFailed,
      findingId: planPath,
    });
    return failedResult(state, []);
  }

  try {
    const readmeText = await readFile(join(root, readmePath), "utf8");
    const readmeFindings = validatePhase006ReadmeText(readmeText);
    if (readmeFindings.length > 0) {
      state = transitionPhase006PlanState(
        state.state,
        Phase006PlanEvent.Fail,
        readmeFindings[0],
      );
      return failedResult(state, readmeFindings);
    }
    state = transitionPhase006PlanState(state.state, Phase006PlanEvent.ReadmeChecked);
  } catch {
    state = transitionPhase006PlanState(state.state, Phase006PlanEvent.Fail, {
      errorCode: Phase006PlanErrorCode.Phase006ReadmeMissing,
      findingId: readmePath,
    });
    return failedResult(state, []);
  }

  return {
    passed: true,
    state: state.state,
    archiveFileCount: requiredPhase005ArchiveFiles.length,
    requiredSectionCount: requiredSections.length,
    requiredTermCount: requiredTerms.length,
    requiredReadmeTermCount: requiredReadmeTerms.length,
    findings: [],
  };
}

export function renderPhase006PlanValidationResult(result) {
  if (result.passed) {
    return [
      "phase006_plan_validation=passed",
      `validation_state=${result.state}`,
      `archive_file_count=${result.archiveFileCount}`,
      `required_section_count=${result.requiredSectionCount}`,
      `required_term_count=${result.requiredTermCount}`,
      `required_readme_term_count=${result.requiredReadmeTermCount}`,
    ].join("\n");
  }

  const lines = [
    "phase006_plan_validation=failed",
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

export function renderPhase006PlanValidationArtifact(result) {
  const rendered = renderPhase006PlanValidationResult(result);
  const status = result.passed ? "passed" : "failed";
  return [
    "# Phase 006 Plan Validation Result",
    "",
    rendered,
    "",
    `- phase: \`Phase 006\``,
    "- gate: `Plan Validation`",
    `- status: \`${status}\``,
    `- state: \`${result.state}\``,
    "- commands:",
    "  - `npm run run:phase006-plan-validator-tests`",
    "  - `npm run run:phase006-plan-validator`",
    "  - `npm run run:security-log-scanner`",
    "- duration: recorded by command output when executed",
    "- sensitive data exclusion: this artifact records markers, counts, state, and stable error codes only. It does not record document body, asset content, AI prompt, AI answer, provider key, token, credential, or personal absolute path.",
    "- follow-up limitation: Phase 006.1 local desktop runtime gate remains incomplete.",
    "",
  ].join("\n");
}

async function validatePhase005Archive({ root }) {
  for (const relativePath of requiredPhase005ArchiveFiles) {
    try {
      await readFile(join(root, relativePath), "utf8");
    } catch {
      return [
        {
          errorCode: Phase006PlanErrorCode.Phase005ArchiveMissing,
          findingId: relativePath,
        },
      ];
    }
  }

  const finalReleasePath = ".tasks/phase005/phase005-release-gate-result.md";
  const finalReleaseText = await readFile(join(root, finalReleasePath), "utf8");
  if (
    !finalReleaseText.includes("phase005_release_gate=passed") ||
    !finalReleaseText.includes("AI and external integration platform complete")
  ) {
    return [
      {
        errorCode: Phase006PlanErrorCode.Phase005ReleaseMarkerMissing,
        findingId: finalReleasePath,
      },
    ];
  }

  const securityManifestFindings = await validatePhase005SecurityManifest({ root });
  if (securityManifestFindings.length > 0) {
    return securityManifestFindings;
  }

  return validateActiveRootTasks({ root });
}

async function validatePhase005SecurityManifest({ root }) {
  const manifestPath = ".tasks/release/security-log-policy-manifest.json";
  let manifest;
  try {
    manifest = JSON.parse(await readFile(join(root, manifestPath), "utf8"));
  } catch {
    return [
      {
        errorCode: Phase006PlanErrorCode.Phase005SecurityManifestMissing,
        findingId: manifestPath,
      },
    ];
  }

  const scanTargets = Array.isArray(manifest.scanTargets) ? manifest.scanTargets : [];
  for (const target of scanTargets) {
    if (!target || typeof target.id !== "string" || typeof target.path !== "string") {
      continue;
    }
    const isPhase005TaskArtifact =
      target.id.startsWith("phase005_") &&
      target.path.startsWith(".tasks/") &&
      !target.path.startsWith(".tasks/phase005/");
    if (isPhase005TaskArtifact) {
      return [
        {
          errorCode: Phase006PlanErrorCode.Phase005SecurityManifestPathInvalid,
          findingId: target.path,
        },
      ];
    }
  }
  return [];
}

async function validateActiveRootTasks({ root }) {
  let entries;
  try {
    entries = await readdir(join(root, ".tasks"));
  } catch {
    return [
      {
        errorCode: Phase006PlanErrorCode.IoFailed,
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
    if (/^# Task \d{3}\. Phase 005\b/m.test(text)) {
      return [
        {
          errorCode: Phase006PlanErrorCode.ActiveArchiveConflict,
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
  const result = await runPhase006PlanValidation();
  const artifact = renderPhase006PlanValidationArtifact(result);
  await writeFile(".tasks/phase006-plan-validation-result.md", artifact);
  const rendered = renderPhase006PlanValidationResult(result);
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
