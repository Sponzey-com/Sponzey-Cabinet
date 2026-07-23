import assert from "node:assert/strict";
import test from "node:test";

import {
  LocalDesktopCommandClientError,
  type DocumentNavigatorQuery,
  type DocumentNavigatorResult,
} from "@sponzey-cabinet/client-core";
import {
  createDocumentNavigatorLoadingModel,
  transitionDocumentNavigatorModel,
} from "@sponzey-cabinet/ui";

import { loadDesktopDocumentNavigator } from "../src/desktop_navigator_controller.ts";

test("desktop navigator controller dispatches model query and maps ready result", async () => {
  const calls: DocumentNavigatorQuery[] = [];
  const model = createDocumentNavigatorLoadingModel({
    workspaceId: "workspace-1",
    view: "Collection",
    viewKey: "work",
    generation: 2,
  });
  const client = {
    async getDocumentNavigator(query: DocumentNavigatorQuery): Promise<DocumentNavigatorResult> {
      calls.push(query);
      return {
        workspaceId: "workspace-1",
        view: "Collection",
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
      };
    },
  };

  const result = await loadDesktopDocumentNavigator(client, model);

  assert.deepEqual(calls, [
    {
      workspaceId: "workspace-1",
      view: "Collection",
      viewKey: "work",
      limit: 50,
    },
  ]);
  assert.equal(result.displayState, "Ready");
  assert.equal(result.items[0]?.documentId, "doc-1");
});

test("desktop navigator controller uses the full-text search port for a search query", async () => {
  let navigatorCalls = 0;
  const searchCalls: Array<{ readonly workspaceId: string; readonly text: string; readonly limit: number }> = [];
  const assetSearchCalls: Array<{ readonly workspaceId: string; readonly text: string; readonly limit: number }> = [];
  const clockValues = [1000, 1042];
  const model = createDocumentNavigatorLoadingModel({
    workspaceId: "workspace-1",
    view: "Tree",
    filter: "본문 전용 키워드",
    generation: 4,
  });
  const client = {
    async getDocumentNavigator(): Promise<DocumentNavigatorResult> {
      navigatorCalls += 1;
      throw new Error("navigator must not serve full-text search");
    },
    async searchDocuments(query: { readonly workspaceId: string; readonly text: string; readonly limit: number }) {
      searchCalls.push(query);
      return {
        queryName: "search-documents" as const,
        workspaceId: "workspace-1",
        text: "본문 전용 키워드",
        results: [{ documentId: "doc-2", workspaceId: "workspace-1", title: "검색 대상", path: "notes/target.md", snippet: "본문 전용 키워드가 있습니다", score: 1 }],
      };
    },
    async searchAssets(query: { readonly workspaceId: string; readonly text: string; readonly limit: number }) {
      assetSearchCalls.push(query);
      return {
        queryName: "search-assets" as const,
        workspaceId: "workspace-1",
        text: "본문 전용 키워드",
        results: [
          {
            assetId: "asset-1",
            fileName: "reference.pdf",
            mediaType: "application/pdf",
            byteSize: 1024,
            score: 2,
          },
        ],
      };
    },
  };

  const result = await loadDesktopDocumentNavigator(client, model, {
    nowMs: () => clockValues.shift() ?? 1042,
  });

  assert.equal(navigatorCalls, 0);
  assert.deepEqual(searchCalls, [{ workspaceId: "workspace-1", text: "본문 전용 키워드", limit: 50, queryName: "search-documents" }]);
  assert.deepEqual(assetSearchCalls, [{ workspaceId: "workspace-1", text: "본문 전용 키워드", limit: 50, queryName: "search-assets" }]);
  assert.equal(result.displayState, "Ready");
  assert.equal(result.items[0]?.documentId, "doc-2");
  assert.equal(result.assetResults[0]?.assetId, "asset-1");
  assert.equal(result.items[0]?.snippet, "본문 전용 키워드가 있습니다");
  assert.equal(result.searchMetrics?.durationMs, 42);
});

test("desktop navigator controller does not invent search metrics for ordinary navigator queries", async () => {
  const model = createDocumentNavigatorLoadingModel({
    workspaceId: "workspace-1",
    view: "Tree",
    generation: 5,
  });
  const client = {
    async getDocumentNavigator(): Promise<DocumentNavigatorResult> {
      return {
        workspaceId: "workspace-1",
        view: "Tree",
        state: "Ready",
        items: [],
      };
    },
  };

  const result = await loadDesktopDocumentNavigator(client, model, {
    nowMs: () => {
      throw new Error("ordinary navigator queries must not measure search duration");
    },
  });

  assert.equal(result.displayState, "Ready");
  assert.equal(result.searchMetrics, undefined);
});

test("desktop navigator controller returns safe retryable failure and rejects invalid query", async () => {
  const failing = {
    async getDocumentNavigator(): Promise<DocumentNavigatorResult> {
      throw new LocalDesktopCommandClientError(
        "DOCUMENT_NAVIGATOR_PROJECTION_UNAVAILABLE",
        true,
      );
    },
  };
  const invalid = transitionDocumentNavigatorModel(
    createDocumentNavigatorLoadingModel({
      workspaceId: "workspace-1",
      view: "Tree",
      generation: 1,
    }),
    { type: "ViewSelected", view: "Tag", generation: 2 },
  );
  const failed = await loadDesktopDocumentNavigator(
    failing,
    createDocumentNavigatorLoadingModel({
      workspaceId: "workspace-1",
      view: "Recent",
      generation: 3,
    }),
  );
  const invalidResult = await loadDesktopDocumentNavigator(failing, invalid);

  assert.equal(failed.displayState, "Failed");
  assert.equal(failed.error?.code, "DOCUMENT_NAVIGATOR_PROJECTION_UNAVAILABLE");
  assert.equal(failed.error?.retryable, true);
  assert.equal(invalidResult.displayState, "Failed");
  assert.equal(invalidResult.error?.code, "DOCUMENT_NAVIGATOR_INVALID_QUERY");
  assert.equal(JSON.stringify(failed).includes("raw document body"), false);
});

test("desktop navigator controller result stays generation-bound for stale response guard", async () => {
  let resolveResult: ((value: DocumentNavigatorResult) => void) | undefined;
  const client = {
    getDocumentNavigator(): Promise<DocumentNavigatorResult> {
      return new Promise((resolve) => {
        resolveResult = resolve;
      });
    },
  };
  const model = createDocumentNavigatorLoadingModel({
    workspaceId: "workspace-1",
    view: "Tree",
    generation: 7,
  });
  const pending = loadDesktopDocumentNavigator(client, model);
  resolveResult?.({
    workspaceId: "workspace-1",
    view: "Tree",
    state: "EmptyResult",
    items: [],
  });
  const result = await pending;

  assert.equal(result.generation, 7);
  assert.equal(result.displayState, "EmptyResult");
});
