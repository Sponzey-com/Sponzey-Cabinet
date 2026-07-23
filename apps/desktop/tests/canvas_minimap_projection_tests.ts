import assert from "node:assert/strict";
import test from "node:test";

import { projectCanvasMinimap } from "../src/canvas_minimap_projection.ts";

const nodes = [
  { nodeId: "node-a", targetKind: "text" as const, targetId: "a", displayLabel: "A", targetStatus: "available" as const, x: -200, y: 100, width: 200, height: 100 },
  { nodeId: "node-b", targetKind: "text" as const, targetId: "b", displayLabel: "B", targetStatus: "available" as const, x: 800, y: 500, width: 300, height: 180 },
];

test("minimap projects actual node and visible viewport bounds inside its frame", () => {
  const projection = projectCanvasMinimap(nodes, { centerX: 600, centerY: 360, zoomPercent: 100 });

  assert.equal(projection.nodes.length, 2);
  for (const rectangle of [...projection.nodes, projection.viewport]) {
    assert.ok(rectangle.left >= 0 && rectangle.top >= 0);
    assert.ok(rectangle.left + rectangle.width <= 120);
    assert.ok(rectangle.top + rectangle.height <= 72);
  }
  assert.notDeepEqual(projection.nodes[0], projection.nodes[1]);
});

test("minimap viewport responds deterministically to pan and zoom", () => {
  const base = projectCanvasMinimap(nodes, { centerX: 300, centerY: 300, zoomPercent: 100 });
  const panned = projectCanvasMinimap(nodes, { centerX: 700, centerY: 300, zoomPercent: 100 });
  const zoomed = projectCanvasMinimap(nodes, { centerX: 300, centerY: 300, zoomPercent: 200 });

  assert.notEqual(base.viewport.left, panned.viewport.left);
  assert.ok(zoomed.viewport.width < base.viewport.width);
  assert.ok(zoomed.viewport.height < base.viewport.height);
});

test("empty canvas contains only a meaningful viewport and no decorative nodes", () => {
  const projection = projectCanvasMinimap([], { centerX: 0, centerY: 0, zoomPercent: 100 });

  assert.deepEqual(projection.nodes, []);
  assert.ok(projection.viewport.width > 0);
  assert.ok(projection.viewport.height > 0);
});

test("minimap omits invalid node geometry instead of producing non-finite styles", () => {
  const projection = projectCanvasMinimap([
    ...nodes,
    { ...nodes[0], nodeId: "invalid", x: Number.NaN },
  ], { centerX: 0, centerY: 0, zoomPercent: 100 });

  assert.equal(projection.nodes.length, 2);
  assert.ok([...projection.nodes, projection.viewport].every((rectangle) =>
    Object.values(rectangle).every(Number.isFinite)));
});
