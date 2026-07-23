import assert from "node:assert/strict";
import test from "node:test";

import { presentGlobalSearchResultMetadata } from "../src/global_search_result_metadata_presenter.ts";

test("global search metadata presenter shows result count without duration", () => {
  assert.equal(
    presentGlobalSearchResultMetadata({ documentCount: 1 }),
    "1개 결과",
  );
  assert.equal(
    presentGlobalSearchResultMetadata({ documentCount: 2, assetCount: 3 }),
    "5개 결과",
  );
});

test("global search metadata presenter shows bounded integer duration when provided", () => {
  assert.equal(
    presentGlobalSearchResultMetadata({ documentCount: 1, durationMs: 42 }),
    "1개 결과 · 42ms",
  );
  assert.equal(
    presentGlobalSearchResultMetadata({ documentCount: 1, assetCount: 1, durationMs: 42.8 }),
    "2개 결과 · 43ms",
  );
});

test("global search metadata presenter omits invalid duration values", () => {
  assert.equal(
    presentGlobalSearchResultMetadata({ documentCount: 1, durationMs: -1 }),
    "1개 결과",
  );
  assert.equal(
    presentGlobalSearchResultMetadata({ documentCount: 1, durationMs: Number.NaN }),
    "1개 결과",
  );
  assert.equal(
    presentGlobalSearchResultMetadata({ documentCount: 1, durationMs: Number.POSITIVE_INFINITY }),
    "1개 결과",
  );
});
