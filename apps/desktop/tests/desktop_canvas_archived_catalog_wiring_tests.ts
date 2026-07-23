import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";
import { resolveDesktopCanvasMenuTarget } from "../src/desktop_canvas_catalog_controller.ts";

const source = await readFile(new URL("../src/desktop_entry.ts", import.meta.url), "utf8");

test("archived Canvas selection opens an exact route without durable active selection", () => {
  assert.match(source, /entry\?\.lifecycle === "archived"/);
  assert.match(source, /\{ kind: "Canvas", canvasId \}/);
  assert.match(source, /canvasCatalogClient/);
  assert.match(source, /requestDesktopCanvasSelection/);
});

test("Canvas catalog refresh observes identity title and lifecycle but not geometry revision", () => {
  assert.match(source, /canvasCatalogSignatureRef/);
  assert.match(source, /canvasSnapshot\.canvas\?\.title/);
  assert.match(source, /canvasSnapshot\.canvas\?\.lifecycle/);
  assert.doesNotMatch(source, /canvasCatalogSignatureRef[^\n]*revision/);
});

test("Canvas menu can reopen the last displayed archived Canvas in the current session", () => {
  assert.match(source, /resolveDesktopCanvasMenuTarget/);
  assert.match(source, /displayedLifecycle: canvasSnapshotRef\.current\.canvas\?\.lifecycle/);
  assert.match(source, /canvasSnapshotRef\.current\.canvasId/);
});

test("Canvas menu falls back to the displayed archived Canvas when the ready catalog has no active selection", () => {
  assert.equal(resolveDesktopCanvasMenuTarget({
    catalogState: "Ready",
    selectedCanvasId: undefined,
    entries: [{ canvasId: "archived-canvas", title: "보관 지도", lifecycle: "archived", revision: 4 }],
    displayedCanvasId: "archived-canvas",
    displayedLifecycle: "archived",
  }), "archived-canvas");
  assert.equal(resolveDesktopCanvasMenuTarget({
    catalogState: "Ready",
    selectedCanvasId: "active-canvas",
    entries: [{ canvasId: "archived-canvas", title: "보관 지도", lifecycle: "archived", revision: 4 }],
    displayedCanvasId: "archived-canvas",
    displayedLifecycle: "archived",
  }), "active-canvas");
});
