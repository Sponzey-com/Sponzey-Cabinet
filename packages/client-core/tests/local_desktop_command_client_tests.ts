import assert from "node:assert/strict";
import test from "node:test";

import {
  createLocalDesktopCommandClient,
  LocalDesktopCommandClientError,
  LocalDesktopCommandEvent,
  LocalDesktopCommandState,
  PHASE009_LOCAL_DESKTOP_COMMAND_NAMES,
  transitionLocalDesktopCommandState,
  type LocalDesktopCommandEnvelope,
  type LocalDesktopCommandResponse,
  type LocalDesktopCommandTransport,
} from "../src/index.ts";

test("local desktop command registry matches the Phase 009 plan exactly", () => {
  assert.deepEqual(PHASE009_LOCAL_DESKTOP_COMMAND_NAMES, [
    "local_workspace_bootstrap",
    "local_workspace_home",
    "local_document_navigator",
    "create_document",
    "rename_document",
    "save_document_revision",
    "get_current_document",
    "update_current_document",
    "get_document_history",
    "get_document_version",
    "compare_document_versions",
    "preview_document_restore",
    "restore_document_version",
    "search_documents",
    "search_assets",
    "get_link_overview",
    "get_graph_projection",
    "list_document_assets",
    "attach_document_asset",
    "create_backup",
    "preview_import",
    "preview_restore",
    "apply_restore",
  ]);
  assert.equal(PHASE009_LOCAL_DESKTOP_COMMAND_NAMES.includes("open_default_workspace"), false);
  assert.equal(PHASE009_LOCAL_DESKTOP_COMMAND_NAMES.includes("save_current_document"), false);
  assert.equal(PHASE009_LOCAL_DESKTOP_COMMAND_NAMES.includes("list_document_history"), false);
  assert.equal(PHASE009_LOCAL_DESKTOP_COMMAND_NAMES.includes("get_asset_metadata"), false);
});

test("local desktop command client dispatches typed command names and payloads", async () => {
  const transport = new RecordingLocalDesktopTransport();
  const client = createLocalDesktopCommandClient(transport.invoke);

  await client.openDefaultWorkspace();
  const home = await client.getWorkspaceHome({
    workspaceId: "workspace-1",
    recentDocuments: 12,
    favorites: 8,
    tags: 10,
    recentChanges: 14,
    unfinishedItems: 6,
  });
  await client.getCurrentDocument({ queryName: "get-current-document", workspaceId: "workspace-1", documentId: "doc-1" });
  await client.saveCurrentDocument({
    workspaceId: "workspace-1",
    documentId: "doc-1",
    title: "Source",
    path: "docs/source.md",
    body: "raw body must stay in payload only",
    expectedVersionId: "version-1",
  });
  await client.listDocumentHistory({
    queryName: "get-document-history",
    workspaceId: "workspace-1",
    documentId: "doc-1",
    limit: 20,
  });
  await client.getDocumentVersion({
    queryName: "get-document-version",
    workspaceId: "workspace-1",
    documentId: "doc-1",
    versionId: "version-1",
  });
  await client.previewDocumentRestore({
    queryName: "preview-document-restore",
    workspaceId: "workspace-1",
    documentId: "doc-1",
    targetVersionId: "version-1",
  });
  await client.restoreDocumentVersion({
    commandName: "restore-document-version",
    operationId: "operation-restore-1",
    workspaceId: "workspace-1",
    documentId: "doc-1",
    targetVersionId: "version-1",
    expectedCurrentVersionId: "version-2",
    author: "local-user",
    summary: "Restore version",
  });
  await client.searchDocuments({
    queryName: "search-documents",
    workspaceId: "workspace-1",
    text: "needle",
    limit: 10,
  });
  await client.searchAssets({
    queryName: "search-assets",
    workspaceId: "workspace-1",
    text: "attachment needle",
    limit: 10,
  });
  await client.getLinkOverview({ queryName: "get-link-overview", workspaceId: "workspace-1", documentId: "doc-1" });
  const graph = await client.getKnowledgeGraph({
    queryName: "get-knowledge-graph",
    workspaceId: "workspace-1",
    documentId: "doc-1",
    depth: 2,
    direction: "both",
    includeUnresolved: true,
    includeAssets: false,
    nodeLimit: 120,
    edgeLimit: 240,
  });
  await client.getAssetMetadata({ queryName: "list-document-assets", workspaceId: "workspace-1", documentId: "doc-1" });

  assert.deepEqual(transport.calls.map((call) => call.commandName), [
    "local_workspace_bootstrap",
    "local_workspace_home",
    "get_current_document",
    "update_current_document",
    "get_document_history",
    "get_document_version",
    "preview_document_restore",
    "restore_document_version",
    "search_documents",
    "search_assets",
    "get_link_overview",
    "get_graph_projection",
    "list_document_assets",
  ]);
  assert.deepEqual(transport.calls[1]?.payload, {
    workspaceId: "workspace-1",
    recentDocuments: 12,
    favorites: 8,
    tags: 10,
    recentChanges: 14,
    unfinishedItems: 6,
  });
  assert.equal(home.state, "Ready");
  assert.equal(home.recentDocuments[0]?.documentId, "doc-1");
  assert.equal(transport.calls[3]?.payload.body, "raw body must stay in payload only");
  assert.deepEqual(transport.calls[9]?.payload, {
    queryName: "search-assets",
    workspaceId: "workspace-1",
    text: "attachment needle",
    limit: 10,
  });
  assert.deepEqual(transport.calls[11]?.payload, {
    queryName: "get-knowledge-graph",
    workspaceId: "workspace-1",
    documentId: "doc-1",
    depth: 2,
    direction: "both",
    includeUnresolved: true,
    includeAssets: false,
    nodeLimit: 120,
    edgeLimit: 240,
  });
  assert.equal(graph.centerDocumentId, "doc-1");
});

