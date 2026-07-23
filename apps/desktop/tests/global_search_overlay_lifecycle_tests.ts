import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  createGlobalSearchOverlayLifecycle,
  transitionGlobalSearchOverlay,
} from "../src/global_search_overlay_lifecycle.ts";

test("global search overlay lifecycle opens and submits a normalized search query", () => {
  const opened = transitionGlobalSearchOverlay(createGlobalSearchOverlayLifecycle(), {
    type: "OpenRequested",
    originRoute: { kind: "Canvas", canvasId: "canvas-1" },
  });
  assert.equal(opened.state.status, "Open");
  assert.deepEqual(opened.sideEffects, [{ type: "FocusSearchInput" }]);

  const searching = transitionGlobalSearchOverlay(opened.state, {
    type: "QuerySubmitted",
    query: "  설계  ",
  });
  assert.equal(searching.state.status, "Searching");
  assert.equal(searching.state.query, "설계");
  assert.deepEqual(searching.sideEffects, [{ type: "RunSearch", query: "설계" }]);
});

test("global search overlay lifecycle resolves results empty and failure states", () => {
  const searching = transitionGlobalSearchOverlay(createGlobalSearchOverlayLifecycle(), {
    type: "OpenRequested",
    originRoute: { kind: "Home" },
    query: "문서",
  }).state;

  assert.equal(transitionGlobalSearchOverlay(searching, { type: "SearchSucceeded", resultCount: 3 }).state.status, "ResultsReady");
  assert.equal(transitionGlobalSearchOverlay(searching, { type: "SearchSucceeded", resultCount: 0 }).state.status, "Empty");
  const failed = transitionGlobalSearchOverlay(searching, {
    type: "SearchFailed",
    errorCode: "DOCUMENT_SEARCH_INDEX_UNAVAILABLE",
  });
  assert.equal(failed.state.status, "Failed");
  assert.equal(failed.state.errorCode, "DOCUMENT_SEARCH_INDEX_UNAVAILABLE");
});

test("global search overlay lifecycle closes and opens a selected result through side effects", () => {
  const ready = transitionGlobalSearchOverlay(createGlobalSearchOverlayLifecycle(), {
    type: "OpenRequested",
    originRoute: { kind: "Graph", scope: "Global" },
    query: "결과",
  }).state;

  const resultOpened = transitionGlobalSearchOverlay(
    transitionGlobalSearchOverlay(ready, { type: "SearchSucceeded", resultCount: 1 }).state,
    { type: "ResultOpened", result: { kind: "Document", documentId: "doc-1" } },
  );
  assert.equal(resultOpened.state.status, "Closed");
  assert.deepEqual(resultOpened.sideEffects, [{ type: "OpenResult", result: { kind: "Document", documentId: "doc-1" } }]);

  const closed = transitionGlobalSearchOverlay(ready, { type: "CloseRequested" });
  assert.equal(closed.state.status, "Closed");
  assert.deepEqual(closed.sideEffects, [{ type: "RestoreFocus", originRoute: { kind: "Graph", scope: "Global" } }]);
});

test("global search overlay lifecycle rejects blank search and invalid transitions", () => {
  const open = transitionGlobalSearchOverlay(createGlobalSearchOverlayLifecycle(), {
    type: "OpenRequested",
    originRoute: { kind: "Home" },
  }).state;
  const blank = transitionGlobalSearchOverlay(open, { type: "QuerySubmitted", query: "  " });
  assert.equal(blank.state.status, "Open");
  assert.deepEqual(blank.sideEffects, []);
  assert.equal(blank.errorCode, "GLOBAL_SEARCH_EMPTY_QUERY");

  const invalid = transitionGlobalSearchOverlay(createGlobalSearchOverlayLifecycle(), {
    type: "SearchSucceeded",
    resultCount: 1,
  });
  assert.equal(invalid.state.status, "Closed");
  assert.equal(invalid.errorCode, "GLOBAL_SEARCH_INVALID_TRANSITION");
});

test("global search overlay lifecycle is free from runtime IO and framework imports", async () => {
  const source = await readFile(new URL("../src/global_search_overlay_lifecycle.ts", import.meta.url), "utf8");

  assert.doesNotMatch(source, /React|document\.|window\.|localStorage|sessionStorage|process\.env|console\.|@tauri/);
});
