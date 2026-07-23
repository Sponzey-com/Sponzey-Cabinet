import assert from "node:assert/strict";
import test from "node:test";

import {
  applyDocumentNavigatorResult,
  createDocumentNavigatorFailedModel,
  createDocumentNavigatorLoadingModel,
  createDocumentNavigatorQuery,
  transitionDocumentNavigatorModel,
} from "../src/index.ts";

test("navigator model maps ready empty and degraded command results", () => {
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
    nextCursor: "20",
  });
  const empty = applyDocumentNavigatorResult(
    createDocumentNavigatorLoadingModel({
      workspaceId: "workspace-1",
      view: "Favorite",
      generation: 2,
      filter: "missing",
    }),
    2,
    {
      workspaceId: "workspace-1",
      view: "Favorite",
      state: "EmptyResult",
      items: [],
    },
  );
  const degraded = applyDocumentNavigatorResult(loading, 1, {
    workspaceId: "workspace-1",
    view: "Tree",
    state: "Degraded",
    items: [],
  });

  assert.equal(ready.displayState, "Ready");
  assert.equal(ready.items[0]?.documentId, "doc-1");
  assert.equal(ready.nextCursor, "20");
  assert.equal(empty.displayState, "EmptyResult");
  assert.equal(empty.filter, "missing");
  assert.equal(degraded.displayState, "Degraded");
});

test("navigator model carries explicit search metrics and clears them on new work", () => {
  const loading = createDocumentNavigatorLoadingModel({
    workspaceId: "workspace-1",
    view: "Tree",
    filter: "성능",
    generation: 10,
  });
  const ready = applyDocumentNavigatorResult(loading, 10, {
    workspaceId: "workspace-1",
    view: "Tree",
    state: "Ready",
    items: [],
    searchMetrics: { durationMs: 42 },
    assetResults: [
      {
        assetId: "asset-1",
        fileName: "source.pdf",
        mediaType: "application/pdf",
        byteSize: 1024,
        score: 3,
      },
    ],
  });
  const filtering = transitionDocumentNavigatorModel(ready, {
    type: "FilterChanged",
    filter: "다음 검색",
    generation: 11,
  });
  const closed = transitionDocumentNavigatorModel(ready, { type: "CloseRequested" });

  assert.equal(ready.searchMetrics?.durationMs, 42);
  assert.equal(ready.assetResults[0]?.assetId, "asset-1");
  assert.equal(filtering.searchMetrics, undefined);
  assert.deepEqual(filtering.assetResults, []);
  assert.equal(closed.searchMetrics, undefined);
  assert.deepEqual(closed.assetResults, []);
});

test("navigator query builder normalizes five views and validates keyed views", () => {
  for (const view of ["Tree", "Recent", "Favorite"] as const) {
    assert.deepEqual(
      createDocumentNavigatorQuery({
        workspaceId: "workspace-1",
        view,
        filter: "  ARCH  ",
        limit: 20,
      }),
      {
        workspaceId: "workspace-1",
        view,
        filter: "arch",
        limit: 20,
      },
    );
  }
  assert.equal(
    createDocumentNavigatorQuery({
      workspaceId: "workspace-1",
      view: "Collection",
      viewKey: " ",
      limit: 20,
    }),
    undefined,
  );
  assert.deepEqual(
    createDocumentNavigatorQuery({
      workspaceId: "workspace-1",
      view: "Tag",
      viewKey: " RUST ",
      limit: 20,
    }),
    {
      workspaceId: "workspace-1",
      view: "Tag",
      viewKey: "rust",
      limit: 20,
    },
  );
  assert.equal(
    createDocumentNavigatorQuery({
      workspaceId: "workspace-1",
      view: "Tree",
      limit: 0,
    }),
    undefined,
  );
});

test("navigator state transition covers filtering retry close and invalid events", () => {
  const loading = createDocumentNavigatorLoadingModel({
    workspaceId: "workspace-1",
    view: "Tree",
    generation: 1,
  });
  const filtering = transitionDocumentNavigatorModel(loading, {
    type: "FilterChanged",
    filter: "arch",
    generation: 2,
  });
  const failed = createDocumentNavigatorFailedModel({
    workspaceId: "workspace-1",
    view: "Tree",
    generation: 2,
    errorCode: "DOCUMENT_NAVIGATOR_PROJECTION_UNAVAILABLE",
    retryable: true,
  });
  const retry = transitionDocumentNavigatorModel(failed, {
    type: "RetryRequested",
    generation: 3,
  });
  const closed = transitionDocumentNavigatorModel(retry, { type: "CloseRequested" });
  const invalid = transitionDocumentNavigatorModel(loading, { type: "RetryRequested", generation: 2 });

  assert.equal(filtering.displayState, "Filtering");
  assert.equal(filtering.filter, "arch");
  assert.equal(retry.displayState, "Loading");
  assert.equal(retry.generation, 3);
  assert.equal(closed.displayState, "Closed");
  assert.equal(invalid.displayState, "Failed");
  assert.equal(invalid.error?.code, "DOCUMENT_NAVIGATOR_INVALID_TRANSITION");
});

test("navigator model ignores stale results and sanitizes failures", () => {
  const loading = createDocumentNavigatorLoadingModel({
    workspaceId: "workspace-1",
    view: "Recent",
    generation: 4,
  });
  const stale = applyDocumentNavigatorResult(loading, 3, {
    workspaceId: "workspace-1",
    view: "Recent",
    state: "Ready",
    items: [],
  });
  const failed = createDocumentNavigatorFailedModel({
    workspaceId: "workspace-1",
    view: "Recent",
    generation: 4,
    errorCode: "DOCUMENT_NAVIGATOR_PROJECTION_UNAVAILABLE",
    retryable: true,
  });

  assert.equal(stale, loading);
  assert.equal(failed.error?.code, "DOCUMENT_NAVIGATOR_PROJECTION_UNAVAILABLE");
  assert.equal(JSON.stringify(failed).includes("/Users/"), false);
  assert.equal(JSON.stringify(failed).includes("raw document body"), false);
});
