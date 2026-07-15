import assert from "node:assert/strict";
import test from "node:test";

import {
  createSelfHostApiClient,
  createSelfHostApiClientConfig,
  createKnowledgeGraphQuery,
  type CabinetHttpRequest,
  type CabinetHttpResponse,
  type CabinetHttpTransport,
} from "../src/index.ts";

test("self-host collaboration API client maps knowledge graph route without client-side filtering", async () => {
  const transport = new CapturingTransport([
    jsonResponse(200, {
      centerDocumentId: "doc-center",
      status: "clean",
      nodes: [
        { id: "doc-center", kind: "document" },
        { id: "visible-doc", kind: "document" },
      ],
      edges: [
        {
          id: "edge-visible",
          sourceId: "doc-center",
          targetId: "visible-doc",
          kind: "document_link",
        },
      ],
      stats: { candidateCount: 3, filteredCount: 1 },
    }),
  ]);
  const client = createSelfHostApiClient(
    createSelfHostApiClientConfig({ baseUrl: "https://cabinet.local" }),
    transport.handle,
  );

  const graph = await client.getKnowledgeGraph(
    createKnowledgeGraphQuery("workspace-1", "doc-center"),
  );

  assert.equal(graph.centerDocumentId, "doc-center");
  assert.equal(graph.status, "clean");
  assert.equal(graph.nodes.length, 2);
  assert.equal(graph.stats.filteredCount, 1);
  assert.deepEqual(
    transport.requests.map((request) => [request.method, request.url]),
    [
      [
        "GET",
        "https://cabinet.local/api/workspaces/workspace-1/documents/doc-center/graph",
      ],
    ],
  );
});

test("self-host collaboration API client maps accessible document, search, and sharing routes", async () => {
  const transport = new CapturingTransport([
    jsonResponse(200, {
      workspaceId: "workspace-1",
      documentId: "doc-1",
      title: "Source",
      path: "docs/source.md",
      body: "# Source",
      versionId: "version-1",
      permissionDecision: { result: "allowed", reasonCode: "ROLE_ALLOWED" },
    }),
    jsonResponse(200, {
      queryName: "search-documents",
      workspaceId: "workspace-1",
      text: "needle",
      results: [],
      permissionFilteredCount: 0,
      durationMs: 12,
    }),
    jsonResponse(200, {
      workspaceId: "workspace-1",
      documentId: "doc-1",
      entries: [
        {
          subject: { kind: "user", id: "user-1" },
          permission: "read",
          effect: "allow",
        },
      ],
      effectivePermissions: ["read", "write"],
    }),
    jsonResponse(200, {
      workspaceId: "workspace-1",
      documentId: "doc-1",
      entries: [
        {
          subject: { kind: "group", id: "group-1" },
          permission: "write",
          effect: "allow",
        },
      ],
      effectivePermissions: ["read", "write"],
    }),
  ]);
  const client = createSelfHostApiClient(
    createSelfHostApiClientConfig({ baseUrl: "https://cabinet.local" }),
    transport.handle,
  );

  const document = await client.getAccessibleDocument({
    workspaceId: "workspace-1",
    documentId: "doc-1",
  });
  const search = await client.searchAccessibleDocuments({
    workspaceId: "workspace-1",
    text: "needle",
    limit: 20,
  });
  const sharing = await client.getDocumentSharing({
    workspaceId: "workspace-1",
    documentId: "doc-1",
  });
  const updated = await client.updateDocumentSharing({
    workspaceId: "workspace-1",
    documentId: "doc-1",
    subject: { kind: "group", id: "group-1" },
    permission: "write",
    effect: "allow",
  });

  assert.equal(document.permissionDecision.result, "allowed");
  assert.equal(search.durationMs, 12);
  assert.equal(sharing.effectivePermissions[1], "write");
  assert.equal(updated.entries[0].subject.kind, "group");
  assert.deepEqual(
    transport.requests.map((request) => [request.method, request.url]),
    [
      ["GET", "https://cabinet.local/api/workspaces/workspace-1/documents/doc-1/current"],
      ["GET", "https://cabinet.local/api/workspaces/workspace-1/search?text=needle&limit=20"],
      ["GET", "https://cabinet.local/api/documents/doc-1/sharing?workspaceId=workspace-1"],
      ["PUT", "https://cabinet.local/api/documents/doc-1/sharing"],
    ],
  );
  assert.deepEqual(JSON.parse(transport.requests[3].body ?? "{}"), {
    workspaceId: "workspace-1",
    subject: { kind: "group", id: "group-1" },
    permission: "write",
    effect: "allow",
  });
});

