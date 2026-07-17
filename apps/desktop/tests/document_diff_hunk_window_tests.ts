import assert from "node:assert/strict";
import test from "node:test";

import {
  createDocumentDiffHunkWindow,
  nextDocumentDiffHunkWindow,
  previousDocumentDiffHunkWindow,
} from "../src/document_diff_hunk_window.ts";

test("hunk window normalizes empty first and last ranges", () => {
  assert.deepEqual(createDocumentDiffHunkWindow(0), {
    start: 0,
    endExclusive: 0,
    total: 0,
    size: 50,
    hasPrevious: false,
    hasNext: false,
  });
  assert.deepEqual(createDocumentDiffHunkWindow(1_000, -20), {
    start: 0,
    endExclusive: 50,
    total: 1_000,
    size: 50,
    hasPrevious: false,
    hasNext: true,
  });
  assert.deepEqual(createDocumentDiffHunkWindow(123, 10_000), {
    start: 100,
    endExclusive: 123,
    total: 123,
    size: 50,
    hasPrevious: true,
    hasNext: false,
  });
});

test("hunk window moves one bounded page and refuses boundary events", () => {
  const first = createDocumentDiffHunkWindow(123);
  const second = nextDocumentDiffHunkWindow(first);
  const last = nextDocumentDiffHunkWindow(second);

  assert.equal(previousDocumentDiffHunkWindow(first), first);
  assert.deepEqual(second, { ...second, start: 50, endExclusive: 100 });
  assert.deepEqual(last, { ...last, start: 100, endExclusive: 123, hasNext: false });
  assert.equal(nextDocumentDiffHunkWindow(last), last);
  assert.deepEqual(previousDocumentDiffHunkWindow(last), second);
});
