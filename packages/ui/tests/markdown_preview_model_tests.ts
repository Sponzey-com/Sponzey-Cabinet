import assert from "node:assert/strict";
import test from "node:test";

import type { CurrentDocumentView, DocumentHistoryPage } from "../../client-core/src/index.ts";
import {
  createDocumentReadingWorkspaceModel,
  createMarkdownPreviewModel,
} from "../src/index.ts";

test("markdown preview renders table grid while source remains markdown text", () => {
  const source = [
    "# Source Document",
    "",
    "| 항목 | 내용 | 상태 |",
    "| :--- | :---: | ---: |",
    "| 1번 그리드 | 좌측 정렬 | 우측 정렬 |",
    "| 2번 그리드 | 가운데 정렬 | 우측 정렬 |",
  ].join("\n");

  const preview = createMarkdownPreviewModel({
    documentId: "doc-1",
    versionId: "version-current",
    source,
  });
  const table = preview.blocks.find((block) => block.kind === "table");

  assert.equal(preview.state, "Rendered");
  assert.equal(preview.sourceMode, "markdown-source");
  assert.equal(table?.kind, "table");
  assert.deepEqual(table?.headers, ["항목", "내용", "상태"]);
  assert.deepEqual(table?.alignments, ["left", "center", "right"]);
  assert.deepEqual(table?.rows, [
    ["1번 그리드", "좌측 정렬", "우측 정렬"],
    ["2번 그리드", "가운데 정렬", "우측 정렬"],
  ]);
  assert.equal(JSON.stringify(preview).includes("<table"), false);
});

test("markdown preview exposes wikilink and asset reference actions without unsafe html", () => {
  const assetId = "a".repeat(64);
  const preview = createMarkdownPreviewModel({
    documentId: "doc-1",
    versionId: "version-current",
    source: [
      "This links to [[Target Document|Target]] and ![[asset:" + assetId + "|Diagram]].",
      "<script>alert('x')</script>",
      "<img src=x onerror=alert('x')>",
      "provider_api_key_fixture",
    ].join("\n"),
  });
  const serialized = JSON.stringify(preview);
  const paragraph = preview.blocks.find((block) => block.kind === "paragraph");

  assert.equal(preview.state, "Rendered");
  assert.equal(paragraph?.kind, "paragraph");
  assert.deepEqual(paragraph?.inlineActions.map((action) => action.kind), [
    "open-wikilink",
    "open-asset-reference",
  ]);
  assert.equal(paragraph?.inlineActions[0]?.label, "Target");
  assert.equal(paragraph?.inlineActions[1]?.assetId, assetId);
  assert.equal(serialized.includes("<script"), false);
  assert.equal(serialized.includes("onerror"), false);
  assert.equal(serialized.includes("provider_api_key_fixture"), false);
  assert.equal(serialized.includes("<img"), false);
});

test("document reading workspace keeps current and history query paths separated", () => {
  const current: CurrentDocumentView = {
    workspaceId: "workspace-1",
    documentId: "doc-1",
    title: "Source Document",
    path: "docs/source.md",
    body: "current body should only belong to current/editor/preview surface",
    versionId: "version-current",
  };
  const history: DocumentHistoryPage = {
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

  const workspace = createDocumentReadingWorkspaceModel(current, history);

  assert.equal(workspace.current.queryName, "get-current-document");
  assert.equal(workspace.history.queryName, "get-document-history");
  assert.equal(workspace.preview.sourceVersionId, "version-current");
  assert.equal(workspace.querySeparation.currentReadQueryName, "get-current-document");
  assert.equal(workspace.querySeparation.historyReadQueryName, "get-document-history");
  assert.equal(JSON.stringify(workspace.history).includes(current.body), false);
  assert.equal(workspace.history.entries[0]?.versionId, "version-1");
});
