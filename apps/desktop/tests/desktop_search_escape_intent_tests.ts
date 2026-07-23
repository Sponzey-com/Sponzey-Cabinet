import assert from "node:assert/strict";
import test from "node:test";

import { createDesktopSearchEscapeIntent } from "../src/desktop_search_escape_intent.ts";

test("search Escape clears a non-empty query before returning", () => {
  assert.deepEqual(createDesktopSearchEscapeIntent("  cabinet  ", { kind: "Graph", scope: "Global" }), {
    kind: "ClearQuery",
  });
});

test("search Escape returns to a valid origin and falls back home", () => {
  assert.deepEqual(createDesktopSearchEscapeIntent("", { kind: "Canvas", canvasId: "canvas-1" }), {
    kind: "ReturnToOrigin",
    route: { kind: "Canvas", canvasId: "canvas-1" },
  });
  assert.deepEqual(createDesktopSearchEscapeIntent("", { kind: "Search", query: "old" }), {
    kind: "ReturnToOrigin",
    route: { kind: "Home" },
  });
});
