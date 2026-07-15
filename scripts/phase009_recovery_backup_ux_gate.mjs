import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const RecoveryBackupUxGateErrorCode = Object.freeze({
  DiscoveryGateMissing: "PHASE009_RECOVERY_BACKUP_DISCOVERY_GATE_MISSING",
  BackupRestoreUiTestMissing: "PHASE009_RECOVERY_BACKUP_UI_TEST_MISSING",
  ImportPreviewUiTestMissing: "PHASE009_RECOVERY_IMPORT_UI_TEST_MISSING",
  DesktopBackupTestMissing: "PHASE009_RECOVERY_BACKUP_DESKTOP_TEST_MISSING",
  DesktopImportTestMissing: "PHASE009_RECOVERY_IMPORT_DESKTOP_TEST_MISSING",
  WebMarkerMissing: "PHASE009_RECOVERY_BACKUP_WEB_MARKER_MISSING",
  BrowserSmokeEvidenceMissing: "PHASE009_RECOVERY_BACKUP_BROWSER_SMOKE_MISSING",
  SensitiveArtifactContent: "PHASE009_RECOVERY_BACKUP_SENSITIVE_ARTIFACT_CONTENT",
  IoFailed: "PHASE009_RECOVERY_BACKUP_IO_FAILED",
});

const requiredBackupRestoreUiTerms = [
  "backup settings uses platform default path and does not block startup",
  "backup artifact manifest summary exposes counts and excludes raw data",
  "restore staging validation blocks apply until ready",
  "restore staging state machine exposes explicit transitions and rejects invalid apply",
];

const requiredImportPreviewUiTerms = [
  "import preview summary supports markdown folder without raw local data",
  "import preview state machine exposes explicit transitions and rejects invalid apply",
];

const requiredDesktopBackupTerms = [
  "desktop backup settings smoke uses install-once defaults",
  "desktop backup restore smoke exposes backup summary without raw local data",
  "desktop restore staging smoke blocks apply before validation passes",
];

const requiredDesktopImportTerms = [
  "desktop import preview smoke supports markdown and obsidian without raw paths",
  "desktop import preview smoke blocks apply for blocking conflicts",
];

const requiredWebMarkers = [
  "data-cabinet-backup-panel",
  "data-cabinet-backup-manifest",
  "data-cabinet-import-panel",
  "data-cabinet-restore-panel",
  "data-cabinet-recovery-panel",
];

const requiredBrowserSmokeTerms = [
  "backupPanelReady",
  "backupManifestSummaryRendered",
  "importPreviewReady",
  "restoreConfirmationReady",
  "recoveryActionReady",
  "backupSensitiveDataHidden",
];

