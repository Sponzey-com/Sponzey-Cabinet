import assert from "node:assert/strict";
import test from "node:test";

import {
  createBackupArtifactManifestViewModel,
  createBackupSettingsViewModel,
  createRestoreStagingValidationModel,
  transitionRestoreStagingState,
} from "../src/index.ts";
import type {
  BackupArtifactManifestSummaryView,
  RestoreStagingIssueView,
} from "../src/index.ts";

test("backup settings uses platform default path and does not block startup", () => {
  const model = createBackupSettingsViewModel({
    locationState: "PlatformDefault",
    defaultLocationLabel: "platform app data",
    latestBackupState: "NeverCreated",
    lastBackupAtIso: undefined,
  });
  const serialized = JSON.stringify(model);

  assert.equal(model.mode, "backup-settings");
  assert.equal(model.blocksLocalStartup, false);
  assert.equal(model.locationState, "PlatformDefault");
  assert.equal(model.latestBackupState, "NeverCreated");
  assert.deepEqual(
    model.actions.map((action) => action.id),
    ["create-backup", "choose-backup-location"],
  );
  assert.equal(serialized.includes("/Users/example/private/workspace"), false);
  assert.equal(serialized.includes("manual-config-file"), false);
});

test("backup artifact manifest summary exposes counts and excludes raw data", () => {
  const model = createBackupArtifactManifestViewModel(manifestSummary());
  const serialized = JSON.stringify(model);

  assert.equal(model.mode, "backup-artifact-manifest");
  assert.equal(model.artifactId, "artifact-1");
  assert.equal(model.operation, "backup");
  assert.equal(model.documentCount, 12);
  assert.equal(model.assetCount, 5);
  assert.equal(model.versionCount, 24);
  assert.equal(model.byteSizeBucket, "10mb-100mb");
  assert.deepEqual(
    model.actions.map((action) => action.id),
    ["inspect-backup-manifest", "restore-from-backup"],
  );
  assert.equal(serialized.includes("raw markdown body should not leak"), false);
  assert.equal(serialized.includes("asset binary content should not leak"), false);
  assert.equal(serialized.includes("/Users/example/private/workspace"), false);
  assert.equal(serialized.includes("phase005-provider-api-key-should-not-log"), false);
  assert.equal(serialized.includes("phase005-connector-access-token-should-not-log"), false);
  assert.equal(serialized.includes("field-debug-activation-secret"), false);
});

test("export artifact manifest exposes export action without restore-only assumptions", () => {
  const model = createBackupArtifactManifestViewModel({
    ...manifestSummary(),
    operation: "export",
    sealed: false,
  });

  assert.deepEqual(
    model.actions.map((action) => action.id),
    ["inspect-backup-manifest", "export-package"],
  );
  assert.equal(model.sealed, false);
});

test("restore staging validation blocks apply until ready", () => {
  const staging = createRestoreStagingValidationModel({
    stagingId: "staging-1",
    manifest: manifestSummary(),
    state: "Staging",
    issues: [],
  });
  const validating = createRestoreStagingValidationModel({
    stagingId: "staging-1",
    manifest: manifestSummary(),
    state: "Validating",
    issues: [],
  });
  const ready = createRestoreStagingValidationModel({
    stagingId: "staging-1",
    manifest: manifestSummary(),
    state: "ReadyToApply",
    issues: [],
  });
  const failed = createRestoreStagingValidationModel({
    stagingId: "staging-1",
    manifest: manifestSummary(),
    state: "Failed",
    issues: [issue("BACKUP_ARTIFACT_CORRUPTED", "error")],
  });

  assert.equal(staging.canApply, false);
  assert.deepEqual(staging.actions.map((action) => action.id), ["validate-restore-staging"]);
  assert.equal(validating.canApply, false);
  assert.deepEqual(validating.actions.map((action) => action.id), []);
  assert.equal(ready.canApply, true);
  assert.equal(ready.requiresConfirmation, true);
  assert.equal(ready.currentWorkspaceMutationAllowed, false);
  assert.deepEqual(ready.actions.map((action) => action.id), ["apply-restore-staging"]);
  assert.equal(failed.canApply, false);
  assert.deepEqual(failed.actions.map((action) => action.id), ["validate-restore-staging"]);
});

test("restore staging state machine exposes explicit transitions and rejects invalid apply", () => {
  const validating = transitionRestoreStagingState("Staging", "ValidateRequested");
  const ready = transitionRestoreStagingState(validating.state, "ValidationPassed");
  const applying = transitionRestoreStagingState(ready.state, "ApplyRequested");
  const completed = transitionRestoreStagingState(applying.state, "ApplySucceeded");
  const invalid = transitionRestoreStagingState("Staging", "ApplyRequested");

  assert.deepEqual(validating, { state: "Validating" });
  assert.deepEqual(ready, { state: "ReadyToApply" });
  assert.deepEqual(applying, { state: "Applying" });
  assert.deepEqual(completed, { state: "Completed" });
  assert.equal(invalid.state, "Failed");
  assert.equal(invalid.errorCode, "RESTORE_STAGING_INVALID_TRANSITION");
});

function manifestSummary(): BackupArtifactManifestSummaryView {
  return {
    artifactId: "artifact-1",
    operation: "backup",
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
