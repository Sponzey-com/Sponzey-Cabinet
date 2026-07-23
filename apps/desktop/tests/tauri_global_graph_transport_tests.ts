import assert from "node:assert/strict"; import test from "node:test";
import { createTauriGlobalGraphTransport } from "../src/tauri_global_graph_transport.ts";
test("global graph transport maps bounded request without fake center", async () => { let received: unknown; const client = createTauriGlobalGraphTransport(async (command, args) => { received = { command, args }; return { ok: true, data: { status: "clean", nodes: [{ id: "doc-1", kind: "document", label: "문서 1", breadcrumbLabel: "설계", availability: "available", canNavigate: true }], edges: [], candidateCount: 1, nextCursor: null }, retryable: false }; }); const result = await client.getGlobalGraph({ workspaceId: "workspace-1", includeUnresolved: true, includeAssets: false, projectionLimit: 50, nodeLimit: 120, edgeLimit: 240 }); assert.equal(result.nodes.length, 1); assert.equal(result.nodes[0]?.label, "문서 1"); assert.equal("centerDocumentId" in result, false); assert.deepEqual(received, { command: "get_desktop_global_knowledge_graph", args: { request: { workspaceId: "workspace-1", includeUnresolved: true, includeAssets: false, projectionLimit: 50, nodeLimit: 120, edgeLimit: 240 } } }); });

test("global graph transport forwards an explicit continuation cursor", async () => {
  let received: unknown;
  const client = createTauriGlobalGraphTransport(async (_command, args) => {
    received = args;
    return { ok: true, data: { status: "clean", nodes: [], edges: [], candidateCount: 0, nextCursor: null } };
  });

  await client.getGlobalGraph({
    workspaceId: "workspace-1",
    cursor: "projection-50",
    includeUnresolved: false,
    includeAssets: true,
    projectionLimit: 50,
    nodeLimit: 120,
    edgeLimit: 240,
  });

  assert.deepEqual(received, {
    request: {
      workspaceId: "workspace-1",
      cursor: "projection-50",
      includeUnresolved: false,
      includeAssets: true,
      projectionLimit: 50,
      nodeLimit: 120,
      edgeLimit: 240,
    },
  });
});

test("global graph transport rejects graph nodes without safe display fields", async () => {
  const client = createTauriGlobalGraphTransport(async () => ({
    ok: true,
    data: { status: "clean", nodes: [{ id: "private-id", kind: "document" }], edges: [], candidateCount: 1 },
  }));

  await assert.rejects(
    client.getGlobalGraph({ workspaceId: "workspace-1", includeUnresolved: true, includeAssets: false, projectionLimit: 50, nodeLimit: 120, edgeLimit: 240 }),
    (error: unknown) => error instanceof Error && error.message === "COMMAND_BRIDGE_FAILED",
  );
});
