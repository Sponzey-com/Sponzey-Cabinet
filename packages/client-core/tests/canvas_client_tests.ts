import assert from "node:assert/strict";
import test from "node:test";

import {
  createSelfHostApiClient,
  createSelfHostApiClientConfig,
  type CanvasApiClient,
  type CabinetHttpRequest,
  type CabinetHttpResponse,
  type CabinetHttpTransport,
} from "../src/index.ts";

test("self-host canvas API client maps create node and embed routes without raw UI state", async () => {
  const transport = new CapturingTransport([
    jsonResponse(200, {
      canvasId: "canvas-1",
      state: "draft",
      nodeCount: 0,
      edgeCount: 0,
      productLogEvent: "canvas.created",
    }),
    jsonResponse(200, {
      canvasId: "canvas-1",
      state: "updated",
      nodeCount: 1,
      edgeCount: 0,
      productLogEvent: "canvas.node.added",
    }),
    jsonResponse(200, {
      reference: "canvas:canvas-1",
      productLogEvent: "canvas.embedded",
    }),
  ]);
  const client = createSelfHostApiClient(
    createSelfHostApiClientConfig({ baseUrl: "https://cabinet.local" }),
    transport.handle,
  );
  const canvasClient: CanvasApiClient = client;

  const created = await canvasClient.createCanvas({
    workspaceId: "workspace-1",
    canvasId: "canvas-1",
  });
  const nodeAdded = await canvasClient.addCanvasNode({
    workspaceId: "workspace-1",
    canvasId: "canvas-1",
    nodeId: "doc-node",
    target: { kind: "document", documentId: "doc-1" },
    x: 0,
    y: 0,
  });
  const embed = await canvasClient.embedCanvas({
    workspaceId: "workspace-1",
    documentId: "doc-1",
    canvasId: "canvas-1",
  });

  assert.equal(created.productLogEvent, "canvas.created");
  assert.equal(nodeAdded.nodeCount, 1);
  assert.equal(embed.reference, "canvas:canvas-1");
  assert.deepEqual(
    transport.requests.map((request) => [request.method, request.url]),
    [
      ["POST", "https://cabinet.local/api/workspaces/workspace-1/canvases"],
      ["POST", "https://cabinet.local/api/workspaces/workspace-1/canvases/canvas-1/nodes"],
      [
        "POST",
        "https://cabinet.local/api/workspaces/workspace-1/documents/doc-1/canvas-embeds",
      ],
    ],
  );
  assert.doesNotMatch(transport.requests.map((request) => request.body ?? "").join("\n"), /rawUiState/);
});

class CapturingTransport {
  readonly requests: CabinetHttpRequest[] = [];
  private responses: CabinetHttpResponse[];

  constructor(responses: CabinetHttpResponse[]) {
    this.responses = [...responses];
  }

  readonly handle: CabinetHttpTransport = async (request) => {
    this.requests.push(request);
    const response = this.responses.shift();
    if (!response) {
      throw new Error("unexpected request");
    }
    return response;
  };
}

function jsonResponse(status: number, body: unknown): CabinetHttpResponse {
  return {
    status,
    body: JSON.stringify(body),
    headers: { "content-type": "application/json" },
  };
}
