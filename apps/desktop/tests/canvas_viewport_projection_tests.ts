import assert from "node:assert/strict";
import { performance } from "node:perf_hooks";
import test from "node:test";

import { createCanvasWorldTransform, projectDesktopCanvasViewport } from "../src/canvas_viewport_projection.ts";

test("canvas world transform centers the durable viewport in the current responsive stage", () => {
  assert.equal(createCanvasWorldTransform({ centerX: 600, centerY: 360, zoomPercent: 100 }), "translate(50%, 50%) scale(1) translate(-600px, -360px)");
  assert.equal(createCanvasWorldTransform({ centerX: -20, centerY: 40, zoomPercent: 125 }), "translate(50%, 50%) scale(1.25) translate(20px, -40px)");
});
import type { DesktopCanvasData } from "../src/tauri_canvas_transport.ts";

test("Canvas viewport projection filters geometry, edges and reports truncation", () => {
  const canvas = fixture(2_000, 4_000, true);
  const projection = projectDesktopCanvasViewport(canvas, { width: 1_200, height: 720, overscan: 120, nodeLimit: 250, edgeLimit: 500 });

  assert.equal(projection.totalNodeCount, 2_000);
  assert.equal(projection.totalEdgeCount, 4_000);
  assert.equal(projection.matchingNodeCount, 2_000);
  assert.equal(projection.matchingEdgeCount, 498);
  assert.equal(projection.nodes.length, 250);
  assert.equal(projection.edges.length, 498);
  const visible = new Set(projection.nodes.map((node) => node.nodeId));
  assert.ok(projection.edges.every((edge) => visible.has(edge.sourceNodeId) && visible.has(edge.targetNodeId)));
  assert.equal(projection.truncated, true);
});

test("Canvas viewport projection p95 remains below 300ms for 2,000 nodes and 4,000 edges", () => {
  const canvas = fixture(2_000, 4_000, false);
  for (let index = 0; index < 20; index += 1) projectDesktopCanvasViewport(canvas, options);
  const samples: number[] = [];
  for (let index = 0; index < 200; index += 1) {
    const started = performance.now();
    projectDesktopCanvasViewport(canvas, options);
    samples.push(performance.now() - started);
  }
  samples.sort((left, right) => left - right);
  const p95 = samples[Math.ceil(samples.length * 0.95) - 1] ?? Number.POSITIVE_INFINITY;
  console.log(`canvas_viewport_projection_nodes=2000 edges=4000 samples=200 p95_ms=${p95.toFixed(3)}`);
  assert.ok(p95 < 300, `viewport projection p95=${p95.toFixed(3)}ms`);
});

const options = { width: 1_200, height: 720, overscan: 120, nodeLimit: 250, edgeLimit: 500 } as const;

function fixture(nodeCount: number, edgeCount: number, overlap: boolean): DesktopCanvasData {
  const nodes = Array.from({ length: nodeCount }, (_, index) => ({
    nodeId: `node-${index}`,
    targetKind: "text" as const,
    targetId: `Memo ${index}`,
    x: overlap ? 0 : (index % 50) * 360,
    y: overlap ? 0 : Math.floor(index / 50) * 240,
    width: 320,
    height: 180,
  }));
  const edges = Array.from({ length: edgeCount }, (_, index) => ({
    edgeId: `edge-${index}`,
    sourceNodeId: `node-${index % nodeCount}`,
    targetNodeId: `node-${(index + 1) % nodeCount}`,
  }));
  return {
    canvasId: "canvas-performance",
    title: "Performance fixture",
    revision: 1,
    lifecycle: "updated",
    viewport: { centerX: 600, centerY: 360, zoomPercent: 100 },
    nodes,
    edges,
  };
}
