import assert from "node:assert/strict";
import test from "node:test";
import { renderToStaticMarkup } from "react-dom/server";

import { createDesktopDocumentEmptyStateElement } from "../src/react_document_empty_state.ts";

test("document empty state stays inside the shared document workspace", () => {
  const html = renderToStaticMarkup(createDesktopDocumentEmptyStateElement({
    onCreateDocument() {},
    onHome() {},
    onSearch() {},
  }));

  assert.match(html, /data-shell-route="Document"/);
  assert.match(html, /data-document-empty-state="true"/);
  assert.match(html, />문서가 없습니다</);
  assert.match(html, /data-action="new-document"/);
  assert.match(html, /data-action="navigate-search"/);
  assert.doesNotMatch(html, /doc-[0-9a-f-]{8,}/i);
  assert.doesNotMatch(html, /notes\//i);
});

test("document empty state exposes one primary create action without creating on render", () => {
  let createCalls = 0;
  const element = createDesktopDocumentEmptyStateElement({
    onCreateDocument() { createCalls += 1; },
    onHome() {},
    onSearch() {},
  });

  renderToStaticMarkup(element);
  assert.equal(createCalls, 0);
});
