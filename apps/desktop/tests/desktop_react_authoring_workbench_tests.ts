import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";

import { DocumentSaveCoordinatorState } from "@sponzey-cabinet/ui";

import { createDesktopDocumentAuthoringWorkbenchElement } from "../src/react_document_authoring_workbench.ts";
import {
  applyDesktopAssetResult,
  createDesktopAssetSnapshot,
  requestDesktopAssetLoad,
  requestDesktopWorkspaceAssetLoad,
} from "../src/desktop_asset_controller.ts";
import {
  applyAttachmentFileStatus,
  createAttachmentFileSnapshot,
} from "../src/attachment_operation_presenter.ts";
import {
  applyDesktopGraphResult,
  createDesktopGraphSnapshot,
  requestDesktopGraphLoad,
  selectDesktopGraphNode,
} from "../src/desktop_graph_controller.ts";

test("React authoring keeps the global workspace search available", () => {
  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(
    snapshot(),
    callbacks(),
  ));

  assert.match(markup, /data-action="workspace-search-input"/);
  assert.match(markup, /data-action="submit-workspace-search"/);
  assert.match(markup, /data-action="workspace-search-input"[^>]*aria-label="검색"/);
  assert.match(markup, /data-action="submit-workspace-search"[^>]*aria-label="검색"/);
  assert.match(markup, /placeholder="문서와 첨부 파일 검색"/);
  assert.equal((markup.match(/role="search"/g) ?? []).length, 1);
});

test("React authoring search button uses an explicit click handler instead of submit-only behavior", async () => {
  const source = await readFile(new URL("../src/react_document_authoring_workbench.ts", import.meta.url), "utf8");

  assert.match(source, /type:\s*"button",[\s\S]{0,260}"data-action":\s*"submit-workspace-search"[\s\S]{0,260}onClick:/);
  assert.match(source, /event\.currentTarget\.form/);
});

test("React authoring source keeps legacy editor modes out of the desktop boundary", async () => {
  const workbench = await readFile(new URL("../src/react_document_authoring_workbench.ts", import.meta.url), "utf8");
  const entry = await readFile(new URL("../src/desktop_entry.ts", import.meta.url), "utf8");
  const smoke = await readFile(new URL("../src/packaged_ui_smoke.ts", import.meta.url), "utf8");
  const manifest = await readFile(new URL("../src/core_ui_action_manifest.ts", import.meta.url), "utf8");

  assert.doesNotMatch(workbench, /DocumentEditorViewMode|viewMode|renderPreviewBlock|createMarkdownPreviewModel/);
  assert.doesNotMatch(entry, /editorViewMode|setEditorViewMode|DocumentEditorViewMode/);
  for (const source of [workbench, entry, smoke, manifest]) {
    assert.doesNotMatch(source, /authoring-mode-(?:source|split|preview)/);
  }
});

test("React authoring WYSIWYG edits flow through the synchronization session guard", async () => {
  const workbench = await readFile(new URL("../src/react_document_authoring_workbench.ts", import.meta.url), "utf8");

  assert.match(workbench, /applyWysiwygPatchToSyncSession/);
  assert.match(workbench, /createWysiwygPlainTextSyncSession/);
  assert.match(workbench, /baseRevision:\s*revision/);
  assert.doesNotMatch(workbench, /callbacks\.onBodyChange\(result\.nextBody\)/);
});

test("React authoring exposes Penpot 20260721 formatting commands in icon order", () => {
  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(
    snapshot(),
    callbacks(),
  ));
  const actions = [...markup.matchAll(/data-action="format-([^"]+)"/g)].map((match) => match[1]);

  assert.deepEqual(actions, ["heading", "bold", "italic", "link", "list", "checklist", "table"]);
  for (const [action, label] of [
    ["heading", "제목"],
    ["bold", "굵게"],
    ["italic", "기울임"],
    ["link", "링크"],
    ["list", "목록"],
    ["checklist", "체크리스트"],
    ["table", "표"],
  ]) {
    assert.match(markup, new RegExp(`data-action="format-${action}"[^>]*aria-label="${label}"`));
  }

  const toolbar = markup.match(/<div class="formatting-toolbar"[\s\S]*?<\/div>/)?.[0] ?? "";
  assert.doesNotMatch(toolbar, />제목<|>굵게<|>기울임<|>링크<|>목록<|>체크리스트<|>표</);
});

test("React authoring formatting commands dispatch through an explicit callback boundary", () => {
  const commands: string[] = [];
  const tree = renderFunctionElement(createDesktopDocumentAuthoringWorkbenchElement(
    snapshot(),
    { ...callbacks(), onFormatCommand(command) { commands.push(command); } },
  ));

  clickElement(tree, (props) => props["data-action"] === "format-bold");
  clickElement(tree, (props) => props["data-action"] === "format-table");

  assert.deepEqual(commands, ["bold", "table"]);
});

test("React authoring workbench renders the current document local graph instead of decorative topology", () => {
  const loading = requestDesktopGraphLoad(createDesktopGraphSnapshot("workspace-1"), {
    centerDocumentId: "doc-1",
    scope: "local",
  });
  const graph = applyDesktopGraphResult(loading, loading.generation, {
    centerDocumentId: "doc-1",
    status: "clean",
    nodes: [
      { id: "doc-1", kind: "document", label: "Source", availability: "available", canNavigate: true },
      { id: "doc-2", kind: "document", label: "연결된 설계", availability: "available", canNavigate: true },
    ],
    edges: [{ id: "edge-1", sourceId: "doc-1", targetId: "doc-2", kind: "document_link" }],
    stats: { candidateCount: 2, filteredCount: 0 },
    freshnessRevision: "version-1",
  });
  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(
    snapshot(),
    {
      ...callbacks(),
      onGraph() {},
      onLocalGraphNodeSelect() {},
      onLocalGraphRetry() {},
      onLocalGraphRepair() {},
      onOpenLinkedDocument() {},
    },
    { graph },
  ));

  assert.match(markup, /data-authoring-local-graph-state="Ready"/);
  assert.match(markup, /data-topology-renderer-host="accelerated"/);
  assert.match(markup, /data-topology-semantic-list="available"/);
  assert.match(markup, />연결된 설계</);
  assert.match(markup, /data-action="select-graph-node"/);
  assert.match(markup, /data-action="open-graph-document"/);
  assert.doesNotMatch(markup, /map-dot|map-spoke|authoring-map-preview/);
  assert.doesNotMatch(markup, /notes\/source\.md|version-1/);
});

test("React authoring local graph controls dispatch bounded query, recenter, and attachment actions", () => {
  const loading = requestDesktopGraphLoad(createDesktopGraphSnapshot("workspace-1"), {
    centerDocumentId: "doc-1",
    scope: "local",
  });
  const ready = applyDesktopGraphResult(loading, loading.generation, {
    centerDocumentId: "doc-1",
    status: "clean",
    nodes: [
      { id: "doc-1", kind: "document", label: "Source", availability: "available", canNavigate: true },
      { id: "doc-2", kind: "document", label: "연결된 설계", availability: "available", canNavigate: true },
      { id: "asset-1", kind: "attachment", label: "설계 자료", availability: "available", canNavigate: true },
    ],
    edges: [
      { id: "edge-1", sourceId: "doc-1", targetId: "doc-2", kind: "document_link" },
      { id: "edge-2", sourceId: "doc-1", targetId: "asset-1", kind: "attachment_reference" },
    ],
    stats: { candidateCount: 3, filteredCount: 0 },
    freshnessRevision: "version-1",
  });
  const patches: unknown[] = [];
  let openedAsset: string | undefined;
  const baseCallbacks = {
    ...callbacks(),
    onGraph() {},
    onLocalGraphNodeSelect() {},
    onLocalGraphRetry() {},
    onLocalGraphRepair() {},
    onLocalGraphQuery(patch: unknown) { patches.push(patch); },
    onOpenLocalGraphAsset(assetId: string) { openedAsset = assetId; },
    onOpenLinkedDocument() {},
  };
  const readyTree = renderFunctionElement(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), baseCallbacks, { graph: ready }));

  clickElement(readyTree, (props) => props["data-action"] === "authoring-graph-depth-2");
  clickElement(readyTree, (props) => props["data-action"] === "authoring-graph-direction-outgoing");
  clickElement(readyTree, (props) => props["data-action"] === "authoring-graph-toggle-assets");
  assert.deepEqual(patches, [{ depth: 2 }, { direction: "outgoing" }, { includeAssets: true }]);

  const documentTree = renderFunctionElement(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), baseCallbacks, {
    graph: selectDesktopGraphNode(ready, "doc-2"),
  }));
  clickElement(documentTree, (props) => props["data-action"] === "recenter-authoring-graph");
  assert.deepEqual(patches.at(-1), { scope: "local", centerDocumentId: "doc-2" });

  const assetTree = renderFunctionElement(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), baseCallbacks, {
    graph: selectDesktopGraphNode(ready, "asset-1"),
  }));
  clickElement(assetTree, (props) => props["data-action"] === "open-authoring-graph-asset");
  assert.equal(openedAsset, "asset-1");
  const assetMarkup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), baseCallbacks, {
    graph: selectDesktopGraphNode(ready, "asset-1"),
  }));
  assert.match(assetMarkup, /설계 자료/);
  assert.match(assetMarkup, /들어오는 연결/);
  assert.doesNotMatch(assetMarkup, />asset-1</);
});

