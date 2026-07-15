import assert from "node:assert/strict";
import test from "node:test";

import type {
  BackupArtifactManifestSummaryView,
  RestoreStagingIssueView,
} from "../../../packages/ui/src/index.ts";
import {
  createDesktopBackupArtifactManifest,
  createDesktopBackupSettings,
  createDesktopRestoreStagingValidation,
} from "../src/index.ts";

test("desktop backup settings smoke uses install-once defaults", () => {
  const model = createDesktopBackupSettings({
    locationState: "PlatformDefault",
    defaultLocationLabel: "platform app data",
    latestBackupState: "Fresh",
    lastBackupAtIso: "2026-07-10T00:00:00.000Z",
  });
  const serialized = JSON.stringify(model);

  assert.equal(model.blocksLocalStartup, false);
  assert.equal(model.latestBackupState, "Fresh");
  assert.deepEqual(
    model.actions.map((action) => action.id),
    ["create-backup", "choose-backup-location"],
  );
  assert.equal(serialized.includes("/Users/example/private/workspace"), false);
});

test("desktop backup restore smoke exposes backup summary without raw local data", () => {
  const model = createDesktopBackupArtifactManifest(manifestSummary("backup"));
  const serialized = JSON.stringify(model);

  assert.equal(model.mode, "backup-artifact-manifest");
  assert.deepEqual(
    model.actions.map((action) => action.id),
    ["inspect-backup-manifest", "restore-from-backup"],
  );
  assert.equal(serialized.includes("/Users/example/private/workspace"), false);
  assert.equal(serialized.includes("raw markdown body should not leak"), false);
  assert.equal(serialized.includes("asset binary content should not leak"), false);
  assert.equal(serialized.includes("phase005-provider-api-key-should-not-log"), false);
});

test("desktop restore staging smoke blocks apply before validation passes", () => {
  const staging = createDesktopRestoreStagingValidation({
    stagingId: "staging-1",
    manifest: manifestSummary("backup"),
    state: "Validating",
    issues: [],
  });
  const ready = createDesktopRestoreStagingValidation({
    stagingId: "staging-1",
    manifest: manifestSummary("backup"),
    state: "ReadyToApply",
    issues: [],
  });
  const failed = createDesktopRestoreStagingValidation({
    stagingId: "staging-1",
    manifest: manifestSummary("backup"),
    state: "Failed",
    issues: [issue("BACKUP_ARTIFACT_CORRUPTED", "error")],
  });

  assert.equal(staging.canApply, false);
  assert.deepEqual(staging.actions, []);
  assert.equal(ready.canApply, true);
  assert.equal(ready.requiresConfirmation, true);
  assert.equal(ready.currentWorkspaceMutationAllowed, false);
  assert.deepEqual(ready.actions.map((action) => action.id), ["apply-restore-staging"]);
  assert.equal(failed.canApply, false);
  assert.deepEqual(failed.issues.map((item) => item.code), ["BACKUP_ARTIFACT_CORRUPTED"]);
});

function manifestSummary(
  operation: BackupArtifactManifestSummaryView["operation"],
): BackupArtifactManifestSummaryView {
  return {
    artifactId: "artifact-1",
    operation,
    documentCount: 12,
    assetCount: 5,
    versionCount: 24,
    byteSizeBucket: "10mb-100mb",
    createdAtIso: "2026-07-09T00:00:00.000Z",
    sealed: true,
    excludedSecretCategories: [
      "provider-key",
      "token",
      "field-debug-activation",
      "local-machine-secret",
    ],
  };
}

function issue(code: string, severity: RestoreStagingIssueView["severity"]): RestoreStagingIssueView {
  return {
    code,
    severity,
  };
}
