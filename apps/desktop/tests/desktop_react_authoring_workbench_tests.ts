import assert from "node:assert/strict";
import test from "node:test";
import { renderToStaticMarkup } from "react-dom/server";

import { DocumentSaveCoordinatorState } from "@sponzey-cabinet/ui";

import { createDesktopDocumentAuthoringWorkbenchElement } from "../src/react_document_authoring_workbench.ts";

test("React authoring workbench renders split source preview table and semantic controls", () => {
  const markup = renderToStaticMarkup(
    createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks(), {
      viewMode: "split",
    }),
  );

  assert.match(markup, /data-cabinet-authoring-state="Dirty"/);
  assert.match(markup, /data-design-reference="penpot-20260713"/);
  assert.match(markup, />Cabinet</);
  assert.match(markup, /내 지식 지도/);
  assert.match(markup, /연결된 문서/);
  assert.match(markup, /aria-label="편집 화면"/);
  assert.match(markup, /data-editor-mode="source"/);
  assert.match(markup, /data-editor-mode="split"/);
  assert.match(markup, /data-editor-mode="preview"/);
  assert.match(markup, /aria-pressed="true"[^>]*>나란히/);
  assert.match(markup, /aria-label="Markdown 원문"/);
  assert.match(markup, /data-codemirror-host="pending"/);
  assert.match(markup, /aria-label="Markdown 미리보기"/);
  assert.match(markup, /<table/);
  assert.match(markup, /<th[^>]*>Item/);
  assert.match(markup, /<td[^>]*>Grid/);
  assert.match(markup, /저장되지 않음/);
  assert.match(markup, /data-action="navigate-graph"[^>]*disabled/);
  assert.match(markup, /class="sidebar-current-document"[^>]*aria-current="page"/);
  assert.doesNotMatch(markup, /data-action="(?:open-settings|toggle-theme|open-ai)"/);
  assert.match(markup, /data-action="open-authoring-graph"[^>]*disabled/);
  assert.match(markup, /연결된 문서가 없습니다/);
  assert.equal((markup.match(/class="desktop-sidebar"/g) ?? []).length, 1);
  assert.equal((markup.match(/class="desktop-topbar"/g) ?? []).length, 1);
  assert.equal((markup.match(/<main/g) ?? []).length, 1);
  assert.doesNotMatch(markup, /로컬 우선 원칙|검색 인덱스|백업과 복원/);
  assert.doesNotMatch(markup, /server|tenant|billing|admin-console/i);
  assertNoUnidentifiedInteractiveControls(markup);
});

function assertNoUnidentifiedInteractiveControls(markup: string): void {
  const controls = markup.match(/<(?:button|input|select|textarea|a)\b[^>]*>/g) ?? [];
  assert.ok(controls.length > 0);
  assert.deepEqual(controls.filter((control) => !control.includes("data-action=")), []);
}

test("React authoring workbench supports source preview modes and escapes unsafe source", () => {
  const unsafe = {
    ...snapshot(),
    body: "# Safe\n\n<script>globalThis.compromised=true</script>",
  };
  const source = renderToStaticMarkup(
    createDesktopDocumentAuthoringWorkbenchElement(unsafe, callbacks(), { viewMode: "source" }),
  );
  const preview = renderToStaticMarkup(
    createDesktopDocumentAuthoringWorkbenchElement(unsafe, callbacks(), { viewMode: "preview" }),
  );

  assert.match(source, /aria-label="Markdown 원문"/);
  assert.doesNotMatch(source, /aria-label="Markdown 미리보기"/);
  assert.match(preview, /aria-label="Markdown 미리보기"/);
  assert.doesNotMatch(preview, /data-codemirror-host/);
  assert.doesNotMatch(preview, /<script>/);
  assert.doesNotMatch(preview, /globalThis\.compromised/);
});

