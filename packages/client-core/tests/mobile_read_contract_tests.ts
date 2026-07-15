import assert from "node:assert/strict";
import test from "node:test";

import {
  createMobileReadApiContract,
  createPlatformCapabilityMatrix,
  validateMobileReadApiResponse,
} from "../src/index.ts";

test("mobile read API contract exposes stable read-only document, search, comment, and review endpoints", () => {
  const contract = createMobileReadApiContract();

  assert.equal(contract.version, "phase002.mobile.read.v1");
  assert.deepEqual(
    contract.endpoints.map((endpoint) => [endpoint.method, endpoint.path, endpoint.responseName]),
    [
      ["GET", "/api/workspaces/{workspaceId}/documents/{documentId}/current", "MobileCurrentDocumentResponse"],
      ["GET", "/api/workspaces/{workspaceId}/documents/{documentId}/history", "MobileDocumentHistoryResponse"],
      ["GET", "/api/workspaces/{workspaceId}/search", "MobileSearchResponse"],
      ["GET", "/api/documents/{documentId}/comments", "MobileCommentThreadsResponse"],
      ["GET", "/api/review-requests", "MobileReviewRequestsResponse"],
    ],
  );
  assert.equal(contract.endpoints.every((endpoint) => endpoint.method === "GET"), true);
  assert.equal(
    contract.endpoints.some((endpoint) => /POST|PUT|DELETE/.test(endpoint.method)),
    false,
  );
});

test("mobile read API contract validates required response fields and permission result", () => {
  const contract = createMobileReadApiContract();

  const validCurrent = validateMobileReadApiResponse(contract, "MobileCurrentDocumentResponse", {
    workspaceId: "workspace-1",
    documentId: "doc-1",
    title: "Source",
    path: "docs/source.md",
    body: "# Source",
    versionId: "version-1",
    permissionDecision: { result: "allowed", reasonCode: "ROLE_ALLOWED" },
  });
  const invalidCurrent = validateMobileReadApiResponse(contract, "MobileCurrentDocumentResponse", {
    workspaceId: "workspace-1",
    documentId: "doc-1",
    title: "Source",
    path: "docs/source.md",
    body: "# Source",
    versionId: "version-1",
  });

  assert.equal(validCurrent.valid, true);
  assert.equal(invalidCurrent.valid, false);
  assert.deepEqual(invalidCurrent.missingFields, ["permissionDecision"]);
});

test("platform capability matrix documents web desktop and mobile differences without domain rules", () => {
  const matrix = createPlatformCapabilityMatrix();

  assert.equal(matrix.web.supportsSelfHostAdminUi, true);
  assert.equal(matrix.desktop.supportsLocalWorkspace, true);
  assert.equal(matrix.desktop.supportsRemoteWorkspace, true);
  assert.equal(matrix.ios.supportsMobileReadApi, true);
  assert.equal(matrix.android.supportsMobileReadApi, true);
  assert.equal(matrix.ios.supportsRemoteEdit, false);
  assert.equal(matrix.android.supportsRemoteEdit, false);
  assert.equal(matrix.web.knowledgeGraphSupport, "interactive");
  assert.equal(matrix.desktop.knowledgeGraphSupport, "interactive");
  assert.equal(matrix.web.canvasSupport, "interactive");
  assert.equal(matrix.desktop.canvasSupport, "interactive");
  assert.equal(matrix.web.realtimeCollaborationSupport, "interactive");
  assert.equal(matrix.desktop.realtimeCollaborationSupport, "interactive");
  assert.equal(matrix.ios.knowledgeGraphSupport, "view_only");
  assert.equal(matrix.android.knowledgeGraphSupport, "view_only");
  assert.equal(matrix.ios.canvasSupport, "view_only");
  assert.equal(matrix.android.canvasSupport, "view_only");
  assert.equal(matrix.ios.realtimeCollaborationSupport, "unsupported");
  assert.equal(matrix.android.realtimeCollaborationSupport, "unsupported");
  assert.equal(matrix.ios.supportsCanvasFullEdit, false);
  assert.equal(matrix.android.supportsCanvasFullEdit, false);
  assert.equal("permissionRules" in matrix.ios, false);
});
