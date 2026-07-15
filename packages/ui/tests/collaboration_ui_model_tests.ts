import assert from "node:assert/strict";
import test from "node:test";

import type {
  AddDocumentCommentCommand,
  AddInlineDocumentCommentCommand,
  AuditEventPageView,
  CabinetCollaborationApiClient,
  CollaborationSearchQuery,
  CommentThreadMutationView,
  CommentThreadPageView,
  DocumentLockCommand,
  DocumentLockView,
  DocumentSharingView,
  GetAccessibleDocumentQuery,
  GetDocumentLockQuery,
  GetDocumentSharingQuery,
  ListAuditEventsQuery,
  ListDocumentCommentsQuery,
  ListReviewRequestsQuery,
  PublishDocumentCommand,
  ReviewDecisionCommand,
  ResolveDocumentCommentCommand,
  ReviewRequestCommand,
  ReviewRequestPageView,
  ReviewWorkflowActionView,
  SearchAccessibleDocumentsView,
  UpdateDocumentSharingCommand,
} from "../../client-core/src/index.ts";
import {
  addCollaborationComment,
  addCollaborationInlineComment,
  createInitialCollaborationViewModel,
  loadCollaborationDocumentViewModel,
  publishCollaborationDocument,
  requestCollaborationReview,
  resolveCollaborationComment,
  searchCollaborationDocuments,
  updateCollaborationSharing,
  type CollaborationDevelopmentLogger,
  type EditorInlineAnchorAdapter,
} from "../src/index.ts";

test("collaboration sharing panel loads server output and delegates sharing changes to API client", async () => {
  const client = new FakeCollaborationApiClient();
  const logger = new CapturingDevelopmentLogger();
  const initial = createInitialCollaborationViewModel({
    workspaceId: "workspace-1",
    documentId: "doc-1",
  });

  const loaded = await loadCollaborationDocumentViewModel(initial, client, logger);
  const searched = await searchCollaborationDocuments(loaded, "needle", client, logger);
  const updated = await updateCollaborationSharing(
    searched,
    {
      subject: { kind: "group", id: "group-1" },
      permission: "write",
      effect: "allow",
    },
    client,
    logger,
  );

  assert.equal(loaded.displayState, "Loaded");
  assert.equal(loaded.currentDocument?.permissionDecision.result, "allowed");
  assert.equal(searched.search?.durationMs, 18);
  assert.equal(updated.sharing?.entries[0].subject.kind, "group");
  assert.equal("permissionRules" in updated, false);
  assert.deepEqual(client.calls.slice(0, 7), [
    "getAccessibleDocument",
    "getDocumentSharing",
    "listDocumentComments",
    "listReviewRequests",
    "getDocumentLock",
    "listAuditEvents",
    "searchAccessibleDocuments",
  ]);
  assert.deepEqual(logger.events, [
    "collaboration.load",
    "collaboration.search",
    "collaboration.sharing.update",
  ]);
});

test("collaboration comment panel uses fake editor anchor adapter and displays stale anchor result", async () => {
  const client = new FakeCollaborationApiClient();
  const anchorAdapter = new FakeEditorInlineAnchorAdapter("stale");
  const loaded = await loadCollaborationDocumentViewModel(
    createInitialCollaborationViewModel({ workspaceId: "workspace-1", documentId: "doc-1" }),
    client,
  );

  const commented = await addCollaborationComment(loaded, "Plain comment", client);
  const inlineCommented = await addCollaborationInlineComment(
    commented,
    "Inline comment",
    anchorAdapter,
    client,
  );
  const resolved = await resolveCollaborationComment(inlineCommented, "thread-1", client);

  assert.equal(commented.commentThreads[0].comments.length, 1);
  assert.equal(inlineCommented.lastInlineAnchorStatus, "stale");
  assert.equal(anchorAdapter.calls, 1);
  assert.equal(resolved.commentThreads[0].state, "resolved");
  assert.equal(client.calls.includes("addInlineDocumentComment"), true);
});

test("collaboration review, publish, lock, and audit panels display server state transitions", async () => {
  const client = new FakeCollaborationApiClient();
  const loaded = await loadCollaborationDocumentViewModel(
    createInitialCollaborationViewModel({ workspaceId: "workspace-1", documentId: "doc-1" }),
    client,
  );

  const requested = await requestCollaborationReview(loaded, "review-2", client);
  const published = await publishCollaborationDocument(requested, client);

  assert.equal(requested.lastReviewAction?.nextState, "review_requested");
  assert.equal(published.lastReviewAction?.nextState, "published");
  assert.equal(published.lock?.status, "unlocked");
  assert.equal(published.auditEvents[0].action, "review.requested");
});

class FakeCollaborationApiClient implements CabinetCollaborationApiClient {
  readonly calls: string[] = [];

  async getAccessibleDocument(_query: GetAccessibleDocumentQuery) {
    this.calls.push("getAccessibleDocument");
    return {
      workspaceId: "workspace-1",
      documentId: "doc-1",
      title: "Source",
      path: "docs/source.md",
      body: "# Source",
      versionId: "version-1",
      permissionDecision: { result: "allowed", reasonCode: "ROLE_ALLOWED" },
    };
  }

  async searchAccessibleDocuments(_query: CollaborationSearchQuery): Promise<SearchAccessibleDocumentsView> {
    this.calls.push("searchAccessibleDocuments");
    return {
      queryName: "search-documents",
      workspaceId: "workspace-1",
      text: "needle",
      results: [
        {
          workspaceId: "workspace-1",
          documentId: "doc-1",
          title: "Source",
          path: "docs/source.md",
          snippet: "needle",
          score: 1,
        },
      ],
      permissionFilteredCount: 1,
      durationMs: 18,
    };
  }

