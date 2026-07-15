import assert from "node:assert/strict";
import test from "node:test";

import type { CurrentDocumentView, DocumentHistoryPage } from "../../client-core/src/index.ts";
import {
  DocumentEditorEvent,
  DocumentEditorState,
  DocumentEditorViewModeEvent,
  createDocumentAuthoringWorkspaceModel,
  createMarkdownPreviewModel,
  transitionDocumentEditorState,
  transitionDocumentEditorViewMode,
} from "../src/index.ts";

test("markdown preview renders heading anchor, callout, link resolution, and missing asset state", () => {
  const preview = createMarkdownPreviewModel({
    documentId: "doc-1",
    versionId: "version-current",
    source: [
      "# Source Document",
      "",
      "> [!note] 확인 필요",
      "> 콜아웃 본문",
      "",
      "Link to [[Target Document|Target]] and [[Missing Page]].",
      "![[asset:asset-1|MVP Asset]]",
    ].join("\n"),
    resolvedWikilinkTargets: ["Target Document"],
    availableAssetIds: [],
  });

  const heading = preview.blocks.find((block) => block.kind === "heading");
  const callout = preview.blocks.find((block) => block.kind === "callout");
  const paragraph = preview.blocks.find(
    (block) => block.kind === "paragraph" && block.inlineActions.length === 2,
  );
  const assetParagraph = preview.blocks.find(
    (block) => block.kind === "paragraph" && block.inlineActions.some((action) => action.kind === "open-asset-reference"),
  );

  assert.equal(heading?.kind, "heading");
  assert.equal(heading?.anchor, "source-document");
  assert.equal(callout?.kind, "callout");
  assert.equal(callout?.calloutType, "note");
  assert.equal(paragraph?.kind, "paragraph");
  assert.deepEqual(paragraph?.inlineActions.map((action) => action.resolutionState), [
    "resolved",
    "unresolved",
  ]);
  assert.equal(assetParagraph?.kind, "paragraph");
  assert.equal(assetParagraph?.inlineActions[0]?.assetState, "missing");
});

test("document editor view mode transitions between source preview and split explicitly", () => {
  const preview = transitionDocumentEditorViewMode("source", DocumentEditorViewModeEvent.ShowPreview);
  const split = transitionDocumentEditorViewMode(preview.mode, DocumentEditorViewModeEvent.ShowSplit);
  const source = transitionDocumentEditorViewMode(split.mode, DocumentEditorViewModeEvent.ShowSource);

  assert.equal(preview.mode, "preview");
  assert.equal(split.mode, "split");
  assert.equal(source.mode, "source");
});

test("document authoring workspace keeps current history paths split and defaults to split mode", () => {
  const workspace = createDocumentAuthoringWorkspaceModel(currentDocument(), historyPage());

  assert.equal(workspace.mode, "document-authoring-workspace");
  assert.equal(workspace.viewMode, "split");
  assert.equal(workspace.current.queryName, "get-current-document");
  assert.equal(workspace.history.queryName, "get-document-history");
  assert.equal(workspace.querySeparation.currentReadQueryName, "get-current-document");
  assert.equal(workspace.querySeparation.historyReadQueryName, "get-document-history");
  assert.equal(JSON.stringify(workspace.history).includes(currentDocument().body), false);
});

test("document editor state machine marks dirty content and save success explicitly", () => {
  const loaded = transitionDocumentEditorState(DocumentEditorState.Loading, {
    type: DocumentEditorEvent.DocumentLoaded,
    currentVersionId: "version-current",
  });
  const dirty = transitionDocumentEditorState(loaded, {
    type: DocumentEditorEvent.ContentChanged,
    dirtyContentRef: "draft-doc-1",
  });
  const saving = transitionDocumentEditorState(dirty, {
    type: DocumentEditorEvent.SaveRequested,
  });
  const saved = transitionDocumentEditorState(saving, {
    type: DocumentEditorEvent.SaveSucceeded,
    savedVersionId: "version-next",
  });

  assert.equal(loaded.state, DocumentEditorState.ReadyClean);
  assert.equal(dirty.state, DocumentEditorState.ReadyDirty);
  assert.equal(dirty.dirtyContentRef, "draft-doc-1");
  assert.equal(saving.state, DocumentEditorState.Saving);
  assert.equal(saved.state, DocumentEditorState.Saved);
  assert.equal(saved.savedVersionId, "version-next");
});

test("document editor state machine returns stable error code for invalid transitions and save failure", () => {
  const invalid = transitionDocumentEditorState(DocumentEditorState.Loading, {
    type: DocumentEditorEvent.SaveRequested,
  });
  const failed = transitionDocumentEditorState(
    {
      state: DocumentEditorState.Saving,
      dirtyContentRef: "draft-doc-1",
    },
    {
      type: DocumentEditorEvent.SaveFailed,
      errorCode: "STORE_UNAVAILABLE",
    },
  );
  const retry = transitionDocumentEditorState(failed, {
    type: DocumentEditorEvent.SaveRequested,
  });

  assert.equal(invalid.state, DocumentEditorState.Loading);
  assert.equal(invalid.errorCode, "DOCUMENT_EDITOR_INVALID_TRANSITION");
  assert.equal(failed.state, DocumentEditorState.SaveFailed);
  assert.equal(failed.errorCode, "STORE_UNAVAILABLE");
  assert.equal(retry.state, DocumentEditorState.Saving);
});

function currentDocument(): CurrentDocumentView {
  return {
    workspaceId: "workspace-1",
    documentId: "doc-1",
    title: "Source Document",
    path: "docs/source.md",
    body: "# Source Document\n\n| 항목 | 내용 |\n| --- | --- |\n| 1 | 테스트 |",
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
