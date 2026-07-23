import assert from "node:assert/strict";
import test from "node:test";

import {
  applyDesktopCanvasCatalogResult,
  createDesktopCanvasCatalogSnapshot,
  loadDesktopCanvasCatalog,
  requestDesktopCanvasCatalogLoad,
  requestDesktopCanvasSelection,
  selectDesktopCanvas,
} from "../src/desktop_canvas_catalog_controller.ts";
import {
  DesktopCanvasCatalogTransportError,
  type DesktopCanvasCatalogClient,
} from "../src/tauri_canvas_catalog_transport.ts";

const readyCatalog = Object.freeze({
  entries: Object.freeze([
    Object.freeze({ canvasId: "canvas-a", title: "제품 지도", lifecycle: "updated" as const, revision: 3 }),
    Object.freeze({ canvasId: "canvas-b", title: "아이디어", lifecycle: "draft" as const, revision: 1 }),
  ]),
  selectedCanvasId: "canvas-a",
  selectionSource: "last_used" as const,
});

test("canvas catalog controller loads last-used data and represents an explicit empty catalog", async () => {
  const initial = createDesktopCanvasCatalogSnapshot("workspace-1");
  const loading = requestDesktopCanvasCatalogLoad(initial);
  const client: DesktopCanvasCatalogClient = {
    async getCatalog() { return readyCatalog; },
    async selectCanvas(_workspaceId, canvasId) { return canvasId; },
  };
  const ready = await loadDesktopCanvasCatalog(client, loading, 20);
  assert.equal(ready.state, "Ready");
  assert.equal(ready.selectedCanvasId, "canvas-a");
  assert.equal(ready.entries[0]?.title, "제품 지도");

  const empty = await loadDesktopCanvasCatalog({
    ...client,
    async getCatalog() {
      return { entries: [], selectionSource: "empty" as const };
    },
  }, requestDesktopCanvasCatalogLoad(ready), 20);
  assert.equal(empty.state, "Empty");
  assert.equal(empty.selectedCanvasId, undefined);
  assert.deepEqual(empty.entries, []);
});

test("canvas catalog controller preserves stable failure and rejects stale load results", async () => {
  const loading = requestDesktopCanvasCatalogLoad(createDesktopCanvasCatalogSnapshot("workspace-1"));
  const failed = await loadDesktopCanvasCatalog({
    async getCatalog() { throw new DesktopCanvasCatalogTransportError("canvas_catalog.corrupted", false); },
    async selectCanvas() { throw new Error("unused"); },
  }, loading, 20);
  assert.equal(failed.state, "Failed");
  assert.equal(failed.errorCode, "canvas_catalog.corrupted");
  assert.equal(failed.retryable, false);

  const newer = requestDesktopCanvasCatalogLoad(loading);
  const stale = applyDesktopCanvasCatalogResult(newer, loading.generation, readyCatalog);
  assert.equal(stale, newer);
});

test("canvas catalog controller selects only active entries and keeps failure retryable", async () => {
  const ready = await loadDesktopCanvasCatalog({
    async getCatalog() { return readyCatalog; },
    async selectCanvas(_workspaceId, canvasId) { return canvasId; },
  }, requestDesktopCanvasCatalogLoad(createDesktopCanvasCatalogSnapshot("workspace-1")), 20);
  const selecting = requestDesktopCanvasSelection(ready, "canvas-b");
  assert.equal(selecting.state, "Selecting");
  const selected = await selectDesktopCanvas({
    async getCatalog() { return readyCatalog; },
    async selectCanvas(_workspaceId, canvasId) { return canvasId; },
  }, selecting);
  assert.equal(selected.state, "Ready");
  assert.equal(selected.selectedCanvasId, "canvas-b");

  const archivedReady = Object.freeze({
    ...ready,
    entries: Object.freeze([
      Object.freeze({ canvasId: "archived", title: "보관 지도", lifecycle: "archived" as const, revision: 4 }),
    ]),
  });
  assert.equal(requestDesktopCanvasSelection(archivedReady, "archived"), archivedReady);

  const failed = await selectDesktopCanvas({
    async getCatalog() { return readyCatalog; },
    async selectCanvas() { throw new DesktopCanvasCatalogTransportError("canvas_selection.unavailable", true); },
  }, requestDesktopCanvasSelection(ready, "canvas-b"));
  assert.equal(failed.state, "Failed");
  assert.equal(failed.retryable, true);
});
