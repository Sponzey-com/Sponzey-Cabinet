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