test("React authoring local graph uses the controlled shared visual search", () => {
  const loading = requestDesktopGraphLoad(createDesktopGraphSnapshot("workspace-1"), { centerDocumentId: "doc-1" });
  const graph = applyDesktopGraphResult(loading, loading.generation, {
    centerDocumentId: "doc-1", status: "clean",
    nodes: [
      { id: "doc-1", kind: "document", label: "Source", availability: "available", canNavigate: true },
      { id: "doc-2", kind: "document", label: "설계 결정", availability: "available", canNavigate: true },
    ],
    edges: [{ id: "edge-1", sourceId: "doc-1", targetId: "doc-2", kind: "document_link" }],
    stats: { candidateCount: 2, filteredCount: 0 }, freshnessRevision: "version-1",
  });
  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), {
    ...callbacks(), onGraph() {}, onLocalGraphVisualSearch() {},
  }, { graph, graphVisualSearch: "설계" }));

  assert.match(markup, /data-action="search-authoring-graph"/);
  assert.match(markup, /aria-label="이 문서의 지식 지도 검색"/);
  assert.match(markup, /value="설계"/);
  assert.match(markup, /설계 결정/);
  assert.equal((markup.match(/data-action="select-graph-node"/g) ?? []).length, 1);
  assert.doesNotMatch(markup, /<strong>Source<\/strong><small>문서<\/small>/);
});

test("React authoring workbench renders split source preview table and semantic controls", () => {
  const assetLoading = requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1");
  const assets = applyDesktopAssetResult(assetLoading, assetLoading.generation, {
    queryName: "list-document-assets",
    workspaceId: "workspace-1",
    documentId: "doc-1",
    assets: [{ assetId: "internal-asset-id", label: "설계 자료", fileName: "architecture.pdf", mediaType: "application/pdf", byteSize: 2048, status: "available" }],
  });
  const markup = renderToStaticMarkup(
    createDesktopDocumentAuthoringWorkbenchElement(snapshot(), {
      ...callbacks(),
      onAssetImport() {},
      onAssetSelect() {},
      onAssetPreview() {},
      onAssetPreviewClose() {},
      onAssetOpen() {},
      onAssetUnlink() {},
      onAssetRetry() {},
    }, {
      viewMode: "split",
      assets,
      inspector: { tab: "attachments", unlink: { status: "Closed" } },
      documentShortcuts: [{
        label: "고정된 최근 문서",
        actionId: "open-sidebar-document",
        onOpen() {},
      }],
    }),
  );

  assert.match(markup, /data-cabinet-authoring-state="Dirty"/);
  assert.match(markup, /data-design-reference="penpot-20260721"/);
  assert.match(markup, />Cabinet</);
  assert.match(markup, />프로젝트 \/ Cabinet</);
  assert.doesNotMatch(markup, /내 캐비닛 \/ 프로젝트 \/ Cabinet/);
  assert.match(markup, /이 문서의 지식 지도/);
  assert.match(markup, />연결<\/button>/);
  assert.match(markup, /첨부 파일/);
  assert.match(markup, /role="tablist"/);
  assert.match(markup, /role="tab"[^>]*aria-selected="true"[^>]*data-action="select-document-inspector-attachments"/);
  assert.match(markup, /architecture\.pdf/);
  assert.match(markup, /data-action="import-document-asset"(?![^>]*disabled)/);
  assert.match(markup, /data-action="select-document-asset"/);
  assert.match(markup, /data-action="open-document-asset-externally"/);
  assert.match(markup, /기본 앱으로 열기/);
  assert.doesNotMatch(markup.replace(/\sdata-asset-id="[^"]*"/g, ""), /internal-asset-id/);
  assert.match(markup, /aria-label="편집 화면"/);
  assert.match(markup, /data-editor-surface="wysiwyg"/);
  assert.match(markup, /data-action="open-plain-text-editor"/);
  assert.doesNotMatch(markup, /data-editor-mode="(?:source|split|preview)"/);
  assert.doesNotMatch(markup, /data-action="authoring-mode-(?:source|split|preview)"/);
  assert.doesNotMatch(markup, /aria-label="Markdown 원문"/);
  assert.doesNotMatch(markup, /data-codemirror-host="pending"/);
  assert.doesNotMatch(markup, /aria-label="Markdown 미리보기"/);
  assert.match(markup, /<table/);
  assert.match(markup, /<th[^>]*>Item/);
  assert.match(markup, /<td[^>]*>Grid/);
  assert.match(markup, /저장되지 않음/);
  assert.match(markup, /data-action="navigate-graph"[^>]*disabled/);
  assert.match(markup, /data-action="open-sidebar-document"[^>]*>고정된 최근 문서<\/button>/);
  assert.doesNotMatch(markup, /data-action="(?:open-settings|toggle-theme|open-ai)"/);
  assert.match(markup, /data-action="open-authoring-graph"[^>]*disabled/);
  assert.doesNotMatch(markup, /연결된 문서가 없습니다|이력 불러오기/);
  assert.equal((markup.match(/class="desktop-sidebar"/g) ?? []).length, 1);
  assert.equal((markup.match(/class="desktop-topbar"/g) ?? []).length, 1);
  assert.equal((markup.match(/<main/g) ?? []).length, 1);
  assert.doesNotMatch(markup, /로컬 우선 원칙|검색 인덱스/);
  assert.doesNotMatch(markup, /server|tenant|billing|admin-console/i);
  assertNoUnidentifiedInteractiveControls(markup);
});

test("React authoring workbench exposes a WYSIWYG surface placeholder and plain text action contract", () => {
  const markup = renderToStaticMarkup(
    createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks()),
  );

  assert.match(markup, /data-editor-surface="wysiwyg"/);
  assert.match(markup, /aria-label="WYSIWYG 문서 편집"/);
  assert.match(markup, /data-action="open-plain-text-editor"/);
  assert.match(markup, /aria-label="Markdown 원문 편집"/);
  assert.match(markup, />원문 편집<\/button>/);
  assert.doesNotMatch(markup, /data-codemirror-host="pending"/);
  assert.doesNotMatch(markup, /aria-label="Markdown 미리보기"/);
  assert.doesNotMatch(markup, /authoring-mode-(?:source|split|preview)/);
});

