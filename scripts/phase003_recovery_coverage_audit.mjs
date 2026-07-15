import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const RecoveryCoverageAuditState = Object.freeze({
  NotStarted: "NotStarted",
  ReadingSource: "ReadingSource",
  Auditing: "Auditing",
  Reported: "Reported",
  Failed: "Failed",
});

export const RecoveryCoverageAuditEvent = Object.freeze({
  Start: "Start",
  SourceLoaded: "SourceLoaded",
  AuditComplete: "AuditComplete",
  ReportWritten: "ReportWritten",
  Fail: "Fail",
});

export const RecoveryCoverageAuditErrorCode = Object.freeze({
  InvalidTransition: "PHASE003_RECOVERY_COVERAGE_INVALID_TRANSITION",
  SourceSetEmpty: "PHASE003_RECOVERY_COVERAGE_SOURCE_SET_EMPTY",
  SourceReadFailed: "PHASE003_RECOVERY_COVERAGE_SOURCE_READ_FAILED",
  ReportWriteFailed: "PHASE003_RECOVERY_COVERAGE_REPORT_WRITE_FAILED",
});

const STATUS = Object.freeze({
  Covered: "covered",
  Partial: "partial",
  Missing: "missing",
});

const TARGETS = Object.freeze([
  target("migration_state_machine", "Migration state machine and failure transitions", {
    requiredFiles: [
      "crates/cabinet-core/src/migration.rs",
      "crates/cabinet-core/tests/migration_tests.rs",
    ],
    evidence: [
      "MigrationState",
      "MigrationEvent",
      "migration_rejects_invalid_transition",
      "migration_failure_state_carries_error_code_and_retry_policy",
      "migration_runner_is_idempotent_when_initial_version_is_already_recorded",
    ],
    priority: 100,
  }),
  target("clean_install_smoke", "Clean install without external services", {
    requiredFiles: [
      "crates/cabinet-platform/src/release_smoke.rs",
      "crates/cabinet-platform/tests/clean_install_smoke.rs",
    ],
    evidence: [
      "run_clean_install_smoke",
      "LocalSetupHealthChecker",
      "created_directories",
      "without_external_services",
    ],
    priority: 96,
  }),
  target("data_preservation_smoke", "Data preservation after reinitialization", {
    requiredFiles: [
      "crates/cabinet-platform/src/release_smoke.rs",
      "crates/cabinet-platform/tests/data_preservation_smoke.rs",
    ],
    evidence: [
      "run_data_preservation_smoke",
      "migration_idempotent",
      "current_document_preserved",
      "version_history_preserved",
      "asset_object_preserved",
    ],
    priority: 94,
  }),
  target("phase002_fixture_migration_smoke", "Phase 002 self-host fixture migration", {
    requiredFiles: [
      "crates/cabinet-platform/src/release_smoke.rs",
      "crates/cabinet-platform/tests/phase002_migration_fixture_smoke.rs",
    ],
    evidence: [
      "run_phase002_migration_fixture_smoke",
      "fixture_record_count",
      "migration_failure_preserved_current_fixture",
      "product_log_sensitive_data_absent",
    ],
    priority: 92,
  }),
  target("restore_staging_validation", "Backup restore staging validation and safe failure", {
    requiredFiles: [
      "crates/cabinet-adapters/tests/local_backup_store_tests.rs",
      "crates/cabinet-ports/tests/backup_store_contract_tests.rs",
      "crates/cabinet-usecases/tests/backup_usecase_tests.rs",
    ],
    evidence: [
      "validate_restore_staging",
      "apply_restore_staging",
      "BACKUP_ARTIFACT_MISSING",
      "restore_failure_preserves_workspace_current_data_and_logs_safe_failure",
    ],
    priority: 90,
  }),
  target("corrupted_record_detection", "Corrupted durable record detection", {
    requiredFiles: [
      "crates/cabinet-adapters/tests/local_document_repository_tests.rs",
      "crates/cabinet-adapters/tests/local_version_store_tests.rs",
      "crates/cabinet-adapters/tests/local_audit_log_store_tests.rs",
      "crates/cabinet-adapters/tests/local_backup_store_tests.rs",
    ],
    evidence: [
      "local_document_repository_reports_corrupted_metadata",
      "local_version_store_reports_corrupted_version_metadata",
      "local_audit_log_store_reports_corrupted_audit_event_file",
      "local_backup_store_reports_missing_and_corrupted_job_files",
    ],
    priority: 88,
  }),
  target("startup_repair_corrupted_index_rebuild", "Startup repair and corrupted index rebuild", {
    requiredFiles: [
      "crates/cabinet-platform/src/release_smoke.rs",
      "crates/cabinet-platform/tests/startup_repair_smoke.rs",
    ],
    evidence: [
      "run_startup_repair_smoke",
      "corrupted_index_rebuilt",
      "startup_repair_completed",
    ],
    priority: 86,
  }),
]);

