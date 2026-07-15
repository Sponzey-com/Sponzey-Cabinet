import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import { CORE_UI_ACTION_MANIFEST } from "../src/core_ui_action_manifest.ts";
import { EXPLORATION_UI_ACTION_CONTRACTS } from "../src/exploration_ui_action_manifest.ts";

test("core and exploration contracts form one unique manifest without pending actions", () => {
  const combined = [...CORE_UI_ACTION_MANIFEST, ...EXPLORATION_UI_ACTION_CONTRACTS];
  const ids = combined.map((entry) => entry.actionId);
  assert.equal(new Set(ids).size, ids.length);
  assert.equal(combined.some((entry) => (entry.availability as string) === "disabled_pending"), false);
});

test("unified contracts cover every literal exploration action and exclude current mode indicator", async () => {
  const source = await readFile(new URL("../src/react_exploration_surfaces.ts", import.meta.url), "utf8");
  const literal = [...source.matchAll(/"data-action": "([a-z0-9-]+)"/g)].map((match) => match[1]);
  const ids = new Set([...CORE_UI_ACTION_MANIFEST, ...EXPLORATION_UI_ACTION_CONTRACTS].map((entry) => entry.actionId));
  assert.deepEqual([...new Set(literal)].filter((id) => !ids.has(id)), []);
  assert.doesNotMatch(source, /data-action": "canvas-select-mode"/);
});
