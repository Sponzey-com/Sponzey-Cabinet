import assert from "node:assert/strict";
import { performance } from "node:perf_hooks";
import test from "node:test";

import React from "react";

import { createPersonalLocalDesktopCapabilityProfile } from "@sponzey-cabinet/client-core";
import { createPersonalWorkspaceHomeModelFromResult } from "@sponzey-cabinet/ui";
import { createDesktopCanvasElement } from "../src/react_exploration_surfaces.ts";

const model = createPersonalWorkspaceHomeModelFromResult(
  createPersonalLocalDesktopCapabilityProfile(),
  {
    workspaceId: "workspace-1",
    state: "Ready",
    healthStatus: "Healthy",
    backupStatus: "Fresh",
    recentDocuments: [],
    favorites: [],
    tags: [],
    recentChanges: [],
    unfinishedItems: [],
  },
);

const callbacks = {
  onHome() {}, onSearch() {}, onGraph() {}, onCanvas() {}, onAssets() {},
  onOpenDocument() {}, onOpenAsset() {}, onCanvasCreate() {}, onCanvasRetry() {}, onCanvasRecover() {},
  onCanvasAddNote() {}, onCanvasAutoArrange() {}, onCanvasApplyArrange() {}, onCanvasCancelArrange() {},
  onCanvasZoom() {}, onCanvasPan() {}, onCanvasRemoveNode() {}, onCanvasAddDocument() {},
  onCanvasAddAsset() {}, onCanvasConnect() {}, onCanvasRemoveEdge() {}, onCanvasNodeSelect() {},
  onCanvasEdgeSelect() {}, onCanvasDragStart() {}, onCanvasDragEnd() {}, onCanvasResizeStart() {},
  onCanvasResizeEnd() {}, canPlaceDocument: true, canPlaceAsset: true,
};

test("Canvas bounded React tree preparation p95 remains below the Phase 012 300ms budget", () => {
  const snapshot = fixture();
  for (let index = 0; index < 20; index += 1) prepareTree(snapshot);
  const samples: number[] = [];
  let lastTree: React.ReactElement | undefined;
  for (let index = 0; index < 200; index += 1) {
    const started = performance.now();
    lastTree = prepareTree(snapshot);
    samples.push(performance.now() - started);
  }
  samples.sort((left, right) => left - right);
  const p95 = samples[Math.ceil(samples.length * 0.95) - 1] ?? Number.POSITIVE_INFINITY;
  console.log(`canvas_bounded_render_preparation_nodes=2000 edges=4000 samples=200 p95_ms=${p95.toFixed(3)}`);
  assert.ok(lastTree);
  assert.ok(p95 < 300, `bounded React tree preparation p95=${p95.toFixed(3)}ms`);
});

function prepareTree(snapshot: ReturnType<typeof fixture>): React.ReactElement {
  const element = createDesktopCanvasElement(model, snapshot, callbacks);
  const component = element.type as (props: Record<string, unknown>) => React.ReactElement;
  return component(element.props as Record<string, unknown>);
}

function fixture() {
  const nodes = Array.from({ length: 2_000 }, (_, index) => ({
    nodeId: `node-${index}`,
    targetKind: "text" as const,
    targetId: `Memo ${index}`,
    x: (index % 50) * 360,
    y: Math.floor(index / 50) * 240,
    width: 320,
    height: 180,
  }));
  const edges = Array.from({ length: 4_000 }, (_, index) => ({
    edgeId: `edge-${index}`,
    sourceNodeId: `node-${index % 2_000}`,
    targetNodeId: `node-${(index + 1) % 2_000}`,
  }));
  return {
    state: "Ready" as const,
    workspaceId: "workspace-1",
    canvasId: "canvas-performance",
    generation: 1,
    selectedNodeIds: [],
    canvas: {
      canvasId: "canvas-performance",
      title: "Performance fixture",
      revision: 1,
      lifecycle: "updated" as const,
      viewport: { centerX: 600, centerY: 360, zoomPercent: 100 },
      nodes,
      edges,
    },
  };
}
