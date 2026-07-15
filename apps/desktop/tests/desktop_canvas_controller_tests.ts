import assert from "node:assert/strict";
import test from "node:test";

import {
  applyDesktopCanvasResult,
  beginDesktopCanvasDrag,
  beginDesktopCanvasResize,
  createDesktopCanvasSnapshot,
  createDesktopCanvasViewportDraft,
  finishDesktopCanvasDrag,
  finishDesktopCanvasResize,
  loadDesktopCanvas,
  applyDesktopCanvasArrangePreview,
  cancelDesktopCanvasArrangePreview,
  requestDesktopCanvasArrangePreview,
  requestDesktopCanvasArrangeApply,
  requestDesktopCanvasLoad,
  requestDesktopCanvasMutation,
  requestDesktopCanvasRecovery,
  runDesktopCanvasRecovery,
  runDesktopCanvasMutation,
  selectDesktopCanvasEdge,
  selectDesktopCanvasNode,
} from "../src/desktop_canvas_controller.ts";
import { DesktopCanvasTransportError, type DesktopCanvasData } from "../src/tauri_canvas_transport.ts";

const readyCanvas = (revision = 1, lifecycle: DesktopCanvasData["lifecycle"] = "draft"): DesktopCanvasData => ({
  canvasId: "canvas-1",
  title: "Product map",
  revision,
  lifecycle,
  viewport: { centerX: 0, centerY: 0, zoomPercent: 100 },
  nodes: [],
  edges: [],
});

test("Canvas controller loads durable DTO and ignores stale generation result", async () => {
  const idle = createDesktopCanvasSnapshot("workspace-1");
  const first = requestDesktopCanvasLoad(idle, "canvas-1");
  const second = requestDesktopCanvasLoad(first, "canvas-2");
  const stale = applyDesktopCanvasResult(second, first.generation, readyCanvas());
  assert.strictEqual(stale, second);

  let requestKind = "";
  const loaded = await loadDesktopCanvas({ async execute(request) { requestKind = request.kind; return readyCanvas(); } }, first);
  assert.equal(loaded.state, "Ready");
  assert.equal(loaded.canvas?.revision, 1);
  assert.equal(requestKind, "get_viewport");
});

test("Canvas controller acknowledges mutation revision and never silently retries conflict", async () => {
  const loading = requestDesktopCanvasLoad(createDesktopCanvasSnapshot("workspace-1"), "canvas-1");
  const ready = applyDesktopCanvasResult(loading, loading.generation, readyCanvas());
  const mutating = requestDesktopCanvasMutation(ready, { kind: "add_text_node", nodeId: "note-1", text: "Decision", x: 80, y: 80, width: 320, height: 180 }, "operation-1");
  assert.equal(mutating.state, "Mutating");
  assert.equal(mutating.pendingRequest?.expectedRevision, 1);
  assert.equal(mutating.pendingRequest?.operationId, "operation-1");

  const acknowledged = await runDesktopCanvasMutation({ async execute() { return { ...readyCanvas(2, "updated"), operationId: "operation-1" }; } }, mutating);
  assert.equal(acknowledged.state, "Ready");
  assert.equal(acknowledged.canvas?.revision, 2);

  const mismatched = await runDesktopCanvasMutation({ async execute() {
    return { ...readyCanvas(9, "updated"), operationId: "different-operation" };
  } }, mutating);
  assert.strictEqual(mismatched, mutating);

  let calls = 0;
  const conflicted = await runDesktopCanvasMutation({ async execute() {
    calls += 1;
    throw new DesktopCanvasTransportError("CANVAS_VERSION_CONFLICT", false, false);
  } }, mutating);
  assert.equal(conflicted.state, "Conflict");
  assert.equal(conflicted.canvas?.revision, 1);
  assert.equal(calls, 1);
});