test("local desktop command client maps command failures to stable safe errors", async () => {
  const transport: LocalDesktopCommandTransport = async () => ({
    ok: false,
    errorCode: "STORE_UNAVAILABLE",
    retryable: true,
    message: "raw body should not leak into client error",
  });
  const client = createLocalDesktopCommandClient(transport);

  await assert.rejects(
    () =>
      client.saveCurrentDocument({
        workspaceId: "workspace-1",
        documentId: "doc-1",
        title: "Source",
        path: "docs/source.md",
        body: "secret document body",
        expectedVersionId: "version-1",
      }),
    (error) => {
      assert.ok(error instanceof LocalDesktopCommandClientError);
      assert.equal(error.code, "STORE_UNAVAILABLE");
      assert.equal(error.retryable, true);
      assert.equal(error.message.includes("secret document body"), false);
      assert.equal(error.message.includes("raw body should not leak"), false);
      return true;
    },
  );
});

test("local desktop command state transition covers success and failure", () => {
  const dispatching = transitionLocalDesktopCommandState(
    LocalDesktopCommandState.Idle,
    LocalDesktopCommandEvent.Dispatch,
  );
  const succeeded = transitionLocalDesktopCommandState(
    dispatching.state,
    LocalDesktopCommandEvent.Resolve,
  );
  const failed = transitionLocalDesktopCommandState(
    LocalDesktopCommandState.Dispatching,
    LocalDesktopCommandEvent.Reject,
    { errorCode: "COMMAND_BRIDGE_FAILED", retryable: true },
  );
  const invalid = transitionLocalDesktopCommandState(
    LocalDesktopCommandState.Idle,
    LocalDesktopCommandEvent.Resolve,
  );

  assert.equal(dispatching.state, LocalDesktopCommandState.Dispatching);
  assert.equal(succeeded.state, LocalDesktopCommandState.Succeeded);
  assert.equal(failed.state, LocalDesktopCommandState.Failed);
  assert.equal(failed.errorCode, "COMMAND_BRIDGE_FAILED");
  assert.equal(failed.retryable, true);
  assert.equal(invalid.state, LocalDesktopCommandState.Failed);
  assert.equal(invalid.errorCode, "COMMAND_INVALID_TRANSITION");
});

