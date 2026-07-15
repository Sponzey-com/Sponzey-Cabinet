import { readFile, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

export const Phase012PlanState = Object.freeze({
  NotStarted: "NotStarted",
  PrerequisiteValidated: "PrerequisiteValidated",
  PlanValidated: "PlanValidated",
  EvidenceValidated: "EvidenceValidated",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase012PlanEvent = Object.freeze({
  PrerequisiteAccepted: "PrerequisiteAccepted",
  PlanAccepted: "PlanAccepted",
  EvidenceAccepted: "EvidenceAccepted",
  Complete: "Complete",
  Fail: "Fail",
});

export const Phase012PlanErrorCode = Object.freeze({
  PrerequisiteInvalid: "PHASE012_PLAN_PREREQUISITE_INVALID",
  SectionMissing: "PHASE012_PLAN_SECTION_MISSING",
  PhaseSequenceInvalid: "PHASE012_PLAN_PHASE_SEQUENCE_INVALID",
  PhaseFieldMissing: "PHASE012_PLAN_PHASE_FIELD_MISSING",
  RequirementInvalid: "PHASE012_PLAN_REQUIREMENT_INVALID",
  PolicyMissing: "PHASE012_PLAN_POLICY_MISSING",
  VagueLanguage: "PHASE012_PLAN_VAGUE_LANGUAGE",
  EvidenceMismatch: "PHASE012_PLAN_EVIDENCE_MISMATCH",
  InvalidTransition: "PHASE012_PLAN_INVALID_TRANSITION",
  IoFailed: "PHASE012_PLAN_IO_FAILED",
});

const phaseFields = Object.freeze([
  "Goal", "Scope", "Required Changes", "Architecture Notes", "TDD Requirements",
  "Configuration Rules", "Logging Rules", "State Management", "Validation", "Done Criteria", "Risks",
]);
const requiredPolicies = Object.freeze([
  "Layered Architecture", "Clean Architecture", "Tidy First", "TDD Strategy",
  "Configuration And Runtime Environment Policy", "Product Log", "Field Debug Log",
  "Development Log", "State Machine Strategy", "p95 300ms", "UI Feature Connection Contract",
]);
const vaguePhrases = Object.freeze([
  "적절히 처리한다", "필요에 따라 설정한다", "나중에 개선한다", "테스트를 추가한다",
  "로그를 남긴다", "상태를 관리한다", "환경 값을 사용한다", "공통화한다", "리팩터링한다",
]);

export function transitionPhase012PlanState(state, event, failure = {}) {
  const next = new Map([
    [`${Phase012PlanState.NotStarted}:${Phase012PlanEvent.PrerequisiteAccepted}`, Phase012PlanState.PrerequisiteValidated],
    [`${Phase012PlanState.PrerequisiteValidated}:${Phase012PlanEvent.PlanAccepted}`, Phase012PlanState.PlanValidated],
    [`${Phase012PlanState.PlanValidated}:${Phase012PlanEvent.EvidenceAccepted}`, Phase012PlanState.EvidenceValidated],
    [`${Phase012PlanState.EvidenceValidated}:${Phase012PlanEvent.Complete}`, Phase012PlanState.Passed],
  ]).get(`${state}:${event}`);
  if (event === Phase012PlanEvent.Fail) return { state: Phase012PlanState.Failed, ...failure };
  return next ? { state: next } : { state: Phase012PlanState.Failed, errorCode: Phase012PlanErrorCode.InvalidTransition };
}

export function validatePhase012PlanText(plan) {
  const findings = [];
  for (let section = 1; section <= 18; section += 1) {
    if (!new RegExp(`^## ${section}\\.`, "m").test(plan)) {
      findings.push(finding(Phase012PlanErrorCode.SectionMissing, `section-${section}`));
    }
  }
  const phases = [...plan.matchAll(/^## Phase 012\.(\d+)\./gm)].map((match) => Number(match[1]));
  if (phases.length !== 9 || phases.some((phase, index) => phase !== index)) {
    findings.push(finding(Phase012PlanErrorCode.PhaseSequenceInvalid, "phase-sequence"));
  }
  for (const field of phaseFields) {
    const count = [...plan.matchAll(new RegExp(`^\\* ${escapeRegex(field)}:`, "gm"))].length;
    if (count !== 9) findings.push(finding(Phase012PlanErrorCode.PhaseFieldMissing, field));
  }
  const requirementIds = [...plan.matchAll(/^\| `([A-Z-]+012-[0-9]+)`/gm)].map((match) => match[1]);
  const duplicates = requirementIds.filter((id, index) => requirementIds.indexOf(id) !== index);
  if (requirementIds.length === 0 || duplicates.length > 0) {
    findings.push(finding(Phase012PlanErrorCode.RequirementInvalid, duplicates[0] ?? "requirement-register"));
  }
  for (const policy of requiredPolicies) {
    if (!plan.includes(policy)) findings.push(finding(Phase012PlanErrorCode.PolicyMissing, policy));
  }
  for (const phrase of vaguePhrases) {
    if (plan.includes(phrase)) findings.push(finding(Phase012PlanErrorCode.VagueLanguage, phrase));
  }
  return { findings, phaseCount: phases.length, requirementIds };
}

export function validatePhase012EvidenceFingerprints({ archiveFingerprint, inventory, matrix, release }) {
  if (value(inventory, "source_fingerprint") !== archiveFingerprint) return false;
  const matrixStatus = value(matrix, "phase012_requirement_evidence");
  const matrixFingerprint = value(matrix, "source_fingerprint");
  if (matrixStatus === "pending") return matrixFingerprint === archiveFingerprint;
  return matrixStatus === "passed" &&
    release.includes("phase012_release_gate=passed") &&
    matrixFingerprint === value(release, "source_fingerprint");
}

export async function runPhase012PlanValidation({ root, writeArtifact = true }) {
  try {
    const base = resolve(root);
    const archive = await readFile(join(base, ".tasks/phase012-archive-validation-result.md"), "utf8");
    if (!archive.includes("phase012_archive_validation=passed")) {
      return failed(Phase012PlanErrorCode.PrerequisiteInvalid, "phase012_archive_validation");
    }
    const sourceFingerprint = value(archive, "source_fingerprint");
    let state = transitionPhase012PlanState(Phase012PlanState.NotStarted, Phase012PlanEvent.PrerequisiteAccepted);
    const plan = await readFile(join(base, ".tasks/plan.md"), "utf8");
    const validation = validatePhase012PlanText(plan);
    if (validation.findings.length > 0) return failed(validation.findings[0].errorCode, validation.findings[0].findingId);
    state = transitionPhase012PlanState(state.state, Phase012PlanEvent.PlanAccepted);
    const inventory = await readFile(join(base, ".tasks/phase012-current-implementation-inventory.md"), "utf8");
    const matrix = await readFile(join(base, ".tasks/release/requirement-evidence-matrix-phase012.md"), "utf8");
    const release = await readFile(join(base, ".tasks/phase012-release-gate-result.md"), "utf8");
    const matrixIds = [...matrix.matchAll(/^\| `([A-Z-]+012-[0-9]+)`/gm)].map((match) => match[1]);
    if (
      !validatePhase012EvidenceFingerprints({ archiveFingerprint: sourceFingerprint, inventory, matrix, release }) ||
      validation.requirementIds.join("\0") !== matrixIds.join("\0")
    ) return failed(Phase012PlanErrorCode.EvidenceMismatch, "requirement-evidence-matrix");
    state = transitionPhase012PlanState(state.state, Phase012PlanEvent.EvidenceAccepted);
    state = transitionPhase012PlanState(state.state, Phase012PlanEvent.Complete);
    const result = { passed: true, state: state.state, sourceFingerprint, requirementCount: validation.requirementIds.length };
    if (writeArtifact) {
      await writeFile(join(base, ".tasks/phase012-plan-validation-result.md"), [
        "phase012_plan_validation=passed", `validation_state=${result.state}`,
        `source_fingerprint=${sourceFingerprint}`, `requirement_count=${result.requirementCount}`, "",
      ].join("\n"));
    }
    return result;
  } catch {
    return failed(Phase012PlanErrorCode.IoFailed, "plan-validator-io");
  }
}

function finding(errorCode, findingId) { return { errorCode, findingId }; }
function failed(errorCode, findingId) { return { passed: false, state: Phase012PlanState.Failed, errorCode, findingId }; }
function value(text, key) { return text.match(new RegExp(`^${key}=([^\\n]+)$`, "m"))?.[1]; }
function escapeRegex(text) { return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"); }

async function main() {
  const result = await runPhase012PlanValidation({ root: process.argv[2] ?? process.cwd(), writeArtifact: true });
  if (!result.passed) {
    process.stderr.write(`${result.errorCode} finding=${result.findingId}\n`);
    process.exitCode = 1;
    return;
  }
  process.stdout.write(`phase012_plan_validation=passed requirements=${result.requirementCount}\n`);
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) await main();