test("Canvas controller preserves explicit recovery and blocks archived mutation", async () => {
  const loading = requestDesktopCanvasLoad(createDesktopCanvasSnapshot("workspace-1"), "canvas-1");
  const recovery = await loadDesktopCanvas({ async execute() {
    throw new DesktopCanvasTransportError("CANVAS_RECOVERY_REQUIRED", false, true);
  } }, loading);
  assert.equal(recovery.state, "RecoveryRequired");
  assert.equal(recovery.canvas, undefined);

  const archived = applyDesktopCanvasResult(loading, loading.generation, readyCanvas(4, "archived"));
  const blocked = requestDesktopCanvasMutation(archived, { kind: "auto_arrange" }, "operation-archived");
  assert.strictEqual(blocked, archived);
});

test("Canvas controller acknowledges rename then archive lifecycle and blocks later mutation", async () => {
  const loading = requestDesktopCanvasLoad(createDesktopCanvasSnapshot("workspace-1"), "canvas-1");
  const ready = applyDesktopCanvasResult(loading, loading.generation, readyCanvas(7, "updated"));
  const renaming = requestDesktopCanvasMutation(ready, { kind: "rename", title: "Knowledge Canvas" }, "operation-rename");
  const renamed = await runDesktopCanvasMutation({ async execute(request) {
    assert.equal(request.kind, "rename");
    assert.equal(request.expectedRevision, 7);
    return { ...readyCanvas(8, "updated"), title: "Knowledge Canvas", operationId: "operation-rename" };
  } }, renaming);
  assert.equal(renamed.state, "Ready");
  assert.equal(renamed.canvas?.title, "Knowledge Canvas");
  assert.equal(renamed.canvas?.revision, 8);

  const archiving = requestDesktopCanvasMutation(renamed, { kind: "archive" }, "operation-archive");
  const archived = await runDesktopCanvasMutation({ async execute(request) {
    assert.equal(request.kind, "archive");
    assert.equal(request.expectedRevision, 8);
    return { ...readyCanvas(9, "archived"), title: "Knowledge Canvas", operationId: "operation-archive" };
  } }, archiving);
  assert.equal(archived.state, "Ready");
  assert.equal(archived.canvas?.lifecycle, "archived");
  assert.equal(archived.canvas?.revision, 9);
  assert.strictEqual(
    requestDesktopCanvasMutation(archived, { kind: "add_text_node", nodeId: "blocked", text: "blocked", x: 0, y: 0, width: 320, height: 180 }, "operation-blocked"),
    archived,
  );
});

test("Canvas controller recovers with operation identity and returns to durable Ready state", async () => {
  const loading = requestDesktopCanvasLoad(createDesktopCanvasSnapshot("workspace-1"), "canvas-1");
  const recovery = await loadDesktopCanvas({ async execute() {
    throw new DesktopCanvasTransportError("CANVAS_RECOVERY_REQUIRED", false, true);
  } }, loading);
  const recovering = requestDesktopCanvasRecovery(recovery, "recover-1");
  assert.equal(recovering.state, "Recovering");

  const ready = await runDesktopCanvasRecovery({ async execute(request) {
    assert.equal(request.kind, "recover");
    return { ...readyCanvas(1), operationId: "recover-1" };
  } }, recovering);
  assert.equal(ready.state, "Ready");
  assert.equal(ready.canvas?.revision, 1);

  const failed = await runDesktopCanvasRecovery({ async execute() {
    throw new DesktopCanvasTransportError("CANVAS_RECOVERY_NO_VALID_REVISION", false, true);
  } }, recovering);
  assert.equal(failed.state, "RecoveryRequired");
});

