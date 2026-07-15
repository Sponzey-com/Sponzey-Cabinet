import assert from "node:assert/strict";

import { createPlatformCapabilityMatrix } from "@sponzey-cabinet/client-core";

import {
  createMobilePushNotificationPayload,
  createMobileReadSelfHostApiClient,
  createMobileReadSkeleton,
  createMobileReadSkeletonConfig,
  type MobilePlatform,
} from "../src/index.ts";

const platforms: readonly MobilePlatform[] = ["ios", "android"];

async function main() {
  const serverBaseUrl = requireArg("--server-base-url");
  const sessionToken = requireArg("--session-token");

  for (const platform of platforms) {
    await runPlatformReadOnlySmoke(platform, serverBaseUrl, sessionToken);
  }
  verifyMobileCapabilityMatrix();

  console.log("mobile_read_product_smoke=passed");
}

async function runPlatformReadOnlySmoke(
  platform: MobilePlatform,
  serverBaseUrl: string,
  sessionToken: string,
) {
  console.log(`mobile_read_product_step_start=${platform}`);
  const config = createMobileReadSkeletonConfig({
    platform,
    apiBaseUrl: serverBaseUrl,
    sessionToken,
    contractVersion: "phase002.mobile.read.v1",
  });
  const skeleton = createMobileReadSkeleton(config, createMobileReadSelfHostApiClient(config));

  const current = await skeleton.loadCurrentDocument({
    workspaceId: "workspace-1",
    documentId: "doc-allowed",
  });
  assert.equal(current.state, "Loaded", JSON.stringify(current.error));
  assert.equal(current.content?.kind, "current-document");
  if (current.content?.kind === "current-document") {
    assert.equal(current.content.documentId, "doc-allowed");
    assert.equal(current.content.canEdit, false);
  }

  const history = await skeleton.loadDocumentHistory({
    workspaceId: "workspace-1",
    documentId: "doc-allowed",
    limit: 10,
  });
  assert.equal(history.state, "Loaded", JSON.stringify(history.error));
  assert.equal(history.content?.kind, "document-history");
  if (history.content?.kind === "document-history") {
    assert.ok(history.content.entries.length >= 1);
  }

  const search = await skeleton.searchDocuments({
    workspaceId: "workspace-1",
    text: "needle",
    limit: 10,
  });
  assert.equal(search.state, "Loaded", JSON.stringify(search.error));
  assert.equal(search.content?.kind, "search-results");
  if (search.content?.kind === "search-results") {
    assert.ok(search.content.results.some((result) => result.documentId === "doc-allowed"));
    assert.ok(search.content.durationMs <= 300);
  }

  const comments = await skeleton.loadDocumentComments({
    workspaceId: "workspace-1",
    documentId: "doc-allowed",
  });
  assert.equal(comments.state, "Loaded", JSON.stringify(comments.error));
  assert.equal(comments.content?.kind, "comments");
  if (comments.content?.kind === "comments") {
    assert.ok(comments.content.threads.length >= 1);
  }

  const reviews = await skeleton.loadReviewRequests({
    workspaceId: "workspace-1",
    documentId: "doc-allowed",
  });
  assert.equal(reviews.state, "Loaded", JSON.stringify(reviews.error));
  assert.equal(reviews.content?.kind, "review-requests");

  const approved = await skeleton.approveReviewRequest({
    workspaceId: "workspace-1",
    reviewRequestId: "review-request-1",
  });
  const rejected = await skeleton.rejectReviewRequest({
    workspaceId: "workspace-1",
    reviewRequestId: "review-request-2",
  });
  assert.equal(approved.state, "Loaded", JSON.stringify(approved.error));
  assert.equal(approved.content?.kind, "review-decision");
  if (approved.content?.kind === "review-decision") {
    assert.equal(approved.content.decision, "approved");
    assert.equal(approved.content.documentId, "doc-allowed");
    assert.equal(approved.content.reviewRequestId, "review-request-1");
  }
  assert.equal(rejected.state, "Loaded", JSON.stringify(rejected.error));
  assert.equal(rejected.content?.kind, "review-decision");
  if (rejected.content?.kind === "review-decision") {
    assert.equal(rejected.content.decision, "rejected");
    assert.equal(rejected.content.documentId, "doc-allowed");
    assert.equal(rejected.content.reviewRequestId, "review-request-2");
  }
  assert.doesNotMatch(
    JSON.stringify([approved, rejected]),
    /E2E document body should not be logged|comment body should not leak|token|secret/i,
  );

  const unsupported = skeleton.requestEdit();
  assert.equal(unsupported.state, "UnsupportedAction");
  assert.equal(unsupported.error?.code, "MOBILE_UNSUPPORTED_EDIT");

  const canvasUnsupported = skeleton.requestCanvasEdit();
  assert.equal(canvasUnsupported.state, "UnsupportedAction");
  assert.equal(canvasUnsupported.error?.code, "MOBILE_UNSUPPORTED_CANVAS_EDIT");
  assert.equal(canvasUnsupported.capabilities.canvasSupport, "view_only");
  assert.equal(canvasUnsupported.capabilities.supportsCanvasFullEdit, false);

  const pushPayload = createMobilePushNotificationPayload({
    eventName: "review.state_changed",
    target: { kind: "review_request", id: "review-request-1" },
    title: "Review state changed",
    correlationId: `corr-${platform}`,
    deliveryState: "Queued",
    unsafeDocumentBody: "mobile product raw document body",
    unsafeCommentBody: "mobile product raw comment body",
    unsafeSessionToken: "mobile-product-push-token",
    unsafeSessionId: "mobile-product-push-session",
    unsafeRawCanvasState: "{\"secret\":\"mobile-product-canvas\"}",
  });
  assert.deepEqual(pushPayload, {
    eventName: "review.state_changed",
    targetKind: "review_request",
    targetId: "review-request-1",
    title: "Review state changed",
    correlationId: `corr-${platform}`,
    deliveryState: "Queued",
  });
  assert.doesNotMatch(
    JSON.stringify(pushPayload),
    /mobile product raw document body|mobile product raw comment body|mobile-product-push-token|mobile-product-push-session|mobile-product-canvas/i,
  );

  console.log(`mobile_review_decision_product_platform_${platform}=passed`);
  console.log(`mobile_canvas_unsupported_product_platform_${platform}=passed`);
  console.log(`mobile_push_payload_product_platform_${platform}=passed`);
  console.log(`mobile_read_product_platform_${platform}=passed`);
}

function verifyMobileCapabilityMatrix() {
  const matrix = createPlatformCapabilityMatrix();
  for (const platform of [matrix.ios, matrix.android]) {
    assert.equal(platform.supportsMobileReadApi, true);
    assert.equal(platform.supportsRemoteEdit, false);
    assert.equal(platform.supportsOfflineRemoteEdit, false);
    assert.equal(platform.knowledgeGraphSupport, "view_only");
    assert.equal(platform.canvasSupport, "view_only");
    assert.equal(platform.realtimeCollaborationSupport, "unsupported");
    assert.equal(platform.supportsCanvasFullEdit, false);
  }
  console.log("mobile_review_decision_product_smoke=passed");
  console.log("mobile_canvas_unsupported_product_smoke=passed");
  console.log("mobile_push_payload_product_smoke=passed");
}

function requireArg(name: string): string {
  const index = process.argv.indexOf(name);
  const value = index >= 0 ? process.argv[index + 1] : undefined;
  if (!value?.trim()) {
    throw new Error(`missing required argument: ${name}`);
  }
  return value;
}

main().catch((error) => {
  console.error("mobile_read_product_smoke=failed");
  console.error(`failure_category=${error instanceof Error ? error.message : "unexpected_failure"}`);
  process.exit(1);
});