class RecordingLocalDesktopTransport {
  readonly calls: LocalDesktopCommandEnvelope[] = [];

  readonly invoke: LocalDesktopCommandTransport = async (request) => {
    this.calls.push(request);
    return commandResponse(request.commandName);
  };
}

function commandResponse(commandName: string): LocalDesktopCommandResponse<unknown> {
  switch (commandName) {
    case "local_workspace_bootstrap":
      return {
        ok: true,
        data: { workspaceId: "workspace-1", displayName: "Personal Workspace", setupHealth: "Ready" },
      };
    case "local_workspace_home":
      return {
        ok: true,
        data: {
          workspaceId: "workspace-1",
          state: "Ready",
          recentDocuments: [{ documentId: "doc-1", title: "Source", path: "docs/source.md" }],
          favorites: [],
          tags: [],
          recentChanges: [],
          unfinishedItems: [],
          backupStatus: "Fresh",
          healthStatus: "Healthy",
        },
      };
    case "get_current_document":
      return {
        ok: true,
        data: {
          workspaceId: "workspace-1",
          documentId: "doc-1",
          title: "Source",
          path: "docs/source.md",
          body: "current body",
          versionId: "version-2",
        },
      };
    case "update_current_document":
      return {
        ok: true,
        data: {
          status: "saved-local",
          workspaceId: "workspace-1",
          documentId: "doc-1",
          currentVersionId: "version-2",
          versionAppended: true,
        },
      };
    case "get_document_history":
      return {
        ok: true,
        data: {
          workspaceId: "workspace-1",
          documentId: "doc-1",
          entries: [],
        },
      };
    case "get_document_version":
      return {
        ok: true,
        data: {
          workspaceId: "workspace-1",
          documentId: "doc-1",
          versionId: "version-1",
          body: "historical body",
        },
      };
    case "preview_document_restore":
      return {
        ok: true,
        data: {
          workspaceId: "workspace-1",
          documentId: "doc-1",
          targetVersionId: "version-1",
          expectedCurrentVersionId: "version-2",
          canRestore: true,
          lines: [
            { kind: "removed", text: "redacted current" },
            { kind: "added", text: "redacted target" },
          ],
        },
      };
    case "restore_document_version":
      return {
        ok: true,
        data: {
          workspaceId: "workspace-1",
          documentId: "doc-1",
          restoredVersionId: "version-restore-1",
          currentVersionId: "version-restore-1",
          finalState: "Completed",
        },
      };
    case "search_documents":
      return {
        ok: true,
        data: { queryName: "search-documents", workspaceId: "workspace-1", text: "needle", results: [] },
      };
    case "search_assets":
      return {
        ok: true,
        data: {
          queryName: "search-assets",
          workspaceId: "workspace-1",
          text: "attachment needle",
          results: [
            {
              assetId: "asset-1",
              fileName: "source.pdf",
              mediaType: "application/pdf",
              byteSize: 1024,
              score: 3,
            },
          ],
        },
      };
    case "get_link_overview":
      return {
        ok: true,
        data: {
          queryName: "get-link-overview",
          workspaceId: "workspace-1",
          documentId: "doc-1",
          backlinks: [],
          unresolvedLinks: [],
          orphanDocuments: [],
        },
      };
    case "get_graph_projection":
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
    case "list_document_assets":
      return {
        ok: true,
        data: {
          queryName: "list-document-assets",
          workspaceId: "workspace-1",
          documentId: "doc-1",
          assets: [],
        },
      };
    default:
      return { ok: false, errorCode: "COMMAND_BRIDGE_FAILED", retryable: false };
  }
}
