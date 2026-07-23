import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";

import {
  applyDocumentNavigatorResult,
  createDocumentNavigatorFailedModel,
  createDocumentNavigatorLoadingModel,
} from "@sponzey-cabinet/ui";

import { createDesktopDocumentNavigatorElement } from "../src/react_document_navigator.ts";
import { createDesktopSearchResultWindow } from "../src/desktop_search_result_window.ts";

const navigatorSource = await readFile(new URL("../src/react_document_navigator.ts", import.meta.url), "utf8");

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
  assert.match(markup, /data-design-reference="penpot-20260721"/);
  assert.match(markup, /data-global-search-overlay="open"/);
  assert.match(markup, /class="global-search-overlay"/);
  assert.match(markup, /role="dialog"/);
  assert.match(markup, /aria-modal="true"/);
  assert.match(markup, /aria-labelledby="navigator-title"/);
  assert.match(markup, /data-global-search-state="ResultsReady"/);
  assert.match(markup, /전체 검색/);
  assert.match(markup, /제목, 본문, 첨부 파일 이름을 한 번에 검색합니다\./);
  assert.doesNotMatch(markup, /검색과 발견|문서 제목과 본문에서 검색합니다\./);
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
  assert.match(markup, />백업과 복원</);
  assert.equal((markup.match(/class="desktop-sidebar"/g) ?? []).length, 1);
  assert.equal((markup.match(/class="desktop-topbar"/g) ?? []).length, 1);
  assert.doesNotMatch(markup, /data-action="navigate-search"/);
  assert.match(markup, /data-action="navigator-search-field"/);
  assert.match(markup, /data-action="navigate-graph"[^>]*disabled/);
  assert.doesNotMatch(markup, /data-action="(?:open-settings|toggle-theme|open-ai)"/);
  assert.doesNotMatch(markup, /data-action="static-document"/);
  assert.doesNotMatch(markup, /data-action="navigate-document"[^>]*disabled/);
  assert.match(markup, /class="search-result-group"/);
  assert.match(markup, /<h2 id="navigator-document-results-title">문서<\/h2>/);
  assert.match(markup, /1개 결과/);
  assert.match(markup, /class="global-search-footer"/);
  assert.match(markup, /data-search-result-group="document"/);
  assert.doesNotMatch(markup, /data-search-result-group="asset"/);
  assert.doesNotMatch(markup, /server|tenant|billing|admin-console/i);
  assertNoUnidentifiedInteractiveControls(markup);
});

test("React navigator renders explicit global search duration metadata without inventing it", () => {
  const loading = createDocumentNavigatorLoadingModel({
    workspaceId: "workspace-1",
    view: "Tree",
    filter: "성능",
    generation: 2,
  });
  const ready = applyDocumentNavigatorResult(loading, 2, {
    workspaceId: "workspace-1",
    view: "Tree",
    state: "Ready",
    items: [
      {
        documentId: "doc-1",
        title: "성능 기준",
        path: "notes/performance.md",
        collections: [],
        tags: [],
        favorite: false,
      },
    ],
  });

  const withoutDuration = renderToStaticMarkup(
    createDesktopDocumentNavigatorElement(ready, callbacks()),
  );
  const withDuration = renderToStaticMarkup(
    createDesktopDocumentNavigatorElement(ready, callbacks(), {
      searchMetrics: { durationMs: 42 },
    }),
  );
  const modelDuration = renderToStaticMarkup(
    createDesktopDocumentNavigatorElement(
      {
        ...ready,
        searchMetrics: { durationMs: 47 },
      },
      callbacks(),
    ),
  );

  assert.match(withoutDuration, /1개 결과/);
  assert.doesNotMatch(withoutDuration, /1개 결과 · \d+ms/);
  assert.match(withDuration, /1개 결과 · 42ms/);
  assert.match(modelDuration, /1개 결과 · 47ms/);
});

test("React navigator renders explicit asset search results as a separate truthful group", () => {
  const loading = createDocumentNavigatorLoadingModel({
    workspaceId: "workspace-1",
    view: "Tree",
    filter: "첨부",
    generation: 3,
  });
  const ready = applyDocumentNavigatorResult(loading, 3, {
    workspaceId: "workspace-1",
    view: "Tree",
    state: "Ready",
    items: [
      {
        documentId: "doc-1",
        title: "첨부 설계",
        path: "notes/attachments.md",
        collections: [],
        tags: [],
        favorite: false,
      },
    ],
    searchMetrics: { durationMs: 42 },
  });

  const markup = renderToStaticMarkup(
    createDesktopDocumentNavigatorElement({
      ...ready,
      assetResults: [
        {
          assetId: "asset-secret-1",
          fileName: "/Users/private/specification.pdf",
          mediaType: "application/pdf",
          byteSize: 1536,
          score: 3,
        },
      ],
    }, callbacks()),
  );

  assert.match(markup, /data-search-result-group="document"/);
  assert.match(markup, /data-search-result-group="asset"/);
  assert.match(markup, /<h2 id="navigator-asset-results-title">첨부 파일<\/h2>/);
  assert.match(markup, /첨부 파일/);
  assert.match(markup, /PDF 문서 · 1.5 KB/);
  assert.match(markup, /2개 결과 · 42ms/);
  assert.doesNotMatch(markup, /asset-secret-1<\/|\/Users\/private|application\/pdf|specification\.pdf/);
});

