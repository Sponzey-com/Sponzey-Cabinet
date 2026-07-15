import assert from "node:assert/strict";
import test from "node:test";

import {
  createPersonalLocalDesktopCapabilityProfile,
  isForbiddenPersonalLocalDesktopAction,
} from "../src/index.ts";

test("personal local desktop profile exposes local-first actions and hides server actions", () => {
  const profile = createPersonalLocalDesktopCapabilityProfile();

  assert.equal(profile.productScope, "personal_local_desktop");
  assert.equal(profile.runtime, "desktop-local");
  assert.equal(profile.supportsLocalWorkspace, true);
  assert.equal(profile.supportsRemoteWorkspace, false);
  assert.deepEqual(profile.platforms, ["windows", "macos", "linux"]);

  const actionIds = profile.actions.map((action) => action.id);
  assert.deepEqual(actionIds, [
    "open-home",
    "new-document",
    "quick-search",
    "open-graph",
    "open-assets",
    "ask-ai",
    "create-backup",
    "import-markdown",
    "export-package",
    "open-settings",
  ]);

  for (const forbidden of [
    "server-url",
    "tenant-admin",
    "organization-admin",
    "team-invite",
    "tenant-settings",
    "sso-settings",
    "server-workspace-connect",
    "billing",
    "admin-console",
  ]) {
    assert.equal(actionIds.includes(forbidden), false);
    assert.equal(isForbiddenPersonalLocalDesktopAction(forbidden), true);
  }
});

test("personal local desktop profile carries no hidden remote setup or secret fields", () => {
  const profile = createPersonalLocalDesktopCapabilityProfile();
  const serialized = JSON.stringify(profile);

  assert.equal(serialized.includes("serverBaseUrl"), false);
  assert.equal(serialized.includes("sessionToken"), false);
  assert.equal(serialized.includes("provider_api_key_fixture"), false);
  assert.equal(serialized.includes("tenant"), false);
  assert.equal(serialized.includes("billing"), false);
});
