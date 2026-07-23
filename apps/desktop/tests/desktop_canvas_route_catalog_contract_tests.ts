import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";

import { createDesktopRouteControllerState } from "../src/desktop_route_controller.ts";

test("Canvas route permits a catalog entry without inventing an identity", () => {
  const state = createDesktopRouteControllerState(
    { kind: "Canvas" },
    { workspaceId: "workspace-1", originRoute: "Home" },
  );
  assert.equal(state.status, "Stable");
  assert.deepEqual(state.route, { kind: "Canvas" });
});

test("Canvas route still rejects mismatched explicit identities", () => {
  assert.throws(() => createDesktopRouteControllerState(
    { kind: "Canvas", canvasId: "canvas-a" },
    { workspaceId: "workspace-1", canvasId: "canvas-b", originRoute: "Home" },
  ), /INVALID_ROUTE_SELECTION/);
});

test("desktop product source contains no hard-coded default Canvas identity", async () => {
  const source = await readFile(new URL("../src/desktop_entry.ts", import.meta.url), "utf8");
  assert.doesNotMatch(source, /default-canvas/);
});
