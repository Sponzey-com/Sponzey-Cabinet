import assert from "node:assert/strict";
import test from "node:test";

import { createRecoveryActionPanelModel } from "../src/index.ts";

test("recovery action panel maps local failures to safe user actions", () => {
  const degraded = createRecoveryActionPanelModel({
    workspaceId: "workspace-1",
    state: "RepairAvailable",
    issueCode: "INDEX_STALE",
  });
  const readOnly = createRecoveryActionPanelModel({
    workspaceId: "workspace-1",
    state: "ReadOnlyRecovery",
    issueCode: "DOCUMENT_AUTHORING_POINTER_UPDATE_FAILED",
  });
  const failed = createRecoveryActionPanelModel({
    workspaceId: "workspace-1",
    state: "RepairFailed",
    issueCode: "BACKUP_RESTORE_FAILED",
  });
  const serialized = JSON.stringify([degraded, readOnly, failed]);

  assert.deepEqual(degraded.actions.map((action) => action.id), ["repair-workspace", "export-safe-copy"]);
  assert.deepEqual(readOnly.actions.map((action) => action.id), ["export-safe-copy", "open-backup-settings"]);
  assert.deepEqual(failed.actions.map((action) => action.id), ["retry-repair", "open-runbook", "export-safe-copy"]);
  assert.equal(readOnly.readOnly, true);
  assert.equal(failed.productLogEvent, "workspace.repair.failed");
  assert.equal(serialized.includes("/Users/example/private/workspace"), false);
  assert.equal(serialized.includes("raw markdown body should not leak"), false);
  assert.equal(serialized.includes("provider_api_key_fixture"), false);
});

test("healthy and repairing recovery states avoid destructive actions", () => {
  const healthy = createRecoveryActionPanelModel({
    workspaceId: "workspace-1",
    state: "Healthy",
  });
  const repairing = createRecoveryActionPanelModel({
    workspaceId: "workspace-1",
    state: "Repairing",
    issueCode: "INDEX_REBUILDING",
  });

  assert.deepEqual(healthy.actions, []);
  assert.deepEqual(repairing.actions.map((action) => action.id), ["view-progress"]);
  assert.equal(repairing.productLogEvent, undefined);
});
