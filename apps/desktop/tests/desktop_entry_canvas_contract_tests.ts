import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

test("desktop entry routes Canvas interactions through controller and typed durable mutations", async () => {
  const source = await readFile(new URL("../src/desktop_entry.ts", import.meta.url), "utf8");

  assert.match(source, /createTauriCanvasTransport\(bootstrapInvoke\)/);
  assert.match(source, /requestDesktopCanvasLoad/);
  assert.match(source, /runDesktopCanvasMutation/);
  assert.match(source, /runDesktopCanvasArrangePreview/);
  assert.match(source, /requestDesktopCanvasArrangeApply/);
  assert.match(source, /cancelDesktopCanvasArrangePreview/);
  assert.match(source, /kind: "update_node_geometry"|finishDesktopCanvasDrag/);
  assert.match(source, /beginDesktopCanvasResize/);
  assert.match(source, /finishDesktopCanvasResize/);
  assert.match(source, /createDesktopCanvasViewportDebouncer/);
  assert.match(source, /createDesktopCanvasMutationQueue/);
  assert.match(source, /canvasMutationQueueRef\.current\?\.enqueue/);
  assert.match(source, /createDesktopCanvasViewportDraft/);
  assert.match(source, /kind: "connect_edge"/);
  assert.match(source, /kind: "add_document_node"/);
  assert.match(source, /kind: "add_asset_node"/);
  assert.match(source, /canvasRenameDraft/);
  assert.match(source, /onCanvasRenameRequest/);
  assert.match(source, /kind: "rename", title/);
  assert.doesNotMatch(source, /kind: "rename", title: "Knowledge Canvas"/);
  assert.match(source, /kind: "Assets", assetId/);
  assert.doesNotMatch(source, /localStorage/);
});
