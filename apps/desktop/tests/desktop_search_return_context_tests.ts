import assert from "node:assert/strict";
import test from "node:test";

import {
  captureDesktopSearchViewport,
  createDesktopSearchReturnContext,
  restoreDesktopSearchViewport,
  transitionDesktopSearchReturnContext,
} from "../src/desktop_search_return_context.ts";

test("search return context follows Results DocumentOpen Results without flags", () => {
  const results = transitionDesktopSearchReturnContext(
    createDesktopSearchReturnContext(),
    { type: "SearchStarted", query: "지식 지도" },
  );
  const captured = transitionDesktopSearchReturnContext(results, {
    type: "ViewportCaptured", query: "지식 지도", selectedIndex: 3, scrollOffset: 240,
  });
  const opened = transitionDesktopSearchReturnContext(captured, {
    type: "ResultOpened", query: "지식 지도", documentId: "doc-4",
  });
  const returned = transitionDesktopSearchReturnContext(opened, { type: "ReturnRequested" });

  assert.equal(opened.status, "DocumentOpen");
  assert.deepEqual(returned, {
    status: "Results", query: "지식 지도", selectedIndex: 3, scrollOffset: 240,
  });
});

test("search return context ignores stale events and rejects unsafe values", () => {
  const results = transitionDesktopSearchReturnContext(createDesktopSearchReturnContext(), {
    type: "SearchStarted", query: "current",
  });
  assert.deepEqual(transitionDesktopSearchReturnContext(results, {
    type: "ViewportCaptured", query: "stale", selectedIndex: 1, scrollOffset: 20,
  }), results);
  assert.throws(() => transitionDesktopSearchReturnContext(results, {
    type: "ViewportCaptured", query: "current", selectedIndex: -1, scrollOffset: 0,
  }), /INVALID_SEARCH_RETURN_CONTEXT/);
  assert.throws(() => transitionDesktopSearchReturnContext(results, {
    type: "ResultOpened", query: "current", documentId: " ",
  }), /INVALID_SEARCH_RETURN_CONTEXT/);
});

test("search viewport adapter captures and restores bounded scroll and result focus", () => {
  let scrollTop = 180;
  let focused = 0;
  const buttons = [
    { dataset: { documentId: "doc-1" }, focus() {} },
    { dataset: { documentId: "doc-2" }, focus() { focused += 1; } },
  ];
  const main = {
    get scrollTop() { return scrollTop; },
    set scrollTop(value: number) { scrollTop = value; },
  };
  const root = {
    querySelector: () => main,
    querySelectorAll: () => buttons,
  };
  assert.deepEqual(captureDesktopSearchViewport(root, "doc-2"), {
    selectedIndex: 1,
    scrollOffset: 180,
  });
  scrollTop = 0;
  assert.equal(restoreDesktopSearchViewport(root, {
    status: "Results", query: "query", selectedIndex: 1, scrollOffset: 180,
  }), true);
  assert.equal(scrollTop, 180);
  assert.equal(focused, 1);
});
