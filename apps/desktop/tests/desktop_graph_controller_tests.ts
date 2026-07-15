import assert from "node:assert/strict";
import test from "node:test";

import type { KnowledgeGraphView, LocalDesktopCommandClient } from "@sponzey-cabinet/client-core";
import {
  applyDesktopProjectionFreshness,
  applyDesktopGraphResult,
  createDesktopGraphSnapshot,
  loadDesktopKnowledgeGraph,
  loadDesktopGlobalKnowledgeGraph,
  repairDesktopKnowledgeGraph,
  requestDesktopGraphRepair,
  requestDesktopGraphLoad,
} from "../src/desktop_graph_controller.ts";

test("Graph controller loads explicit bounded query into Ready snapshot", async () => {
  let received: unknown;
  const client = {
    async getKnowledgeGraph(query: unknown) {
      received = query;
      return graph("clean");
    },
  } as Pick<LocalDesktopCommandClient, "getKnowledgeGraph">;
  const loading = requestDesktopGraphLoad(createDesktopGraphSnapshot("workspace-1"), {
    centerDocumentId: "doc-1",
    depth: 2,
    direction: "outgoing",
    includeUnresolved: false,
    includeAssets: true,
  });

  const ready = await loadDesktopKnowledgeGraph(client, loading);

  assert.equal(ready.state, "Ready");
  assert.equal(ready.graph?.nodes.length, 2);
  assert.deepEqual(received, {
    queryName: "get-knowledge-graph",
    workspaceId: "workspace-1",
    documentId: "doc-1",
    depth: 2,
    direction: "outgoing",
    includeUnresolved: false,
    includeAssets: true,
    nodeLimit: 120,
    edgeLimit: 240,
  });
});

test("Graph controller loads global scope without a center document or fake center", async () => {
  const loading = requestDesktopGraphLoad(createDesktopGraphSnapshot("workspace-1"), { scope: "global", centerDocumentId: undefined });
  assert.equal(loading.state, "Loading");
  const ready = await loadDesktopGlobalKnowledgeGraph({ async getGlobalGraph() { return { status: "clean", nodes: [{ id: "doc-1", kind: "document" }], edges: [], candidateCount: 1 }; } }, loading);
  assert.equal(ready.state, "Ready");
  assert.equal(ready.graph?.centerDocumentId, undefined);
  assert.equal(ready.selectedNodeId, "doc-1");
});

test("Graph controller forwards the bounded global cursor", async () => {
  let received: unknown;
  const loading = requestDesktopGraphLoad(createDesktopGraphSnapshot("workspace-1"), {
    scope: "global",
    centerDocumentId: undefined,
    globalCursor: "projection-50",
  });

  await loadDesktopGlobalKnowledgeGraph({
    async getGlobalGraph(query) {
      received = query;
      return { status: "clean", nodes: [], edges: [], candidateCount: 0 };
    },
  }, loading);

  assert.deepEqual(received, {
    workspaceId: "workspace-1",
    cursor: "projection-50",
    projectionLimit: 50,
    nodeLimit: 120,
    edgeLimit: 240,
  });
});

test("Graph controller distinguishes Empty, Stale, and safe Failed states", async () => {
  const base = requestDesktopGraphLoad(createDesktopGraphSnapshot("workspace-1"), {
    centerDocumentId: "doc-1",
  });
  const empty = applyDesktopGraphResult(base, base.generation, { ...graph("clean"), nodes: [], edges: [] });
  const stale = applyDesktopGraphResult(base, base.generation, graph("degraded"));
  const failed = await loadDesktopKnowledgeGraph({
    async getKnowledgeGraph() { throw new Error("/Users/private/raw"); },
  }, base);

  assert.equal(empty.state, "Empty");
  assert.equal(stale.state, "Stale");
  assert.equal(failed.state, "Failed");
  assert.equal(failed.errorCode, "COMMAND_BRIDGE_FAILED");
  assert.equal(JSON.stringify(failed).includes("/Users/private"), false);
});

test("Graph controller ignores stale generation completion", () => {
  const first = requestDesktopGraphLoad(createDesktopGraphSnapshot("workspace-1"), { centerDocumentId: "doc-1" });
  const second = requestDesktopGraphLoad(first, { depth: 2 });
  const unchanged = applyDesktopGraphResult(second, first.generation, graph("clean"));

  assert.strictEqual(unchanged, second);
  assert.equal(unchanged.state, "Loading");
});

