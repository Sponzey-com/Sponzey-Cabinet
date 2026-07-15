import assert from "node:assert/strict";
import test from "node:test";

import { createWebCanvasViewModel } from "../src/index.ts";

test("web canvas model maps API canvas view without raw UI state or permission rules", () => {
  const model = createWebCanvasViewModel({
    canvasId: "canvas-1",
    state: "saved",
    nodes: [
      { id: "doc-node", targetKind: "document", x: 0, y: 0 },
      { id: "asset-node", targetKind: "attachment", x: 240, y: 0 },
    ],
    edges: [{ id: "edge-1", sourceId: "doc-node", targetId: "asset-node" }],
    embedReference: "canvas:canvas-1",
  });

  assert.equal(model.canvasId, "canvas-1");
  assert.equal(model.statusLabel, "Saved");
  assert.equal(model.summary, "2 nodes, 1 edge");
  assert.equal(model.embedReference, "canvas:canvas-1");
  assert.deepEqual(model.nodeRows[0], {
    id: "doc-node",
    targetLabel: "Document",
    positionLabel: "0,0",
  });
  assert.doesNotMatch(JSON.stringify(model), /rawUiState|document body|card text/i);
});