  async getDocumentSharing(_query: GetDocumentSharingQuery): Promise<DocumentSharingView> {
    this.calls.push("getDocumentSharing");
    return {
      workspaceId: "workspace-1",
      documentId: "doc-1",
      entries: [],
      effectivePermissions: ["read"],
    };
  }

  async updateDocumentSharing(command: UpdateDocumentSharingCommand): Promise<DocumentSharingView> {
    this.calls.push("updateDocumentSharing");
    return {
      workspaceId: command.workspaceId,
      documentId: command.documentId,
      entries: [
        {
          subject: command.subject,
          permission: command.permission,
          effect: command.effect,
        },
      ],
      effectivePermissions: ["read", "write"],
    };
  }

  async listDocumentComments(_query: ListDocumentCommentsQuery): Promise<CommentThreadPageView> {
    this.calls.push("listDocumentComments");
    return {
      threads: [
        {
          threadId: "thread-1",
          documentId: "doc-1",
          state: "open",
          comments: [],
        },
      ],
    };
  }

  async addDocumentComment(command: AddDocumentCommentCommand): Promise<CommentThreadMutationView> {
    this.calls.push("addDocumentComment");
    return {
      thread: {
        threadId: command.threadId,
        documentId: command.documentId,
        state: "open",
        comments: [
          {
            commentId: command.commentId,
            authorUserId: "user-1",
            body: command.body,
            createdAt: "2026-06-26T00:00:00.000Z",
          },
        ],
      },
    };
  }

  async addInlineDocumentComment(
    command: AddInlineDocumentCommentCommand,
  ): Promise<CommentThreadMutationView> {
    this.calls.push("addInlineDocumentComment");
    return {
      thread: {
        threadId: command.threadId,
        documentId: command.documentId,
        state: "open",
        anchor: {
          versionId: command.versionId,
          startOffset: command.startOffset,
          endOffset: command.endOffset,
          status: command.startOffset === 99 ? "stale" : "valid",
        },
        comments: [
          {
            commentId: command.commentId,
            authorUserId: "user-1",
            body: command.body,
            createdAt: "2026-06-26T00:00:00.000Z",
          },
        ],
      },
      anchorStatus: command.startOffset === 99 ? "stale" : "valid",
    };
  }

  async resolveDocumentComment(command: ResolveDocumentCommentCommand): Promise<CommentThreadMutationView> {
    this.calls.push("resolveDocumentComment");
    return {
      thread: {
        threadId: command.threadId,
        documentId: command.documentId,
        state: "resolved",
        comments: [],
      },
    };
  }

  async reopenDocumentComment(command: ResolveDocumentCommentCommand): Promise<CommentThreadMutationView> {
    this.calls.push("reopenDocumentComment");
    return {
      thread: {
        threadId: command.threadId,
        documentId: command.documentId,
        state: "reopened",
        comments: [],
      },
    };
  }

  async listReviewRequests(_query: ListReviewRequestsQuery): Promise<ReviewRequestPageView> {
    this.calls.push("listReviewRequests");
    return { requests: [] };
  }

  async requestDocumentReview(command: ReviewRequestCommand): Promise<ReviewWorkflowActionView> {
    this.calls.push("requestDocumentReview");
    return {
      documentId: command.documentId,
      reviewRequestId: command.reviewRequestId,
      previousState: "editing",
      nextState: "review_requested",
    };
  }

  async approveDocumentReview(command: ReviewDecisionCommand): Promise<ReviewWorkflowActionView> {
    this.calls.push("approveDocumentReview");
    return {
      documentId: command.documentId,
      reviewRequestId: command.reviewRequestId,
      previousState: "review_requested",
      nextState: "approved",
    };
  }

  async rejectDocumentReview(command: ReviewDecisionCommand): Promise<ReviewWorkflowActionView> {
    this.calls.push("rejectDocumentReview");
    return {
      documentId: command.documentId,
      reviewRequestId: command.reviewRequestId,
      previousState: "review_requested",
      nextState: "rejected",
    };
  }

  async publishDocument(command: PublishDocumentCommand): Promise<ReviewWorkflowActionView> {
    this.calls.push("publishDocument");
    return {
      documentId: command.documentId,
      previousState: "approved",
      nextState: "published",
    };
  }

  async getDocumentLock(_query: GetDocumentLockQuery): Promise<DocumentLockView> {
    this.calls.push("getDocumentLock");
    return { documentId: "doc-1", status: "unlocked" };
  }

  async lockDocument(command: DocumentLockCommand): Promise<DocumentLockView> {
    this.calls.push("lockDocument");
    return {
      documentId: command.documentId,
      status: "locked",
      lockId: command.lockId,
      ownerUserId: "user-1",
      expiresAt: "2026-06-26T00:10:00.000Z",
    };
  }

  async unlockDocument(command: DocumentLockCommand): Promise<DocumentLockView> {
    this.calls.push("unlockDocument");
    return { documentId: command.documentId, status: "unlocked" };
  }

  async listAuditEvents(_query: ListAuditEventsQuery): Promise<AuditEventPageView> {
    this.calls.push("listAuditEvents");
    return {
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
    };
  }
}

class FakeEditorInlineAnchorAdapter implements EditorInlineAnchorAdapter {
  calls = 0;
  private status: "valid" | "stale";

  constructor(status: "valid" | "stale") {
    this.status = status;
  }

  getInlineAnchor() {
    this.calls += 1;
    return {
      versionId: "version-1",
      startOffset: this.status === "stale" ? 99 : 3,
      endOffset: this.status === "stale" ? 110 : 8,
    };
  }
}

class CapturingDevelopmentLogger implements CollaborationDevelopmentLogger {
  readonly events: string[] = [];

  writeDevelopment(eventName: string): void {
    this.events.push(eventName);
  }
}
