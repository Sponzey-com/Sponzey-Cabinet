import { readdir, readFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const Phase005PlanState = Object.freeze({
  Pending: "Pending",
  CheckingArchive: "CheckingArchive",
  CheckingPlan: "CheckingPlan",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase005PlanEvent = Object.freeze({
  Start: "Start",
  ArchiveChecked: "ArchiveChecked",
  PlanChecked: "PlanChecked",
  Fail: "Fail",
});

export const Phase005PlanErrorCode = Object.freeze({
  ActiveArchiveConflict: "PHASE005_ACTIVE_ARCHIVE_CONFLICT",
  Phase004ArchiveMissing: "PHASE004_ARCHIVE_MISSING",
  Phase004ReleaseMarkerMissing: "PHASE004_RELEASE_MARKER_MISSING",
  Phase004SecurityManifestMissing: "PHASE004_SECURITY_MANIFEST_MISSING",
  Phase004SecurityManifestPathInvalid: "PHASE004_SECURITY_MANIFEST_PATH_INVALID",
  Phase005ActivePhaseMismatch: "PHASE005_ACTIVE_PHASE_MISMATCH",
  Phase005PlanSectionMissing: "PHASE005_PLAN_SECTION_MISSING",
  Phase005PlanTermMissing: "PHASE005_PLAN_TERM_MISSING",
  IoFailed: "PHASE005_PLAN_IO_FAILED",
  InvalidTransition: "PHASE005_PLAN_INVALID_TRANSITION",
});

const requiredPhase004ArchiveFiles = [
  ".tasks/phase004/README.md",
  ".tasks/phase004/plan.md",
  ".tasks/phase004/phase-gates.md",
  ".tasks/phase004/graph-coverage-audit.md",
  ".tasks/phase004/graph-product-gate-result.md",
  ".tasks/phase004/realtime-collaboration-smoke-result.md",
  ".tasks/phase004/collaboration-coverage-audit.md",
  ".tasks/phase004/realtime-collaboration-product-gate-result.md",
  ".tasks/phase004/canvas-coverage-audit.md",
  ".tasks/phase004/canvas-product-smoke-result.md",
  ".tasks/phase004/mobile-capability-audit.md",
  ".tasks/phase004/phase004-product-smoke-gate-result.md",
  ".tasks/phase004/phase004-final-release-gate-result.md",
  ...Array.from(
    { length: 37 },
    (_, index) => `.tasks/phase004/task${String(index + 1).padStart(3, "0")}.md`,
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
  "## 8. Tidy First Policy",
  "## 9. Configuration and Runtime Environment Policy",
  "## 10. Logging Strategy",
  "## 11. State Machine Strategy",
  "## 12. Performance Strategy",
  "## 13. Dependency and Boundary Rules",
  "## 14. Release Gates",
  "## 15. Risk and Mitigation",
  "## 16. Review Checklist",
  "## 16.1. Definition of Ready",
  "## 17. Definition of Done",
  "## 18. Prohibited Implementation Patterns",
  "## 19. Next Actions",
];

const requiredTerms = [
  "현재 단계: Phase 005",
  "AI and External Integration Platform",
  "AI",
  "semantic search",
  "permission-aware retrieval",
  "MCP",
  "webhook",
  "event stream",
  "connector",
  "citation",
  "provider boundary",
  "retrieval contract complete",
  "privacy safe",
  "async by default",
  "query p95 measured",
  "Layered Architecture",
  "Clean Architecture",
  "Tidy First",
  "TDD",
  "bootstrap/composition root",
  "Product Log",
  "Field Debug Log",
  "Development Log",
  "state machine",
  "p95 300ms",
  "phase005_plan_validation",
  "phase005_release_gate",
  "Web, iOS, Android, Windows, macOS, Linux",
];

export function transitionPhase005PlanState(currentState, event, detail = {}) {
  if (currentState === Phase005PlanState.Pending && event === Phase005PlanEvent.Start) {
    return { state: Phase005PlanState.CheckingArchive };
  }
  if (
    currentState === Phase005PlanState.CheckingArchive &&
    event === Phase005PlanEvent.ArchiveChecked
  ) {
    return { state: Phase005PlanState.CheckingPlan };
  }
  if (
    currentState === Phase005PlanState.CheckingPlan &&
    event === Phase005PlanEvent.PlanChecked
  ) {
    return { state: Phase005PlanState.Passed };
  }
  if (
    [Phase005PlanState.CheckingArchive, Phase005PlanState.CheckingPlan].includes(
      currentState,
    ) &&
    event === Phase005PlanEvent.Fail
  ) {
    return {
      state: Phase005PlanState.Failed,
      errorCode: detail.errorCode ?? Phase005PlanErrorCode.IoFailed,
      findingId: detail.findingId,
    };
  }
  return {
    state: Phase005PlanState.Failed,
    errorCode: Phase005PlanErrorCode.InvalidTransition,
  };
}

export function validatePhase005PlanText(text) {
  if (!text.includes("현재 단계: Phase 005")) {
    return [
      {
        errorCode: Phase005PlanErrorCode.Phase005ActivePhaseMismatch,
        findingId: "current_phase",
      },
    ];
  }
  if (text.includes("현재 단계: Phase 004 -")) {
    return [
      {
        errorCode: Phase005PlanErrorCode.Phase005ActivePhaseMismatch,
        findingId: "phase004_active_state",
      },
    ];
  }
  for (const section of requiredSections) {
    if (!text.includes(section)) {
      return [
        {
          errorCode: Phase005PlanErrorCode.Phase005PlanSectionMissing,
          findingId: section,
        },
      ];
    }
  }
  for (const term of requiredTerms) {
    if (!text.includes(term)) {
      return [
        {
          errorCode: Phase005PlanErrorCode.Phase005PlanTermMissing,
          findingId: term,
        },
      ];
    }
  }
  return [];
}

export async function runPhase005PlanValidation({
  root = process.cwd(),
  planPath = ".tasks/plan.md",
} = {}) {
  let state = transitionPhase005PlanState(
    Phase005PlanState.Pending,
    Phase005PlanEvent.Start,
  );

  const archiveFindings = await validatePhase004Archive({ root });
  if (archiveFindings.length > 0) {
    state = transitionPhase005PlanState(
      state.state,
      Phase005PlanEvent.Fail,
      archiveFindings[0],
    );
    return failedResult(state, archiveFindings);
  }
  state = transitionPhase005PlanState(state.state, Phase005PlanEvent.ArchiveChecked);

  try {
    const text = await readFile(join(root, planPath), "utf8");
    const planFindings = validatePhase005PlanText(text);
    if (planFindings.length > 0) {
      state = transitionPhase005PlanState(
        state.state,
        Phase005PlanEvent.Fail,
        planFindings[0],
      );
      return failedResult(state, planFindings);
    }
    state = transitionPhase005PlanState(state.state, Phase005PlanEvent.PlanChecked);
    return {
      passed: true,
      state: state.state,
      archiveFileCount: requiredPhase004ArchiveFiles.length,
      requiredSectionCount: requiredSections.length,
      requiredTermCount: requiredTerms.length,
      findings: [],
    };
  } catch {
    state = transitionPhase005PlanState(state.state, Phase005PlanEvent.Fail, {
      errorCode: Phase005PlanErrorCode.IoFailed,
      findingId: planPath,
    });
    return failedResult(state, []);
  }
}

export function renderPhase005PlanValidationResult(result) {
  if (result.passed) {
    return [
      "phase005_plan_validation=passed",
      `validation_state=${result.state}`,
      `archive_file_count=${result.archiveFileCount}`,
      `required_section_count=${result.requiredSectionCount}`,
      `required_term_count=${result.requiredTermCount}`,
    ].join("\n");
  }

  const lines = [
    "phase005_plan_validation=failed",
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

async function validatePhase004Archive({ root }) {
  const findings = [];
  for (const relativePath of requiredPhase004ArchiveFiles) {
    try {
      await readFile(join(root, relativePath), "utf8");
    } catch {
      findings.push({
        errorCode: Phase005PlanErrorCode.Phase004ArchiveMissing,
        findingId: relativePath,
      });
      return findings;
    }
  }

  const finalReleasePath = ".tasks/phase004/phase004-final-release-gate-result.md";
  const finalReleaseText = await readFile(join(root, finalReleasePath), "utf8");
  if (
    !finalReleaseText.includes("phase004_release_gate=passed") ||
    !finalReleaseText.includes(
      "knowledge graph and realtime collaboration UX expansion complete",
    )
  ) {
    return [
      {
        errorCode: Phase005PlanErrorCode.Phase004ReleaseMarkerMissing,
        findingId: finalReleasePath,
      },
    ];
  }

  const securityManifestFindings = await validatePhase004SecurityManifest({ root });
  if (securityManifestFindings.length > 0) {
    return securityManifestFindings;
  }

  return validateActiveRootTasks({ root });
}

async function validatePhase004SecurityManifest({ root }) {
  const manifestPath = ".tasks/release/security-log-policy-manifest.json";
  let manifest;
  try {
    manifest = JSON.parse(await readFile(join(root, manifestPath), "utf8"));
  } catch {
    return [
      {
        errorCode: Phase005PlanErrorCode.Phase004SecurityManifestMissing,
        findingId: manifestPath,
      },
    ];
  }

  const scanTargets = Array.isArray(manifest.scanTargets) ? manifest.scanTargets : [];
  for (const target of scanTargets) {
    if (!target || typeof target.id !== "string" || typeof target.path !== "string") {
      continue;
    }
    const isPhase004TaskArtifact =
      target.id.startsWith("phase004_") &&
      target.path.startsWith(".tasks/") &&
      !target.path.startsWith(".tasks/phase004/");
    const isArchivedPhaseGateAlias =
      target.id === "phase003_gate_rules" &&
      target.path.startsWith(".tasks/") &&
      !target.path.startsWith(".tasks/phase004/");
    if (isPhase004TaskArtifact || isArchivedPhaseGateAlias) {
      return [
        {
          errorCode: Phase005PlanErrorCode.Phase004SecurityManifestPathInvalid,
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
        errorCode: Phase005PlanErrorCode.IoFailed,
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
    if (/^# Task \d{3}\. Phase 004\b/m.test(text)) {
      return [
        {
          errorCode: Phase005PlanErrorCode.ActiveArchiveConflict,
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
  const result = await runPhase005PlanValidation();
  const rendered = renderPhase005PlanValidationResult(result);
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
