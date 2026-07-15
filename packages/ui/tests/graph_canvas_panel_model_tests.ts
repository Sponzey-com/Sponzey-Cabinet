import assert from "node:assert/strict";
import test from "node:test";

import type { CanvasView, KnowledgeGraphView } from "../../client-core/src/index.ts";
import {
  createCanvasViewportPanelModel,
  createGraphPanelViewModel,
} from "../src/index.ts";

test("graph panel uses neighborhood mode with depth limit and no full workspace scan", () => {
  const graph = createGraphPanelViewModel(knowledgeGraph("clean"), {
    depthLimit: 2,
    pageSize: 50,
  });
  const serialized = JSON.stringify(graph);

  assert.equal(graph.mode, "graph");
  assert.equal(graph.loadMode, "neighborhood");
  assert.equal(graph.fullWorkspaceScan, false);
  assert.equal(graph.depthLimit, 2);
  assert.equal(graph.pageSize, 50);
  assert.equal(graph.nodeCount, 3);
  assert.equal(graph.edgeCount, 2);
  assert.equal(graph.state, "Ready");
  assert.equal(serialized.includes("full-workspace-scan"), false);
});

test("graph panel exposes reindex action for degraded graph without raw scan fallback", () => {
  const graph = createGraphPanelViewModel(knowledgeGraph("degraded"), {
    depthLimit: 1,
    pageSize: 25,
  });

  assert.equal(graph.state, "Degraded");
  assert.deepEqual(graph.actions.map((action) => action.id), ["rebuild-index"]);
  assert.equal(JSON.stringify(graph).includes("raw-scan"), false);
});

test("canvas viewport model includes only visible nodes and safe edge metadata", () => {
  const canvas = createCanvasViewportPanelModel(canvasView("saved"), {
    viewport: { x: 0, y: 0, width: 200, height: 200 },
    pageSize: 25,
  });
  const serialized = JSON.stringify(canvas);

  assert.equal(canvas.mode, "canvas");
  assert.equal(canvas.loadState, "ViewportReady");
  assert.equal(canvas.viewportNodeCount, 2);
  assert.deepEqual(
    canvas.visibleNodes.map((node) => node.id),
    ["node-1", "node-2"],
  );
  assert.deepEqual(
    canvas.visibleEdges.map((edge) => edge.id),
    ["edge-1"],
  );
  assert.equal(serialized.includes("card text should not leak"), false);
  assert.equal(serialized.includes("canvas_raw_ui_state"), false);
});

test("archived canvas is represented as view only", () => {
  const canvas = createCanvasViewportPanelModel(canvasView("archived"), {
    viewport: { x: 0, y: 0, width: 200, height: 200 },
    pageSize: 25,
  });

  assert.equal(canvas.loadState, "ViewportReady");
  assert.equal(canvas.viewOnly, true);
  assert.deepEqual(canvas.actions, []);
});

function knowledgeGraph(status: KnowledgeGraphView["status"]): KnowledgeGraphView {
  return {
    centerDocumentId: "doc-1",
    status,
    nodes: [
      { id: "doc-1", kind: "document" },
      { id: "doc-2", kind: "document" },
      { id: "asset-1", kind: "attachment" },
    ],
    edges: [
      { id: "edge-1", sourceId: "doc-1", targetId: "doc-2", kind: "document_link" },
      { id: "edge-2", sourceId: "doc-1", targetId: "asset-1", kind: "attachment_reference" },
    ],
    stats: {
      candidateCount: 3,
      filteredCount: 3,
    },
    performance: {
      targetMs: 300,
      observedMs: 12,
    },
  };
}

function canvasView(state: CanvasView["state"]): CanvasView {
  return {
    canvasId: "canvas-1",
    state,
    nodes: [
      { id: "node-1", targetKind: "document", x: 10, y: 10 },
      { id: "node-2", targetKind: "attachment", x: 150, y: 150 },
      { id: "node-3", targetKind: "text_card", x: 450, y: 450, text: "card text should not leak" } as never,
    ],
    edges: [
      { id: "edge-1", sourceId: "node-1", targetId: "node-2" },
      { id: "edge-2", sourceId: "node-2", targetId: "node-3" },
    ],
    embedReference: "canvas:canvas-1",
  };
}