test("React authoring workbench renders WYSIWYG blocks from the shared editor presentation model", () => {
  const document = {
    ...snapshot(),
    title: "첫번째 문서",
    body: [
      "# 첫번째 문서",
      "",
      "본문 첫 줄",
      "본문 둘째 줄",
      "",
      "- [ ] 정리 필요",
      "- [x] 정리 완료",
      "",
      "| Item | Value |",
      "| :--- | ---: |",
      "| Grid | Ready |",
    ].join("\n"),
  };
  const markup = renderToStaticMarkup(
    createDesktopDocumentAuthoringWorkbenchElement(document, callbacks()),
  );
  const surface = markup.match(/<section class="wysiwyg-document-surface"[\s\S]*?<\/section>/)?.[0] ?? "";

  assert.match(surface, /data-editor-surface="wysiwyg"/);
  assert.match(surface, /data-wysiwyg-block-type="heading"/);
  assert.match(surface, />첫번째 문서</);
  assert.match(surface, /data-wysiwyg-block-type="paragraph"/);
  assert.match(surface, />본문 첫 줄/);
  assert.match(surface, /data-wysiwyg-block-type="checklist"/);
  assert.match(surface, />정리 완료/);
  assert.match(surface, /data-wysiwyg-block-type="table"/);
  assert.match(surface, /<table/);
  assert.match(surface, /<th[^>]*>Item/);
  assert.doesNotMatch(surface, /WYSIWYG 편집 준비 중/);
  assert.doesNotMatch(markup, /data-codemirror-host="pending"/);
  assert.doesNotMatch(markup, /aria-label="Markdown 미리보기"/);
});

test("React authoring workbench opens a plain text editor modal contract without a separate body owner", () => {
  let opened = false;
  let closed = false;
  const tree = renderFunctionElement(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), {
    ...callbacks(),
    onOpenPlainTextEditor() { opened = true; },
    onClosePlainTextEditor() { closed = true; },
  }));

  clickElement(tree, (props) => props["data-action"] === "open-plain-text-editor");
  assert.equal(opened, true);

  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), {
    ...callbacks(),
    onOpenPlainTextEditor() {},
    onClosePlainTextEditor() { closed = true; },
  }, {
    viewMode: "split",
    plainTextEditorOpen: true,
  }));

  assert.match(markup, /role="dialog"/);
  assert.match(markup, /data-editor-surface="plain-text"/);
  assert.match(markup, /aria-label="Markdown 원문 편집"/);
  assert.match(markup, /data-action="close-plain-text-editor"/);
  assert.match(markup, /data-codemirror-host="pending"/);
  assert.doesNotMatch(markup, /data-plain-text-body=/);

  const modalTree = renderFunctionElement(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), {
    ...callbacks(),
    onOpenPlainTextEditor() {},
    onClosePlainTextEditor() { closed = true; },
  }, { plainTextEditorOpen: true }));
  clickElement(modalTree, (props) => props["data-action"] === "close-plain-text-editor");
  assert.equal(closed, true);
});

test("React authoring plain text modal forwards source changes through the canonical body callback", () => {
  const document = {
    ...snapshot(),
    body: "# 첫번째 문서\n\n원문 편집 대상",
  };
  const changes: string[] = [];
  const tree = renderFunctionElement(createDesktopDocumentAuthoringWorkbenchElement(document, {
    ...callbacks(),
    onBodyChange(body) { changes.push(body); },
  }, {
    viewMode: "preview",
    plainTextEditorOpen: true,
  }));
  const sourceEditor = findElement(tree, (props) =>
    props.documentId === document.documentId &&
    props.body === document.body &&
    typeof props.onChange === "function",
  );

  assert.ok(sourceEditor, "plain text CodeMirror source region must exist");
  const onChange = (sourceEditor.props as { readonly onChange: (body: string) => void }).onChange;
  onChange("# 첫번째 문서\n\n수정한 원문");
  assert.deepEqual(changes, ["# 첫번째 문서\n\n수정한 원문"]);
});

test("React authoring WYSIWYG heading and paragraph edits flow through canonical body changes", () => {
  const document = {
    ...snapshot(),
    body: [
      "# 첫번째 문서",
      "",
      "본문 첫 줄",
      "본문 둘째 줄",
      "",
      "| Item | Value |",
      "| :--- | ---: |",
      "| Grid | Ready |",
    ].join("\n"),
  };
  const changes: string[] = [];
  const tree = renderFunctionElement(createDesktopDocumentAuthoringWorkbenchElement(document, {
    ...callbacks(),
    onBodyChange(body) { changes.push(body); },
  }));

  blurElement(tree, (props) => props["data-wysiwyg-block-type"] === "heading", "새 제목");
  assert.equal(changes.at(-1)?.startsWith("# 새 제목\n\n본문 첫 줄"), true);

  blurElement(tree, (props) => props["data-wysiwyg-block-type"] === "paragraph", "수정한 본문");
  assert.equal(changes.at(-1), document.body.replace("본문 첫 줄\n본문 둘째 줄", "수정한 본문"));

  const unchangedCount = changes.length;
  blurElement(tree, (props) => props["data-wysiwyg-block-type"] === "paragraph", "본문 첫 줄\n본문 둘째 줄");
  assert.equal(changes.length, unchangedCount);

  const table = findElement(tree, (props) => props["data-wysiwyg-block-type"] === "table");
  assert.ok(table);
  assert.notEqual((table.props as Record<string, unknown>).contentEditable, true);
});

test("React authoring WYSIWYG checklist toggles update only Markdown checkbox markers", () => {
  const document = {
    ...snapshot(),
    body: [
      "# 첫번째 문서",
      "",
      "- [ ] 정리 필요",
      "- [x] 정리 완료",
      "",
      "본문",
    ].join("\n"),
  };
  const changes: string[] = [];
  const tree = renderFunctionElement(createDesktopDocumentAuthoringWorkbenchElement(document, {
    ...callbacks(),
    onBodyChange(body) { changes.push(body); },
  }));

  clickElement(tree, (props) =>
    props["data-action"] === "toggle-wysiwyg-checklist-item" && props["data-wysiwyg-checklist-index"] === 0,
  );
  assert.equal(changes.at(-1), document.body.replace("- [ ] 정리 필요\n- [x] 정리 완료", "- [x] 정리 필요\n- [x] 정리 완료"));

  clickElement(tree, (props) =>
    props["data-action"] === "toggle-wysiwyg-checklist-item" && props["data-wysiwyg-checklist-index"] === 1,
  );
  assert.equal(changes.at(-1), document.body.replace("- [ ] 정리 필요\n- [x] 정리 완료", "- [ ] 정리 필요\n- [ ] 정리 완료"));

  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(document, {
    ...callbacks(),
    onBodyChange() {},
  }));
  assert.match(markup, /data-action="toggle-wysiwyg-checklist-item"[^>]*aria-label="체크리스트 항목 완료로 표시"/);
  assert.match(markup, /data-action="toggle-wysiwyg-checklist-item"[^>]*aria-label="체크리스트 항목 미완료로 표시"/);
});

test("React authoring WYSIWYG table cell edits preserve Markdown table structure", () => {
  const document = {
    ...snapshot(),
    body: [
      "# 첫번째 문서",
      "",
      "| 항목 | 내용 | 상태 |",
      "| :--- | :---: | ---: |",
      "| 1번 | 가운데 | 완료 |",
    ].join("\n"),
  };
  const changes: string[] = [];
  const tree = renderFunctionElement(createDesktopDocumentAuthoringWorkbenchElement(document, {
    ...callbacks(),
    onBodyChange(body) { changes.push(body); },
  }));

  blurElement(tree, (props) =>
    props["data-wysiwyg-table-row"] === 0 && props["data-wysiwyg-table-cell"] === 1,
  "수정 | 값");

  assert.equal(changes.at(-1), document.body.replace(
    "| 1번 | 가운데 | 완료 |",
    "| 1번 | 수정 \\| 값 | 완료 |",
  ));

  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(document, {
    ...callbacks(),
    onBodyChange() {},
  }));
  assert.match(markup, /data-wysiwyg-table-row="0"[^>]*data-wysiwyg-table-cell="1"[^>]*contentEditable="true"/);
  assert.match(markup, /aria-label="표 셀 편집"/);
  assert.doesNotMatch(markup, />:---</);
});

