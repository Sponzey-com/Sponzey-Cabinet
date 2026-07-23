import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  applyDocumentNavigatorResult,
  createDocumentNavigatorFailedModel,
  createDocumentNavigatorLoadingModel,
  type DocumentNavigatorDisplayState,
} from "@sponzey-cabinet/ui";

import { presentGlobalSearchOverlay } from "../src/global_search_overlay_presenter.ts";

test("global search overlay presenter maps navigator states to explicit overlay states", () => {
  const states: Readonly<Record<DocumentNavigatorDisplayState, string>> = {
    Closed: "Closed",
    Loading: "Searching",
    Filtering: "Searching",
    Ready: "ResultsReady",
    EmptyResult: "Empty",
    Degraded: "Degraded",
    Failed: "Failed",
  };

  for (const [displayState, expected] of Object.entries(states)) {
    assert.equal(presentGlobalSearchOverlay(modelWithState(displayState as DocumentNavigatorDisplayState)).state, expected);
  }
});

test("global search overlay presenter owns Penpot copy and safe close affordance labels", () => {
  const model = createDocumentNavigatorLoadingModel({
    workspaceId: "workspace-1",
    view: "Tree",
    filter: "  링크  ",
    generation: 1,
  });
  const overlay = presentGlobalSearchOverlay(model);

  assert.equal(overlay.title, "전체 검색");
  assert.equal(overlay.description, "제목, 본문, 첨부 파일 이름을 한 번에 검색합니다.");
  assert.equal(overlay.closeLabel, "검색 닫기");
  assert.equal(overlay.query, "링크");
});

test("global search overlay presenter stays free of runtime IO and React imports", async () => {
  const source = await readFile(new URL("../src/global_search_overlay_presenter.ts", import.meta.url), "utf8");
  const navigatorSource = await readFile(new URL("../src/react_document_navigator.ts", import.meta.url), "utf8");

  assert.doesNotMatch(source, /React|document\.|window\.|localStorage|sessionStorage|process\.env|console\./);
  assert.match(navigatorSource, /presentGlobalSearchOverlay/);
  assert.doesNotMatch(navigatorSource, /function globalSearchOverlayState/);
});

function modelWithState(displayState: DocumentNavigatorDisplayState) {
  const loading = createDocumentNavigatorLoadingModel({
    workspaceId: "workspace-1",
    view: "Tree",
    generation: 1,
  });
  if (displayState === "Failed") {
    return createDocumentNavigatorFailedModel({
      workspaceId: "workspace-1",
      view: "Tree",
      generation: 1,
      errorCode: "DOCUMENT_NAVIGATOR_PROJECTION_UNAVAILABLE",
      retryable: true,
    });
  }
  if (displayState === "Ready" || displayState === "EmptyResult" || displayState === "Degraded") {
    return applyDocumentNavigatorResult(loading, 1, {
      workspaceId: "workspace-1",
      view: "Tree",
      state: displayState,
      items: [],
    });
  }
  return { ...loading, displayState };
}
