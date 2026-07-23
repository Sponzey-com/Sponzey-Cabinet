import assert from "node:assert/strict";
import test from "node:test";

import {
  DesktopCanvasCatalogTransportError,
  createTauriCanvasCatalogTransport,
} from "../src/tauri_canvas_catalog_transport.ts";

test("canvas catalog transport maps bounded query and persists an exact selection", async () => {
  const calls: unknown[] = [];
  const client = createTauriCanvasCatalogTransport(async (command, args) => {
    calls.push({ command, args });
    if (command === "get_desktop_canvas_catalog") {
      return {
        ok: true,
        data: {
          entries: [
            { canvasId: "canvas-a", title: "제품 지도", lifecycle: "updated", revision: 3 },
          ],
          selectedCanvasId: "canvas-a",
          selectionSource: "last_used",
        },
        selectedCanvasId: "canvas-a",
        retryable: false,
      };
    }
    return {
      ok: true,
      data: null,
      selectedCanvasId: "canvas-a",
      retryable: false,
    };
  });

  const catalog = await client.getCatalog({
    workspaceId: "workspace-1",
    limit: 20,
    includeArchived: false,
  });
  assert.equal(catalog.entries[0]?.title, "제품 지도");
  assert.equal(catalog.selectedCanvasId, "canvas-a");
  assert.equal(await client.selectCanvas("workspace-1", "canvas-a"), "canvas-a");
  assert.deepEqual(calls, [
    {
      command: "get_desktop_canvas_catalog",
      args: {
        request: { workspaceId: "workspace-1", limit: 20, includeArchived: false },
      },
    },
    {
      command: "select_desktop_canvas",
      args: { request: { workspaceId: "workspace-1", canvasId: "canvas-a" } },
    },
  ]);
});

test("canvas catalog transport accepts the explicit empty state", async () => {
  const client = createTauriCanvasCatalogTransport(async () => ({
    ok: true,
    data: { entries: [], selectedCanvasId: null, selectionSource: "empty" },
    selectedCanvasId: null,
    retryable: false,
  }));

  const catalog = await client.getCatalog({
    workspaceId: "workspace-1",
    limit: 20,
    includeArchived: false,
  });
  assert.deepEqual(catalog.entries, []);
  assert.equal(catalog.selectedCanvasId, undefined);
  assert.equal(catalog.selectionSource, "empty");
});

test("canvas catalog transport rejects malformed or storage-revealing DTOs", async () => {
  const malformedValues = [
    { entries: [{ canvasId: "canvas-a", title: "지도", lifecycle: "unknown", revision: 1 }], selectedCanvasId: "canvas-a", selectionSource: "fallback" },
    { entries: [{ canvasId: "canvas-a", title: "지도", lifecycle: "draft", revision: 0 }], selectedCanvasId: "canvas-a", selectionSource: "fallback" },
    { entries: [{ canvasId: "canvas-a", title: "지도", lifecycle: "draft", revision: 1 }], selectedCanvasId: "missing", selectionSource: "fallback" },
    { entries: [{ canvasId: "canvas-a", title: "지도", lifecycle: "draft", revision: 1, path: "/private/canvas" }], selectedCanvasId: "canvas-a", selectionSource: "fallback" },
  ];

  for (const data of malformedValues) {
    const client = createTauriCanvasCatalogTransport(async () => ({
      ok: true,
      data,
      selectedCanvasId: data.selectedCanvasId,
      retryable: false,
    }));
    await assert.rejects(
      client.getCatalog({ workspaceId: "workspace-1", limit: 20, includeArchived: false }),
      (error: unknown) =>
        error instanceof DesktopCanvasCatalogTransportError &&
        error.code === "COMMAND_BRIDGE_FAILED",
    );
  }
});

test("canvas catalog transport preserves stable native failures", async () => {
  const client = createTauriCanvasCatalogTransport(async () => ({
    ok: false,
    data: null,
    selectedCanvasId: null,
    errorCode: "canvas_catalog.corrupted",
    retryable: false,
  }));

  await assert.rejects(
    client.getCatalog({ workspaceId: "workspace-1", limit: 20, includeArchived: false }),
    (error: unknown) =>
      error instanceof DesktopCanvasCatalogTransportError &&
      error.code === "canvas_catalog.corrupted" &&
      error.retryable === false,
  );
});
