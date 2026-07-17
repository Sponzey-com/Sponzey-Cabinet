import assert from "node:assert/strict";
import test from "node:test";

import {
  createDocumentHistoryWindow,
  historyIdentityChangeRequiresReset,
  nextDocumentHistoryWindow,
  previousDocumentHistoryWindow,
  reconcileDocumentHistoryWindow,
} from "../src/document_history_window.ts";

test("history window virtualizes only above one hundred entries", () => {
  assert.deepEqual(createDocumentHistoryWindow(0), {
    start: 0, endExclusive: 0, total: 0, windowSize: 50,
    virtualized: false, hasPrevious: false, hasNext: false,
  });
  assert.deepEqual(createDocumentHistoryWindow(100), {
    start: 0, endExclusive: 100, total: 100, windowSize: 50,
    virtualized: false, hasPrevious: false, hasNext: false,
  });
  assert.deepEqual(createDocumentHistoryWindow(101), {
    start: 0, endExclusive: 50, total: 101, windowSize: 50,
    virtualized: true, hasPrevious: false, hasNext: true,
  });
});

test("history window transitions are bounded and request deterministic focus", () => {
  const first = createDocumentHistoryWindow(201);
  const second = nextDocumentHistoryWindow(first);
  const last = nextDocumentHistoryWindow(nextDocumentHistoryWindow(nextDocumentHistoryWindow(second.window).window).window);

  assert.equal(second.focusRequest, "FocusFirstVisible");
  assert.deepEqual(second.window, { ...second.window, start: 50, endExclusive: 100 });
  assert.deepEqual(last.window, { ...last.window, start: 200, endExclusive: 201, hasNext: false });
  assert.equal(nextDocumentHistoryWindow(last.window).window, last.window);
  assert.equal(nextDocumentHistoryWindow(last.window).focusRequest, "None");
  assert.equal(previousDocumentHistoryWindow(first).window, first);
  assert.deepEqual(previousDocumentHistoryWindow(last.window).window, {
    ...last.window, start: 150, endExclusive: 200, hasNext: true,
  });
});

test("history reconciliation preserves a valid window and resets changed identities", () => {
  const second = nextDocumentHistoryWindow(createDocumentHistoryWindow(200)).window;
  assert.deepEqual(reconcileDocumentHistoryWindow(second, 151, false), {
    ...second, total: 151, endExclusive: 100,
  });
  assert.deepEqual(reconcileDocumentHistoryWindow(second, 20, false), {
    start: 0, endExclusive: 20, total: 20, windowSize: 50,
    virtualized: false, hasPrevious: false, hasNext: false,
  });
  assert.deepEqual(reconcileDocumentHistoryWindow(second, 200, true), createDocumentHistoryWindow(200));
  assert.equal(historyIdentityChangeRequiresReset(["a", "b"], ["a", "b", "c"]), false);
  assert.equal(historyIdentityChangeRequiresReset(["a", "b"], ["new", "a", "b"]), true);
  assert.equal(historyIdentityChangeRequiresReset(["a", "b"], ["a"]), true);
});
