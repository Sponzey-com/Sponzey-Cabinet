import assert from "node:assert/strict";
import test from "node:test";

import {
  createDesktopSearchResultWindow,
  selectDesktopSearchResultWindow,
  transitionDesktopSearchResultWindow,
} from "../src/desktop_search_result_window.ts";

test("search result window bounds zero through fifty results", () => {
  for (const total of [0, 1, 20, 21, 50]) {
    const state = createDesktopSearchResultWindow(1, total);
    const selected = selectDesktopSearchResultWindow(state, Array.from({ length: total }, (_, index) => index));
    assert.equal(selected.items.length, Math.min(20, total));
    assert.equal(selected.start, total === 0 ? 0 : 1);
    assert.equal(selected.end, Math.min(20, total));
  }
});

test("search result window moves by twenty and clamps at boundaries", () => {
  const first = createDesktopSearchResultWindow(2, 50);
  const second = transitionDesktopSearchResultWindow(first, { type: "Next" });
  const third = transitionDesktopSearchResultWindow(second, { type: "Next" });
  assert.equal(second.offset, 20);
  assert.equal(third.offset, 40);
  assert.strictEqual(transitionDesktopSearchResultWindow(third, { type: "Next" }), third);
  assert.equal(transitionDesktopSearchResultWindow(third, { type: "Previous" }).offset, 20);
});

test("search result window resets for a new generation and reconciles shrink", () => {
  const second = transitionDesktopSearchResultWindow(
    createDesktopSearchResultWindow(3, 50),
    { type: "Next" },
  );
  const reset = transitionDesktopSearchResultWindow(second, { type: "Reconcile", generation: 4, total: 21 });
  assert.equal(reset.offset, 0);
  const shrunk = transitionDesktopSearchResultWindow(second, { type: "Reconcile", generation: 3, total: 1 });
  assert.equal(shrunk.offset, 0);
  assert.throws(() => createDesktopSearchResultWindow(-1, 1), /INVALID_SEARCH_RESULT_WINDOW/);
});
