import assert from "node:assert/strict";
import test from "node:test";

import {
  RecoveryCoverageAuditErrorCode,
  RecoveryCoverageAuditEvent,
  RecoveryCoverageAuditState,
  analyzeRecoveryCoverageSources,
  renderRecoveryCoverageAuditMarkdown,
  transitionRecoveryCoverageAuditState,
} from "./phase003_recovery_coverage_audit.mjs";

const completeSources = {
  "crates/cabinet-core/src/migration.rs":
    "pub enum MigrationState { NotStarted, Locked, Running, Completed, Failed } pub enum MigrationEvent { AcquireLock, RunMigration, MigrationSucceeded, MigrationFailed, Retry }",
  "crates/cabinet-core/tests/migration_tests.rs":
    "migration_transitions_to_completed_through_explicit_events migration_rejects_invalid_transition migration_failure_state_carries_error_code_and_retry_policy migration_runner_is_idempotent_when_initial_version_is_already_recorded",
  "crates/cabinet-platform/src/release_smoke.rs":
    "run_clean_install_smoke LocalSetupHealthChecker run_data_preservation_smoke current_document_preserved version_history_preserved asset_object_preserved run_phase002_migration_fixture_smoke fixture_record_count migration_failure_preserved_current_fixture product_log_sensitive_data_absent run_startup_repair_smoke corrupted_index_rebuilt startup_repair_completed",
  "crates/cabinet-platform/tests/clean_install_smoke.rs":
    "clean_install_smoke_initializes_local_profile_once_without_external_services created_directories without_external_services",
  "crates/cabinet-platform/tests/data_preservation_smoke.rs":
    "local_data_preservation_smoke_keeps_documents_versions_and_assets_after_reinit migration_idempotent current_document_preserved version_history_preserved asset_object_preserved",
  "crates/cabinet-platform/tests/phase002_migration_fixture_smoke.rs":
    "phase002_migration_fixture_smoke_preserves_self_host_runtime_records fixture_record_count migration_failure_preserved_current_fixture product_log_sensitive_data_absent",
  "crates/cabinet-platform/tests/startup_repair_smoke.rs":
    "startup_repair_smoke_rebuilds_corrupted_indexes_without_losing_current_workspace_data corrupted_index_rebuilt startup_repair_completed",
  "crates/cabinet-adapters/tests/local_backup_store_tests.rs":
    "validate_restore_staging apply_restore_staging BACKUP_ARTIFACT_MISSING local_backup_store_reports_missing_and_corrupted_job_files corrupted",
  "crates/cabinet-ports/tests/backup_store_contract_tests.rs":
    "validate_restore_staging apply_restore_staging",
  "crates/cabinet-usecases/tests/backup_usecase_tests.rs":
    "restore_failure_preserves_workspace_current_data_and_logs_safe_failure validate_restore_staging apply_restore_staging",
  "crates/cabinet-adapters/tests/local_document_repository_tests.rs":
    "local_document_repository_reports_corrupted_metadata corrupted",
  "crates/cabinet-adapters/tests/local_version_store_tests.rs":
    "local_version_store_reports_corrupted_version_metadata corrupted",
  "crates/cabinet-adapters/tests/local_search_index_tests.rs":
    "local_search_index_handles_queries",
  "crates/cabinet-adapters/tests/local_audit_log_store_tests.rs":
    "local_audit_log_store_reports_corrupted_audit_event_file corrupted",
};

test("recovery coverage audit marks complete fixture as fully covered", () => {
  const audit = analyzeRecoveryCoverageSources({ sources: completeSources });

  assert.equal(audit.summary.totalTargets, 7);
  assert.equal(audit.summary.covered, 7);
  assert.equal(audit.summary.targetsNeedingWork, 0);
  assert.equal(audit.findings.length, 0);
  assert.equal(audit.nextImplementationTarget, null);
});

