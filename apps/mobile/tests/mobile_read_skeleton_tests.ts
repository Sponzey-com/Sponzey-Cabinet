import assert from "node:assert/strict";
import test from "node:test";

import { CabinetApiClientError, createMobileReadApiContract } from "@sponzey-cabinet/client-core";
import type {
  AccessibleDocumentView,
  CabinetHttpRequest,
  CabinetHttpResponse,
  CommentThreadPageView,
  DocumentHistoryPage,
  ReviewRequestPageView,
  SearchAccessibleDocumentsView,
} from "@sponzey-cabinet/client-core";

import {
  createMobileReadInitialDisplayModel,
  createMobileReadSelfHostApiClient,
  createMobileReadSkeleton,
  createMobileReadSkeletonConfig,
  transitionMobileReadState,
  type MobileHistoryQuery,
  type MobileReadApiClient,
  type MobileReviewDecisionCommand,
  type MobileReviewRequestsQuery,
  type MobileSearchQuery,
} from "../src/index.ts";

const validConfig = createMobileReadSkeletonConfig({
  platform: "ios",
  apiBaseUrl: "https://cabinet.example",
  sessionToken: "mobile-session-token",
  contractVersion: "phase002.mobile.read.v1",
});

test("mobile skeleton validates contract version and exposes read-only unsupported edit state", async () => {
  const fakeClient = new FakeMobileReadApiClient();
  const invalidConfig = createMobileReadSkeletonConfig({
    ...validConfig,
    contractVersion: "phase999.mobile.read.v9",
  });
  const skeleton = createMobileReadSkeleton(invalidConfig, fakeClient);

  const initial = createMobileReadInitialDisplayModel(validConfig);
  const failed = await skeleton.loadCurrentDocument({
    workspaceId: "workspace-1",
    documentId: "doc-1",
  });
  const unsupported = createMobileReadSkeleton(validConfig, fakeClient).requestEdit();

  assert.equal(initial.state, "Idle");
  assert.equal(initial.capabilities.supportsMobileReadApi, true);
  assert.equal(initial.capabilities.supportsRemoteEdit, false);
  assert.equal(initial.capabilities.supportsOfflineRemoteEdit, false);
  assert.equal(initial.capabilities.knowledgeGraphSupport, "view_only");
  assert.equal(initial.capabilities.canvasSupport, "view_only");
  assert.equal(initial.capabilities.realtimeCollaborationSupport, "unsupported");
  assert.equal(initial.capabilities.supportsCanvasFullEdit, false);
  assert.equal(failed.state, "Error");
  assert.equal(failed.error?.code, "MOBILE_CONTRACT_VERSION_MISMATCH");
  assert.deepEqual(fakeClient.calls, []);
  assert.equal(unsupported.state, "UnsupportedAction");
  assert.equal(unsupported.error?.code, "MOBILE_UNSUPPORTED_EDIT");
});

test("mobile skeleton exposes explicit unsupported Canvas full edit action", () => {
  const skeleton = createMobileReadSkeleton(validConfig, new FakeMobileReadApiClient());

  const result = skeleton.requestCanvasEdit();

  assert.equal(result.state, "UnsupportedAction");
  assert.equal(result.capabilities.canvasSupport, "view_only");
  assert.equal(result.capabilities.supportsCanvasFullEdit, false);
  assert.equal(result.error?.code, "MOBILE_UNSUPPORTED_CANVAS_EDIT");
  assert.doesNotMatch(result.error?.message ?? "", /document body|card text|raw canvas/i);
});