test("Canvas controller limits selection and converts drag pixels through durable zoom", () => {
  const canvas = {
    ...readyCanvas(5, "updated"),
    viewport: { centerX: 0, centerY: 0, zoomPercent: 125 },
    nodes: [
      { nodeId: "node-1", targetKind: "text" as const, targetId: "One", x: 80, y: 80, width: 320, height: 180 },
      { nodeId: "node-2", targetKind: "text" as const, targetId: "Two", x: 440, y: 80, width: 320, height: 180 },
      { nodeId: "node-3", targetKind: "text" as const, targetId: "Three", x: 800, y: 80, width: 320, height: 180 },
    ],
  };
  const loading = requestDesktopCanvasLoad(createDesktopCanvasSnapshot("workspace-1"), "canvas-1");
  let ready = applyDesktopCanvasResult(loading, loading.generation, canvas);
  ready = selectDesktopCanvasNode(ready, "node-1");
  ready = selectDesktopCanvasNode(ready, "node-2");
  const unchanged = selectDesktopCanvasNode(ready, "node-3");
  assert.strictEqual(unchanged, ready);
  assert.deepEqual(ready.selectedNodeIds, ["node-1", "node-2"]);

  const dragging = beginDesktopCanvasDrag(ready, "node-1", 100, 100);
  const finished = finishDesktopCanvasDrag(dragging, "node-1", 150, 125);
  assert.equal(finished.draft?.kind, "update_node_geometry");
  assert.deepEqual(finished.draft, {
    kind: "update_node_geometry",
    nodeId: "node-1",
    x: 120,
    y: 100,
    width: 320,
    height: 180,
  });
  assert.equal(finished.snapshot.drag, undefined);
});

test("Canvas controller clears connected selection after durable acknowledgement", async () => {
  const canvas = {
    ...readyCanvas(),
    nodes: [
      { nodeId: "node-1", targetKind: "text" as const, targetId: "One", x: 0, y: 0, width: 320, height: 180 },
      { nodeId: "node-2", targetKind: "text" as const, targetId: "Two", x: 400, y: 0, width: 320, height: 180 },
    ],
  };
  const loading = requestDesktopCanvasLoad(createDesktopCanvasSnapshot("workspace-1"), "canvas-1");
  let ready = applyDesktopCanvasResult(loading, loading.generation, canvas);
  ready = selectDesktopCanvasNode(selectDesktopCanvasNode(ready, "node-1"), "node-2");
  const mutating = requestDesktopCanvasMutation(ready, {
    kind: "connect_edge",
    edgeId: "edge-1",
    sourceNodeId: "node-1",
    targetNodeId: "node-2",
  }, "operation-connect");
  const result = await runDesktopCanvasMutation({ async execute() {
    return { ...canvas, revision: 2, operationId: "operation-connect", edges: [{ edgeId: "edge-1", sourceNodeId: "node-1", targetNodeId: "node-2" }] };
  } }, mutating);
  assert.deepEqual(result.selectedNodeIds, []);
});

test("Canvas controller selects one edge and clears it after remove acknowledgement", async () => {
  const canvas = {
    ...readyCanvas(4, "updated"),
    nodes: [
      { nodeId: "node-1", targetKind: "text" as const, targetId: "One", x: 0, y: 0, width: 320, height: 180 },
      { nodeId: "node-2", targetKind: "text" as const, targetId: "Two", x: 400, y: 0, width: 320, height: 180 },
    ],
    edges: [{ edgeId: "edge-1", sourceNodeId: "node-1", targetNodeId: "node-2" }],
  };
  const loading = requestDesktopCanvasLoad(createDesktopCanvasSnapshot("workspace-1"), "canvas-1");
  const ready = selectDesktopCanvasEdge(
    applyDesktopCanvasResult(loading, loading.generation, canvas),
    "edge-1",
  );
  assert.equal(ready.selectedEdgeId, "edge-1");
  const mutating = requestDesktopCanvasMutation(ready, { kind: "remove_edge", edgeId: "edge-1" }, "operation-remove-edge");
  const removed = await runDesktopCanvasMutation({ async execute() {
    return { ...canvas, revision: 5, operationId: "operation-remove-edge", edges: [] };
  } }, mutating);
  assert.equal(removed.selectedEdgeId, undefined);
  assert.deepEqual(removed.canvas?.edges, []);
});

