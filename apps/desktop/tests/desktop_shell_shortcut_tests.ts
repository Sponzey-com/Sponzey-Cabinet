import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import { isMacWorkspaceSearchShortcut } from "../src/desktop_shell_shortcut.ts";

test("workspace search shortcut accepts only non-repeating macOS Cmd+K", () => {
  assert.equal(isMacWorkspaceSearchShortcut({ key: "k", metaKey: true, ctrlKey: false, altKey: false, shiftKey: false, repeat: false }), true);
  assert.equal(isMacWorkspaceSearchShortcut({ key: "K", metaKey: true, ctrlKey: false, altKey: false, shiftKey: false, repeat: false }), true);
  assert.equal(isMacWorkspaceSearchShortcut({ key: "k", metaKey: false, ctrlKey: true, altKey: false, shiftKey: false, repeat: false }), false);
  assert.equal(isMacWorkspaceSearchShortcut({ key: "k", metaKey: true, ctrlKey: false, altKey: false, shiftKey: false, repeat: true }), false);
  assert.equal(isMacWorkspaceSearchShortcut({ key: "k", metaKey: true, ctrlKey: false, altKey: false, shiftKey: true, repeat: false }), false);
});

test("desktop entry routes the workspace shortcut through the existing navigator action", async () => {
  const source = await readFile(new URL("../src/desktop_entry.ts", import.meta.url), "utf8");
  assert.match(source, /isMacWorkspaceSearchShortcut/);
  assert.match(source, /addEventListener\("keydown", handleSearchShortcut\)/);
  assert.match(source, /handleSearchShortcut[\s\S]*openNavigator\(\)/);
  assert.match(source, /removeEventListener\("keydown", handleSearchShortcut\)/);
});