test("recovery coverage audit selects startup repair and index rebuild when missing", () => {
  const { "crates/cabinet-platform/tests/startup_repair_smoke.rs": _removed, ...sources } =
    completeSources;
  sources["crates/cabinet-platform/src/release_smoke.rs"] = sources[
    "crates/cabinet-platform/src/release_smoke.rs"
  ]
    .replace("run_startup_repair_smoke", "")
    .replace("corrupted_index_rebuilt", "")
    .replace("startup_repair_completed", "");

  const audit = analyzeRecoveryCoverageSources({ sources });

  assert.equal(audit.summary.targetsNeedingWork, 1);
  assert.equal(audit.nextImplementationTarget.id, "startup_repair_corrupted_index_rebuild");
  assert.equal(audit.findings[0].id, "PHASE003_RECOVERY_COVERAGE_GAP");
});

test("recovery coverage audit distinguishes partial evidence from missing target", () => {
  const sources = {
    ...completeSources,
    "crates/cabinet-core/tests/migration_tests.rs":
      "migration_transitions_to_completed_through_explicit_events",
  };

  const audit = analyzeRecoveryCoverageSources({ sources });
  const migrationTarget = audit.targets.find(
    (target) => target.id === "migration_state_machine",
  );

  assert.equal(migrationTarget.status, "partial");
  assert.ok(
    migrationTarget.missingEvidence.includes("migration_rejects_invalid_transition"),
  );
});

test("recovery coverage audit fails with stable code when source set is empty", () => {
  assert.throws(
    () => analyzeRecoveryCoverageSources({ sources: {} }),
    (error) => error.code === RecoveryCoverageAuditErrorCode.SourceSetEmpty,
  );
});

test("recovery coverage audit state machine rejects invalid transitions", () => {
  assert.equal(
    transitionRecoveryCoverageAuditState(
      RecoveryCoverageAuditState.NotStarted,
      RecoveryCoverageAuditEvent.Start,
    ),
    RecoveryCoverageAuditState.ReadingSource,
  );
  assert.equal(
    transitionRecoveryCoverageAuditState(
      RecoveryCoverageAuditState.ReadingSource,
      RecoveryCoverageAuditEvent.SourceLoaded,
    ),
    RecoveryCoverageAuditState.Auditing,
  );
  assert.equal(
    transitionRecoveryCoverageAuditState(
      RecoveryCoverageAuditState.Auditing,
      RecoveryCoverageAuditEvent.AuditComplete,
    ),
    RecoveryCoverageAuditState.Reported,
  );
  assert.equal(
    transitionRecoveryCoverageAuditState(
      RecoveryCoverageAuditState.Reported,
      RecoveryCoverageAuditEvent.ReportWritten,
    ),
    RecoveryCoverageAuditState.Reported,
  );
  assert.throws(
    () =>
      transitionRecoveryCoverageAuditState(
        RecoveryCoverageAuditState.NotStarted,
        RecoveryCoverageAuditEvent.ReportWritten,
      ),
    (error) => error.code === RecoveryCoverageAuditErrorCode.InvalidTransition,
  );
});

test("recovery coverage markdown records next target and review notes", () => {
  const { "crates/cabinet-platform/tests/startup_repair_smoke.rs": _removed, ...sources } =
    completeSources;
  sources["crates/cabinet-platform/src/release_smoke.rs"] = sources[
    "crates/cabinet-platform/src/release_smoke.rs"
  ]
    .replace("run_startup_repair_smoke", "")
    .replace("corrupted_index_rebuilt", "")
    .replace("startup_repair_completed", "");
  const audit = analyzeRecoveryCoverageSources({ sources });
  const markdown = renderRecoveryCoverageAuditMarkdown(audit);

  assert.match(markdown, /# Phase 003 Recovery Coverage Audit/);
  assert.match(markdown, /Phase 003\.2/);
  assert.match(markdown, /startup_repair_corrupted_index_rebuild/);
  assert.match(markdown, /Current workspace data must be preserved/);
});
