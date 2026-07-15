import assert from "node:assert/strict";
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  RecoveryBackupUxGateErrorCode,
  renderPhase009RecoveryBackupUxGateArtifact,
  validatePhase009RecoveryBackupUxEvidence,
} from "./phase009_recovery_backup_ux_gate.mjs";

test("Phase009 recovery backup UX gate accepts backup import restore and recovery evidence", () => {
  const result = validatePhase009RecoveryBackupUxEvidence(completeEvidence());

  assert.equal(result.ok, true);
  assert.equal(result.marker, "phase009_recovery_backup_ux_gate=passed");
  assert.equal(result.changedLayers.includes("browser-smoke"), true);
  assert.equal(result.changedLayers.includes("gate-tooling"), true);
});

test("Phase009 recovery backup UX gate rejects missing discovery prerequisite", () => {
  const result = validatePhase009RecoveryBackupUxEvidence({
    ...completeEvidence(),
    discoveryGateText: "phase009_discovery_assets_gate=failed",
  });

  assert.equal(result.ok, false);
  assert.equal(result.errorCode, RecoveryBackupUxGateErrorCode.DiscoveryGateMissing);
});

test("Phase009 recovery backup UX gate rejects missing visible backup panel marker", () => {
  const result = validatePhase009RecoveryBackupUxEvidence({
    ...completeEvidence(),
    webAppText: completeEvidence().webAppText.replace("data-cabinet-backup-panel", "missing-backup-panel"),
  });

  assert.equal(result.ok, false);
  assert.equal(result.errorCode, RecoveryBackupUxGateErrorCode.WebMarkerMissing);
});

test("Phase009 recovery backup UX artifact excludes package content and local paths", () => {
  const result = validatePhase009RecoveryBackupUxEvidence(completeEvidence());
  const artifact = renderPhase009RecoveryBackupUxGateArtifact(result);

  assert.equal(artifact.includes("phase009_recovery_backup_ux_gate=passed"), true);
  assert.equal(artifact.includes("raw markdown body should not leak"), false);
  assert.equal(artifact.includes("asset binary content should not leak"), false);
  assert.equal(artifact.includes("/Users/example/private/backup.scz"), false);
  assert.equal(artifact.includes("provider_api_key_fixture"), false);
});

test("Phase009 recovery backup UX gate CLI writes marker artifact", async () => {
  const root = await mkdtemp(join(tmpdir(), "phase009-recovery-backup-gate-"));
  await mkdir(join(root, ".tasks"), { recursive: true });
  await mkdir(join(root, "packages/ui/tests"), { recursive: true });
  await mkdir(join(root, "apps/desktop/tests"), { recursive: true });
  await mkdir(join(root, "apps/web/public"), { recursive: true });
  await mkdir(join(root, "scripts"), { recursive: true });

  await writeFile(
    join(root, ".tasks/phase009-discovery-assets-gate-result.md"),
    "phase009_discovery_assets_gate=passed",
  );
  await writeFile(
    join(root, "packages/ui/tests/backup_restore_staging_model_tests.ts"),
    completeEvidence().backupRestoreUiTestText,
  );
  await writeFile(
    join(root, "packages/ui/tests/import_preview_model_tests.ts"),
    completeEvidence().importPreviewUiTestText,
  );
  await writeFile(
    join(root, "apps/desktop/tests/desktop_backup_restore_staging_smoke_tests.ts"),
    completeEvidence().desktopBackupTestText,
  );
  await writeFile(
    join(root, "apps/desktop/tests/desktop_import_preview_smoke_tests.ts"),
    completeEvidence().desktopImportTestText,
  );
  await writeFile(join(root, "apps/web/public/app.js"), completeEvidence().webAppText);
  await writeFile(join(root, "scripts/run_browser_smoke.mjs"), completeEvidence().browserSmokeText);

  const { runPhase009RecoveryBackupUxGate } = await import("./phase009_recovery_backup_ux_gate.mjs");
  const result = await runPhase009RecoveryBackupUxGate({ rootDir: root });

  assert.equal(result.ok, true);
});

function completeEvidence() {
  return {
    discoveryGateText: "phase009_discovery_assets_gate=passed",
    backupRestoreUiTestText:
      "backup settings uses platform default path and does not block startup backup artifact manifest summary exposes counts and excludes raw data restore staging validation blocks apply until ready restore staging state machine exposes explicit transitions and rejects invalid apply",
    importPreviewUiTestText:
      "import preview summary supports markdown folder without raw local data import preview state machine exposes explicit transitions and rejects invalid apply",
    desktopBackupTestText:
      "desktop backup settings smoke uses install-once defaults desktop backup restore smoke exposes backup summary without raw local data desktop restore staging smoke blocks apply before validation passes",
    desktopImportTestText:
      "desktop import preview smoke supports markdown and obsidian without raw paths desktop import preview smoke blocks apply for blocking conflicts",
    webAppText:
      "data-cabinet-backup-panel data-cabinet-backup-manifest data-cabinet-import-panel data-cabinet-restore-panel data-cabinet-recovery-panel",
    browserSmokeText:
      "backupPanelReady backupManifestSummaryRendered importPreviewReady restoreConfirmationReady recoveryActionReady backupSensitiveDataHidden",
  };
}
