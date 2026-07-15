import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const HardeningCoverageAuditState = Object.freeze({
  Pending: "Pending",
  ReadingSources: "ReadingSources",
  Classifying: "Classifying",
  Reported: "Reported",
  Failed: "Failed",
});

export const HardeningCoverageAuditEvent = Object.freeze({
  Start: "Start",
  SourcesRead: "SourcesRead",
  Classified: "Classified",
  ReportWritten: "ReportWritten",
  Fail: "Fail",
});

export const HardeningCoverageAuditErrorCode = Object.freeze({
  InvalidTransition: "PHASE003_HARDENING_INVALID_TRANSITION",
  SourceSetEmpty: "PHASE003_HARDENING_SOURCE_SET_EMPTY",
  ReportWriteFailed: "PHASE003_HARDENING_REPORT_WRITE_FAILED",
});

const STATUS = Object.freeze({
  Covered: "covered",
  Partial: "partial",
  Missing: "missing",
});

const TARGETS = Object.freeze([
  target("security_log_scanner_active_manifest", "Active security log scanner manifest", {
    requiredFiles: [
      ".tasks/release/security-log-policy-manifest.json",
      "scripts/run_security_log_scanner.sh",
      "scripts/security_log_scanner.mjs",
      "scripts/security_log_scanner_tests.mjs",
    ],
    requiredEvidence: [
      "Product Log",
      "Field Debug Log",
      "Development Log",
      "deniedFixtures",
      "scanTargets",
      "security_log_scan=passed",
      "SensitiveFixtureFound",
    ],
    supportFiles: ["scripts/security_log_scanner.mjs", "scripts/security_log_scanner_tests.mjs"],
    supportEvidence: ["security_log_scan=passed", "deniedFixtures", "scanTargets"],
    priority: 100,
  }),
  target("runbook_validator_active_manifest", "Active runbook validator manifest and runbooks", {
    requiredFiles: [
      ".tasks/release/runbook-validation-manifest.json",
      ".tasks/release/runbooks/backup-restore.md",
      ".tasks/release/runbooks/field-debug-approval.md",
      ".tasks/release/runbooks/upgrade-migration.md",
      "scripts/run_runbook_validator.sh",
      "scripts/runbook_validator.mjs",
      "scripts/runbook_validator_tests.mjs",
    ],
    requiredEvidence: [
      "requiredSections",
      "requiredPhrases",
      "forbiddenText",
      "runbook_validation=passed",
      "Backup Restore",
      "Field Debug",
      "Upgrade Migration",
    ],
    supportFiles: ["scripts/runbook_validator.mjs", "scripts/runbook_validator_tests.mjs"],
    supportEvidence: ["runbook_validation=passed", "requiredSections", "forbiddenText"],
    priority: 98,
  }),
  target("product_log_event_matrix", "Product, Field Debug, and Development log event matrix", {
    requiredFiles: [".tasks/release/product-log-event-matrix.md"],
    requiredEvidence: [
      "Product Log",
      "Field Debug Log",
      "Development Log",
      "stable error code",
      "event name",
      "sensitive data denied",
    ],
    supportFiles: ["AGENTS.md"],
    supportEvidence: ["Product Log", "Field Debug Log", "Development Log"],
    priority: 96,
  }),
  target("field_debug_scope_ttl_revoke", "Field Debug scoped approval TTL and revoke validation", {
    requiredFiles: [
      "crates/cabinet-domain/src/field_debug.rs",
      "crates/cabinet-domain/tests/field_debug_tests.rs",
      "crates/cabinet-usecases/src/field_debug.rs",
      "crates/cabinet-usecases/tests/field_debug_usecase_tests.rs",
      ".tasks/release/runbooks/field-debug-approval.md",
    ],
    requiredEvidence: [
      "FieldDebugSession",
      "Approved",
      "Expired",
      "revoke",
      "scope",
      "ttl",
      "admin approval",
    ],
    supportFiles: [
      "crates/cabinet-domain/src/field_debug.rs",
      "crates/cabinet-domain/tests/field_debug_tests.rs",
      "crates/cabinet-usecases/src/field_debug.rs",
      "crates/cabinet-usecases/tests/field_debug_usecase_tests.rs",
    ],
    supportEvidence: ["FieldDebugSession", "Approved", "Expired"],
    priority: 94,
  }),
  target("development_log_exclusion", "Development Log exclusion from production default", {
    requiredFiles: [
      "crates/cabinet-core/src/server_config.rs",
      "crates/cabinet-core/tests/server_config_tests.rs",
      ".tasks/release/security-log-policy-manifest.json",
    ],
    requiredEvidence: [
      "DevelopmentLogMode",
      "development_log_mode",
      "disabled",
      "Development Log",
    ],
    supportFiles: [
      "crates/cabinet-core/src/server_config.rs",
      "crates/cabinet-core/tests/server_config_tests.rs",
    ],
    supportEvidence: ["DevelopmentLogMode", "development_log_mode"],
    priority: 92,
  }),
  target("recovery_runbook_smoke", "Recovery runbook smoke evidence", {
    requiredFiles: [
      ".tasks/release/runbooks/backup-restore.md",
      "scripts/phase003_recovery_coverage_audit.mjs",
      ".tasks/phase003/recovery-coverage-audit.md",
    ],
    requiredEvidence: [
      "Backup Restore",
      "restore staging",
      "startup repair",
      "corrupted indexes",
      "targets needing work | 0",
    ],
    supportFiles: [
      "scripts/phase003_recovery_coverage_audit.mjs",
      ".tasks/phase003/recovery-coverage-audit.md",
    ],
    supportEvidence: ["startup repair", "backup", "restore"],
    priority: 90,
  }),
  target("final_release_gate_artifact", "Final Phase 003 release gate artifact", {
    requiredFiles: [
      "scripts/phase003_release_gate.mjs",
      "scripts/phase003_release_gate_tests.mjs",
      ".tasks/phase003/final-release-gate-result.md",
    ],
    requiredEvidence: [
      "phase003_release_gate=passed",
      "production hardening complete",
      "security_log_scan",
      "runbook_validation",
      "packaging_gate",
      "product_smoke_gate",
    ],
    supportFiles: [
      "scripts/phase003_gate.mjs",
      "scripts/phase003_product_smoke_gate.mjs",
      "scripts/phase003_packaging_gate.mjs",
    ],
    supportEvidence: ["phase003", "gate"],
    priority: 88,
  }),
]);