const sensitivePatterns = [
  /raw markdown body should not leak/i,
  /asset binary content should not leak/i,
  /backup package contents should not leak/i,
  /\/Users\/[^`\s]+/i,
  /[A-Za-z]:\\Users\\/i,
  /provider_api_key_fixture/i,
  /token_fixture/i,
  /credential_fixture/i,
];

export function validatePhase009RecoveryBackupUxEvidence(evidence) {
  if (!evidence.discoveryGateText.includes("phase009_discovery_assets_gate=passed")) {
    return failed(RecoveryBackupUxGateErrorCode.DiscoveryGateMissing, "discovery_assets_marker");
  }

  for (const term of requiredBackupRestoreUiTerms) {
    if (!evidence.backupRestoreUiTestText.includes(term)) {
      return failed(RecoveryBackupUxGateErrorCode.BackupRestoreUiTestMissing, term);
    }
  }

  for (const term of requiredImportPreviewUiTerms) {
    if (!evidence.importPreviewUiTestText.includes(term)) {
      return failed(RecoveryBackupUxGateErrorCode.ImportPreviewUiTestMissing, term);
    }
  }

  for (const term of requiredDesktopBackupTerms) {
    if (!evidence.desktopBackupTestText.includes(term)) {
      return failed(RecoveryBackupUxGateErrorCode.DesktopBackupTestMissing, term);
    }
  }

  for (const term of requiredDesktopImportTerms) {
    if (!evidence.desktopImportTestText.includes(term)) {
      return failed(RecoveryBackupUxGateErrorCode.DesktopImportTestMissing, term);
    }
  }

  for (const marker of requiredWebMarkers) {
    if (!evidence.webAppText.includes(marker)) {
      return failed(RecoveryBackupUxGateErrorCode.WebMarkerMissing, marker);
    }
  }

  for (const term of requiredBrowserSmokeTerms) {
    if (!evidence.browserSmokeText.includes(term)) {
      return failed(RecoveryBackupUxGateErrorCode.BrowserSmokeEvidenceMissing, term);
    }
  }

  return {
    ok: true,
    marker: "phase009_recovery_backup_ux_gate=passed",
    changedLayers: ["web-presenter", "browser-smoke", "gate-tooling"],
    validationCommands: [
      "node --test packages/ui/tests/backup_restore_staging_model_tests.ts packages/ui/tests/import_preview_model_tests.ts",
      "node --test apps/desktop/tests/desktop_backup_restore_staging_smoke_tests.ts apps/desktop/tests/desktop_import_preview_smoke_tests.ts",
      "npm run run:desktop-dist-browser-smoke",
      "npm run run:phase009-recovery-backup-ux-gate-tests",
      "npm run run:phase009-recovery-backup-ux-gate",
    ],
  };
}

export function renderPhase009RecoveryBackupUxGateArtifact(result) {
  const lines = [
    "# Phase 009 Recovery Backup UX Gate Result",
    "",
    result.ok ? "phase009_recovery_backup_ux_gate=passed" : "phase009_recovery_backup_ux_gate=failed",
    `validation_state=${result.ok ? "Passed" : "Failed"}`,
    "",
    "- phase: `Phase 009.5`",
    "- gate: `Backup, Import, Restore, and Recovery UX`",
    `- status: \`${result.ok ? "passed" : "failed"}\``,
    "- prerequisites:",
    "  - `.tasks/phase009-discovery-assets-gate-result.md` with `phase009_discovery_assets_gate=passed`",
    "- changed layers:",
    ...((result.changedLayers ?? []).map((layer) => `  - \`${layer}\``)),
    "- validation commands:",
    ...((result.validationCommands ?? []).map((command) => `  - \`${command}\``)),
    "- visible UX evidence: backup summary, import preview, restore confirmation, and recovery action panels are validated by browser smoke.",
    "- state machine evidence: restore staging and import preview transitions are covered by UI model tests.",
    "- Product Log candidates: `backup.created`, `backup.failed`, `import.preview.completed`, `restore.validation.completed`, `restore.apply.completed`, `restore.apply.failed`, `recovery.action.completed`, `recovery.action.failed` with stable error code only.",
    "- Field Debug metadata candidates: package hash, item counts, warning counts, masked workspace id, and recovery action id only.",
    "- Development Log scope: browser smoke diagnostics and recovery-backup gate failures remain test/development only.",
    "- runbook follow-up: local desktop recovery runbook is completed in Phase 009.6.",
    "- sensitive data exclusion: this artifact records marker names, panel ids, counts, layer ids, and stable error codes only. It does not record raw document body, asset bytes, backup package contents, local absolute path, provider key, token, credential, secret, or personal absolute path.",
  ];

  if (!result.ok) {
    lines.push(`- error_code: \`${result.errorCode}\``);
    lines.push(`- finding_id: \`${result.findingId}\``);
  }

  const artifact = `${lines.join("\n")}\n`;
  for (const pattern of sensitivePatterns) {
    if (pattern.test(artifact)) {
      return [
        "# Phase 009 Recovery Backup UX Gate Result",
        "",
        "phase009_recovery_backup_ux_gate=failed",
        "validation_state=Failed",
        `- error_code: \`${RecoveryBackupUxGateErrorCode.SensitiveArtifactContent}\``,
        "- finding_id: `artifact_sensitive_content`",
        "",
      ].join("\n");
    }
  }
  return artifact;
}

export async function runPhase009RecoveryBackupUxGate({ rootDir = process.cwd() } = {}) {
  const evidence = await readEvidence(rootDir);
  const result = validatePhase009RecoveryBackupUxEvidence(evidence);
  const artifact = renderPhase009RecoveryBackupUxGateArtifact(result);
  await mkdir(join(rootDir, ".tasks"), { recursive: true });
  await writeFile(join(rootDir, ".tasks/phase009-recovery-backup-ux-gate-result.md"), artifact);
  if (!result.ok) {
    throw new Error(`${result.errorCode}:${result.findingId}`);
  }
  return result;
}

async function readEvidence(rootDir) {
  try {
    const [
      discoveryGateText,
      backupRestoreUiTestText,
      importPreviewUiTestText,
      desktopBackupTestText,
      desktopImportTestText,
      webAppText,
      browserSmokeText,
    ] = await Promise.all([
      readFile(join(rootDir, ".tasks/phase009-discovery-assets-gate-result.md"), "utf8"),
      readFile(join(rootDir, "packages/ui/tests/backup_restore_staging_model_tests.ts"), "utf8"),
      readFile(join(rootDir, "packages/ui/tests/import_preview_model_tests.ts"), "utf8"),
      readFile(join(rootDir, "apps/desktop/tests/desktop_backup_restore_staging_smoke_tests.ts"), "utf8"),
      readFile(join(rootDir, "apps/desktop/tests/desktop_import_preview_smoke_tests.ts"), "utf8"),
      readFile(join(rootDir, "apps/web/public/app.js"), "utf8"),
      readFile(join(rootDir, "scripts/run_browser_smoke.mjs"), "utf8"),
    ]);
    return {
      discoveryGateText,
      backupRestoreUiTestText,
      importPreviewUiTestText,
      desktopBackupTestText,
      desktopImportTestText,
      webAppText,
      browserSmokeText,
    };
  } catch (error) {
    throw new Error(`${RecoveryBackupUxGateErrorCode.IoFailed}:${error.code ?? "read_failed"}`);
  }
}

function failed(errorCode, findingId) {
  return {
    ok: false,
    errorCode,
    findingId,
    changedLayers: [],
    validationCommands: [],
  };
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  runPhase009RecoveryBackupUxGate()
    .then((result) => {
      console.log(result.marker);
    })
    .catch((error) => {
      console.error(error.message);
      process.exit(1);
    });
}
