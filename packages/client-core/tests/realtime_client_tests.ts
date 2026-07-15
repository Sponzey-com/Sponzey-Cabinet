import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  CabinetRealtimeClientError,
  createCollaborationRealtimeClient,
  type CollaborationRealtimeRequest,
  type CollaborationRealtimeResponse,
} from "../src/index.ts";

test("collaboration realtime client sends join operation presence and replay as plain DTOs", async () => {
  const transport = new CapturingRealtimeTransport([
    accepted("workspace-1", "doc-1"),
    accepted("workspace-1", "doc-1"),
    accepted("workspace-1", "doc-1"),
    accepted("workspace-1", "doc-1"),
  ]);
  const client = createCollaborationRealtimeClient(transport.handle);

  await client.joinDocumentRoom({
    workspaceId: "workspace-1",
    documentId: "doc-1",
    sessionId: "session-1",
    actorUserId: "user-1",
  });
  await client.broadcastOperation({
    workspaceId: "workspace-1",
    documentId: "doc-1",
    sessionId: "session-1",
    actorUserId: "user-1",
    operationId: "op-1",
    baseRevision: 3,
    currentRevision: 3,
    startOffset: 4,
    endOffset: 7,
    insertedText: "next",
  });
  await client.broadcastPresence({
    workspaceId: "workspace-1",
    documentId: "doc-1",
    sessionId: "session-1",
    actorUserId: "user-1",
    cursorStart: 5,
    cursorEnd: 5,
  });
  await client.requestReplay({
    workspaceId: "workspace-1",
    documentId: "doc-1",
    sessionId: "session-1",
    lastAcknowledgedSequence: 2,
  });

  assert.deepEqual(transport.requests, [
    {
      eventName: "join-document-room",
      workspaceId: "workspace-1",
      documentId: "doc-1",
      sessionId: "session-1",
      actorUserId: "user-1",
    },
    {
      eventName: "broadcast-operation",
      workspaceId: "workspace-1",
      documentId: "doc-1",
      sessionId: "session-1",
      actorUserId: "user-1",
      operationId: "op-1",
      baseRevision: 3,
      currentRevision: 3,
      startOffset: 4,
      endOffset: 7,
      insertedText: "next",
    },
    {
      eventName: "broadcast-presence",
      workspaceId: "workspace-1",
      documentId: "doc-1",
      sessionId: "session-1",
      actorUserId: "user-1",
      cursorStart: 5,
      cursorEnd: 5,
    },
    {
      eventName: "request-replay",
      workspaceId: "workspace-1",
      documentId: "doc-1",
      sessionId: "session-1",
      lastAcknowledgedSequence: 2,
    },
  ]);
});

test("collaboration realtime client strips sensitive presence draft fields", async () => {
  const transport = new CapturingRealtimeTransport([accepted("workspace-1", "doc-1")]);
  const client = createCollaborationRealtimeClient(transport.handle);

  await client.broadcastPresence({
    workspaceId: "workspace-1",
    documentId: "doc-1",
    sessionId: "session-1",
    actorUserId: "user-1",
    cursorStart: 5,
    cursorEnd: 8,
    selectedText: "selected text must not leave client-core",
    documentBody: "document body must not leave client-core",
    token: "token must not leave client-core",
  });

  assert.deepEqual(transport.requests[0], {
    eventName: "broadcast-presence",
    workspaceId: "workspace-1",
    documentId: "doc-1",
    sessionId: "session-1",
    actorUserId: "user-1",
    cursorStart: 5,
    cursorEnd: 8,
  });
  assert.equal(Object.keys(transport.requests[0]).includes("selectedText"), false);
  assert.equal(Object.keys(transport.requests[0]).includes("documentBody"), false);
  assert.equal(Object.keys(transport.requests[0]).includes("token"), false);
});

test("collaboration realtime client maps rejected acknowledgement to stable error", async () => {
  const transport = new CapturingRealtimeTransport([
    {
      status: "rejected",
      workspaceId: "workspace-1",
      documentId: "doc-1",
      errorCode: "realtime_transport.room_not_joined",
    },
  ]);
  const client = createCollaborationRealtimeClient(transport.handle);

  await assert.rejects(
    () =>
      client.requestReplay({
        workspaceId: "workspace-1",
        documentId: "doc-1",
        sessionId: "session-1",
      }),
    (error) =>
      error instanceof CabinetRealtimeClientError &&
      error.code === "realtime_transport.room_not_joined",
  );
});

test("collaboration realtime client does not import runtime transport or editor types", async () => {
  const source = await readFile(new URL("../src/index.ts", import.meta.url), "utf8");

  assert.doesNotMatch(source, /WebSocket|@codemirror|CodeMirror|Tauri/);
});

class CapturingRealtimeTransport {
  readonly requests: CollaborationRealtimeRequest[] = [];
  private readonly responses: CollaborationRealtimeResponse[];

  constructor(responses: CollaborationRealtimeResponse[]) {
    this.responses = [...responses];
  }

  readonly handle = async (
    request: CollaborationRealtimeRequest,
  ): Promise<CollaborationRealtimeResponse> => {
    this.requests.push(request);
    const response = this.responses.shift();
    assert.ok(response, "missing realtime response fixture");
    return response;
  };
}

function accepted(workspaceId: string, documentId: string): CollaborationRealtimeResponse {
  return {
    status: "accepted",
    workspaceId,
    documentId,
  };
}