class HardeningCoverageAuditError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "HardeningCoverageAuditError";
    this.code = code;
  }
}

export function transitionHardeningCoverageAuditState(state, event) {
  const key = `${state}:${event}`;
  const transitions = new Map([
    [
      `${HardeningCoverageAuditState.Pending}:${HardeningCoverageAuditEvent.Start}`,
      HardeningCoverageAuditState.ReadingSources,
    ],
    [
      `${HardeningCoverageAuditState.ReadingSources}:${HardeningCoverageAuditEvent.SourcesRead}`,
      HardeningCoverageAuditState.Classifying,
    ],
    [
      `${HardeningCoverageAuditState.Classifying}:${HardeningCoverageAuditEvent.Classified}`,
      HardeningCoverageAuditState.Reported,
    ],
    [
      `${HardeningCoverageAuditState.Reported}:${HardeningCoverageAuditEvent.ReportWritten}`,
      HardeningCoverageAuditState.Reported,
    ],
    [
      `${HardeningCoverageAuditState.ReadingSources}:${HardeningCoverageAuditEvent.Fail}`,
      HardeningCoverageAuditState.Failed,
    ],
    [
      `${HardeningCoverageAuditState.Classifying}:${HardeningCoverageAuditEvent.Fail}`,
      HardeningCoverageAuditState.Failed,
    ],
  ]);
  const nextState = transitions.get(key);
  if (!nextState) {
    throw new HardeningCoverageAuditError(
      HardeningCoverageAuditErrorCode.InvalidTransition,
      `invalid hardening coverage audit transition: ${key}`,
    );
  }
  return nextState;
}

export function analyzeHardeningCoverageSources({ sources }) {
  if (!sources || Object.keys(sources).length === 0) {
    throw new HardeningCoverageAuditError(
      HardeningCoverageAuditErrorCode.SourceSetEmpty,
      "phase003 hardening coverage audit source set is empty",
    );
  }

  const targets = TARGETS.map((entry) => analyzeTarget(entry, sources));
  const targetsNeedingWork = targets.filter((entry) => entry.status !== STATUS.Covered);

  return {
    phase: "Phase 003.5",
    sourceFiles: Object.keys(sources).sort(),
    summary: {
      totalTargets: targets.length,
      covered: countStatus(targets, STATUS.Covered),
      partial: countStatus(targets, STATUS.Partial),
      missing: countStatus(targets, STATUS.Missing),
      targetsNeedingWork: targetsNeedingWork.length,
    },
    findings:
      targetsNeedingWork.length === 0
        ? []
        : [
            {
              id: "PHASE003_HARDENING_COVERAGE_GAP",
              message:
                "Some Phase 003.5 observability, security, runbook, or release targets are missing or partial.",
              targetIds: targetsNeedingWork.map((entry) => entry.id),
            },
          ],
    nextImplementationTarget: pickNextImplementationTarget(targetsNeedingWork),
    targets,
  };
}

