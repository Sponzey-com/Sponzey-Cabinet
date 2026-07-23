import assert from "node:assert/strict";
import test from "node:test";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";

import { createPersonalLocalDesktopCapabilityProfile } from "@sponzey-cabinet/client-core";
import { createPersonalWorkspaceHomeModel } from "@sponzey-cabinet/ui";
import { createDesktopCanvasElement } from "../src/react_exploration_surfaces.ts";
import { createDesktopCanvasSnapshot } from "../src/desktop_canvas_controller.ts";

const model = createPersonalWorkspaceHomeModel({
  profile: createPersonalLocalDesktopCapabilityProfile(),
  healthState: "Ready",
});

const callbacks = {
  onHome() {}, onSearch() {}, onDocument() {}, onGraph() {}, onCanvas() {}, onAssets() {}, onBackup() {},
  onCreateDocument() {}, onOpenDocument() {}, onOpenAsset() {},
  onCanvasCreate() {}, onCanvasRetry() {}, onCanvasRecover() {}, onCanvasAddNote() {},
  onCanvasAutoArrange() {}, onCanvasApplyArrange() {}, onCanvasCancelArrange() {},
  onCanvasZoom() {}, onCanvasPan() {}, onCanvasRemoveNode() {}, onCanvasAddDocument() {}, onCanvasAddAsset() {},
  onCanvasConnect() {}, onCanvasRemoveEdge() {}, canvasArchiveConfirmationOpen: false,
  canvasRenameDialogOpen: false, canvasRenameDraft: "", onCanvasArchiveRequest() {},
  onCanvasArchiveCancel() {}, onCanvasRenameRequest() {}, onCanvasRenameDraftChange() {},
  onCanvasRenameCancel() {}, onCanvasRename() {}, onCanvasArchive() {}, onCanvasNodeSelect() {},
  onCanvasEdgeSelect() {}, onCanvasDragStart() {}, onCanvasDragEnd() {}, onCanvasResizeStart() {},
  onCanvasResizeEnd() {}, canPlaceDocument: false, canPlaceAsset: false,
  onCanvasCatalogRetry() {}, onCanvasSelect() {},
};

test("Canvas catalog picker renders titles and lifecycle without visible identities", () => {
  const html = renderToStaticMarkup(createDesktopCanvasElement(
    model,
    createDesktopCanvasSnapshot("workspace-1"),
    {
      ...callbacks,
      canvasCatalog: {
        state: "Ready",
        workspaceId: "workspace-1",
        generation: 2,
        entries: [
          { canvasId: "private-canvas-a", title: "제품 지도", lifecycle: "updated", revision: 3 },
          { canvasId: "private-canvas-b", title: "보관 지도", lifecycle: "archived", revision: 4 },
        ],
        selectedCanvasId: "private-canvas-a",
        selectionSource: "last_used",
      },
      displayedCanvasId: "private-canvas-b",
    },
  ));

  assert.match(html, /data-action="select-canvas-catalog"/);
  assert.match(html, />제품 지도</);
  assert.match(html, />보관 지도 \(보관됨\)</);
  assert.match(html, /value="private-canvas-b" selected=""/);
  assert.doesNotMatch(html, /value="private-canvas-b" disabled/);
  assert.doesNotMatch(html, />private-canvas-[ab]</);
});

test("Canvas catalog empty and failed states expose create and retry actions", () => {
  const empty = renderToStaticMarkup(createDesktopCanvasElement(model, createDesktopCanvasSnapshot("workspace-1"), {
    ...callbacks,
    canvasCatalog: { state: "Empty", workspaceId: "workspace-1", generation: 1, entries: [], selectionSource: "empty" },
  }));
  assert.match(empty, /아직 캔버스가 없습니다/);
  assert.match(empty, /data-action="create-canvas"/);

  const failed = renderToStaticMarkup(createDesktopCanvasElement(model, createDesktopCanvasSnapshot("workspace-1"), {
    ...callbacks,
    canvasCatalog: { state: "Failed", workspaceId: "workspace-1", generation: 2, entries: [], errorCode: "canvas_catalog.unavailable", retryable: true },
  }));
  assert.match(failed, /캔버스 목록을 불러오지 못했습니다/);
  assert.match(failed, /data-action="retry-canvas-catalog"/);
  assert.doesNotMatch(failed, /canvas_catalog\.unavailable/);
});