test("React authoring workbench exposes retry and read-only recovery controls without raw error", () => {
  const failed = renderToStaticMarkup(
    createDesktopDocumentAuthoringWorkbenchElement(
      {
        ...snapshot(),
        saveState: DocumentSaveCoordinatorState.SaveFailed,
        errorCode: "DOCUMENT_AUTHORING_STORAGE_UNAVAILABLE",
        retryable: true,
      },
      callbacks(),
    ),
  );
  const readOnly = renderToStaticMarkup(
    createDesktopDocumentAuthoringWorkbenchElement(
      {
        ...snapshot(),
        saveState: DocumentSaveCoordinatorState.ReadOnlyRecovery,
        errorCode: "DOCUMENT_AUTHORING_POINTER_UPDATE_FAILED",
        repairRequired: true,
      },
      callbacks(),
    ),
  );
  const closeBlocked = renderToStaticMarkup(
    createDesktopDocumentAuthoringWorkbenchElement(
      {
        ...snapshot(),
        saveState: DocumentSaveCoordinatorState.CloseBlocked,
      },
      callbacks(),
    ),
  );

  assert.match(failed, /role="alert"/);
  assert.match(failed, /data-workspace-global-host[^>]*>[\s\S]*role="alert"/);
  assert.match(failed, /다시 시도/);
  assert.match(readOnly, /읽기 전용 복구/);
  assert.match(readOnly, /변경 취소/);
  assert.match(closeBlocked, /저장하지 않은 변경/);
  assert.match(closeBlocked, /저장 다시 시도/);
  assert.match(closeBlocked, /변경 취소/);
  assert.match(closeBlocked, /계속 편집/);
  assert.doesNotMatch(`${failed}${readOnly}`, /DOCUMENT_AUTHORING_STORAGE_UNAVAILABLE|DOCUMENT_AUTHORING_POINTER_UPDATE_FAILED/);
  assert.equal(
    readOnly.includes("body one") && readOnly.includes("DOCUMENT_AUTHORING"),
    false,
  );
});

test("React authoring maps link and history failures without raw codes or English controls", () => {
  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks(), {
    links: { state: "Failed", workspaceId: "workspace-1", documentId: "doc-1", generation: 2, errorCode: "LINK_OVERVIEW_UNAVAILABLE", retryable: false },
    history: { status: "Failed", entries: [], errorCode: "DOCUMENT_HISTORY_QUERY_FAILED" },
  }));

  assert.match(markup, /연결된 문서를 불러오지 못했습니다/);
  assert.match(markup, /문서 이력/);
  assert.match(markup, /이력 불러오기/);
  assert.match(markup, /문서 이력을 불러오지 못했습니다/);
  assert.doesNotMatch(markup, /LINK_OVERVIEW_UNAVAILABLE|DOCUMENT_HISTORY_QUERY_FAILED|Load history|History/);
});

test("React authoring workbench renders real backlink identities and bounded link state", () => {
  const markup = renderToStaticMarkup(
    createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks(), {
      links: {
        state: "Ready",
        workspaceId: "workspace-1",
        documentId: "doc-1",
        generation: 1,
        panel: {
          mode: "links",
          queryName: "get-link-overview",
          workspaceId: "workspace-1",
          documentId: "doc-1",
          backlinks: [{
            workspaceId: "workspace-1",
            sourceDocumentId: "doc-linked",
            targetDocumentId: "doc-1",
            sourceTitle: "Linked architecture",
            sourcePath: "fixture/linked",
          }],
          unresolvedLinks: [],
          orphanDocuments: [],
        },
      },
    }),
  );

  assert.match(markup, /data-link-overview-state="Ready"/);
  assert.match(markup, /data-linked-document-id="doc-linked"/);
  assert.match(markup, /Linked architecture/);
  assert.doesNotMatch(markup, /연결된 문서가 없습니다/);
});

test("React authoring presents the derived title without a separate metadata editor", () => {
  const focused = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks(), {
  }));
  assert.match(focused, /class="authoring-title-display"/);
  assert.match(focused, />Source</);
  assert.doesNotMatch(focused, /data-action="edit-document-title"/);
  assert.doesNotMatch(focused, /authoring-title-input/);
  assert.doesNotMatch(focused, /notes\/source\.md/);
});

test("React authoring history renders user labels without visible version identity", () => {
  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks(), {
    history: {
      status: "Ready",
      entries: [{
        versionId: "internal-version-secret",
        versionLabel: "버전 1",
        createdAtLabel: "2026. 7. 15. 오후 3:00",
        summaryLabel: "문서 저장",
      }],
    },
  }));
  const visible = markup.replace(/\sdata-version-id="[^"]*"/g, "");

  assert.match(visible, /버전 1/);
  assert.match(visible, /2026\. 7\. 15/);
  assert.match(visible, /문서 저장/);
  assert.doesNotMatch(visible, /internal-version-secret/);
});

function snapshot() {
  return {
    workspaceId: "workspace-1",
    documentId: "doc-1",
    title: "Source",
    path: "notes/source.md",
    body: [
      "# Source",
      "",
      "| Item | Value |",
      "| :--- | ---: |",
      "| Grid | Ready |",
    ].join("\n"),
    revision: 1,
    persistedRevision: 0,
    expectedVersionId: "v1",
    saveState: DocumentSaveCoordinatorState.Dirty,
  } as const;
}

function callbacks() {
  return {
    onHome() {},
    onMode() {},
    onBodyChange() {},
    onSave() {},
    onRetry() {},
    onDiscard() {},
    onCancel() {},
  };
}