export function renderHardeningCoverageAuditMarkdown(audit) {
  const lines = [
    "# Phase 003 Hardening Coverage Audit",
    "",
    "현재 단계: Phase 003.5 - Observability, Security, and Recovery Release Gate",
    "",
    "## Purpose",
    "",
    "- Phase 003.5 observability/security/recovery/release 범위를 코드 evidence 기준으로 고정한다.",
    "- Phase 002 archive 산출물을 active Phase 003 release hardening 완료 상태로 오판하지 않는다.",
    "- 다음 task는 모든 hardening gap을 한 번에 구현하지 않고 가장 높은 우선순위 target 하나를 선택한다.",
    "",
    "## Summary",
    "",
    "| Metric | Count |",
    "| --- | ---: |",
    `| total targets | ${audit.summary.totalTargets} |`,
    `| covered | ${audit.summary.covered} |`,
    `| partial | ${audit.summary.partial} |`,
    `| missing | ${audit.summary.missing} |`,
    `| targets needing work | ${audit.summary.targetsNeedingWork} |`,
    "",
    "## Target Status",
    "",
    "| Target | Label | Status | Missing Files | Missing Required Evidence | Missing Support Evidence |",
    "| --- | --- | --- | --- | --- | --- |",
    ...audit.targets.map((entry) => {
      const missingFiles =
        entry.missingFiles.length > 0 ? entry.missingFiles.map(code).join(", ") : "none";
      const missingRequiredEvidence =
        entry.missingRequiredEvidence.length > 0
          ? entry.missingRequiredEvidence.map(code).join(", ")
          : "none";
      const missingSupportEvidence =
        entry.missingSupportEvidence.length > 0
          ? entry.missingSupportEvidence.map(code).join(", ")
          : "none";
      return `| \`${entry.id}\` | ${entry.label} | ${entry.status} | ${missingFiles} | ${missingRequiredEvidence} | ${missingSupportEvidence} |`;
    }),
    "",
    "## Findings",
    "",
  ];

  if (audit.findings.length === 0) {
    lines.push("- No hardening coverage gap was detected.");
  } else {
    for (const finding of audit.findings) {
      lines.push(`- ${finding.id}: ${finding.message}`);
      lines.push(`- affected target count: ${finding.targetIds.length}`);
      lines.push(`- affected targets: ${finding.targetIds.map(code).join(", ")}`);
    }
  }

  lines.push("", "## Next implementation target", "");
  if (audit.nextImplementationTarget) {
    lines.push(
      `- target id: \`${audit.nextImplementationTarget.id}\``,
      `- label: ${audit.nextImplementationTarget.label}`,
      `- current status: ${audit.nextImplementationTarget.status}`,
      "- selected reason: highest priority Phase 003.5 hardening target that is not covered.",
      "- next task should implement or harden this selected target only.",
    );
  } else {
    lines.push("- No next hardening target remains.");
  }

  lines.push(
    "",
    "## Review Notes",
    "",
    "- Active Phase 003 release manifests must live under `.tasks/release`, not only under `.tasks/phase002/release` archive.",
    "- Security scan coverage must include Product Log, Field Debug Log, Development Log, denied fixtures, and explicit scan targets.",
    "- Field Debug coverage must include scope, TTL, admin approval, expiration, and revoke evidence.",
    "- Final release gate must produce a Phase 003 result artifact that states production hardening complete.",
  );

  return `${lines.join("\n")}\n`;
}

