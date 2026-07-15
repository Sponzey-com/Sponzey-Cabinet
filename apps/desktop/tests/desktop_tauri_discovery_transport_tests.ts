import assert from "node:assert/strict";
import test from "node:test";

import { createTauriDiscoveryTransport } from "../src/tauri_discovery_transport.ts";

test("Tauri discovery transport maps bounded Graph query to native snake case request", async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const transport = createTauriDiscoveryTransport(async (command, args) => {
    calls.push({ command, args });
    return graphResponse();
  });

  const response = await transport(graphEnvelope());

  assert.deepEqual(calls, [{
    command: "get_desktop_knowledge_graph",
    args: {
      request: {
        command_name: "get_graph_projection",
        payload: {
          kind: "graph_projection",
          workspace_id: "workspace-1",
          document_id: "doc-1",
          depth: 2,
          direction: "both",
          include_unresolved: true,
          include_assets: false,
          node_limit: 120,
          edge_limit: 240,
        },
      },
    },
  }]);
  assert.equal(response.ok, true);
});

test("Tauri discovery transport rejects invalid query before native invocation", async () => {
  let invocationCount = 0;
  const transport = createTauriDiscoveryTransport(async () => {
    invocationCount += 1;
    return graphResponse();
  });

  const response = await transport({
    commandName: "get_graph_projection",
    payload: { ...graphEnvelope().payload, depth: 3 },
  });

  assert.equal(invocationCount, 0);
  assert.deepEqual(response, {
    ok: false,
    errorCode: "COMMAND_BRIDGE_FAILED",
    retryable: false,
  });
});

test("Tauri discovery transport fails safely for throw, malformed response, and unsupported command", async () => {
  const throwing = createTauriDiscoveryTransport(async () => {
    throw new Error("/Users/private/raw graph response");
  });
  const malformed = createTauriDiscoveryTransport(async () => ({ ok: true, data: { nodes: [] } }));

  const results = [
    await throwing(graphEnvelope()),
    await malformed(graphEnvelope()),
    await malformed({ commandName: "get_current_document", payload: {} }),
  ];

  for (const response of results) {
    assert.deepEqual(response, {
      ok: false,
      errorCode: "COMMAND_BRIDGE_FAILED",
      retryable: false,
    });
    assert.equal(JSON.stringify(response).includes("/Users/private"), false);
  }
});

test("Tauri discovery transport maps document asset metadata query and validates response", async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const transport = createTauriDiscoveryTransport(async (command, args) => {
    calls.push({ command, args });
    return {
      ok: true,
      data: {
        queryName: "list-document-assets",
        workspaceId: "workspace-1",
        documentId: "doc-1",
        assets: [{ assetId: "a".repeat(64), label: "Spec", fileName: "spec.pdf", mediaType: "application/pdf", byteSize: 42, status: "metadata_only" }],
      },
    };
  });

  const response = await transport({
    commandName: "list_document_assets",
    payload: { queryName: "list-document-assets", workspaceId: "workspace-1", documentId: "doc-1" },
  });

  assert.equal(response.ok, true);
  assert.deepEqual(calls, [{
    command: "get_desktop_document_assets",
    args: { request: { command_name: "list_document_assets", payload: { kind: "document_identity", workspace_id: "workspace-1", document_id: "doc-1" } } },
  }]);
});

function graphEnvelope() {
  return {
    commandName: "get_graph_projection" as const,
    payload: {
      queryName: "get-knowledge-graph",
      workspaceId: "workspace-1",
      documentId: "doc-1",
      depth: 2,
      direction: "both",
      includeUnresolved: true,
      includeAssets: false,
      nodeLimit: 120,
      edgeLimit: 240,
    },
  };
}

function graphResponse() {
  return {
    ok: true,
    data: {
      centerDocumentId: "doc-1",
      status: "clean",
      nodes: [{ id: "doc-1", kind: "document" }],
      edges: [],
      stats: { candidateCount: 1, filteredCount: 0 },
      freshnessRevision: "version-2",
    },
  };
}