test("Graph controller uses native freshness for Ready, Stale, Repairing, and Failed", () => {
  const loading = requestDesktopGraphLoad(createDesktopGraphSnapshot("workspace-1"), { centerDocumentId: "doc-1" });
  const graphReady = applyDesktopGraphResult(loading, loading.generation, graph("degraded"));
  const ready = applyDesktopProjectionFreshness(graphReady, graphReady.generation, freshness("ready"));
  const stale = applyDesktopProjectionFreshness(ready, ready.generation, freshness("stale"));
  const repairing = applyDesktopProjectionFreshness(stale, stale.generation, freshness("repairing"));
  const failed = applyDesktopProjectionFreshness(repairing, repairing.generation, freshness("failed"));

  assert.equal(ready.state, "Ready");
  assert.equal(stale.state, "Stale");
  assert.equal(repairing.state, "Repairing");
  assert.equal(failed.state, "Failed");
  assert.equal(failed.errorCode, "PROJECTION_REPAIR_FAILED");
});

test("Graph repair runs reindex, worker, freshness, and graph reload in order", async () => {
  const calls: string[] = [];
  const loaded = applyDesktopGraphResult(
    requestDesktopGraphLoad(createDesktopGraphSnapshot("workspace-1"), { centerDocumentId: "doc-1" }),
    1,
    graph("degraded"),
  );
  const repairing = requestDesktopGraphRepair(loaded);
  assert.equal(repairing.state, "Repairing");

  const result = await repairDesktopKnowledgeGraph({
    async startRepair() { calls.push("start"); return operation("repair-1", "queued"); },
    async runRepair(_workspace, operationId) { calls.push(`run:${operationId}`); return operation(operationId, "succeeded"); },
    async getRepairStatus(_workspace, operationId) { calls.push(`status:${operationId}`); return operation(operationId, "succeeded"); },
    async getFreshness() { calls.push("freshness"); return freshness("ready"); },
  }, {
    async getKnowledgeGraph() { calls.push("graph"); return graph("clean"); },
  }, repairing);

  assert.deepEqual(calls, ["start", "run:repair-1", "status:repair-1", "freshness", "graph"]);
  assert.equal(result.state, "Ready");
});

test("Graph repair maps safe retryable transport failure", async () => {
  const repairing = requestDesktopGraphRepair(applyDesktopGraphResult(
    requestDesktopGraphLoad(createDesktopGraphSnapshot("workspace-1"), { centerDocumentId: "doc-1" }),
    1,
    graph("degraded"),
  ));
  const result = await repairDesktopKnowledgeGraph({
    async startRepair() { throw Object.assign(new Error("private"), { code: "projection.reindex.unavailable", retryable: true }); },
    async runRepair() { throw new Error("not called"); },
    async getRepairStatus() { throw new Error("not called"); },
    async getFreshness() { throw new Error("not called"); },
  }, { async getKnowledgeGraph() { throw new Error("not called"); } }, repairing);

  assert.equal(result.state, "Failed");
  assert.equal(result.errorCode, "projection.reindex.unavailable");
  assert.equal(result.retryable, true);
  assert.equal(JSON.stringify(result).includes("private"), false);
});

function graph(status: KnowledgeGraphView["status"]): KnowledgeGraphView {
  return {
    centerDocumentId: "doc-1",
    status,
    nodes: [
      { id: "doc-1", kind: "document" },
      { id: "doc-2", kind: "document" },
    ],
    edges: [{ id: "edge-1", sourceId: "doc-1", targetId: "doc-2", kind: "document_link" }],
    stats: { candidateCount: 2, filteredCount: 0 },
    freshnessRevision: "version-1",
  };
}

function operation(operationId: string, state: "queued" | "succeeded") {
  return { operationId, state, attempt: state === "queued" ? 0 : 1, completedUnits: state === "queued" ? 0 : 3, totalUnits: 3 } as const;
}

function freshness(state: "ready" | "stale" | "repairing" | "failed") {
  return {
    state,
    currentVersionId: "version-1",
    projections: [{ kind: "Graph", state }],
  } as const;
}
