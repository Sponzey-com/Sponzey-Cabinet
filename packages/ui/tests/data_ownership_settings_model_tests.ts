import assert from "node:assert/strict";
import test from "node:test";

import {
  createDataOwnershipSettingsModel,
  createFieldDebugSettingsModel,
} from "../src/index.ts";

test("data ownership settings exposes local personal sections and excludes server admin settings", () => {
  const model = createDataOwnershipSettingsModel({
    workspaceId: "workspace-1",
    storageLabel: "platform app data /Users/example/private/workspace",
    backupState: "Fresh",
    importState: "PreviewReady",
    restoreState: "ReadyToApply",
    aiProviderState: "Disabled",
    fieldDebugState: "Disabled",
    workspaceHealthState: "Healthy",
  });
  const serialized = JSON.stringify(model);

  assert.equal(model.mode, "data-ownership-settings");
  assert.equal(model.productScope, "personal_local_desktop");
  assert.deepEqual(
    model.sections.map((section) => section.id),
    ["storage", "backup-export", "import", "restore", "ai-provider", "field-debug", "workspace-health"],
  );
  assert.deepEqual(model.forbiddenSectionIds, []);
  assert.equal(serialized.includes("/Users/example/private/workspace"), false);
  assert.equal(serialized.includes("server-url"), false);
  assert.equal(serialized.includes("tenant-admin"), false);
  assert.equal(serialized.includes("sso-settings"), false);
  assert.equal(serialized.includes("billing"), false);
});

test("field debug settings require scope expiry reason and masking before activation", () => {
  const rejected = createFieldDebugSettingsModel({
    state: "ActivationRequested",
    scope: "workspace:workspace-1",
    expiryMinutes: 0,
    reason: " ",
    maskingPolicyAccepted: false,
  });
  const active = createFieldDebugSettingsModel({
    state: "ActivationRequested",
    scope: "workspace:workspace-1",
    expiryMinutes: 15,
    reason: "support investigation",
    maskingPolicyAccepted: true,
  });
  const sensitive = createFieldDebugSettingsModel({
    state: "ActivationRequested",
    scope: "document_body:secret",
    expiryMinutes: 15,
    reason: "support investigation",
    maskingPolicyAccepted: true,
  });

  assert.equal(rejected.canActivate, false);
  assert.deepEqual(rejected.requiredFixes, ["expiry", "reason", "masking-policy"]);
  assert.equal(active.canActivate, true);
  assert.deepEqual(active.requiredFixes, []);
  assert.equal(sensitive.canActivate, false);
  assert.deepEqual(sensitive.requiredFixes, ["scope"]);
  assert.equal(JSON.stringify(active).includes("support investigation"), false);
});