async function runCli() {
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const reportPath = path.join(repoRoot, ".tasks/phase003/hardening-coverage-audit.md");
  let state = HardeningCoverageAuditState.Pending;
  try {
    state = transitionHardeningCoverageAuditState(
      state,
      HardeningCoverageAuditEvent.Start,
    );
    const sources = await readProjectSources(repoRoot);
    state = transitionHardeningCoverageAuditState(
      state,
      HardeningCoverageAuditEvent.SourcesRead,
    );
    const audit = analyzeHardeningCoverageSources({ sources });
    state = transitionHardeningCoverageAuditState(
      state,
      HardeningCoverageAuditEvent.Classified,
    );
    await mkdir(path.dirname(reportPath), { recursive: true });
    await writeFile(reportPath, renderHardeningCoverageAuditMarkdown(audit), "utf8");
    console.log("phase003_hardening_coverage_audit=passed");
    console.log(`total_targets=${audit.summary.totalTargets}`);
    console.log(`covered_targets=${audit.summary.covered}`);
    console.log(`partial_targets=${audit.summary.partial}`);
    console.log(`missing_targets=${audit.summary.missing}`);
    console.log(`targets_needing_work=${audit.summary.targetsNeedingWork}`);
    console.log(`next_target_id=${audit.nextImplementationTarget?.id ?? "none"}`);
    console.log(`report_path=${reportPath}`);
    return transitionHardeningCoverageAuditState(
      state,
      HardeningCoverageAuditEvent.ReportWritten,
    );
  } catch (error) {
    if (
      state === HardeningCoverageAuditState.ReadingSources ||
      state === HardeningCoverageAuditState.Classifying
    ) {
      transitionHardeningCoverageAuditState(state, HardeningCoverageAuditEvent.Fail);
    }
    const code = error?.code ?? HardeningCoverageAuditErrorCode.ReportWriteFailed;
    console.error(`phase003_hardening_coverage_audit=failed code=${code}`);
    throw error;
  }
}

function target(id, label, options) {
  return {
    id,
    label,
    requiredFiles: options.requiredFiles,
    requiredEvidence: options.requiredEvidence,
    supportFiles: options.supportFiles,
    supportEvidence: options.supportEvidence,
    priority: options.priority,
  };
}

function analyzeTarget(entry, sources) {
  const missingRequiredFiles = entry.requiredFiles.filter((file) => !hasSource(sources, file));
  const missingSupportFiles = entry.supportFiles.filter((file) => !hasSource(sources, file));
  const combinedSource = [...entry.requiredFiles, ...entry.supportFiles]
    .map((file) => sources[file] ?? "")
    .join("\n");
  const missingRequiredEvidence = entry.requiredEvidence.filter(
    (evidence) => !combinedSource.includes(evidence),
  );
  const missingSupportEvidence = entry.supportEvidence.filter(
    (evidence) => !combinedSource.includes(evidence),
  );

  return {
    id: entry.id,
    label: entry.label,
    status: classifyStatus({
      entry,
      missingRequiredFiles,
      missingRequiredEvidence,
      missingSupportFiles,
      missingSupportEvidence,
    }),
    missingFiles: [...missingRequiredFiles, ...missingSupportFiles],
    missingRequiredFiles,
    missingSupportFiles,
    missingRequiredEvidence,
    missingSupportEvidence,
    priority: entry.priority,
  };
}

function classifyStatus({
  entry,
  missingRequiredFiles,
  missingRequiredEvidence,
  missingSupportFiles,
  missingSupportEvidence,
}) {
  if (missingRequiredFiles.length === 0 && missingRequiredEvidence.length === 0) {
    return STATUS.Covered;
  }

  if (
    entry.supportFiles.length > 0 &&
    missingSupportFiles.length === 0 &&
    missingSupportEvidence.length === 0
  ) {
    return STATUS.Partial;
  }

  return STATUS.Missing;
}

function pickNextImplementationTarget(targetsNeedingWork) {
  return [...targetsNeedingWork].sort((left, right) => right.priority - left.priority)[0] ?? null;
}

function countStatus(targets, status) {
  return targets.filter((target) => target.status === status).length;
}

function code(value) {
  return `\`${value}\``;
}

function hasSource(sources, file) {
  return Object.prototype.hasOwnProperty.call(sources, file);
}

async function readProjectSources(repoRoot) {
  const files = [
    ...new Set(TARGETS.flatMap((entry) => [...entry.requiredFiles, ...entry.supportFiles])),
  ].sort();
  const entries = await Promise.all(
    files.map(async (file) => {
      try {
        return [file, await readFile(path.join(repoRoot, file), "utf8")];
      } catch {
        return null;
      }
    }),
  );
  return Object.fromEntries(entries.filter(Boolean));
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await runCli();
}
