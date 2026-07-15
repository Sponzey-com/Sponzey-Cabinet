import assert from "node:assert/strict";
import test from "node:test";

import type {
  CurrentDocumentQuery,
  CurrentDocumentView,
  DocumentHistoryPage,
  DocumentHistoryQuery,
  LocalDesktopCommandClient,
  SaveCurrentDocumentCommand,
  SaveCurrentDocumentResult,
} from "../../../packages/client-core/src/index.ts";
import {
  createDesktopLocalCommandWorkspaceFacade,
  getDesktopLocalCurrentDocument,
  listDesktopLocalDocumentHistory,
  saveDesktopLocalCurrentDocument,
  type DesktopDocumentEditCommand,
} from "../src/index.ts";

test("desktop local command facade adapts existing helpers to command client", async () => {
  const client = new FakeLocalDesktopCommandClient();
  const facade = createDesktopLocalCommandWorkspaceFacade(client);

  const saveResult = await saveDesktopLocalCurrentDocument(editCommand(), facade);
  const current = await getDesktopLocalCurrentDocument(
    { workspaceId: "workspace-1", documentId: "doc-1" },
    facade,
  );
  const history = await listDesktopLocalDocumentHistory(
    { workspaceId: "workspace-1", documentId: "doc-1", limit: 20 },
    facade,
  );

  assert.deepEqual(client.calls, [
    "saveCurrentDocument",
    "getCurrentDocument",
    "listDocumentHistory",
  ]);
  assert.equal(saveResult.status, "saved-local");
  assert.equal(current.versionId, "version-2");
  assert.equal(history.entries[0]?.versionId, "version-1");
  assert.equal(JSON.stringify(saveResult).includes(editCommand().body), false);
});

function editCommand(): DesktopDocumentEditCommand {
  return {
    workspaceId: "workspace-1",
    documentId: "doc-1",
    title: "Source Document",
    path: "docs/source.md",
    body: "raw body stays in command payload",
    expectedVersionId: "version-1",
  };
}

class FakeLocalDesktopCommandClient implements LocalDesktopCommandClient {
  readonly calls: string[] = [];

  async openDefaultWorkspace() {
    this.calls.push("openDefaultWorkspace");
    return { workspaceId: "workspace-1", displayName: "Personal Workspace", setupHealth: "Ready" as const };
  }

  async getCurrentDocument(_query: CurrentDocumentQuery): Promise<CurrentDocumentView> {
    this.calls.push("getCurrentDocument");
    return {
      workspaceId: "workspace-1",
      documentId: "doc-1",
      title: "Source Document",
      path: "docs/source.md",
      body: "current body",
      versionId: "version-2",
    };
  }

  async saveCurrentDocument(_command: SaveCurrentDocumentCommand): Promise<SaveCurrentDocumentResult> {
    this.calls.push("saveCurrentDocument");
    return {
      status: "saved-local",
      workspaceId: "workspace-1",
      documentId: "doc-1",
      currentVersionId: "version-2",
      versionAppended: true,
    };
  }

  async listDocumentHistory(_query: DocumentHistoryQuery): Promise<DocumentHistoryPage> {
    this.calls.push("listDocumentHistory");
    return {
      workspaceId: "workspace-1",
      documentId: "doc-1",
      entries: [
        {
          versionId: "version-1",
          summary: "Created",
          author: "local-user",
          createdAt: "2026-07-09T00:00:00Z",
        },
      ],
    };
  }

  async searchDocuments() {
    this.calls.push("searchDocuments");
    return { queryName: "search-documents" as const, workspaceId: "workspace-1", text: "needle", results: [] };
  }

  async getLinkOverview() {
    this.calls.push("getLinkOverview");
    return {
      queryName: "get-link-overview" as const,
      workspaceId: "workspace-1",
      documentId: "doc-1",
      backlinks: [],
      unresolvedLinks: [],
      orphanDocuments: [],
    };
  }

  async getAssetMetadata() {
    this.calls.push("getAssetMetadata");
    return {
      queryName: "list-document-assets" as const,
      workspaceId: "workspace-1",
      documentId: "doc-1",
      assets: [],
    };
  }
}
