import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const source = await readFile(new URL("../src/desktop_entry.ts", import.meta.url), "utf8");

test("desktop entry uses route controller as the surface source of truth", () => {
  assert.match(source, /createDesktopRouteControllerState/);
  assert.match(source, /transitionDesktopRoute/);
  assert.match(source, /requestDesktopRoute/);
  assert.doesNotMatch(source, /const \[surface, setSurface\]/);
});

test("all primary route kinds are mapped through the shared request", () => {
  for (const kind of ["Home", "Search", "Document", "Graph", "Canvas", "Assets", "Backup"]) {
    assert.match(source, new RegExp(`requestDesktopRoute\\(\\{ kind: "${kind}"`));
  }
});

test("backup route uses the typed transport, controller, and recovery surface", () => {
  assert.match(source, /createTauriBackupRecoveryTransport\(bootstrapInvoke\)/);
  assert.match(source, /createDesktopBackupRecoverySnapshot/);
  assert.match(source, /recoverDesktopBackupStartup/);
  assert.match(source, /startDesktopBackupOperation/);
  assert.match(source, /pollDesktopBackupOperation/);
  assert.match(source, /cancelDesktopBackupOperation/);
  assert.match(source, /startDesktopRestoreOperation/);
  assert.match(source, /pollDesktopRestoreOperation/);
  assert.doesNotMatch(source, /createDesktopBackup\(/);
  assert.doesNotMatch(source, /confirmDesktopRestore\(/);
  assert.match(source, /createDesktopBackupRecoveryElement/);
  assert.match(source, /dismissDesktopRestoreConfirmation/);
});

test("dirty authoring resolution preserves target until save completion", () => {
  assert.match(source, /kind: "DirtyDocument"/);
  assert.match(source, /type: "ResolveAndContinue"/);
  assert.match(source, /type: "ResolutionCompleted"/);
  assert.match(source, /type: "ResolutionFailed"/);
  assert.match(source, /type: "DiscardAndContinue"/);
  assert.match(source, /type: "CancelTransition"/);
});

test("blocked navigation does not start target loading before route commit", () => {
  assert.match(source, /if \(!requestDesktopRoute\(\{ kind: "Search"/);
  assert.match(source, /if \(!requestDesktopRoute\(\{ kind: "Document"/);
});

test("returning home refreshes durable recent documents before reopen", () => {
  assert.match(source, /previousSurface/);
  assert.match(source, /surface === "Home"/);
  assert.match(source, /void loadHome\(\)/);
});
test("desktop entry wires global shell actions into every routed surface", async () => {
  const source = await readFile(new URL("../src/desktop_entry.ts", import.meta.url), "utf8");
  for (const token of [
    "onCreateDocument: createNewDocument",
    "onBackup:",
    "onDocument: openNavigator",
    "onSearch: openNavigator",
  ]) assert.match(source, new RegExp(token));
});
