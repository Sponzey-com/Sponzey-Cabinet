import assert from "node:assert/strict";
import test from "node:test";

import {
  createDesktopCurrentProductShell,
  createDesktopCurrentProductShellDescriptor,
  loadDesktopWorkspaceHome,
} from "../src/index.ts";
import {
  LocalDesktopCommandClientError,
  type LocalDesktopCommandClient,
  type WorkspaceHomeQuery,
} from "../../../packages/client-core/src/index.ts";

test("desktop current product shell starts from personal workspace home", () => {
  const current = createDesktopCurrentProductShell();

  assert.equal(current.capability.productScope, "personal_local_desktop");
  assert.equal(current.home.mode, "personal-workspace-home");
  assert.equal(current.home.firstRoute, "home");
  assert.deepEqual(
    current.home.sections.map((section) => section.id),
    [
      "recent-documents",
      "favorites",
      "tags",
      "recent-changes",
      "unfinished-items",
      "quick-search",
      "ai-entry",
      "backup-status",
      "workspace-health",
    ],
  );
});

test("desktop workspace home facade calls local command once with explicit limits", async () => {
  const calls: WorkspaceHomeQuery[] = [];
  const client = {
    async getWorkspaceHome(query: WorkspaceHomeQuery) {
      calls.push(query);
      return {
        workspaceId: query.workspaceId,
        state: "Ready" as const,
        recentDocuments: [{ documentId: "doc-1", title: "Source", path: "notes/source.md" }],
        favorites: [],
        tags: [],
        recentChanges: [],
        unfinishedItems: [],
        backupStatus: "Fresh" as const,
        healthStatus: "Healthy" as const,
        documentCount: 10_000,
        assetCount: 2_500,
        canvasCount: 24,
        summaryUnavailable: ["Assets"] as const,
      };
    },
  } as unknown as LocalDesktopCommandClient;

  const model = await loadDesktopWorkspaceHome(client, {
    workspaceId: "workspace-1",
    recentDocuments: 12,
    favorites: 8,
    tags: 10,
    recentChanges: 14,
    unfinishedItems: 6,
  });

  assert.equal(calls.length, 1);
  assert.deepEqual(calls[0], {
    workspaceId: "workspace-1",
    recentDocuments: 12,
    favorites: 8,
    tags: 10,
    recentChanges: 14,
    unfinishedItems: 6,
  });
  assert.equal(model.displayState, "Ready");
  assert.equal(model.recentDocuments[0]?.documentId, "doc-1");
  assert.deepEqual(model.workspaceSummary, {
    documentCount: 10_000,
    assetCount: 2_500,
    canvasCount: 24,
    unavailable: ["Assets"],
  });
});

test("desktop workspace home facade maps client failure without leaking transport message", async () => {
  const client = {
    async getWorkspaceHome() {
      throw new LocalDesktopCommandClientError("STORE_UNAVAILABLE", true);
    },
  } as unknown as LocalDesktopCommandClient;

  const model = await loadDesktopWorkspaceHome(client, {
    workspaceId: "workspace-1",
    recentDocuments: 12,
    favorites: 8,
    tags: 10,
    recentChanges: 14,
    unfinishedItems: 6,
  });

  assert.equal(model.displayState, "Failed");
  assert.equal(model.error?.code, "STORE_UNAVAILABLE");
  assert.equal(model.error?.retryable, true);
  assert.equal(JSON.stringify(model).includes("local desktop command failed"), false);
});

test("desktop current product descriptor exposes no hosted workspace command surface", () => {
  const descriptor = createDesktopCurrentProductShellDescriptor();

  assert.deepEqual(
    descriptor.workspace.commandActions.map((action) => action.id),
    [
      "new-document",
      "quick-search",
      "open-graph",
      "ask-ai",
      "create-backup",
      "import-markdown",
      "export-package",
      "open-settings",
    ],
  );

  const serialized = JSON.stringify(descriptor);
  for (const forbidden of [
    "serverBaseUrl",
    "sessionToken",
    "team-invite",
    "admin-console",
    "billing",
    "tenant-settings",
    "sso-settings",
    "server-workspace-connect",
  ]) {
    assert.equal(serialized.includes(forbidden), false);
  }
});
