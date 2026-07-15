import assert from "node:assert/strict";
import test from "node:test";

import {
  createDesktopCanvasSnapshot,
  applyDesktopCanvasResult,
  requestDesktopCanvasLoad,
  type DesktopCanvasSurfaceSnapshot,
} from "../src/desktop_canvas_controller.ts";
import { createDesktopCanvasMutationQueue } from "../src/desktop_canvas_mutation_queue.ts";
import {
  DesktopCanvasTransportError,
  type DesktopCanvasData,
  type DesktopCanvasMutationRequest,
} from "../src/tauri_canvas_transport.ts";

function readyCanvas(revision: number): DesktopCanvasData {
  return {
    canvasId: "canvas-1",
    title: "Product map",
    revision,
    lifecycle: revision === 1 ? "draft" : "updated",
    viewport: { centerX: 0, centerY: 0, zoomPercent: 100 },
    nodes: [],
    edges: [],
  };
}

function readySnapshot(): DesktopCanvasSurfaceSnapshot {
  const loading = requestDesktopCanvasLoad(createDesktopCanvasSnapshot("workspace-1"), "canvas-1");
  return applyDesktopCanvasResult(loading, loading.generation, readyCanvas(1));
}

test("Canvas mutation queue serializes drafts against each durable acknowledgement", async () => {
  let snapshot = readySnapshot();
  const requests: DesktopCanvasMutationRequest[] = [];
  let releaseFirst: (() => void) | undefined;
  const firstGate = new Promise<void>((resolve) => { releaseFirst = resolve; });
  const queue = createDesktopCanvasMutationQueue({
    client: {
      async execute(request) {
        if (!("operationId" in request)) throw new Error("mutation expected");
        requests.push(request);
        if (requests.length === 1) await firstGate;
        return { ...readyCanvas(request.expectedRevision + 1), operationId: request.operationId };
      },
    },
    operationIdSource: (() => { let value = 0; return () => `operation-${++value}`; })(),
    readSnapshot: () => snapshot,
    commitSnapshot: (next) => { snapshot = next; },
    capacity: 4,
  });

  assert.equal(queue.enqueue({ kind: "add_text_node", nodeId: "note-1", text: "one", x: 80, y: 80, width: 320, height: 180 }), "accepted");
  assert.equal(queue.state(), "Saving");
  assert.equal(queue.enqueue({ kind: "update_viewport", centerX: 100, centerY: 0, zoomPercent: 100 }), "accepted");
  assert.equal(queue.state(), "SaveQueued");
  releaseFirst?.();
  await queue.whenIdle();

  assert.deepEqual(requests.map((request) => request.expectedRevision), [1, 2]);
  assert.deepEqual(requests.map((request) => request.operationId), ["operation-1", "operation-2"]);
  assert.equal(snapshot.state, "Ready");
  assert.equal(snapshot.canvas?.revision, 3);
  assert.equal(queue.state(), "Idle");
});

test("Canvas mutation queue stops pending work after an explicit conflict", async () => {
  let snapshot = readySnapshot();
  let calls = 0;
  let releaseFirst: (() => void) | undefined;
  const firstGate = new Promise<void>((resolve) => { releaseFirst = resolve; });
  const queue = createDesktopCanvasMutationQueue({
    client: { async execute() { calls += 1; await firstGate; throw new DesktopCanvasTransportError("CANVAS_VERSION_CONFLICT", false, false); } },
    operationIdSource: () => `operation-${calls + 1}`,
    readSnapshot: () => snapshot,
    commitSnapshot: (next) => { snapshot = next; },
    capacity: 2,
  });
  queue.enqueue({ kind: "auto_arrange" });
  queue.enqueue({ kind: "update_viewport", centerX: 100, centerY: 0, zoomPercent: 100 });
  releaseFirst?.();
  await queue.whenIdle();

  assert.equal(calls, 1);
  assert.equal(snapshot.state, "Conflict");
  assert.equal(queue.pendingCount(), 0);
});

test("Canvas mutation queue bounds pending work and suppresses dispatch after dispose", async () => {
  let snapshot = readySnapshot();
  let calls = 0;
  let releaseFirst: (() => void) | undefined;
  const firstGate = new Promise<void>((resolve) => { releaseFirst = resolve; });
  const queue = createDesktopCanvasMutationQueue({
    client: { async execute(request) { calls += 1; await firstGate; return { ...readyCanvas(request.kind === "get" ? 1 : request.expectedRevision + 1), operationId: "operationId" in request ? request.operationId : undefined }; } },
    operationIdSource: () => `operation-${calls + 1}`,
    readSnapshot: () => snapshot,
    commitSnapshot: (next) => { snapshot = next; },
    capacity: 1,
  });
  assert.equal(queue.enqueue({ kind: "auto_arrange" }), "accepted");
  assert.equal(queue.enqueue({ kind: "update_viewport", centerX: 100, centerY: 0, zoomPercent: 100 }), "accepted");
  assert.equal(queue.enqueue({ kind: "update_viewport", centerX: 200, centerY: 0, zoomPercent: 100 }), "full");
  queue.dispose();
  releaseFirst?.();
  await queue.whenIdle();
  assert.equal(queue.state(), "Disposed");
  assert.equal(calls, 1);
  assert.equal(queue.pendingCount(), 0);
});