test("mobile skeleton maps current history search comments and reviews to read-only view models", async () => {
  const fakeClient = new FakeMobileReadApiClient();
  const skeleton = createMobileReadSkeleton(validConfig, fakeClient);

  const current = await skeleton.loadCurrentDocument({
    workspaceId: "workspace-1",
    documentId: "doc-1",
  });
  const history = await skeleton.loadDocumentHistory({
    workspaceId: "workspace-1",
    documentId: "doc-1",
    limit: 20,
  });
  const search = await skeleton.searchDocuments({
    workspaceId: "workspace-1",
    text: "needle",
    limit: 10,
  });
  const comments = await skeleton.loadDocumentComments({
    workspaceId: "workspace-1",
    documentId: "doc-1",
  });
  const reviews = await skeleton.loadReviewRequests({
    workspaceId: "workspace-1",
    documentId: "doc-1",
  });

  assert.equal(current.state, "Loaded");
  assert.equal(current.content?.kind, "current-document");
  if (current.content?.kind === "current-document") {
    assert.equal(current.content.documentId, "doc-1");
    assert.equal(current.content.permissionDecision.result, "allowed");
    assert.equal(current.content.canEdit, false);
  }
  assert.equal(history.content?.kind, "document-history");
  if (history.content?.kind === "document-history") {
    assert.equal(history.content.entries.length, 2);
  }
  assert.equal(search.content?.kind, "search-results");
  if (search.content?.kind === "search-results") {
    assert.equal(search.content.permissionFilteredCount, 3);
    assert.equal(search.content.durationMs, 24);
  }
  assert.equal(comments.content?.kind, "comments");
  if (comments.content?.kind === "comments") {
    assert.equal(comments.content.threads[0]?.commentCount, 2);
    assert.equal(comments.content.threads[0]?.anchorStatus, "valid");
  }
  assert.equal(reviews.content?.kind, "review-requests");
  if (reviews.content?.kind === "review-requests") {
    assert.equal(reviews.content.requests[0]?.status, "open");
  }
  assert.deepEqual(fakeClient.calls, [
    "getCurrentDocument",
    "getDocumentHistory",
    "searchDocuments",
    "listDocumentComments",
    "listReviewRequests",
  ]);
});

test("mobile skeleton maps review approve and reject decisions without raw body data", async () => {
  const fakeClient = new FakeMobileReadApiClient();
  const skeleton = createMobileReadSkeleton(validConfig, fakeClient);

  const approved = await skeleton.approveReviewRequest({
    workspaceId: "workspace-1",
    reviewRequestId: "review-1",
  });
  const rejected = await skeleton.rejectReviewRequest({
    workspaceId: "workspace-1",
    reviewRequestId: "review-2",
  });

  assert.equal(approved.state, "Loaded");
  assert.equal(approved.content?.kind, "review-decision");
  if (approved.content?.kind === "review-decision") {
    assert.equal(approved.content.decision, "approved");
    assert.equal(approved.content.nextState, "approved");
  }
  assert.equal(rejected.content?.kind, "review-decision");
  if (rejected.content?.kind === "review-decision") {
    assert.equal(rejected.content.decision, "rejected");
    assert.equal(rejected.content.nextState, "rejected");
  }
  assert.deepEqual(fakeClient.calls, ["approveReviewRequest", "rejectReviewRequest"]);
  assert.doesNotMatch(JSON.stringify([approved, rejected]), /document body|comment body|token|secret/i);
});

test("mobile skeleton rejects current document response without permission decision", async () => {
  const fakeClient = new FakeMobileReadApiClient({
    current: {
      workspaceId: "workspace-1",
      documentId: "doc-1",
      title: "Missing Permission",
      path: "docs/missing.md",
      body: "# Missing",
      versionId: "version-1",
    } as AccessibleDocumentView,
  });
  const skeleton = createMobileReadSkeleton(validConfig, fakeClient);

  const result = await skeleton.loadCurrentDocument({
    workspaceId: "workspace-1",
    documentId: "doc-1",
  });

  assert.equal(result.state, "Error");
  assert.equal(result.error?.code, "MOBILE_PERMISSION_DECISION_MISSING");
});

