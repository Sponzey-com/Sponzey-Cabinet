import assert from "node:assert/strict";
import test from "node:test";

import { createWebGraphViewModel } from "../src/index.ts";

test("web graph model maps API graph result without duplicating permission filtering", () => {
  const model = createWebGraphViewModel({
    centerDocumentId: "doc-center",
    status: "degraded",
    nodes: [
      { id: "doc-center", kind: "document" },
      { id: "visible-doc", kind: "document" },
      { id: "server-visible-hidden-name", kind: "document" },
    ],
    edges: [
      {
        id: "edge-visible",
        sourceId: "doc-center",
        targetId: "visible-doc",
        kind: "document_link",
      },
    ],
    stats: { candidateCount: 4, filteredCount: 1 },
  });

  assert.equal(model.centerNodeId, "doc-center");
  assert.equal(model.statusLabel, "Degraded");
  assert.equal(model.summary, "3 nodes, 1 edge, 1 filtered");
  assert.deepEqual(
    model.nodeRows.map((node) => node.id),
    ["doc-center", "visible-doc", "server-visible-hidden-name"],
  );
  assert.deepEqual(model.edgeRows[0], {
    id: "edge-visible",
    sourceId: "doc-center",
    targetId: "visible-doc",
    label: "Document Link",
  });
});
