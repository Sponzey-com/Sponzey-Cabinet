import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import { EXPLORATION_UI_ACTION_MANIFEST } from "../src/exploration_ui_action_manifest.ts";

test("exploration action manifest has unique reviewable identities and explicit boundaries", () => {
  const ids = EXPLORATION_UI_ACTION_MANIFEST.map((entry) => entry.actionId);
  assert.equal(new Set(ids).size, ids.length);
  assert.ok(ids.length >= 40);
  for (const entry of EXPLORATION_UI_ACTION_MANIFEST) {
    assert.equal(entry.selector, `[data-action="${entry.actionId}"]`);
    assert.match(entry.interactionEvidence, /desktop_react_exploration_surfaces_tests\.ts$/);
    if (entry.state === "connected") {
      assert.notEqual(entry.controller, "none");
      assert.notEqual(entry.target, "disabled");
      assert.equal(entry.unavailableReason, undefined);
    }
  }
});

test("manifest covers all required exploration routes mutations and disabled pending controls", () => {
  const byId = new Map(EXPLORATION_UI_ACTION_MANIFEST.map((entry) => [entry.actionId, entry]));
  for (const id of [
    "navigate-home", "navigate-search", "navigate-graph", "navigate-canvas", "navigate-assets",
    "open-graph-document", "reindex-graph", "create-canvas", "add-canvas-document",
    "add-canvas-note", "add-canvas-asset", "connect-canvas-nodes", "remove-canvas-edge",
    "open-canvas-document", "open-canvas-asset", "recover-canvas",
    "rename-canvas", "cancel-canvas-rename", "confirm-canvas-rename", "archive-canvas", "cancel-canvas-archive", "confirm-canvas-archive",
    "graph-zoom-in", "graph-zoom-out", "graph-fit-view",
    "import-asset", "cancel-asset-import", "link-asset", "unlink-asset", "open-linked-document",
  ]) {
    assert.equal(byId.get(id)?.state, "connected", `${id} must be connected`);
  }
  for (const id of ["open-settings", "toggle-theme", "canvas-select-mode"]) assert.equal(byId.has(id), false);
});

test("every literal exploration data action is classified by the manifest", async () => {
  const source = await readFile(new URL("../src/react_exploration_surfaces.ts", import.meta.url), "utf8");
  const literalActions = [...source.matchAll(/"data-action": "([a-z0-9-]+)"/g)].map((match) => match[1]);
  const manifestIds = new Set(EXPLORATION_UI_ACTION_MANIFEST.map((entry) => entry.actionId));
  const unclassified = [...new Set(literalActions)].filter((action) => !manifestIds.has(action));
  assert.deepEqual(unclassified, []);
});
