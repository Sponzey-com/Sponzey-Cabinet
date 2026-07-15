import assert from "node:assert/strict";
import test from "node:test";

import type { CurrentDocumentView, DocumentHistoryPage } from "../../../packages/client-core/src/index.ts";
import {
  type DesktopDocumentEditCommand,
  type DesktopLocalWorkspaceFacade,
  getDesktopLocalCurrentDocument,
  listDesktopLocalDocumentHistory,
  saveDesktopLocalCurrentDocument,
} from "../src/index.ts";

test("desktop local facade saves current document and reports version append", async () => {
  const facade = new FakeLocalWorkspaceFacade();
  const result = await saveDesktopLocalCurrentDocument(editCommand(), facade);

  assert.deepEqual(facade.calls, ["saveCurrentDocument"]);
  assert.equal(result.status, "saved-local");
  assert.equal(result.versionAppended, true);
  assert.equal(result.currentVersionId, "version-2");
  assert.equal(JSON.stringify(result).includes(editCommand().body), false);
});

test("desktop local facade separates current read and history list calls", async () => {
  const facade = new FakeLocalWorkspaceFacade();
  const current = await getDesktopLocalCurrentDocument(
    { workspaceId: "workspace-1", documentId: "doc-1" },
    facade,
  );
  const history = await listDesktopLocalDocumentHistory(
    { workspaceId: "workspace-1", documentId: "doc-1", limit: 20 },
    facade,
  );

  assert.deepEqual(facade.calls, ["getCurrentDocument", "listDocumentHistory"]);
  assert.equal(current.queryName, undefined);
  assert.equal(history.entries[0]?.versionId, "version-1");
  assert.equal(JSON.stringify(history).includes(current.body), false);
});

function editCommand(): DesktopDocumentEditCommand {
  return {
    workspaceId: "workspace-1",
    documentId: "doc-1",
    title: "Source Document",
    path: "docs/source.md",
    body: "raw document body should not be returned from save result",
    expectedVersionId: "version-1",
  };
}

class FakeLocalWorkspaceFacade implements DesktopLocalWorkspaceFacade {
  readonly calls: string[] = [];

  async saveCurrentDocument(): Promise<{
    readonly status: "saved-local";
    readonly documentId: string;
    readonly currentVersionId: string;
    readonly versionAppended: true;
  }> {
    this.calls.push("saveCurrentDocument");
    return {
      status: "saved-local",
      documentId: "doc-1",
      currentVersionId: "version-2",
      versionAppended: true,
    };
  }

  async getCurrentDocument(): Promise<CurrentDocumentView> {
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

  async listDocumentHistory(): Promise<DocumentHistoryPage> {
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
}