test("mobile skeleton maps unauthorized session expired and network errors without leaking config", async () => {
  const unauthorized = await createMobileReadSkeleton(
    validConfig,
    new FakeMobileReadApiClient({
      error: new CabinetApiClientError(
        "UNAUTHORIZED",
        "unauthorized token mobile-session-token https://cabinet.example",
        403,
      ),
    }),
  ).loadCurrentDocument({ workspaceId: "workspace-1", documentId: "doc-1" });
  const expired = await createMobileReadSkeleton(
    validConfig,
    new FakeMobileReadApiClient({
      error: new CabinetApiClientError("SESSION_EXPIRED", "expired mobile-session-token", 401),
    }),
  ).loadCurrentDocument({ workspaceId: "workspace-1", documentId: "doc-1" });
  const network = await createMobileReadSkeleton(
    validConfig,
    new FakeMobileReadApiClient({
      error: new CabinetApiClientError("NETWORK_FAILURE", "network mobile-session-token"),
    }),
  ).loadCurrentDocument({ workspaceId: "workspace-1", documentId: "doc-1" });

  assert.equal(unauthorized.error?.code, "MOBILE_UNAUTHORIZED");
  assert.equal(expired.error?.code, "MOBILE_SESSION_EXPIRED");
  assert.equal(network.error?.code, "MOBILE_NETWORK_FAILURE");
  for (const model of [unauthorized, expired, network]) {
    assert.doesNotMatch(model.error?.message ?? "", /token|secret|cabinet\.example/i);
  }
});

test("mobile self-host API client sends history and review decisions through explicit config and transport", async () => {
  const requests: CabinetHttpRequest[] = [];
  const responses: CabinetHttpResponse[] = [
    {
      status: 200,
      body: JSON.stringify({
        workspaceId: "workspace-1",
        documentId: "doc-1",
        entries: [],
      }),
    },
    {
      status: 200,
      body: JSON.stringify({
        documentId: "doc-1",
        reviewRequestId: "review-1",
        previousState: "review_requested",
        nextState: "approved",
      }),
    },
    {
      status: 200,
      body: JSON.stringify({
        documentId: "doc-2",
        reviewRequestId: "review-2",
        previousState: "review_requested",
        nextState: "rejected",
      }),
    },
  ];
  const transport = async (request: CabinetHttpRequest): Promise<CabinetHttpResponse> => {
    requests.push(request);
    const response = responses.shift();
    if (!response) {
      throw new Error("unexpected request");
    }
    return response;
  };
  const client = createMobileReadSelfHostApiClient(validConfig, transport);

  const history = await client.getDocumentHistory({
    workspaceId: "workspace-1",
    documentId: "doc-1",
    limit: 25,
    cursor: "cursor-1",
  });
  const approved = await client.approveReviewRequest({
    workspaceId: "workspace-1",
    reviewRequestId: "review-1",
  });
  const rejected = await client.rejectReviewRequest({
    workspaceId: "workspace-1",
    reviewRequestId: "review-2",
  });

  assert.equal(history.workspaceId, "workspace-1");
  assert.equal(approved.nextState, "approved");
  assert.equal(rejected.nextState, "rejected");
  assert.deepEqual(
    requests.map((request) => [request.method, request.url]),
    [
      [
        "GET",
        "https://cabinet.example/api/workspaces/workspace-1/documents/doc-1/history?limit=25&cursor=cursor-1",
      ],
      ["POST", "https://cabinet.example/api/review-requests/review-1/approve"],
      ["POST", "https://cabinet.example/api/review-requests/review-2/reject"],
    ],
  );
  assert.equal(requests[0]?.headers.authorization, "Bearer mobile-session-token");
  assert.deepEqual(JSON.parse(requests[1]?.body ?? "{}"), { workspaceId: "workspace-1" });
  assert.deepEqual(JSON.parse(requests[2]?.body ?? "{}"), { workspaceId: "workspace-1" });
});

test("mobile display state transition is explicit", () => {
  assert.equal(transitionMobileReadState("Idle", "LoadCurrent"), "Loading");
  assert.equal(transitionMobileReadState("Loading", "LoadSucceeded"), "Loaded");
  assert.equal(transitionMobileReadState("Loading", "ApiFailure"), "Error");
  assert.equal(transitionMobileReadState("Loaded", "EditRequested"), "UnsupportedAction");
});

class FakeMobileReadApiClient implements MobileReadApiClient {
  readonly calls: string[] = [];
  private readonly contract = createMobileReadApiContract();
  private readonly options: {
    readonly current?: AccessibleDocumentView;
    readonly error?: Error;
  };

