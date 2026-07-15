import assert from "node:assert/strict";
import test from "node:test";

import {
  BackupPackageGateErrorCode,
  BackupPackageGateEvent,
  BackupPackageGateState,
  analyzeBackupPackageEvidence,
  renderBackupPackageGateMarkdown,
  transitionBackupPackageGateState,
} from "./phase006_backup_package_gate.mjs";

test("backup package gate reports complete evidence as passed", () => {
  const result = analyzeBackupPackageEvidence({ sources: completeSources() });
  const markdown = renderBackupPackageGateMarkdown(result);

  assert.equal(result.passed, true);
  assert.equal(result.marker, "phase006_backup_package_gate=passed");
  assert.match(markdown, /phase006_backup_package_gate=passed/);
  assert.doesNotMatch(markdown, /raw markdown body should not leak/);
  assert.doesNotMatch(markdown, /asset binary content should not leak/);
  assert.doesNotMatch(markdown, /\/Users\/example\/private/);
  assert.doesNotMatch(markdown, /phase005-provider-api-key-should-not-log/);
  assert.doesNotMatch(markdown, /local-machine-secret/);
});

test("backup package gate fails when AI UX prerequisite is missing", () => {
  const sources = completeSources();
  sources[".tasks/phase006-ai-ux-gate-result.md"] = "phase006_ai_ux_gate=failed";

  const result = analyzeBackupPackageEvidence({ sources });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, BackupPackageGateErrorCode.RequiredEvidenceMissing);
  assert.equal(result.missingEvidence[0].targetId, "ai_ux_prerequisite");
});

test("backup package gate fails when import preview evidence is missing", () => {
  const sources = completeSources();
  sources["packages/ui/src/index.ts"] = "createBackupArtifactManifestViewModel";

  const result = analyzeBackupPackageEvidence({ sources });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, BackupPackageGateErrorCode.RequiredEvidenceMissing);
  assert.equal(result.missingEvidence[0].targetId, "ui_backup_restore_import_models");
});

test("backup package gate fails when Rust backup import export baseline is missing", () => {
  const sources = completeSources();
  sources["crates/cabinet-usecases/tests/export_markdown_tests.rs"] = "missing export evidence";

  const result = analyzeBackupPackageEvidence({ sources });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, BackupPackageGateErrorCode.RequiredEvidenceMissing);
  assert.equal(result.missingEvidence[0].targetId, "rust_backup_import_export_baseline");
});

test("backup package gate state machine exposes explicit transitions", () => {
  const reading = transitionBackupPackageGateState(
    BackupPackageGateState.Pending,
    BackupPackageGateEvent.Start,
  );
  const validating = transitionBackupPackageGateState(
    reading.state,
    BackupPackageGateEvent.SourcesLoaded,
  );
  const writing = transitionBackupPackageGateState(
    validating.state,
    BackupPackageGateEvent.EvidenceValidated,
  );
  const passed = transitionBackupPackageGateState(
    writing.state,
    BackupPackageGateEvent.ReportWritten,
  );
  const invalid = transitionBackupPackageGateState(
    BackupPackageGateState.Pending,
    BackupPackageGateEvent.ReportWritten,
  );

  assert.equal(reading.state, BackupPackageGateState.ReadingSources);
  assert.equal(validating.state, BackupPackageGateState.ValidatingEvidence);
  assert.equal(writing.state, BackupPackageGateState.WritingReport);
  assert.equal(passed.state, BackupPackageGateState.Passed);
  assert.equal(invalid.errorCode, BackupPackageGateErrorCode.InvalidTransition);
});

function completeSources() {
  return {
    ".tasks/phase006-ai-ux-gate-result.md": "phase006_ai_ux_gate=passed",
    "PROJECT.md": [
      "현재 공식 대상 플랫폼: Windows, macOS, Linux 데스크톱 설치형 앱",
      "백업/복원, import/export",
    ].join("\n"),
    "package.json": [
      "run:desktop-package-smoke",
      "run:desktop-packaged-app-smoke",
      "run:desktop-tauri-build",
    ].join("\n"),
    "packages/client-core/src/index.ts": [
      "BackupArtifactManifestSummaryView",
      "ImportPreviewSummaryView",
      "RestoreStagingStateView",
    ].join("\n"),
    "packages/ui/src/index.ts": [
      "createBackupArtifactManifestViewModel",
      "createRestoreStagingValidationModel",
      "createImportPreviewViewModel",
    ].join("\n"),
    "packages/ui/tests/backup_restore_staging_model_tests.ts": [
      "backup artifact manifest summary exposes counts and excludes raw data",
      "restore staging validation blocks apply until ready",
    ].join("\n"),
    "packages/ui/tests/import_preview_model_tests.ts": [
      "import preview summary supports markdown folder without raw local data",
      "import preview summary supports obsidian vault source kind",
      "import preview blocks apply while scanning failed or blocked by conflicts",
    ].join("\n"),
    "apps/desktop/src/index.ts": [
      "createDesktopBackupArtifactManifest",
      "createDesktopRestoreStagingValidation",
      "createDesktopImportPreview",
    ].join("\n"),
    "apps/desktop/tests/desktop_backup_restore_staging_smoke_tests.ts": [
      "desktop backup restore smoke exposes backup summary without raw local data",
      "desktop restore staging smoke blocks apply before validation passes",
    ].join("\n"),
    "apps/desktop/tests/desktop_import_preview_smoke_tests.ts": [
      "desktop import preview smoke supports markdown and obsidian without raw paths",
      "desktop import preview smoke blocks apply for blocking conflicts",
    ].join("\n"),
    "crates/cabinet-domain/tests/backup_job_tests.rs": "backup_job_transitions_through_retry_and_completion",
    "crates/cabinet-usecases/tests/backup_usecase_tests.rs": [
      "create_backup_returns_async_queued_job_and_records_audit",
      "export_workspace_returns_async_queued_job",
      "product_log_events_cover_backup_restore_export_without_sensitive_payloads",
    ].join("\n"),
    "crates/cabinet-usecases/tests/import_markdown_folder_tests.rs": [
      "import_markdown_folder_stores_current_documents_and_versions",
      "import_markdown_folder_continues_after_duplicate_entry_as_partial_failure",
    ].join("\n"),
    "crates/cabinet-usecases/tests/export_markdown_tests.rs": [
      "export_markdown_returns_current_documents_as_file_plan",
      "export_markdown_preserves_asset_reference_text",
    ].join("\n"),
    "crates/cabinet-adapters/tests/local_backup_store_tests.rs": [
      "local_backup_store_persists_job_snapshot_across_instances",
      "local_backup_store_validates_and_applies_restore_staging",
    ].join("\n"),
  };
}
