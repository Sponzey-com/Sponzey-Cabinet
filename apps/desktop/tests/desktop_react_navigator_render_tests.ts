import assert from "node:assert/strict";
import test from "node:test";
import { renderToStaticMarkup } from "react-dom/server";

import {
  applyDocumentNavigatorResult,
  createDocumentNavigatorFailedModel,
  createDocumentNavigatorLoadingModel,
} from "@sponzey-cabinet/ui";

import { createDesktopDocumentNavigatorElement } from "../src/react_document_navigator.ts";

test("React navigator renders semantic five-view controls filter and ready metadata", () => {
  const loading = createDocumentNavigatorLoadingModel({
    workspaceId: "workspace-1",
    view: "Tree",
    generation: 1,
  });
  const ready = applyDocumentNavigatorResult(loading, 1, {
    workspaceId: "workspace-1",
    view: "Tree",
    state: "Ready",
    items: [
      {
        documentId: "doc-1",
        title: "Architecture",
        path: "notes/architecture.md",
        collections: ["work"],
        tags: ["rust"],
        favorite: true,
      },
    ],
  });
  const markup = renderToStaticMarkup(
    createDesktopDocumentNavigatorElement(ready, callbacks()),
  );

  assert.match(markup, /data-cabinet-navigator-state="Ready"/);
  assert.match(markup, /data-design-reference="penpot-20260713"/);
  assert.match(markup, /검색과 발견/);
  assert.doesNotMatch(markup, /Cabinet 답변|ask-followup|질문 전송/);
  assert.match(markup, /data-action="new-document"/);
  assert.match(markup, /role="tablist"/);
  for (const view of ["Tree", "Collection", "Tag", "Recent", "Favorite"]) {
    assert.match(markup, new RegExp(`data-navigator-view="${view}"`));
  }
  assert.match(markup, /aria-label="문서 검색어"/);
  assert.match(markup, /Architecture/);
  assert.match(markup, /data-document-id="doc-1"/);
  assert.match(markup, />notes · 즐겨찾기 · work · rust</);
  assert.doesNotMatch(markup, /notes\/architecture\.md|>Favorite<| · Favorite|aria-label="Documents"|Document views/);
  assert.match(markup, /rust/);
  assert.match(markup, />홈</);
  assert.match(markup, />백업 및 복원</);
  assert.equal((markup.match(/class="desktop-sidebar"/g) ?? []).length, 1);
  assert.equal((markup.match(/class="desktop-topbar"/g) ?? []).length, 1);
  assert.match(markup, /data-action="navigate-search"[^>]*disabled/);
  assert.match(markup, /data-action="navigate-graph"[^>]*disabled/);
  assert.doesNotMatch(markup, /data-action="(?:open-settings|toggle-theme|open-ai)"/);
  assert.doesNotMatch(markup, /data-action="static-document"/);
  assert.doesNotMatch(markup, /data-action="navigate-document"[^>]*disabled/);
  assert.doesNotMatch(markup, /server|tenant|billing|admin-console/i);
  assertNoUnidentifiedInteractiveControls(markup);
});

test("React navigator renders loading empty degraded failed and retry states", () => {
  const loading = createDocumentNavigatorLoadingModel({
    workspaceId: "workspace-1",
    view: "Tree",
    generation: 1,
  });
  const empty = applyDocumentNavigatorResult(loading, 1, {
    workspaceId: "workspace-1",
    view: "Tree",
    state: "EmptyResult",
    items: [],
  });
  const degraded = applyDocumentNavigatorResult(loading, 1, {
    workspaceId: "workspace-1",
    view: "Tree",
    state: "Degraded",
    items: [],
  });
  const failed = createDocumentNavigatorFailedModel({
    workspaceId: "workspace-1",
    view: "Tree",
    generation: 1,
    errorCode: "DOCUMENT_NAVIGATOR_PROJECTION_UNAVAILABLE",
    retryable: true,
  });

  assert.match(
    renderToStaticMarkup(createDesktopDocumentNavigatorElement(loading, callbacks())),
    /문서를 불러오는 중입니다/,
  );
  assert.match(
    renderToStaticMarkup(createDesktopDocumentNavigatorElement(empty, callbacks())),
    /조건에 맞는 문서가 없습니다/,
  );
  assert.match(
    renderToStaticMarkup(createDesktopDocumentNavigatorElement(degraded, callbacks())),
    /문서 목록을 확인해야 합니다/,
  );
  const failedMarkup = renderToStaticMarkup(
    createDesktopDocumentNavigatorElement(failed, callbacks()),
  );
  assert.match(failedMarkup, /다시 시도/);
  assert.doesNotMatch(failedMarkup, /DOCUMENT_NAVIGATOR_PROJECTION_UNAVAILABLE|COMMAND_BRIDGE_FAILED/);
  assert.doesNotMatch(failedMarkup, /private|app-data|raw document body/i);
});

function callbacks() {
  return {
    onCreateDocument() {},
    onHome() {},
    onDocument() {},
    onView() {},
    onFilter() {},
    onRetry() {},
    onOpenDocument() {},
  };
}

function assertNoUnidentifiedInteractiveControls(markup: string): void {
  const controls = markup.match(/<(?:button|input|select|textarea|a)\b[^>]*>/g) ?? [];
  assert.ok(controls.length > 0);
  assert.deepEqual(controls.filter((control) => !control.includes("data-action=")), []);
}
