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
    filter: "arch",
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
      filter: "arch",
      limit: 50,
    },
  ]);
  assert.equal(result.displayState, "Ready");
  assert.equal(result.items[0]?.documentId, "doc-1");
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
