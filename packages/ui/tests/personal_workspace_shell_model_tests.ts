import assert from "node:assert/strict";
import test from "node:test";

import { createPersonalLocalDesktopCapabilityProfile } from "../../client-core/src/index.ts";
import {
  createPersonalWorkspaceShellModel,
  createWorkspaceHealthActionModel,
} from "../src/index.ts";

test("personal workspace shell exposes local productivity navigation without server admin actions", () => {
  const shell = createPersonalWorkspaceShellModel({
    profile: createPersonalLocalDesktopCapabilityProfile(),
    healthState: "Ready",
  });

  assert.equal(shell.mode, "personal-workspace-shell");
  assert.equal(shell.productScope, "personal_local_desktop");
  assert.deepEqual(
    shell.navigationItems.map((item) => item.id),
    ["home", "documents", "search", "graph", "assets", "ai", "backup", "settings"],
  );
  assert.deepEqual(
    shell.commandActions.map((action) => action.id),
    [
      "new-document",
      "quick-search",
      "open-graph",
      "ask-ai",
      "create-backup",
      "import-markdown",
      "export-package",
      "open-settings",
    ],
  );

  const serialized = JSON.stringify(shell);
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
    assert.equal(serialized.includes(forbidden), false);
  }
});

test("workspace health action model limits writes during read-only recovery", () => {
  const ready = createWorkspaceHealthActionModel("Ready");
  const readOnly = createWorkspaceHealthActionModel("ReadOnlyRecovery");
  const failed = createWorkspaceHealthActionModel("Failed");

  assert.equal(ready.displayState, "ready");
  assert.equal(ready.actions.some((action) => action.id === "new-document"), true);

  assert.equal(readOnly.displayState, "read-only-recovery");
  assert.equal(readOnly.actions.some((action) => action.id === "new-document"), false);
  assert.equal(readOnly.actions.some((action) => action.id === "import-markdown"), false);
  assert.equal(readOnly.actions.some((action) => action.id === "export-package"), true);
  assert.equal(readOnly.actions.some((action) => action.id === "create-backup"), true);

  assert.equal(failed.displayState, "failed");
  assert.deepEqual(
    failed.actions.map((action) => action.id),
    ["open-recovery"],
  );
});

test("personal workspace shell model does not include raw document or secret fixtures", () => {
  const shell = createPersonalWorkspaceShellModel({
    profile: createPersonalLocalDesktopCapabilityProfile(),
    healthState: "NeedsRepair",
  });
  const serialized = JSON.stringify(shell);

  assert.equal(serialized.includes("phase006-raw-document-body-should-not-log"), false);
  assert.equal(serialized.includes("provider_api_key_fixture"), false);
  assert.equal(serialized.includes("sessionToken"), false);
  assert.equal(serialized.includes("serverBaseUrl"), false);
});