class RecoveryCoverageAuditError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "RecoveryCoverageAuditError";
    this.code = code;
  }
}

export function transitionRecoveryCoverageAuditState(state, event) {
  const key = `${state}:${event}`;
  const transitions = new Map([
    [
      `${RecoveryCoverageAuditState.NotStarted}:${RecoveryCoverageAuditEvent.Start}`,
      RecoveryCoverageAuditState.ReadingSource,
    ],
    [
      `${RecoveryCoverageAuditState.ReadingSource}:${RecoveryCoverageAuditEvent.SourceLoaded}`,
      RecoveryCoverageAuditState.Auditing,
    ],
    [
      `${RecoveryCoverageAuditState.Auditing}:${RecoveryCoverageAuditEvent.AuditComplete}`,
      RecoveryCoverageAuditState.Reported,
    ],
    [
      `${RecoveryCoverageAuditState.Reported}:${RecoveryCoverageAuditEvent.ReportWritten}`,
      RecoveryCoverageAuditState.Reported,
    ],
    [
      `${RecoveryCoverageAuditState.ReadingSource}:${RecoveryCoverageAuditEvent.Fail}`,
      RecoveryCoverageAuditState.Failed,
    ],
    [
      `${RecoveryCoverageAuditState.Auditing}:${RecoveryCoverageAuditEvent.Fail}`,
      RecoveryCoverageAuditState.Failed,
    ],
  ]);
  const nextState = transitions.get(key);
  if (!nextState) {
    throw new RecoveryCoverageAuditError(
      RecoveryCoverageAuditErrorCode.InvalidTransition,
      `invalid recovery coverage audit transition: ${key}`,
    );
  }
  return nextState;
}

export function analyzeRecoveryCoverageSources({ sources }) {
  if (!sources || Object.keys(sources).length === 0) {
    throw new RecoveryCoverageAuditError(
      RecoveryCoverageAuditErrorCode.SourceSetEmpty,
      "phase003 recovery coverage audit source set is empty",
    );
  }

  const targets = TARGETS.map((entry) => analyzeTarget(entry, sources));
  const targetsNeedingWork = targets.filter((entry) => entry.status !== STATUS.Covered);

  return {
    phase: "Phase 003.2",
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
              id: "PHASE003_RECOVERY_COVERAGE_GAP",
              message:
                "Some Phase 003.2 migration and recovery requirements are not covered by explicit tests or smoke evidence.",
              targetIds: targetsNeedingWork.map((entry) => entry.id),
            },
          ],
    nextImplementationTarget: pickNextImplementationTarget(targetsNeedingWork),
    targets,
  };
}

export function renderRecoveryCoverageAuditMarkdown(audit) {
  const lines = [
    "# Phase 003 Recovery Coverage Audit",
    "",
    "현재 단계: Phase 003.2 - Persistence, Migration, and Recovery Hardening",
    "",
    "## Purpose",
    "",
    "- migration/recovery hardening 요구를 source evidence 기준으로 고정한다.",
    "- durable persistence 완료를 recovery hardening 완료로 오판하지 않는다.",
    "- 다음 task는 모든 recovery gap을 한 번에 구현하지 않고 가장 높은 우선순위 target 하나를 선택한다.",
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
    "| Target | Label | Status | Missing Files | Missing Evidence |",
    "| --- | --- | --- | --- | --- |",
    ...audit.targets.map((entry) => {
      const missingFiles = entry.missingFiles.length > 0 ? entry.missingFiles.map(code).join(", ") : "none";
      const missingEvidence =
        entry.missingEvidence.length > 0 ? entry.missingEvidence.map(code).join(", ") : "none";
      return `| \`${entry.id}\` | ${entry.label} | ${entry.status} | ${missingFiles} | ${missingEvidence} |`;
    }),
    "",
    "## Findings",
    "",
  ];

  if (audit.findings.length === 0) {
    lines.push("- No recovery coverage gap was detected.");
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
      "- selected reason: highest priority Phase 003.2 recovery coverage target that is not covered.",
      "- next task should implement or harden this selected target only.",
    );
  } else {
    lines.push("- No next recovery coverage target remains.");
  }

  lines.push(
    "",
    "## Review Notes",
    "",
    "- Current workspace data must be preserved when migration, restore, repair, or index rebuild fails.",
    "- Migration and recovery flows with failure or retry states must use explicit state transitions.",
    "- Filesystem access must remain in adapters or validation scripts, not in domain/usecase logic.",
    "- Runtime config must still be read once at bootstrap and injected explicitly.",
    "- Product Log evidence must use stable event names and error codes without document bodies, tokens, secrets, or asset content.",
  );

  return `${lines.join("\n")}\n`;
}

