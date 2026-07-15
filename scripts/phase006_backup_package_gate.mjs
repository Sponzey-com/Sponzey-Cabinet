import { readFile, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

export const BackupPackageGateState = Object.freeze({
  Pending: "Pending",
  ReadingSources: "ReadingSources",
  ValidatingEvidence: "ValidatingEvidence",
  WritingReport: "WritingReport",
  Passed: "Passed",
  Failed: "Failed",
});

export const BackupPackageGateEvent = Object.freeze({
  Start: "Start",
  SourcesLoaded: "SourcesLoaded",
  EvidenceValidated: "EvidenceValidated",
  ReportWritten: "ReportWritten",
  Fail: "Fail",
});

export const BackupPackageGateErrorCode = Object.freeze({
  RequiredEvidenceMissing: "PHASE006_BACKUP_PACKAGE_REQUIRED_EVIDENCE_MISSING",
  SourceReadFailed: "PHASE006_BACKUP_PACKAGE_SOURCE_READ_FAILED",
  InvalidTransition: "PHASE006_BACKUP_PACKAGE_INVALID_TRANSITION",
});

const requiredTargets = Object.freeze([
  target("ai_ux_prerequisite", "Phase 006 AI UX prerequisite", {
    requiredFiles: [".tasks/phase006-ai-ux-gate-result.md"],
    evidence: ["phase006_ai_ux_gate=passed"],
  }),
  target("desktop_package_target_policy", "desktop package target policy", {
    requiredFiles: ["PROJECT.md", "package.json"],
    evidence: [
      "현재 공식 대상 플랫폼: Windows, macOS, Linux 데스크톱 설치형 앱",
      "백업/복원, import/export",
      "run:desktop-package-smoke",
      "run:desktop-packaged-app-smoke",
      "run:desktop-tauri-build",
    ],
  }),
  target("client_core_backup_import_types", "client-core backup import DTO surface", {
    requiredFiles: ["packages/client-core/src/index.ts"],
    evidence: [
      "BackupArtifactManifestSummaryView",
      "RestoreStagingStateView",
      "ImportPreviewSummaryView",
    ],
  }),
  target("ui_backup_restore_import_models", "UI backup restore import models", {
    requiredFiles: [
      "packages/ui/src/index.ts",
      "packages/ui/tests/backup_restore_staging_model_tests.ts",
      "packages/ui/tests/import_preview_model_tests.ts",
    ],
    evidence: [
      "createBackupArtifactManifestViewModel",
      "createRestoreStagingValidationModel",
      "createImportPreviewViewModel",
      "backup artifact manifest summary exposes counts and excludes raw data",
      "restore staging validation blocks apply until ready",
      "import preview summary supports markdown folder without raw local data",
      "import preview summary supports obsidian vault source kind",
      "import preview blocks apply while scanning failed or blocked by conflicts",
    ],
  }),
  target("desktop_backup_import_smoke", "desktop backup restore import smoke", {
    requiredFiles: [
      "apps/desktop/src/index.ts",
      "apps/desktop/tests/desktop_backup_restore_staging_smoke_tests.ts",
      "apps/desktop/tests/desktop_import_preview_smoke_tests.ts",
    ],
    evidence: [
      "createDesktopBackupArtifactManifest",
      "createDesktopRestoreStagingValidation",
      "createDesktopImportPreview",
      "desktop backup restore smoke exposes backup summary without raw local data",
      "desktop restore staging smoke blocks apply before validation passes",
      "desktop import preview smoke supports markdown and obsidian without raw paths",
      "desktop import preview smoke blocks apply for blocking conflicts",
    ],
  }),
  target("rust_backup_import_export_baseline", "Rust backup import export baseline", {
    requiredFiles: [
      "crates/cabinet-domain/tests/backup_job_tests.rs",
      "crates/cabinet-usecases/tests/backup_usecase_tests.rs",
      "crates/cabinet-usecases/tests/import_markdown_folder_tests.rs",
      "crates/cabinet-usecases/tests/export_markdown_tests.rs",
      "crates/cabinet-adapters/tests/local_backup_store_tests.rs",
    ],
    evidence: [
      "backup_job_transitions_through_retry_and_completion",
      "create_backup_returns_async_queued_job_and_records_audit",
      "export_workspace_returns_async_queued_job",
      "product_log_events_cover_backup_restore_export_without_sensitive_payloads",
      "import_markdown_folder_stores_current_documents_and_versions",
      "import_markdown_folder_continues_after_duplicate_entry_as_partial_failure",
      "export_markdown_returns_current_documents_as_file_plan",
      "export_markdown_preserves_asset_reference_text",
      "local_backup_store_persists_job_snapshot_across_instances",
      "local_backup_store_validates_and_applies_restore_staging",
    ],
  }),
]);

export function transitionBackupPackageGateState(currentState, event, detail = {}) {
  if (currentState === BackupPackageGateState.Pending && event === BackupPackageGateEvent.Start) {
    return { state: BackupPackageGateState.ReadingSources };
  }
  if (
    currentState === BackupPackageGateState.ReadingSources &&
    event === BackupPackageGateEvent.SourcesLoaded
  ) {
    return { state: BackupPackageGateState.ValidatingEvidence };
  }
  if (
    currentState === BackupPackageGateState.ValidatingEvidence &&
    event === BackupPackageGateEvent.EvidenceValidated
  ) {
    return { state: BackupPackageGateState.WritingReport };
  }
  if (
    currentState === BackupPackageGateState.WritingReport &&
    event === BackupPackageGateEvent.ReportWritten
  ) {
    return { state: BackupPackageGateState.Passed };
  }
  if (
    [
      BackupPackageGateState.ReadingSources,
      BackupPackageGateState.ValidatingEvidence,
      BackupPackageGateState.WritingReport,
    ].includes(currentState) &&
    event === BackupPackageGateEvent.Fail
  ) {
    return {
      state: BackupPackageGateState.Failed,
      errorCode: detail.errorCode ?? BackupPackageGateErrorCode.RequiredEvidenceMissing,
      findingId: detail.findingId,
    };
  }
  return {
    state: BackupPackageGateState.Failed,
    errorCode: BackupPackageGateErrorCode.InvalidTransition,
  };
}

export function analyzeBackupPackageEvidence({ sources }) {
  if (!sources || Object.keys(sources).length === 0) {
    return failedResult({
      errorCode: BackupPackageGateErrorCode.SourceReadFailed,
      missingEvidence: [{ targetId: "source_set", missing: ["sources"] }],
    });
  }
  const targetResults = requiredTargets.map((entry) => analyzeTarget(entry, sources));
  const missingEvidence = targetResults
    .filter((entry) => entry.missing.length > 0)
    .map((entry) => ({ targetId: entry.id, missing: entry.missing }));
  if (missingEvidence.length > 0) {
    return failedResult({
      errorCode: BackupPackageGateErrorCode.RequiredEvidenceMissing,
      missingEvidence,
      targetResults,
    });
  }
  return {
    passed: true,
    marker: "phase006_backup_package_gate=passed",
    state: BackupPackageGateState.Passed,
    summary: { requiredTargets: requiredTargets.length, missingRequiredEvidence: 0 },
    targetResults,
    missingEvidence: [],
  };
}

export function renderBackupPackageGateMarkdown(result) {
  const lines = [
    "# Phase 006 Backup Import Export Package Gate Result",
    "",
    result.marker,
    "",
    "- phase: `Phase 006`",
    "- gate: `Backup, Import/Export, Packaging Baseline`",
    `- status: \`${result.passed ? "passed" : "failed"}\``,
    `- state: \`${result.state}\``,
  ];
  if (!result.passed) {
    lines.push(`- error code: \`${result.errorCode}\``);
  }
  lines.push(
    `- required targets: \`${result.summary.requiredTargets}\``,
    `- missing required evidence: \`${result.summary.missingRequiredEvidence}\``,
    "",
    "## Evidence",
    "",
    "| Target | Status | Description |",
    "| --- | --- | --- |",
  );
  for (const targetResult of result.targetResults ?? []) {
    lines.push(`| \`${targetResult.id}\` | \`${targetResult.status}\` | ${targetResult.description} |`);
  }
  lines.push(
    "",
    "## Sensitive Data Exclusion",
    "",
    "This artifact records target ids, marker status, counts, and stable error codes only. It does not record document body, asset content, personal path, provider secret, credential, token, local machine secret, Field Debug activation state, or package internal file contents.",
    "",
    "## Package Boundary",
    "",
    "- This gate validates desktop package target policy and deterministic backup/import/export evidence.",
    "- It does not perform OS signing, notarization, app store distribution, or external provider validation.",
    "",
  );
  return lines.join("\n");
}

export async function runBackupPackageGate({ root = process.cwd() } = {}) {
  let state = transitionBackupPackageGateState(
    BackupPackageGateState.Pending,
    BackupPackageGateEvent.Start,
  );
  try {
    const sources = {};
    for (const filePath of collectRequiredFiles()) {
      sources[filePath] = await readFile(`${root}/${filePath}`, "utf8");
    }
    state = transitionBackupPackageGateState(state.state, BackupPackageGateEvent.SourcesLoaded);
    const result = analyzeBackupPackageEvidence({ sources });
    if (!result.passed) {
      state = transitionBackupPackageGateState(state.state, BackupPackageGateEvent.Fail, {
        errorCode: result.errorCode,
        findingId: result.missingEvidence[0]?.targetId,
      });
      return { ...result, state: state.state };
    }
    state = transitionBackupPackageGateState(state.state, BackupPackageGateEvent.EvidenceValidated);
    state = transitionBackupPackageGateState(state.state, BackupPackageGateEvent.ReportWritten);
    return { ...result, state: state.state };
  } catch {
    state = transitionBackupPackageGateState(state.state, BackupPackageGateEvent.Fail, {
      errorCode: BackupPackageGateErrorCode.SourceReadFailed,
    });
    return failedResult({
      errorCode: state.errorCode,
      state: state.state,
      missingEvidence: [{ targetId: "source_read", missing: ["required source file"] }],
    });
  }
}

function analyzeTarget(entry, sources) {
  const texts = entry.requiredFiles.map((filePath) => sources[filePath] ?? "");
  const missingFiles = entry.requiredFiles.filter((filePath) => !(filePath in sources));
  const missingEvidence = entry.evidence.filter((needle) => !texts.some((text) => text.includes(needle)));
  const missing = [...missingFiles, ...missingEvidence];
  return {
    id: entry.id,
    description: entry.description,
    status: missing.length === 0 ? "covered" : "missing",
    missing,
  };
}

function failedResult({ errorCode, state = BackupPackageGateState.Failed, missingEvidence, targetResults = [] }) {
  return {
    passed: false,
    marker: "phase006_backup_package_gate=failed",
    state,
    errorCode,
    summary: { requiredTargets: requiredTargets.length, missingRequiredEvidence: missingEvidence.length },
    targetResults,
    missingEvidence,
  };
}

function target(id, description, { requiredFiles, evidence }) {
  return { id, description, requiredFiles, evidence };
}

function collectRequiredFiles() {
  return [...new Set(requiredTargets.flatMap((entry) => entry.requiredFiles))];
}

async function runCli() {
  const result = await runBackupPackageGate();
  await writeFile(
    ".tasks/phase006-backup-package-gate-result.md",
    renderBackupPackageGateMarkdown(result),
  );
  if (result.passed) {
    console.log(result.marker);
    console.log(`gate_state=${result.state}`);
    console.log(`required_targets=${result.summary.requiredTargets}`);
    return;
  }
  console.error(result.marker);
  console.error(`gate_state=${result.state}`);
  console.error(`error_code=${result.errorCode}`);
  process.exit(1);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runCli();
}