  constructor(
    options: {
      readonly current?: AccessibleDocumentView;
      readonly error?: Error;
    } = {},
  ) {
    this.options = options;
  }

  async getCurrentDocument(): Promise<AccessibleDocumentView> {
    this.calls.push("getCurrentDocument");
    this.throwIfNeeded();
    return this.options.current ?? {
      workspaceId: "workspace-1",
      documentId: "doc-1",
      title: "Mobile Current",
      path: "docs/mobile-current.md",
      body: "# Mobile Current",
      versionId: "version-1",
      permissionDecision: {
        result: "allowed",
        reasonCode: "ROLE_ALLOWED",
      },
    };
  }

  async getDocumentHistory(query: MobileHistoryQuery): Promise<DocumentHistoryPage> {
    this.calls.push("getDocumentHistory");
    this.throwIfNeeded();
    return {
      workspaceId: query.workspaceId,
      documentId: query.documentId,
      entries: [
        {
          versionId: "version-1",
          summary: "Initial",
          author: "actor-a",
          createdAt: "2026-06-28T00:00:00Z",
        },
        {
          versionId: "version-2",
          summary: "Edited",
          author: "actor-b",
          createdAt: "2026-06-28T01:00:00Z",
        },
      ],
    };
  }

  async searchDocuments(query: MobileSearchQuery): Promise<SearchAccessibleDocumentsView> {
    this.calls.push("searchDocuments");
    this.throwIfNeeded();
    return {
      queryName: "search-documents",
      workspaceId: query.workspaceId,
      text: query.text,
      results: [
        {
          workspaceId: query.workspaceId,
          documentId: "doc-1",
          title: "Mobile Current",
          path: "docs/mobile-current.md",
          snippet: "needle",
          score: 1,
        },
      ],
      permissionFilteredCount: 3,
      durationMs: 24,
    };
  }

  async listDocumentComments(): Promise<CommentThreadPageView> {
    this.calls.push("listDocumentComments");
    this.throwIfNeeded();
    return {
      threads: [
        {
          threadId: "thread-1",
          documentId: "doc-1",
          state: "open",
          comments: [
            {
              commentId: "comment-1",
              authorUserId: "actor-a",
              body: "comment one",
              createdAt: "2026-06-28T00:00:00Z",
            },
            {
              commentId: "comment-2",
              authorUserId: "actor-b",
              body: "comment two",
              createdAt: "2026-06-28T01:00:00Z",
            },
          ],
          anchor: {
            versionId: "version-1",
            startOffset: 1,
            endOffset: 3,
            status: "valid",
          },
        },
      ],
    };
  }

  async listReviewRequests(query: MobileReviewRequestsQuery): Promise<ReviewRequestPageView> {
    this.calls.push("listReviewRequests");
    this.throwIfNeeded();
    return {
      requests: [
        {
          reviewRequestId: "review-1",
          documentId: query.documentId ?? "doc-1",
          requestedBy: "actor-a",
          status: "open",
        },
      ],
    };
  }

  async approveReviewRequest(command: MobileReviewDecisionCommand) {
    this.calls.push("approveReviewRequest");
    this.throwIfNeeded();
    return {
      documentId: command.reviewRequestId === "review-1" ? "doc-1" : "doc-2",
      reviewRequestId: command.reviewRequestId,
      previousState: "review_requested" as const,
      nextState: "approved" as const,
    };
  }

  async rejectReviewRequest(command: MobileReviewDecisionCommand) {
    this.calls.push("rejectReviewRequest");
    this.throwIfNeeded();
    return {
      documentId: command.reviewRequestId === "review-1" ? "doc-1" : "doc-2",
      reviewRequestId: command.reviewRequestId,
      previousState: "review_requested" as const,
      nextState: "rejected" as const,
    };
  }

  validateContractVersion() {
    return this.contract.version;
  }

  private throwIfNeeded(): void {
    if (this.options.error) {
      throw this.options.error;
    }
  }
}
