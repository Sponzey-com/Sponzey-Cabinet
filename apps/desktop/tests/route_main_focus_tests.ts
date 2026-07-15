import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import { focusWorkspaceRouteMain } from "../src/route_main_focus.ts";

test("route main focus helper focuses the shared target and reports absence", () => {
  let focused = 0;
  const target = { focus() { focused += 1; } };
  assert.equal(focusWorkspaceRouteMain({ querySelector: () => target }), true);
  assert.equal(focused, 1);
  assert.equal(focusWorkspaceRouteMain({ querySelector: () => null }), false);
});

test("desktop entry gates route main focus on stable route state", async () => {
  const source = await readFile(new URL("../src/desktop_entry.ts", import.meta.url), "utf8");
  assert.match(source, /routeState\.status !== "Stable"/);
  assert.match(source, /focusWorkspaceRouteMain/);
});