async function runCli() {
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const reportPath = path.join(repoRoot, ".tasks/phase003/recovery-coverage-audit.md");
  let state = RecoveryCoverageAuditState.NotStarted;
  try {
    state = transitionRecoveryCoverageAuditState(state, RecoveryCoverageAuditEvent.Start);
    const sources = await readProjectSources(repoRoot);
    state = transitionRecoveryCoverageAuditState(state, RecoveryCoverageAuditEvent.SourceLoaded);
    const audit = analyzeRecoveryCoverageSources({ sources });
    state = transitionRecoveryCoverageAuditState(state, RecoveryCoverageAuditEvent.AuditComplete);
    await mkdir(path.dirname(reportPath), { recursive: true });
    await writeFile(reportPath, renderRecoveryCoverageAuditMarkdown(audit), "utf8");
    console.log("phase003_recovery_coverage_audit=passed");
    console.log(`total_targets=${audit.summary.totalTargets}`);
    console.log(`covered_targets=${audit.summary.covered}`);
    console.log(`partial_targets=${audit.summary.partial}`);
    console.log(`missing_targets=${audit.summary.missing}`);
    console.log(`targets_needing_work=${audit.summary.targetsNeedingWork}`);
    console.log(`next_target_id=${audit.nextImplementationTarget?.id ?? "none"}`);
    console.log(`report_path=${reportPath}`);
    return transitionRecoveryCoverageAuditState(state, RecoveryCoverageAuditEvent.ReportWritten);
  } catch (error) {
    if (
      state === RecoveryCoverageAuditState.ReadingSource ||
      state === RecoveryCoverageAuditState.Auditing
    ) {
      transitionRecoveryCoverageAuditState(state, RecoveryCoverageAuditEvent.Fail);
    }
    const code = error?.code ?? RecoveryCoverageAuditErrorCode.ReportWriteFailed;
    console.error(`phase003_recovery_coverage_audit=failed code=${code}`);
    throw error;
  }
}

function target(id, label, options) {
  return {
    id,
    label,
    requiredFiles: options.requiredFiles,
    evidence: options.evidence,
    priority: options.priority,
  };
}

function analyzeTarget(entry, sources) {
  const missingFiles = entry.requiredFiles.filter((file) => !hasSource(sources, file));
  const combinedSource = entry.requiredFiles.map((file) => sources[file] ?? "").join("\n");
  const missingEvidence = entry.evidence.filter((evidence) => !combinedSource.includes(evidence));
  return {
    id: entry.id,
    label: entry.label,
    status: classifyStatus({ entry, missingFiles, missingEvidence, sources }),
    missingFiles,
    missingEvidence,
    priority: entry.priority,
  };
}

function classifyStatus({ entry, missingFiles, missingEvidence, sources }) {
  if (missingFiles.length === 0 && missingEvidence.length === 0) {
    return STATUS.Covered;
  }
  const presentFiles = entry.requiredFiles.length - missingFiles.length;
  if (
    presentFiles === 0 ||
    (missingFiles.length > 0 && missingEvidence.length === entry.evidence.length)
  ) {
    return STATUS.Missing;
  }
  return STATUS.Partial;
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
  const files = [...new Set(TARGETS.flatMap((entry) => entry.requiredFiles))].sort();
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