test("React authoring WYSIWYG fallback blocks open plain text editor without exposing raw source", () => {
  const document = {
    ...snapshot(),
    body: [
      "# 안전한 문서",
      "",
      "<script>globalThis.compromised=true</script>",
    ].join("\n"),
  };
  let opened = false;
  const tree = renderFunctionElement(createDesktopDocumentAuthoringWorkbenchElement(document, {
    ...callbacks(),
    onOpenPlainTextEditor() { opened = true; },
  }));

  clickElement(tree, (props) => props["data-action"] === "edit-wysiwyg-fallback-in-source");
  assert.equal(opened, true);

  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(document, {
    ...callbacks(),
    onOpenPlainTextEditor() {},
  }));
  assert.match(markup, /data-wysiwyg-block-type="fallback"/);
  assert.match(markup, /data-wysiwyg-fallback-reason="raw_html"/);
  assert.match(markup, /data-action="edit-wysiwyg-fallback-in-source"/);
  assert.match(markup, /aria-label="원문에서 편집"/);
  assert.match(markup, />원문에서 편집<\/button>/);
  assert.doesNotMatch(markup, /globalThis\.compromised|<script>|&lt;script/);

  const disabled = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(document, callbacks()));
  assert.match(disabled, /data-action="edit-wysiwyg-fallback-in-source"[^>]*disabled/);
});

