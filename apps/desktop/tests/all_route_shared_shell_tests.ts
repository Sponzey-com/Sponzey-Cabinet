import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

test("home search exploration and backup modules delegate shell rendering to the shared component", async () => {
  const files = [
    "react_workspace_home.ts", "react_document_navigator.ts", "react_document_authoring_workbench.ts", "react_exploration_surfaces.ts", "react_backup_recovery.ts",
  ];
  for (const file of files) {
    const source = await readFile(new URL(`../src/${file}`, import.meta.url), "utf8");
    assert.match(source, /createWorkspaceShellElement/, file);
    assert.doesNotMatch(source, /e\("aside",\s*\{ className: "desktop-sidebar" \}/, file);
    assert.doesNotMatch(source, /e\("header",\s*\{ className: "desktop-topbar/, file);
    assert.doesNotMatch(source, /const legacy|legacy\.props\.children/, file);
  }
});

test("routed surfaces do not derive the persistent sidebar list from route-local state", async () => {
  const forbidden = new Map([
    ["react_workspace_home.ts", /documentShortcuts:\s*model\.recentDocuments/],
    ["react_document_navigator.ts", /documentShortcuts:\s*model\.items/],
    ["react_document_authoring_workbench.ts", /documentShortcuts:\s*\[\{\s*label:\s*snapshot\.title/],
    ["react_exploration_surfaces.ts", /documentShortcuts:\s*model\.recentDocuments/],
  ]);
  for (const [file, pattern] of forbidden) {
    const source = await readFile(new URL(`../src/${file}`, import.meta.url), "utf8");
    assert.doesNotMatch(source, pattern, file);
  }
});
