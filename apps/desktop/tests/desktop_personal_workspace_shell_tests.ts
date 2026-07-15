import assert from "node:assert/strict";
import test from "node:test";

import {
  createDesktopCurrentProductShell,
  createDesktopCurrentProductShellDescriptor,
  desktopShell,
} from "../src/index.ts";

test("desktop current product shell uses personal local workspace profile", () => {
  const shell = createDesktopCurrentProductShell();

  assert.equal(shell.capability.productScope, "personal_local_desktop");
  assert.equal(shell.capability.runtime, "desktop-local");
  assert.equal(shell.capability.supportsLocalWorkspace, true);
  assert.equal(shell.capability.supportsRemoteWorkspace, false);
  assert.equal(shell.workspace.mode, "personal-workspace-shell");
  assert.equal(shell.workspace.productScope, "personal_local_desktop");
  assert.deepEqual(
    shell.workspace.navigationItems.map((item) => item.id),
    ["home", "documents", "search", "graph", "assets", "ai", "backup", "settings"],
  );
});

test("desktop shell default export does not expose server administration actions", () => {
  const serialized = JSON.stringify(desktopShell);

  assert.equal(desktopShell.capability.productScope, "personal_local_desktop");
  assert.equal(desktopShell.workspace.mode, "personal-workspace-shell");

  for (const forbidden of [
    "server-url",
    "tenant-admin",
    "organization-admin",
    "team-invite",
    "sso-settings",
    "billing",
    "admin-console",
    "serverBaseUrl",
    "sessionToken",
  ]) {
    assert.equal(serialized.includes(forbidden), false);
  }
});

test("desktop current shell limits write actions during recovery states", () => {
  const readOnly = createDesktopCurrentProductShell("ReadOnlyRecovery");
  const failed = createDesktopCurrentProductShell("Failed");

  assert.equal(readOnly.workspace.health.displayState, "read-only-recovery");
  assert.equal(
    readOnly.workspace.health.actions.some((action) => action.id === "new-document"),
    false,
  );
  assert.equal(
    readOnly.workspace.health.actions.some((action) => action.id === "import-markdown"),
    false,
  );
  assert.equal(
    readOnly.workspace.health.actions.some((action) => action.id === "export-package"),
    true,
  );

  assert.equal(failed.workspace.health.displayState, "failed");
  assert.deepEqual(
    failed.workspace.health.actions.map((action) => action.id),
    ["open-recovery"],
  );
});

test("desktop current product descriptor keeps editor boundary and personal shell separate", () => {
  const descriptor = createDesktopCurrentProductShellDescriptor();

  assert.equal(descriptor.shell.appName, "Sponzey Cabinet");
  assert.equal(descriptor.editor, "editor:desktop-local");
  assert.equal(descriptor.workspace.mode, "personal-workspace-shell");
  assert.equal(JSON.stringify(descriptor).includes("provider_api_key_fixture"), false);
});