test("Canvas controller previews auto arrange without replacing durable state and applies from base revision", () => {
  const canvas = {
    ...readyCanvas(7, "updated"),
    nodes: [{ nodeId: "node-1", targetKind: "text" as const, targetId: "One", x: 900, y: 600, width: 320, height: 180 }],
  };
  const loading = requestDesktopCanvasLoad(createDesktopCanvasSnapshot("workspace-1"), "canvas-1");
  const ready = applyDesktopCanvasResult(loading, loading.generation, canvas);
  const previewing = requestDesktopCanvasArrangePreview(ready);
  assert.equal(previewing.state, "PreviewingArrange");

  const preview = applyDesktopCanvasArrangePreview(previewing, {
    ...canvas,
    nodes: [{ ...canvas.nodes[0], x: 80, y: 80 }],
  });
  assert.equal(preview.state, "ArrangePreview");
  assert.equal(preview.canvas?.nodes[0]?.x, 80);
  assert.equal(preview.arrangeBaseCanvas?.nodes[0]?.x, 900);

  const cancelled = cancelDesktopCanvasArrangePreview(preview);
  assert.equal(cancelled.state, "Ready");
  assert.equal(cancelled.canvas?.nodes[0]?.x, 900);

  const applying = requestDesktopCanvasArrangeApply(preview, "operation-arrange");
  assert.equal(applying.state, "Mutating");
  assert.equal(applying.pendingRequest?.kind, "auto_arrange");
  assert.equal(applying.pendingRequest?.expectedRevision, 7);
  assert.equal(applying.canvas?.nodes[0]?.x, 900);
});

test("Canvas controller coalesces zoom-aware resize into one bounded geometry draft", () => {
  const canvas = {
    ...readyCanvas(5, "updated"),
    viewport: { centerX: 0, centerY: 0, zoomPercent: 125 },
    nodes: [{ nodeId: "node-1", targetKind: "text" as const, targetId: "One", x: 80, y: 80, width: 320, height: 180 }],
  };
  const loading = requestDesktopCanvasLoad(createDesktopCanvasSnapshot("workspace-1"), "canvas-1");
  const ready = applyDesktopCanvasResult(loading, loading.generation, canvas);
  const resizing = beginDesktopCanvasResize(ready, "node-1", 100, 100);
  const finished = finishDesktopCanvasResize(resizing, "node-1", 200, 150);
  assert.deepEqual(finished.draft, {
    kind: "update_node_geometry",
    nodeId: "node-1",
    x: 80,
    y: 80,
    width: 400,
    height: 220,
  });
  assert.equal(finished.snapshot.resize, undefined);

  const maxed = finishDesktopCanvasResize(
    beginDesktopCanvasResize(ready, "node-1", 0, 0),
    "node-1",
    10_000,
    10_000,
  );
  assert.equal(maxed.draft?.width, 1_200);
  assert.equal(maxed.draft?.height, 900);
  const noOp = finishDesktopCanvasResize(beginDesktopCanvasResize(ready, "node-1", 1, 1), "node-1", 1, 1);
  assert.equal(noOp.draft, undefined);
  const archived = applyDesktopCanvasResult(loading, loading.generation, { ...canvas, lifecycle: "archived" });
  assert.strictEqual(beginDesktopCanvasResize(archived, "node-1", 0, 0), archived);
});

test("Canvas controller creates bounded viewport drafts only from ready durable state", () => {
  const loading = requestDesktopCanvasLoad(createDesktopCanvasSnapshot("workspace-1"), "canvas-1");
  const ready = applyDesktopCanvasResult(loading, loading.generation, {
    ...readyCanvas(5, "updated"),
    viewport: { centerX: 9_950, centerY: -9_950, zoomPercent: 100 },
  });
  assert.deepEqual(createDesktopCanvasViewportDraft(ready, { deltaX: 500, deltaY: -500 }), {
    kind: "update_viewport", centerX: 10_000, centerY: -10_000, zoomPercent: 100,
  });
  assert.deepEqual(createDesktopCanvasViewportDraft(ready, { zoomPercent: 999 }), {
    kind: "update_viewport", centerX: 9_950, centerY: -9_950, zoomPercent: 400,
  });
  assert.equal(createDesktopCanvasViewportDraft(ready, {}), undefined);
  const archived = applyDesktopCanvasResult(loading, loading.generation, readyCanvas(5, "archived"));
  assert.equal(createDesktopCanvasViewportDraft(archived, { deltaX: 100 }), undefined);
});
