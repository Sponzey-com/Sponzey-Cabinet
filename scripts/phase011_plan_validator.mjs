import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const Phase011PlanState = Object.freeze({
  Pending: "Pending",
  ReadingPrerequisites: "ReadingPrerequisites",
  ValidatingPlan: "ValidatingPlan",
  ValidatingReadme: "ValidatingReadme",
  WritingResult: "WritingResult",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase011PlanEvent = Object.freeze({
  Start: "Start",
  PrerequisitesRead: "PrerequisitesRead",
  PlanValidated: "PlanValidated",
  ReadmeValidated: "ReadmeValidated",
  ResultWritten: "ResultWritten",
  Fail: "Fail",
});

export const Phase011PlanErrorCode = Object.freeze({
  ArchivePrerequisiteMissing: "PHASE011_PLAN_ARCHIVE_PREREQUISITE_MISSING",
  InventoryPrerequisiteMissing: "PHASE011_PLAN_INVENTORY_PREREQUISITE_MISSING",
  RequirementMatrixMissing: "PHASE011_PLAN_REQUIREMENT_MATRIX_MISSING",
  PrerequisiteFingerprintMismatch: "PHASE011_PLAN_PREREQUISITE_FINGERPRINT_MISMATCH",
  PlanStructureInvalid: "PHASE011_PLAN_STRUCTURE_INVALID",
  RequirementRegisterInvalid: "PHASE011_PLAN_REQUIREMENT_REGISTER_INVALID",
  PhaseStructureInvalid: "PHASE011_PLAN_PHASE_STRUCTURE_INVALID",
  FutureScopeActivated: "PHASE011_PLAN_FUTURE_SCOPE_ACTIVATED",
  ReadmeScopeInvalid: "PHASE011_PLAN_README_SCOPE_INVALID",
  PlanFingerprintMismatch: "PHASE011_PLAN_FINGERPRINT_MISMATCH",
  UnsafeArtifact: "PHASE011_PLAN_UNSAFE_ARTIFACT",
  IoFailed: "PHASE011_PLAN_IO_FAILED",
  InvalidTransition: "PHASE011_PLAN_INVALID_TRANSITION",
});

const requirementIds = Object.freeze([
  "SCOPE-01", "BOOT-01", "HOME-01", "NAV-01", "DOC-01", "DOC-02", "DOC-03",
  "HIST-01", "HIST-02", "DISC-01", "DATA-01", "CFG-01", "CFG-02", "LOG-01",
  "STATE-01", "PERF-01", "SEC-01", "UX-01", "PLAT-01", "COMPAT-01",
]);

const topLevelSections = Object.freeze([
  "## 1. Project Goal",
  "## 2. Current Plan Assessment",
  "## 3. Architecture Direction",
  "## 4. Development Principles",
  "## 5. Required Evidence Manifest",
  "## 6. Phase 011 Task Execution Controls",
  "## 7. Phase 011 Active-Scope Validation Command Matrix",
  "## 8. Implementation Phases",
  "## 9. TDD Strategy",
  "## 10. Configuration and Runtime Environment Policy",
  "## 11. Logging Strategy",
  "## 12. State Machine Strategy",
  "## 13. Dependency and Boundary Rules",
  "## 14. Risk and Mitigation",
  "## 15. Review Checklist",
  "## 16. Definition of Done",
  "## 17. Prohibited Implementation Patterns",
  "## 18. Next Actions",
]);

const phaseSubsections = Object.freeze([
  "* Goal:",
  "* Scope:",
  "* Required Changes:",
  "* Architecture Notes:",
  "* TDD Requirements:",
  "* Configuration Rules:",
  "* Logging Rules:",
  "* State Management:",
  "* Validation:",
  "* Done Criteria:",
  "* Risks:",
]);

const requiredPlanTerms = Object.freeze([
  "# Phase 011 Development Plan",
  "Current product scope marker: `personal_local_desktop`",
  "Layered Architecture",
  "Clean Architecture",
  "Tidy First",
  "TDD",
  "bootstrap/composition root",
  "Product Log",
  "Field Debug Log",
  "Development Log",
  "Performance Measurement Contract",
  "p95 300ms",
  "Desktop UI, Accessibility, And Visual Validation Contract",
  "State Machine Strategy",
  "Phase 011 explicitly does not build:",
  "server hosting runtime",
  "SaaS runtime",
  "multi-user collaboration",
  "iOS/Android product implementation",
  "phase011_archive_validation=passed",
  "phase011_plan_validation=passed",
  "phase011_workspace_home_gate=passed",
  "phase011_document_authoring_gate=passed",
  "phase011_history_restore_gate=passed",
  "phase011_discovery_gate=passed",
  "phase011_data_settings_gate=passed",
  "phase011_recovery_observability_gate=passed",
  "phase011_product_smoke_gate=passed",
  "phase011_release_gate=passed",
  "phase011_performance_budget=passed",
  "phase011_product_log_matrix=passed",
  "phase011_security_log_manifest=passed",
  "phase011_runbook=passed",
  "phase011_requirement_evidence=passed",
  "phase011_visual_accessibility=passed",
  "phase011_native_platform_matrix=passed",
  "phase011_phase010_compatibility=passed",
  "task checkbox text",
]);

const requiredReadmeTerms = Object.freeze([
  "Active phase: Phase 011",
  "Current product scope: `personal_local_desktop`",
  "Phase 011 root tasks restart at `.tasks/task001.md`.",
  "Windows, macOS, and Linux",
  "server hosting",
  "SaaS",
  "multi-user",
  "admin console",
  "mobile implementation",
  "Archive phases: Phase 001 through Phase 010",
  "marker files and command results, not task checkbox text",
]);

const unsafeArtifactTerms = Object.freeze([
  "raw_document_body_fixture",
  "provider_api_key_fixture",
  "personal_absolute_path_fixture",
  "raw_prompt_fixture",
  "raw_answer_fixture",
  "/Users/example/private",
  "C:\\Users\\example\\private",
]);

export function transitionPhase011PlanState(currentState, event, detail = {}) {
  if (currentState === Phase011PlanState.Pending && event === Phase011PlanEvent.Start) {
    return { state: Phase011PlanState.ReadingPrerequisites };
  }
  if (
    currentState === Phase011PlanState.ReadingPrerequisites &&
    event === Phase011PlanEvent.PrerequisitesRead
  ) {
    return { state: Phase011PlanState.ValidatingPlan };
  }
  if (
    currentState === Phase011PlanState.ValidatingPlan &&
    event === Phase011PlanEvent.PlanValidated
  ) {
    return { state: Phase011PlanState.ValidatingReadme };
  }
  if (
    currentState === Phase011PlanState.ValidatingReadme &&
    event === Phase011PlanEvent.ReadmeValidated
  ) {
    return { state: Phase011PlanState.WritingResult };
  }
  if (
    currentState === Phase011PlanState.WritingResult &&
    event === Phase011PlanEvent.ResultWritten
  ) {
    return { state: Phase011PlanState.Passed };
  }
  if (
    [
      Phase011PlanState.ReadingPrerequisites,
      Phase011PlanState.ValidatingPlan,
      Phase011PlanState.ValidatingReadme,
      Phase011PlanState.WritingResult,
    ].includes(currentState) &&
    event === Phase011PlanEvent.Fail
  ) {
    return {
      state: Phase011PlanState.Failed,
      errorCode: detail.errorCode ?? Phase011PlanErrorCode.IoFailed,
      findingId: detail.findingId,
    };
  }
  return {
    state: Phase011PlanState.Failed,
    errorCode: Phase011PlanErrorCode.InvalidTransition,
  };
}

export function validatePhase011PlanArtifactFreshness(artifactText, expectedFingerprint) {
  if (!artifactText.includes(`plan_fingerprint=${expectedFingerprint}`)) {
    return [
      {
        errorCode: Phase011PlanErrorCode.PlanFingerprintMismatch,
        findingId: "plan_fingerprint",
      },
    ];
  }
  return [];
}

export async function runPhase011PlanValidation({
  root = process.cwd(),
  writeArtifact = true,
  expectedPlanFingerprint,
} = {}) {
  let state = transitionPhase011PlanState(Phase011PlanState.Pending, Phase011PlanEvent.Start).state;
  try {
    const archive = await readText(root, ".tasks/phase011-archive-validation-result.md");
    if (!archive?.includes("phase011_archive_validation=passed")) {
      return failed(state, Phase011PlanErrorCode.ArchivePrerequisiteMissing, ".tasks/phase011-archive-validation-result.md");
    }
    const inventory = await readText(root, ".tasks/phase011-current-implementation-inventory.md");
    if (!inventory?.includes("phase011_current_inventory=passed")) {
      return failed(state, Phase011PlanErrorCode.InventoryPrerequisiteMissing, ".tasks/phase011-current-implementation-inventory.md");
    }
    const matrix = await readText(root, ".tasks/release/requirement-evidence-matrix-phase011.md");
    if (
      !matrix?.includes("phase011_requirement_evidence=pending") ||
      !matrix.includes("requirement_count=20")
    ) {
      return failed(state, Phase011PlanErrorCode.RequirementMatrixMissing, ".tasks/release/requirement-evidence-matrix-phase011.md");
    }
    const prerequisiteFingerprints = [archive, inventory, matrix].map(readSourceFingerprint);
    if (
      prerequisiteFingerprints.some((fingerprint) => !fingerprint) ||
      new Set(prerequisiteFingerprints).size !== 1
    ) {
      return failed(state, Phase011PlanErrorCode.PrerequisiteFingerprintMismatch, "source_fingerprint");
    }
    const sourceFingerprint = prerequisiteFingerprints[0];
    state = transitionPhase011PlanState(state, Phase011PlanEvent.PrerequisitesRead).state;

    const planText = await readText(root, ".tasks/plan.md");
    if (!planText) return failed(state, Phase011PlanErrorCode.PlanStructureInvalid, ".tasks/plan.md");
    for (const section of topLevelSections) {
      if (!planText.includes(section)) {
        return failed(state, Phase011PlanErrorCode.PlanStructureInvalid, section);
      }
    }
    for (const term of requiredPlanTerms) {
      if (!planText.includes(term)) {
        return failed(state, Phase011PlanErrorCode.PlanStructureInvalid, term);
      }
    }
    const sectionOffsets = topLevelSections.map((section) => planText.indexOf(section));
    if (sectionOffsets.some((offset, index) => index > 0 && offset <= sectionOffsets[index - 1])) {
      return failed(state, Phase011PlanErrorCode.PlanStructureInvalid, "top_level_section_order");
    }

    const parsedRequirementIds = [...planText.matchAll(/^\| `([A-Z]+-\d+)` \|/gm)].map(
      (match) => match[1],
    );
    const duplicateRequirement = parsedRequirementIds.find(
      (id, index) => parsedRequirementIds.indexOf(id) !== index,
    );
    const missingRequirement = requirementIds.find((id) => !parsedRequirementIds.includes(id));
    const unknownRequirement = parsedRequirementIds.find((id) => !requirementIds.includes(id));
    if (
      duplicateRequirement ||
      missingRequirement ||
      unknownRequirement ||
      parsedRequirementIds.length !== requirementIds.length
    ) {
      return failed(
        state,
        Phase011PlanErrorCode.RequirementRegisterInvalid,
        duplicateRequirement ?? missingRequirement ?? unknownRequirement ?? "requirement_count",
      );
    }

    const activePlatformBlock = extractBlock(planText, "Current active platforms:", "Current active stack:");
    if (
      !activePlatformBlock ||
      !activePlatformBlock.includes("Windows") ||
      !activePlatformBlock.includes("macOS") ||
      !activePlatformBlock.includes("Linux") ||
      /\b(Web|iOS|Android|server|SaaS|multi-user|mobile)\b/i.test(activePlatformBlock)
    ) {
      return failed(state, Phase011PlanErrorCode.FutureScopeActivated, "Current active platforms");
    }

    const phaseMatches = [...planText.matchAll(/^### Phase 011\.(\d+)\.[^\n]*$/gm)];
    if (phaseMatches.length !== 9) {
      return failed(state, Phase011PlanErrorCode.PhaseStructureInvalid, "phase_count");
    }
    for (let index = 0; index < phaseMatches.length; index += 1) {
      if (Number(phaseMatches[index][1]) !== index) {
        return failed(state, Phase011PlanErrorCode.PhaseStructureInvalid, `Phase 011.${index}`);
      }
      const start = phaseMatches[index].index;
      const end = phaseMatches[index + 1]?.index ?? planText.indexOf("\n## 9. TDD Strategy", start);
      const phaseText = planText.slice(start, end < 0 ? undefined : end);
      for (const subsection of phaseSubsections) {
        if (!phaseText.includes(subsection)) {
          return failed(
            state,
            Phase011PlanErrorCode.PhaseStructureInvalid,
            `Phase 011.${index}:${subsection}`,
          );
        }
      }
    }
    state = transitionPhase011PlanState(state, Phase011PlanEvent.PlanValidated).state;

    const readmeText = await readText(root, ".tasks/readme.md");
    if (!readmeText) return failed(state, Phase011PlanErrorCode.ReadmeScopeInvalid, ".tasks/readme.md");
    for (const term of requiredReadmeTerms) {
      if (!readmeText.includes(term)) {
        return failed(state, Phase011PlanErrorCode.ReadmeScopeInvalid, term);
      }
    }
    state = transitionPhase011PlanState(state, Phase011PlanEvent.ReadmeValidated).state;

    const planFingerprint = hashTexts([
      [".tasks/plan.md", planText],
      [".tasks/readme.md", readmeText],
    ]);
    if (expectedPlanFingerprint && expectedPlanFingerprint !== planFingerprint) {
      return failed(state, Phase011PlanErrorCode.PlanFingerprintMismatch, "plan_fingerprint");
    }

    const result = {
      passed: true,
      state: Phase011PlanState.Passed,
      productScope: "personal_local_desktop",
      topLevelSectionCount: topLevelSections.length,
      requirementCount: parsedRequirementIds.length,
      phaseCount: phaseMatches.length,
      sourceFingerprint,
      planFingerprint,
    };
    const artifact = renderPhase011PlanValidationArtifact(result);
    const unsafeTerm = unsafeArtifactTerms.find((term) => artifact.includes(term));
    if (unsafeTerm) return failed(state, Phase011PlanErrorCode.UnsafeArtifact, unsafeTerm);

    if (writeArtifact) {
      await mkdir(join(root, ".tasks"), { recursive: true });
      await writeFile(join(root, ".tasks/phase011-plan-validation-result.md"), artifact);
    }
    state = transitionPhase011PlanState(state, Phase011PlanEvent.ResultWritten).state;
    return { ...result, state };
  } catch {
    return failed(state, Phase011PlanErrorCode.IoFailed, "validator_io");
  }
}

export function renderPhase011PlanValidationArtifact(result) {
  const lines = [
    "# Phase 011 Plan Validation Result",
    "",
    `phase011_plan_validation=${result.passed ? "passed" : "failed"}`,
    `validation_state=${result.state}`,
    `release_scope=${result.productScope ?? "personal_local_desktop"}`,
    `source_fingerprint=${result.sourceFingerprint ?? "unavailable"}`,
    `plan_fingerprint=${result.planFingerprint ?? "unavailable"}`,
    "",
    "- phase: `Phase 011.0`",
    "- prerequisite marker: `phase011_archive_validation=passed`.",
    `- top-level section count: ${result.topLevelSectionCount ?? 0}`,
    `- requirement count: ${result.requirementCount ?? 0}`,
    `- implementation phase count: ${result.phaseCount ?? 0}`,
    "- validation commands: `npm run run:phase011-plan-validator-tests`, `npm run run:phase011-plan-validator`.",
    "- changed layers: `task-tooling`, `release-tooling`.",
    "- configuration: explicit repository root only; no environment lookup or mutation.",
    "- logging: Development diagnostics only; no Product Log or Field Debug Log.",
    "- evidence rule: marker files and command results are used; task checkbox text is not completion evidence.",
    "- sensitive data exclusion: counts, marker names, scope, fingerprints, relative evidence ids, and stable error codes only.",
  ];
  if (!result.passed) {
    lines.push(`- error_code: \`${result.errorCode}\``);
    lines.push(`- finding_id: \`${result.findingId ?? "unknown"}\``);
  }
  lines.push("");
  return lines.join("\n");
}

function failed(state, errorCode, findingId) {
  const transition = transitionPhase011PlanState(state, Phase011PlanEvent.Fail, {
    errorCode,
    findingId,
  });
  return {
    passed: false,
    state: Phase011PlanState.Failed,
    errorCode: transition.errorCode,
    findingId: transition.findingId,
    productScope: "personal_local_desktop",
    topLevelSectionCount: 0,
    requirementCount: 0,
    phaseCount: 0,
  };
}

async function readText(root, path) {
  try {
    return await readFile(join(root, path), "utf8");
  } catch {
    return undefined;
  }
}

function readSourceFingerprint(text) {
  return text.match(/^source_fingerprint=([a-f0-9]{64})$/m)?.[1];
}

function extractBlock(text, startMarker, endMarker) {
  const start = text.indexOf(startMarker);
  const end = text.indexOf(endMarker, start + startMarker.length);
  if (start < 0 || end < 0) return undefined;
  return text.slice(start + startMarker.length, end);
}

function hashTexts(entries) {
  const hash = createHash("sha256");
  for (const [path, text] of entries) {
    hash.update(path);
    hash.update("\0");
    hash.update(text);
    hash.update("\0");
  }
  return hash.digest("hex");
}

async function main() {
  const result = await runPhase011PlanValidation({ root: process.cwd(), writeArtifact: true });
  if (!result.passed) {
    console.error(`${result.errorCode} ${result.findingId ?? ""}`.trim());
    process.exit(1);
  }
  console.log("phase011_plan_validation=passed");
  console.log(`plan_fingerprint=${result.planFingerprint}`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    console.error(error);
    process.exit(1);
  });
}