test("React authoring WYSIWYG inline references render as safe chips with plain text fallback", () => {
  const document = {
    ...snapshot(),
    body: [
      "# Cabinet 지도",
      "",
      "연결: [[Target Document|대상 문서]] / [외부 링크](https://example.com/private) / ![[asset:asset-private|설계 파일]]",
    ].join("\n"),
  };
  let opened = false;
  const tree = renderFunctionElement(createDesktopDocumentAuthoringWorkbenchElement(document, {
    ...callbacks(),
    onOpenPlainTextEditor() { opened = true; },
  }));

  clickElement(tree, (props) => props["data-action"] === "edit-wysiwyg-inline-source");
  assert.equal(opened, true);

  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(document, {
    ...callbacks(),
    onOpenPlainTextEditor() {},
  }));
  const surface = markup.match(/<section class="wysiwyg-document-surface"[\s\S]*?<\/section>/)?.[0] ?? "";

  assert.match(surface, /data-wysiwyg-inline-type="wikilink"/);
  assert.match(surface, /data-wysiwyg-inline-type="markdown_link"/);
  assert.match(surface, /data-wysiwyg-inline-type="asset_reference"/);
  assert.match(surface, />대상 문서</);
  assert.match(surface, />외부 링크</);
  assert.match(surface, />설계 파일</);
  assert.match(surface, /data-action="edit-wysiwyg-inline-source"/);
  assert.doesNotMatch(surface, /\[\[Target Document|!\[\[asset:|asset-private|https:\/\/example\.com\/private|\.md\b/);
});

test("React authoring WYSIWYG code and quote blocks render without raw Markdown markers", () => {
  const document = {
    ...snapshot(),
    body: [
      "# 개발 노트",
      "",
      "```rust",
      "fn main() {",
      "  println!(\"cabinet\");",
      "}",
      "```",
      "",
      "> [!NOTE] 참고",
      "> 인용 내용",
    ].join("\n"),
  };
  let opened = 0;
  const tree = renderFunctionElement(createDesktopDocumentAuthoringWorkbenchElement(document, {
    ...callbacks(),
    onOpenPlainTextEditor() { opened += 1; },
  }));

  clickElement(tree, (props) => props["data-action"] === "edit-wysiwyg-code-source");
  clickElement(tree, (props) => props["data-action"] === "edit-wysiwyg-quote-source");
  assert.equal(opened, 2);

  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(document, {
    ...callbacks(),
    onOpenPlainTextEditor() {},
  }));
  const surface = markup.match(/<section class="wysiwyg-document-surface"[\s\S]*?<\/section>/)?.[0] ?? "";

  assert.match(surface, /data-wysiwyg-block-type="code_block"/);
  assert.match(surface, /data-wysiwyg-code-language="rust"/);
  assert.match(surface, /fn main\(\)/);
  assert.match(surface, /data-wysiwyg-block-type="blockquote"/);
  assert.match(surface, /data-wysiwyg-callout-kind="NOTE"/);
  assert.match(surface, />참고/);
  assert.match(surface, />인용 내용/);
  assert.match(surface, /data-action="edit-wysiwyg-code-source"/);
  assert.match(surface, /data-action="edit-wysiwyg-quote-source"/);
  assert.doesNotMatch(surface, /```|&gt; \[!NOTE]|\[!NOTE]|&gt; 인용/);
});

function assertNoUnidentifiedInteractiveControls(markup: string): void {
  const controls = markup.match(/<(?:button|input|select|textarea|a)\b[^>]*>/g) ?? [];
  assert.ok(controls.length > 0);
  assert.deepEqual(controls.filter((control) => !control.includes("data-action=")), []);
}

test("React authoring workbench keeps unsafe source out of the default WYSIWYG surface and source in the modal", () => {
  const unsafe = {
    ...snapshot(),
    body: "# Safe\n\n<script>globalThis.compromised=true</script>",
  };
  const defaultMarkup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(unsafe, callbacks()));
  const modalMarkup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(unsafe, callbacks(), {
    plainTextEditorOpen: true,
  }));

  assert.match(defaultMarkup, /data-editor-surface="wysiwyg"/);
  assert.doesNotMatch(defaultMarkup, /aria-label="Markdown 원문"/);
  assert.doesNotMatch(defaultMarkup, /aria-label="Markdown 미리보기"/);
  assert.doesNotMatch(defaultMarkup, /data-codemirror-host/);
  assert.doesNotMatch(defaultMarkup, /<script>/);
  assert.doesNotMatch(defaultMarkup, /globalThis\.compromised/);

  assert.match(modalMarkup, /role="dialog"/);
  assert.match(modalMarkup, /data-editor-surface="plain-text"/);
  assert.match(modalMarkup, /data-codemirror-host="pending"/);
  assert.doesNotMatch(modalMarkup, /<script>/);
});

test("React authoring attachment panel presents empty, importing, preview, and mutation actions safely", () => {
  const loading = requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1");
  const ready = applyDesktopAssetResult(loading, loading.generation, {
    queryName: "list-document-assets",
    workspaceId: "workspace-1",
    documentId: "doc-1",
    assets: [{ assetId: "asset-secret", label: "회의 자료", fileName: "meeting.txt", mediaType: "text/plain", byteSize: 12, status: "available" }],
  });
  const callbacksWithAssets = {
    ...callbacks(),
    onAssetImport() {}, onAssetRetry() {}, onAssetCancel() {}, onAssetSelect() {},
    onAssetPreview() {}, onAssetPreviewClose() {}, onAssetUnlink() {}, onOpenLibrary() {},
  };
  const empty = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacksWithAssets, {
    assets: { ...loading, state: "Empty", page: { queryName: "list-document-assets", workspaceId: "workspace-1", documentId: "doc-1", assets: [] } },
    inspector: { tab: "attachments", unlink: { status: "Closed" } },
  }));
  const importing = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacksWithAssets, {
    assets: { ...ready, importState: "Importing", importOperationId: "internal-operation" },
    inspector: { tab: "attachments", unlink: { status: "Closed" } },
  }));
  const preview = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacksWithAssets, {
    assets: {
      ...ready,
      selectedAssetId: "asset-secret",
      detailState: "Ready",
      detail: { assetId: "asset-secret", fileName: "meeting.txt", mediaType: "text/plain", byteSize: 12, version: 1, previewCapability: "text", extractionStatus: "not_requested", referenceCount: 1, linkedDocumentIds: ["doc-1"], linkedDocuments: [{ documentId: "doc-1", title: "회의 기록", state: "available" }] },
      previewState: "Ready",
      preview: { assetId: "asset-secret", capability: "text", mediaType: "text/plain", presentation: "text", content: "safe preview" },
    },
    inspector: { tab: "attachments", unlink: { status: "Confirming", fileName: "meeting.txt" } },
  }));
  const submitting = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacksWithAssets, {
    assets: { ...ready, selectedAssetId: "asset-secret", detailState: "Ready", mutationState: "Unlinking" },
    inspector: { tab: "attachments", unlink: { status: "Submitting", fileName: "meeting.txt" } },
  }));
  const failedUnlink = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacksWithAssets, {
    assets: { ...ready, selectedAssetId: "asset-secret", detailState: "Ready", mutationState: "Failed" },
    inspector: { tab: "attachments", unlink: { status: "Failed", fileName: "meeting.txt" } },
  }));

  assert.match(empty, /이 문서에 첨부된 파일이 없습니다/);
  assert.match(importing, /파일을 안전하게 저장하고 있습니다/);
  assert.match(importing, /data-action="cancel-document-asset-import"/);
  assert.match(preview, /role="dialog"/);
  assert.match(preview, /safe preview/);
  assert.match(preview, /data-action="unlink-document-asset"/);
  assert.match(preview, /문서 연결만 해제하며 파일은 보관함에 남습니다/);
  assert.match(preview, /data-action="confirm-document-asset-unlink"/);
  assert.match(preview, /data-action="cancel-document-asset-unlink"/);
  assert.match(submitting, /data-document-asset-unlink-state="Submitting"/);
  assert.match(submitting, /data-action="confirm-document-asset-unlink"[^>]*disabled/);
  assert.match(failedUnlink, /data-document-asset-unlink-state="Failed"/);
  assert.match(failedUnlink, /role="alert"/);
  assert.match(failedUnlink, />다시 시도<\/button>/);
  assert.match(preview, /data-action="close-document-asset-preview"/);
  const visible = preview
    .replace(/\sdata-asset-id="[^"]*"/g, "")
    .replace(/\sdata-document-id="[^"]*"/g, "");
  assert.doesNotMatch(visible, /asset-secret|internal-operation|version-/);
});

test("React authoring attachment panel keeps per-file partial and recovery outcomes truthful", () => {
  const loading = requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1");
  const ready = applyDesktopAssetResult(loading, loading.generation, {
    queryName: "list-document-assets",
    workspaceId: "workspace-1",
    documentId: "doc-1",
    assets: [],
  });
  const completed = createAttachmentFileSnapshot({
    generation: 1,
    operationId: "secret-operation-completed",
    fileName: "done.pdf",
    byteSize: 10,
    state: "completed",
  });
  const recoveryBase = createAttachmentFileSnapshot({
    generation: 1,
    operationId: "secret-operation-recovery",
    fileName: "/private/design/recover.pdf",
    byteSize: 20,
    state: "projecting",
  });
  const recovery = applyAttachmentFileStatus(recoveryBase, {
    generation: 1,
    operationId: "secret-operation-recovery",
    state: "recovery_required",
    errorCode: "asset_graph_reindex.repository_unavailable",
  });
  const failedBase = createAttachmentFileSnapshot({
    generation: 1,
    operationId: "secret-operation-failed",
    fileName: "failed.txt",
    byteSize: 30,
    state: "selected",
  });
  const failed = applyAttachmentFileStatus(failedBase, {
    generation: 1,
    operationId: "secret-operation-failed",
    state: "failed",
    errorCode: "asset_import.handle_not_found",
  });

  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(
    snapshot(),
    { ...callbacks(), onAssetImport() {}, onAssetCancel() {} },
    {
      assets: {
        ...ready,
        importState: "Failed",
        importOperationId: "secret-operation-recovery",
        importOperations: [completed, recovery, failed],
      },
      inspector: { tab: "attachments", unlink: { status: "Closed" } },
    },
  ));

  assert.match(markup, /일부 파일/);
  assert.match(markup, /done\.pdf/);
  assert.match(markup, /첨부 완료/);
  assert.match(markup, /recover\.pdf/);
  assert.match(markup, /복구 필요/);
  assert.match(markup, /failed\.txt/);
  assert.match(markup, /첨부 실패/);
  assert.match(markup, /data-attachment-operation-stage="Completed"/);
  assert.match(markup, /data-attachment-operation-stage="RecoveryRequired"/);
  assert.match(markup, /data-attachment-operation-stage="Failed"/);
  assert.doesNotMatch(markup, /secret-operation|\/private\/|asset_graph_reindex|asset_import\.handle/);
  assert.doesNotMatch(markup, /data-action="repair-document-asset-import"/);
});

test("React authoring attachment panel exposes cancel only for the active cancellable file", () => {
  const loading = requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1");
  const active = createAttachmentFileSnapshot({
    generation: 1,
    operationId: "active-operation",
    fileName: "active.pdf",
    byteSize: 100,
    state: "staging",
  });
  const queued = createAttachmentFileSnapshot({
    generation: 1,
    operationId: "other-operation",
    fileName: "other.pdf",
    byteSize: 100,
    state: "selected",
  });
  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(
    snapshot(),
    { ...callbacks(), onAssetImport() {}, onAssetCancel() {} },
    {
      assets: {
        ...loading,
        importState: "Importing",
        importOperationId: "active-operation",
        importOperations: [active, queued],
      },
      inspector: { tab: "attachments", unlink: { status: "Closed" } },
    },
  ));

  assert.equal((markup.match(/data-action="cancel-document-asset-import"/g) ?? []).length, 1);
  assert.match(markup, /active\.pdf/);
  assert.match(markup, /파일 선택됨/);
  assert.doesNotMatch(markup, /active-operation|other-operation/);
});

test("React authoring attachment panel exposes repair only when a recovery callback exists", () => {
  const loading = requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1");
  const projecting = createAttachmentFileSnapshot({ generation: 1, operationId: "repair-internal", fileName: "repair.pdf", byteSize: 5, state: "projecting" });
  const recovery = applyAttachmentFileStatus(projecting, { generation: 1, operationId: "repair-internal", state: "recovery_required" });
  const options = {
    assets: { ...loading, importState: "Failed" as const, importOperations: [recovery] },
    inspector: { tab: "attachments" as const, unlink: { status: "Closed" as const } },
  };
  const withoutCallback = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks(), options));
  const withCallback = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), {
    ...callbacks(),
    onAssetRepair() {},
  }, options));

  assert.doesNotMatch(withoutCallback, /data-action="repair-document-asset-import"/);
  assert.match(withCallback, /data-action="repair-document-asset-import"/);
  assert.match(withCallback, /aria-label="repair\.pdf 첨부 복구"/);
  assert.doesNotMatch(withCallback, /repair-internal/);
});

test("React authoring attachment panel presents a path-free native drop target", () => {
  const loading = requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1");
  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(
    snapshot(),
    callbacks(),
    {
      assets: { ...loading, dropState: "Entered", dropFileCount: 2 },
      inspector: { tab: "attachments", unlink: { status: "Closed" } },
    },
  ));

  assert.match(markup, /data-document-attachment-drop-state="Entered"/);
  assert.match(markup, /여기에 놓아 첨부/);
  assert.match(markup, /2개 파일/);
  assert.doesNotMatch(markup, /\/private\//i);
  assert.doesNotMatch(markup, /data-(?:file-)?path=/i);
});

test("React authoring attachment panel presents a bounded safe existing-file chooser", () => {
  const assets = requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1");
  const libraryAssets = applyDesktopAssetResult(
    requestDesktopWorkspaceAssetLoad(requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1")),
    2,
    {
      queryName: "list-workspace-assets",
      workspaceId: "workspace-1",
      assets: [{ assetId: "asset-private", label: "Architecture", fileName: "architecture.pdf", mediaType: "application/pdf", byteSize: 2048, status: "available" }],
      nextCursor: "opaque-private-cursor",
    },
  );
  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks(), {
    assets,
    inspector: { tab: "attachments", unlink: { status: "Closed" } },
    assetLibrary: { status: "Ready", workspaceId: "workspace-1", documentId: "doc-1", generation: 1, assets: libraryAssets },
  }));

  assert.match(markup, /role="dialog"/);
  assert.match(markup, /기존 파일 연결/);
  assert.match(markup, /파일명으로 검색/);
  assert.match(markup, /architecture\.pdf/);
  assert.match(markup, /data-action="link-existing-document-asset"/);
  assert.match(markup, /data-action="load-more-document-asset-library"/);
  assert.doesNotMatch(markup, /asset-private|opaque-private-cursor|\/private\//);
});

test("React authoring inspector renders only the selected context panel", () => {
  const common = { links: { state: "Idle", workspaceId: "workspace-1", documentId: "doc-1", generation: 0 } as const };
  const links = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks(), {
    ...common,
    inspector: { tab: "links", unlink: { status: "Closed" } },
  }));
  const history = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks(), {
    ...common,
    history: { status: "Idle", entries: [] },
    inspector: { tab: "history", unlink: { status: "Closed" } },
  }));

  assert.match(links, /연결된 문서가 없습니다/);
  assert.doesNotMatch(links, /이력 불러오기|이 문서에 첨부된 파일/);
  assert.match(history, /이력 불러오기/);
  assert.doesNotMatch(history, /연결된 문서가 없습니다|이 문서에 첨부된 파일/);
});

test("React authoring restore states expose safe missing conflict and recovery actions", () => {
  const restoreCallbacks = {
    ...callbacks(),
    onRefreshRestorePreview() {},
    onContinueRestoreRecovery() {},
  };
  const common = {
    entries: [],
    inspector: { tab: "history", unlink: { status: "Closed" } } as const,
  };
  const blocked = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(
    snapshot(),
    restoreCallbacks,
    {
      history: {
        status: "Failed",
        entries: [],
        restore: {
          status: "BlockedMissingAsset",
          targetVersionId: "opaque-version-token",
          expectedCurrentVersionId: "opaque-current-token",
          targetVersionLabel: "버전 3",
          changedLineCount: 0,
          missingAssetLabels: ["회의 자료"],
          canRestore: false,
          diff: restoreDiff(),
        },
      },
      inspector: common.inspector,
    },
  ));
  const conflict = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(
    snapshot(),
    restoreCallbacks,
    {
      history: {
        status: "Failed",
        entries: [],
        restore: { status: "Conflict", targetVersionId: "opaque-version-token", targetVersionLabel: "버전 3" },
      },
      inspector: common.inspector,
    },
  ));
  const recovery = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(
    snapshot(),
    restoreCallbacks,
    {
      history: {
        status: "Failed",
        entries: [],
        restore: {
          status: "RecoveryRequired",
          operationId: "opaque-operation-token",
          targetVersionId: "opaque-version-token",
          expectedCurrentVersionId: "opaque-current-token",
          targetVersionLabel: "버전 3",
          changedLineCount: 2,
          missingAssetLabels: [],
          canRestore: true,
          diff: restoreDiff(),
        },
      },
      inspector: common.inspector,
    },
  ));

  assert.match(blocked, /파일을 찾을 수 없어 복원할 수 없습니다/);
  assert.match(blocked, /회의 자료/);
  assert.match(blocked, /data-action="apply-restore"[^>]*disabled/);
  assert.match(conflict, /data-action="refresh-restore-preview"/);
  assert.match(conflict, /미리보기 새로고침/);
  assert.match(recovery, /data-action="continue-restore-recovery"/);
  assert.match(recovery, /복구 계속/);
  assert.doesNotMatch(`${blocked}${conflict}${recovery}`, /opaque-(?:version|current|operation)-token/);
});

test("React authoring restore confirmation renders full diff and explicit actions without internal tokens", () => {
  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(
    snapshot(),
    {
      ...callbacks(),
      onRequestRestoreConfirmation() {},
      onCancelRestoreConfirmation() {},
      onApplyRestore() {},
    },
    {
      history: {
        status: "PreviewReady",
        entries: [],
        restore: {
          status: "Confirming",
          targetVersionId: "opaque-version-token",
          expectedCurrentVersionId: "opaque-current-token",
          targetVersionLabel: "버전 3",
          changedLineCount: 2,
          missingAssetLabels: [],
          canRestore: true,
          diff: restoreDiff(),
        },
      },
      inspector: { tab: "history", unlink: { status: "Closed" } },
    },
  ));

  assert.match(markup, /복원 전 변경 내용 확인/);
  assert.match(markup, /현재 제목/);
  assert.match(markup, /복원할 제목/);
  assert.match(markup, /현재 본문/);
  assert.match(markup, /복원할 본문/);
  assert.match(markup, /추가 자료\.pdf/);
  assert.match(markup, /제거 자료\.pdf/);
  assert.match(markup, /data-action="cancel-restore-confirmation"/);
  assert.match(markup, /data-action="confirm-restore"/);
  assert.doesNotMatch(markup, /opaque-(?:workspace|document|current|version)-token/);
});

test("React authoring blocks too-large restore while preserving safe attachment labels", () => {
  const diff = restoreDiff();
  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(
    snapshot(),
    { ...callbacks(), onRequestRestoreConfirmation() {}, onApplyRestore() {} },
    {
      history: {
        status: "PreviewReady",
        entries: [],
        restore: {
          status: "BlockedLargeDiff",
          targetVersionId: "opaque-version-token",
          expectedCurrentVersionId: "opaque-current-token",
          targetVersionLabel: "버전 3",
          changedLineCount: 20_000,
          missingAssetLabels: [],
          canRestore: true,
          diff: { ...diff, status: "TooLarge", limitReason: "hunks" },
        },
      },
      inspector: { tab: "history", unlink: { status: "Closed" } },
    },
  ));

  assert.match(markup, /전체 변경 내용을 확인할 수 없어 복원할 수 없습니다/);
  assert.match(markup, /추가 자료\.pdf/);
  assert.match(markup, /data-action="review-restore"[^>]*disabled/);
  assert.doesNotMatch(markup, /data-action="confirm-restore"/);
  assert.doesNotMatch(markup, /opaque-(?:workspace|document|current|version)-token/);
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
  const linkMarkup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks(), {
    links: { state: "Failed", workspaceId: "workspace-1", documentId: "doc-1", generation: 2, errorCode: "LINK_OVERVIEW_UNAVAILABLE", retryable: false },
    inspector: { tab: "links", unlink: { status: "Closed" } },
  }));
  const historyMarkup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks(), {
    history: { status: "Failed", entries: [], errorCode: "DOCUMENT_HISTORY_QUERY_FAILED" },
    inspector: { tab: "history", unlink: { status: "Closed" } },
  }));

  assert.match(linkMarkup, /연결된 문서를 불러오지 못했습니다/);
  assert.match(historyMarkup, /문서 이력/);
  assert.match(historyMarkup, /이력 불러오기/);
  assert.match(historyMarkup, /문서 이력을 불러오지 못했습니다/);
  assert.doesNotMatch(`${linkMarkup}${historyMarkup}`, /LINK_OVERVIEW_UNAVAILABLE|DOCUMENT_HISTORY_QUERY_FAILED|Load history|History/);
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

test("React authoring exposes search return only for a document opened from results", () => {
  const regular = renderToStaticMarkup(
    createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks()),
  );
  const fromSearch = renderToStaticMarkup(
    createDesktopDocumentAuthoringWorkbenchElement(snapshot(), {
      ...callbacks(),
      onReturnToSearch() {},
    }),
  );

  assert.doesNotMatch(regular, /data-action="return-search-results"/);
  assert.match(fromSearch, /data-action="return-search-results"/);
  assert.match(fromSearch, /검색 결과로 돌아가기/);
  const returnControl = fromSearch.match(/<button[^>]*data-action="return-search-results"[\s\S]*?<\/button>/)?.[0] ?? "";
  assert.doesNotMatch(returnControl, /doc-1|notes\/source\.md/);
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
    inspector: { tab: "history", unlink: { status: "Closed" } },
  }));
  const visible = markup.replace(/\sdata-version-id="[^"]*"/g, "");

  assert.match(visible, /버전 1/);
  assert.match(visible, /2026\. 7\. 15/);
  assert.match(visible, /문서 저장/);
  assert.doesNotMatch(visible, /internal-version-secret/);
});

test("React authoring history keeps entries during cursor load-more and exposes bounded retry", () => {
  const base = {
    entries: [{
      versionId: "internal-version-secret",
      versionLabel: "버전 51",
      createdAtLabel: "2026. 7. 16.",
      summaryLabel: "문서 저장",
    }],
    nextCursor: "opaque-cursor-secret",
  };
  const callbacksWithMore = { ...callbacks(), onLoadMoreHistory() {} };
  const ready = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(
    snapshot(),
    callbacksWithMore,
    { history: { status: "Ready", ...base }, inspector: { tab: "history", unlink: { status: "Closed" } } },
  ));
  const loading = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(
    snapshot(),
    callbacksWithMore,
    { history: { status: "LoadingMore", ...base }, inspector: { tab: "history", unlink: { status: "Closed" } } },
  ));
  const failed = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(
    snapshot(),
    callbacksWithMore,
    {
      history: { status: "Ready", ...base, loadMoreErrorCode: "DOCUMENT_QUERY_STORAGE_UNAVAILABLE" },
      inspector: { tab: "history", unlink: { status: "Closed" } },
    },
  ));

  assert.match(ready, /data-action="load-more-history"(?![^>]*disabled)/);
  assert.match(ready, /이전 이력 더 보기/);
  assert.match(loading, /data-action="load-more-history"[^>]*disabled/);
  assert.match(loading, /불러오는 중/);
  assert.match(failed, /이전 이력을 불러오지 못했습니다/);
  assert.match(failed, /다시 시도/);
  for (const markup of [ready, loading, failed]) {
    const visible = markup.replace(/\sdata-version-id="[^"]*"/g, "");
    assert.match(visible, /버전 51/);
    assert.doesNotMatch(visible, /opaque-cursor-secret|internal-version-secret/);
  }
});

test("React authoring virtualizes two hundred history entries without exposing version identities", () => {
  const entries = Array.from({ length: 200 }, (_, index) => ({
    versionId: `opaque-history-secret-${index}`,
    versionLabel: `버전 ${200 - index}`,
    createdAtLabel: "2026. 7. 17.",
    summaryLabel: "문서 저장",
  }));
  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks(), {
    history: { status: "Ready", entries },
    inspector: { tab: "history", unlink: { status: "Closed" } },
  }));

  assert.equal((markup.match(/data-history-entry="visible"/g) ?? []).length, 50);
  assert.match(markup, /data-history-window="1-50\/200"/);
  assert.match(markup, /이력 1-50 \/ 전체 200개/);
  assert.match(markup, /data-action="next-history-window"/);
  assert.match(markup, /data-action="previous-history-window"[^>]*disabled/);
  assert.doesNotMatch(markup, /opaque-history-secret-/);

  const selectedAcrossWindows = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), {
    ...callbacks(),
    onToggleHistoryCompareSelection() {},
    onCompareSelectedVersions() {},
  }, {
    history: {
      status: "Ready",
      entries,
      comparison: {
        status: "TwoSelected",
        selections: [
          { versionId: "opaque-history-secret-0", versionLabel: "버전 200" },
          { versionId: "opaque-history-secret-100", versionLabel: "버전 100" },
        ],
      },
    },
    inspector: { tab: "history", unlink: { status: "Closed" } },
  }));
  assert.match(selectedAcrossWindows, /data-action="compare-selected-versions"(?![^>]*disabled)/);
  assert.doesNotMatch(selectedAcrossWindows, /opaque-history-secret-/);
});

test("React history window consumes explicit focus requests after a window transition", async () => {
  const source = await readFile(new URL("../src/react_document_authoring_workbench.ts", import.meta.url), "utf8");
  assert.match(source, /focusRequest\.current\s*=\s*transition\.focusRequest/);
  assert.match(source, /firstVisibleAction\.current\?\.focus\(\)/);
  assert.match(source, /index\s*===\s*0\s*\?\s*firstVisibleAction/);
});

test("React authoring history enables version-pair compare only for two user-facing selections", () => {
  const entries = [
    { versionId: "opaque-left-secret", versionLabel: "버전 7", createdAtLabel: "오늘", summaryLabel: "문서 저장" },
    { versionId: "opaque-right-secret", versionLabel: "버전 5", createdAtLabel: "어제", summaryLabel: "문서 저장" },
  ];
  const pairCallbacks = {
    ...callbacks(),
    onToggleHistoryCompareSelection() {},
    onCompareSelectedVersions() {},
  };
  const one = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), pairCallbacks, {
    history: {
      status: "Ready",
      entries,
      comparison: {
        status: "OneSelected",
        selections: [{ versionId: "opaque-left-secret", versionLabel: "버전 7" }],
      },
    },
    inspector: { tab: "history", unlink: { status: "Closed" } },
  }));
  const two = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), pairCallbacks, {
    history: {
      status: "Ready",
      entries,
      comparison: {
        status: "TwoSelected",
        selections: [
          { versionId: "opaque-left-secret", versionLabel: "버전 7" },
          { versionId: "opaque-right-secret", versionLabel: "버전 5" },
        ],
      },
    },
    inspector: { tab: "history", unlink: { status: "Closed" } },
  }));

  assert.match(one, /data-action="compare-selected-versions"[^>]*disabled/);
  assert.match(one, /버전 7[^<]*선택됨/);
  assert.match(two, /data-action="compare-selected-versions"(?![^>]*disabled)/);
  assert.match(two, /선택한 두 버전 비교/);
  assert.match(two, /aria-pressed="true"/);
  const visible = two.replace(/\sdata-version-id="[^"]*"/g, "");
  assert.doesNotMatch(visible, /opaque-left-secret|opaque-right-secret/);
});

test("React authoring history renders compare action and semantic diff without visible tokens", () => {
  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), {
    ...callbacks(),
    onCompareVersion() {},
    onCloseDiff() {},
  }, {
    history: {
      status: "Ready",
      entries: [{
        versionId: "history-token-secret",
        versionLabel: "버전 1",
        createdAtLabel: "2026. 7. 15. 오후 3:00",
        summaryLabel: "문서 저장",
      }],
      diff: {
        status: "Ready",
        targetVersionId: "history-token-secret",
        targetVersionLabel: "버전 1",
        addedCount: 1,
        removedCount: 1,
        attachmentDiff: {
          status: "Known",
          added: [{ label: "새 설계서.pdf", availability: "Available" }],
          removed: [{ label: "이전 설계서.pdf", availability: "Missing" }],
          relabeled: [{ beforeLabel: "초안.pdf", afterLabel: "최종안.pdf", availability: "Available" }],
          unchangedCount: 1,
        },
        titleDelta: { kind: "Changed", before: "현재 제목", after: "이전 제목" },
        hunks: [{
          oldStartLine: 1,
          newStartLine: 1,
          addedCount: 1,
          removedCount: 1,
          lines: [
            { kind: "Removed", text: "현재 본문", oldLineNumber: 2 },
            { kind: "Added", text: "이전 본문", newLineNumber: 2 },
          ],
        }],
      },
    },
    inspector: { tab: "history", unlink: { status: "Closed" } },
  }));
  const visible = markup
    .replace(/\sdata-version-id="[^"]*"/g, "")
    .replace(/\sdata-diff-target="[^"]*"/g, "");

  assert.match(visible, /data-action="compare-current-version"/);
  assert.match(visible, /현재 문서와 비교/);
  assert.match(visible, /class="authoring-workspace mode-compare"/);
  assert.match(visible, /data-action="close-document-diff"/);
  assert.match(visible, /편집기로 돌아가기/);
  assert.match(visible, /버전 1 비교 결과/);
  assert.match(visible, /추가 1줄/);
  assert.match(visible, /삭제 1줄/);
  assert.match(visible, /현재 제목/);
  assert.match(visible, /이전 제목/);
  assert.match(visible, /class="diff-line removed"/);
  assert.match(visible, /class="diff-line added"/);
  assert.match(visible, /현재 본문/);
  assert.match(visible, /이전 본문/);
  assert.match(visible, /첨부 파일 변경/);
  assert.match(visible, /추가됨/);
  assert.match(visible, /새 설계서\.pdf/);
  assert.match(visible, /제거됨/);
  assert.match(visible, /이전 설계서\.pdf/);
  assert.match(visible, /파일을 찾을 수 없음/);
  assert.match(visible, /이름 변경/);
  assert.match(visible, /초안\.pdf/);
  assert.match(visible, /최종안\.pdf/);
  assert.match(visible, /변경 없음 1개/);
  assert.doesNotMatch(visible, /history-token-secret/);
});

test("React authoring diff distinguishes legacy attachment history from no changes", () => {
  const history = {
    status: "Ready" as const,
    entries: [],
    diff: {
      status: "Ready" as const,
      targetVersionId: "legacy-token-secret",
      targetVersionLabel: "버전 1",
      addedCount: 0,
      removedCount: 0,
      attachmentDiff: { status: "LegacyUnknown" as const },
      titleDelta: { kind: "Unchanged" as const },
      hunks: [],
    },
  };
  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(
    snapshot(),
    callbacks(),
    { history },
  )).replace(/\sdata-diff-target="[^"]*"/g, "");

  assert.match(markup, /과거 형식으로 저장된 버전이라 첨부 파일 변경을 확인할 수 없습니다/);
  assert.doesNotMatch(markup, /첨부 파일 변경 없음/);
  assert.doesNotMatch(markup, /legacy-token-secret/);
});

test("React authoring diff exposes loading too-large and safe failure states", () => {
  const baseHistory = {
    status: "Ready" as const,
    entries: [{
      versionId: "v1",
      versionLabel: "버전 1",
      createdAtLabel: "저장 시각 없음",
      summaryLabel: "문서 저장",
    }],
  };
  const loading = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks(), {
    history: { ...baseHistory, diff: { status: "Loading", targetVersionId: "v1", targetVersionLabel: "버전 1" } },
  }));
  const tooLarge = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks(), {
    history: {
      ...baseHistory,
      diff: {
        status: "TooLarge",
        targetVersionId: "v1",
        targetVersionLabel: "버전 1",
        limitReason: "bytes",
        attachmentDiff: {
          status: "Known",
          added: [{ label: "대용량 문서 첨부.pdf", availability: "Available" }],
          removed: [],
          relabeled: [],
          unchangedCount: 0,
        },
      },
    },
  }));
  const failed = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks(), {
    history: { ...baseHistory, diff: { status: "Failed", targetVersionId: "v1", targetVersionLabel: "버전 1", errorCode: "DOCUMENT_DIFF_STORAGE_UNAVAILABLE" } },
  }));

  assert.match(loading, /비교 결과를 불러오는 중/);
  assert.match(tooLarge, /앱에서 바로 비교하기에는 문서가 너무 큽니다/);
  assert.match(tooLarge, /대용량 문서 첨부\.pdf/);
  assert.match(failed, /role="alert"/);
  assert.match(failed, /비교 결과를 불러오지 못했습니다/);
  assert.doesNotMatch(failed, /DOCUMENT_DIFF_STORAGE_UNAVAILABLE/);
});

test("React authoring diff exposes background progress cancel and retry without operation identity", () => {
  const baseHistory = { status: "Ready" as const, entries: [] };
  const activeCallbacks = {
    ...callbacks(),
    onCancelBackgroundDiff() {},
    onRetryBackgroundDiff() {},
  };
  const accepted = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), activeCallbacks, {
    history: { ...baseHistory, diff: { status: "Accepted", targetVersionId: "version-secret", targetVersionLabel: "버전 8" } },
  }));
  const running = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), activeCallbacks, {
    history: { ...baseHistory, diff: { status: "Running", targetVersionId: "version-secret", targetVersionLabel: "버전 8" } },
  }));
  const cancelled = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), activeCallbacks, {
    history: { ...baseHistory, diff: { status: "Cancelled", targetVersionId: "version-secret", targetVersionLabel: "버전 8" } },
  }));
  const expired = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), activeCallbacks, {
    history: { ...baseHistory, diff: { status: "Expired", targetVersionId: "version-secret", targetVersionLabel: "버전 8" } },
  }));

  assert.match(accepted, /큰 문서 비교를 준비하고 있습니다/);
  assert.match(running, /큰 문서 변경 내용을 비교하고 있습니다/);
  assert.match(running, /data-action="cancel-background-document-diff"/);
  assert.match(running, />비교 취소</);
  assert.match(cancelled, /문서 비교를 취소했습니다/);
  assert.match(cancelled, /data-action="retry-background-document-diff"/);
  assert.match(expired, /앱이 다시 시작되어 문서를 다시 비교해야 합니다/);
  assert.match(expired, /data-action="retry-background-document-diff"/);
  for (const markup of [accepted, running, cancelled, expired]) {
    assert.doesNotMatch(markup, /version-secret|operation-token|snapshotRef|\.md/);
  }
});

test("React authoring renders a bounded first window for one thousand diff hunks", () => {
  const hunks = Array.from({ length: 1_000 }, (_, index) => ({
    oldStartLine: index + 1,
    newStartLine: index + 1,
    addedCount: 1,
    removedCount: 0,
    lines: [{ kind: "Added" as const, text: `사용자 변경 ${index + 1}`, newLineNumber: index + 1 }],
  }));
  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks(), {
    history: {
      status: "Ready",
      entries: [],
      diff: {
        status: "Ready",
        targetVersionId: "opaque-version-token",
        targetVersionLabel: "버전 1",
        addedCount: 1_000,
        removedCount: 0,
        attachmentDiff: { status: "Known", added: [], removed: [], relabeled: [], unchangedCount: 0 },
        titleDelta: { kind: "Unchanged" },
        hunks,
      },
    },
    inspector: { tab: "history", unlink: { status: "Closed" } },
  }));

  assert.equal((markup.match(/class="diff-hunk"/g) ?? []).length, 50);
  assert.match(markup, /변경 구간 1–50 \/ 1000/);
  assert.match(markup, /data-action="previous-diff-hunks"[^>]*disabled/);
  assert.match(markup, /data-action="next-diff-hunks"/);
  assert.match(markup, /사용자 변경 50/);
  assert.doesNotMatch(markup, /사용자 변경 51/);
  assert.doesNotMatch(markup, /opaque-version-token/);
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

function renderFunctionElement(element: React.ReactElement): React.ReactElement {
  const component = element.type as (props: Record<string, unknown>) => React.ReactElement;
  return component(element.props as Record<string, unknown>);
}

function clickElement(
  tree: React.ReactNode,
  predicate: (props: Record<string, unknown>) => boolean,
): void {
  const found = findElement(tree, predicate);
  assert.ok(found, "interactive element must exist");
  const onClick = (found.props as { readonly onClick?: () => void }).onClick;
  assert.equal(typeof onClick, "function");
  onClick?.();
}

function blurElement(
  tree: React.ReactNode,
  predicate: (props: Record<string, unknown>) => boolean,
  textContent: string,
): void {
  const found = findElement(tree, predicate);
  assert.ok(found, "editable element must exist");
  const onBlur = (found.props as { readonly onBlur?: (event: { readonly currentTarget: { readonly textContent: string } }) => void }).onBlur;
  assert.equal(typeof onBlur, "function");
  onBlur?.({ currentTarget: { textContent } });
}

function findElement(
  node: React.ReactNode,
  predicate: (props: Record<string, unknown>) => boolean,
): React.ReactElement | undefined {
  if (Array.isArray(node)) {
    for (const child of node) {
      const found = findElement(child, predicate);
      if (found) return found;
    }
    return undefined;
  }
  if (!React.isValidElement(node)) return undefined;
  const props = node.props as Record<string, unknown>;
  if (predicate(props)) return node;
  return findElement(props.children as React.ReactNode, predicate);
}

function restoreDiff() {
  return {
    workspaceId: "opaque-workspace-token",
    documentId: "opaque-document-token",
    status: "Complete",
    leftVersionId: "opaque-current-token",
    rightVersionId: "opaque-version-token",
    addedCount: 1,
    removedCount: 1,
    attachmentDiff: {
      status: "Known",
      added: [{ label: "추가 자료.pdf", availability: "Available" }],
      removed: [{ label: "제거 자료.pdf", availability: "Available" }],
      relabeled: [],
      unchangedCount: 0,
    },
    titleDelta: { kind: "Changed", before: "현재 제목", after: "복원할 제목" },
    hunks: [{
      oldStartLine: 1,
      newStartLine: 1,
      addedCount: 1,
      removedCount: 1,
      lines: [
        { kind: "Removed", text: "현재 본문", oldLineNumber: 1 },
        { kind: "Added", text: "복원할 본문", newLineNumber: 1 },
      ],
    }],
  } as const;
}
