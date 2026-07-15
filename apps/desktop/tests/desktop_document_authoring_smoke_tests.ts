import assert from "node:assert/strict";
import test from "node:test";

import type { CurrentDocumentView, DocumentHistoryPage } from "../../../packages/client-core/src/index.ts";
import { createDesktopDocumentAuthoringWorkspace } from "../src/index.ts";

test("desktop document authoring smoke exposes split source and preview mode", () => {
  const workspace = createDesktopDocumentAuthoringWorkspace(currentDocument(), historyPage());
  const table = workspace.preview.blocks.find((block) => block.kind === "table");

  assert.equal(workspace.mode, "document-authoring-workspace");
  assert.equal(workspace.viewMode, "split");
  assert.equal(workspace.current.queryName, "get-current-document");
  assert.equal(workspace.history.queryName, "get-document-history");
  assert.equal(table?.kind, "table");
  assert.equal(JSON.stringify(workspace).includes("provider_api_key_fixture"), false);
});

function currentDocument(): CurrentDocumentView {
  return {
    workspaceId: "workspace-1",
    documentId: "doc-1",
    title: "Source Document",
    path: "docs/source.md",
    body: [
      "# Source Document",
      "",
      "| 항목 | 내용 | 상태 |",
      "| :--- | :---: | ---: |",
      "| 1번 그리드 | 좌측 정렬 | 우측 정렬 |",
    ].join("\n"),
    versionId: "version-current",
  };
}

function historyPage(): DocumentHistoryPage {
  return {
    workspaceId: "workspace-1",
    documentId: "doc-1",
    entries: [
      {
        versionId: "version-1",
        summary: "Created document",
        author: "local-user",
        createdAt: "2026-07-09T00:00:00Z",
      },
    ],
  };
}