test("self-host collaboration API client maps comment and inline comment routes", async () => {
  const thread = {
    threadId: "thread-1",
    documentId: "doc-1",
    state: "open",
    comments: [
      {
        commentId: "comment-1",
        authorUserId: "user-1",
        body: "Please check this section.",
        createdAt: "2026-06-26T00:00:00.000Z",
      },
    ],
  };
  const transport = new CapturingTransport([
    jsonResponse(200, { threads: [thread] }),
    jsonResponse(200, { thread }),
    jsonResponse(200, {
      thread: {
        ...thread,
        anchor: {
          versionId: "version-1",
          startOffset: 3,
          endOffset: 8,
          status: "valid",
        },
      },
      anchorStatus: "valid",
    }),
    jsonResponse(200, { thread: { ...thread, state: "resolved" } }),
    jsonResponse(200, { thread: { ...thread, state: "reopened" } }),
  ]);
  const client = createSelfHostApiClient(
    createSelfHostApiClientConfig({ baseUrl: "https://cabinet.local" }),
    transport.handle,
  );

  await client.listDocumentComments({ workspaceId: "workspace-1", documentId: "doc-1" });
  await client.addDocumentComment({
    workspaceId: "workspace-1",
    documentId: "doc-1",
    threadId: "thread-1",
    commentId: "comment-1",
    body: "Please check this section.",
  });
  const inline = await client.addInlineDocumentComment({
    workspaceId: "workspace-1",
    documentId: "doc-1",
    versionId: "version-1",
    startOffset: 3,
    endOffset: 8,
    threadId: "thread-1",
    commentId: "comment-2",
    body: "Inline note",
  });
  await client.resolveDocumentComment({
    workspaceId: "workspace-1",
    documentId: "doc-1",
    threadId: "thread-1",
  });
  await client.reopenDocumentComment({
    workspaceId: "workspace-1",
    documentId: "doc-1",
    threadId: "thread-1",
  });

  assert.equal(inline.anchorStatus, "valid");
  assert.deepEqual(
    transport.requests.map((request) => [request.method, request.url]),
    [
      ["GET", "https://cabinet.local/api/documents/doc-1/comments?workspaceId=workspace-1"],
      ["POST", "https://cabinet.local/api/documents/doc-1/comments"],
      ["POST", "https://cabinet.local/api/documents/doc-1/inline-comments"],
      ["POST", "https://cabinet.local/api/comments/thread-1/resolve"],
      ["POST", "https://cabinet.local/api/comments/thread-1/reopen"],
    ],
  );
});

test("self-host collaboration API client maps review, lock, and audit routes", async () => {
  const transport = new CapturingTransport([
    jsonResponse(200, { requests: [] }),
    jsonResponse(200, {
      documentId: "doc-1",
      reviewRequestId: "review-1",
      previousState: "editing",
      nextState: "review_requested",
    }),
    jsonResponse(200, {
      documentId: "doc-1",
      reviewRequestId: "review-1",
      previousState: "review_requested",
      nextState: "approved",
    }),
    jsonResponse(200, {
      documentId: "doc-1",
      previousState: "approved",
      nextState: "published",
    }),
    jsonResponse(200, {
      documentId: "doc-1",
      status: "unlocked",
    }),
    jsonResponse(200, {
      documentId: "doc-1",
      status: "locked",
      lockId: "lock-1",
      ownerUserId: "user-1",
      expiresAt: "2026-06-26T00:10:00.000Z",
    }),
    jsonResponse(200, {
      documentId: "doc-1",
      status: "unlocked",
    }),
    jsonResponse(200, {
      events: [
        {
          eventId: "audit-1",
          action: "review.requested",
          targetType: "document",
          targetId: "doc-1",
          actorId: "user-1",
          occurredAt: "2026-06-26T00:00:00.000Z",
          metadata: [],
        },
      ],
      nextCursor: "cursor-2",
    }),
  ]);
  const client = createSelfHostApiClient(
    createSelfHostApiClientConfig({ baseUrl: "https://cabinet.local" }),
    transport.handle,
  );

  await client.listReviewRequests({ workspaceId: "workspace-1", documentId: "doc-1" });
  await client.requestDocumentReview({
    workspaceId: "workspace-1",
    documentId: "doc-1",
    reviewRequestId: "review-1",
  });
  await client.approveDocumentReview({ workspaceId: "workspace-1", reviewRequestId: "review-1" });
  await client.publishDocument({ workspaceId: "workspace-1", documentId: "doc-1" });
  await client.getDocumentLock({ workspaceId: "workspace-1", documentId: "doc-1" });
  await client.lockDocument({ workspaceId: "workspace-1", documentId: "doc-1", lockId: "lock-1" });
  await client.unlockDocument({ workspaceId: "workspace-1", documentId: "doc-1" });
  const audit = await client.listAuditEvents({
    workspaceId: "workspace-1",
    scope: "workspace",
    limit: 50,
    cursor: "cursor-1",
  });

  assert.equal(audit.nextCursor, "cursor-2");
  assert.deepEqual(
    transport.requests.map((request) => [request.method, request.url]),
    [
      ["GET", "https://cabinet.local/api/review-requests?workspaceId=workspace-1&documentId=doc-1"],
      ["POST", "https://cabinet.local/api/documents/doc-1/review-requests"],
      ["POST", "https://cabinet.local/api/review-requests/review-1/approve"],
      ["POST", "https://cabinet.local/api/documents/doc-1/publish"],
      ["GET", "https://cabinet.local/api/documents/doc-1/locks/current?workspaceId=workspace-1"],
      ["POST", "https://cabinet.local/api/documents/doc-1/locks"],
      ["DELETE", "https://cabinet.local/api/documents/doc-1/locks/current?workspaceId=workspace-1"],
      ["GET", "https://cabinet.local/api/audit-events?workspaceId=workspace-1&scope=workspace&limit=50&cursor=cursor-1"],
    ],
  );
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
      throw new Error(`Unexpected request ${request.method} ${request.url}`);
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