test("React navigator exposes a bounded overlay close action through search escape", () => {
  const loading = createDocumentNavigatorLoadingModel({
    workspaceId: "workspace-1",
    view: "Tree",
    filter: "  연결 문서  ",
    generation: 1,
  });
  const ready = applyDocumentNavigatorResult(loading, 1, {
    workspaceId: "workspace-1",
    view: "Tree",
    state: "Ready",
    items: [],
  });
  let closedQuery = "";
  const tree = createDesktopDocumentNavigatorElement(ready, {
    ...callbacks(),
    onSearchEscape(query) { closedQuery = query; },
  });

  clickElement(tree, (props) => props["data-action"] === "close-global-search");

  assert.equal(closedQuery, "연결 문서");
});

test("React navigator routes global search overlay keyboard handling through the modal policy", () => {
  assert.match(navigatorSource, /handleModalKeyboard/);
  assert.match(navigatorSource, /onKeyDown: callbacks\.onSearchEscape/);
  assert.match(navigatorSource, /handleModalKeyboard\(event, \(\) => callbacks\.onSearchEscape\?\.\(overlay\.query\)\)/);
  assert.doesNotMatch(navigatorSource, /event\.key === "Tab"[\s\S]{0,280}focus\(/);
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
    /검색 결과가 없습니다/,
  );
  assert.match(
    renderToStaticMarkup(createDesktopDocumentNavigatorElement({
      ...empty,
      searchMetrics: { durationMs: 38 },
    }, callbacks())),
    /0개 결과 · 38ms/,
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

test("React navigator renders only the bounded result window and explicit range actions", () => {
  const loading = createDocumentNavigatorLoadingModel({ workspaceId: "workspace-1", view: "Tree", generation: 5 });
  const ready = applyDocumentNavigatorResult(loading, 5, {
    workspaceId: "workspace-1",
    view: "Tree",
    state: "Ready",
    items: Array.from({ length: 50 }, (_, index) => ({
      documentId: `doc-${index + 1}`,
      title: `문서 ${index + 1}`,
      path: `notes/${index + 1}.md`,
      collections: [], tags: [], favorite: false,
    })),
  });
  const first = renderToStaticMarkup(createDesktopDocumentNavigatorElement(ready, callbacks(), {
    resultWindow: createDesktopSearchResultWindow(5, 50),
  }));
  const secondWindow = { ...createDesktopSearchResultWindow(5, 50), offset: 20 };
  const second = renderToStaticMarkup(createDesktopDocumentNavigatorElement(ready, callbacks(), {
    resultWindow: secondWindow,
  }));

  assert.equal((first.match(/data-action="open-navigator-document"/g) ?? []).length, 20);
  assert.match(first, /1-20 \/ 50/);
  assert.match(first, /data-action="previous-search-results"[^>]*disabled/);
  assert.doesNotMatch(first, /data-action="next-search-results"[^>]*disabled/);
  assert.match(second, /21-40 \/ 50/);
  const secondResultList = second.match(/<ul class="item-list">[\s\S]*?<\/ul>/)?.[0] ?? "";
  assert.match(secondResultList, />문서 21</);
  assert.doesNotMatch(secondResultList, />문서 1</);

  const loadingMarkup = renderToStaticMarkup(createDesktopDocumentNavigatorElement(loading, callbacks(), {
    resultWindow: createDesktopSearchResultWindow(5, 0),
  }));
  assert.doesNotMatch(loadingMarkup, /(?:previous|next)-search-results/);
});

test("React navigator presents a bounded escaped search snippet without path fallback", () => {
  const loading = createDocumentNavigatorLoadingModel({ workspaceId: "workspace-1", view: "Tree", filter: "needle", generation: 8 });
  const ready = applyDocumentNavigatorResult(loading, 8, {
    workspaceId: "workspace-1", view: "Tree", state: "Ready",
    items: [{
      documentId: "doc-secret", title: "검색 문서", path: "private/secret.md",
      snippet: `  <script>alert(1)</script>   ${"긴본문".repeat(80)}  `,
      collections: [], tags: [], favorite: false,
    }],
  });
  const markup = renderToStaticMarkup(createDesktopDocumentNavigatorElement(ready, callbacks()));
  assert.match(markup, /class="search-result-snippet"/);
  assert.match(markup, /&lt;script&gt;alert\(1\)&lt;\/script&gt;/);
  assert.doesNotMatch(markup, /<script>|private\/secret\.md|doc-secret<\/small>/);
  const snippet = markup.match(/class="search-result-snippet">([^<]*)<\/span>/)?.[1] ?? "";
  assert.ok(snippet.length <= 200);
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
    onPreviousResults() {},
    onNextResults() {},
  };
}

function assertNoUnidentifiedInteractiveControls(markup: string): void {
  const controls = markup.match(/<(?:button|input|select|textarea|a)\b[^>]*>/g) ?? [];
  assert.ok(controls.length > 0);
  assert.deepEqual(controls.filter((control) => !control.includes("data-action=")), []);
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
