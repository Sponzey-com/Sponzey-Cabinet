import assert from "node:assert/strict";
import test from "node:test";

import {
  createLocalDesktopCommandClient,
  type LocalDesktopCommandEnvelope,
  type LocalDesktopCommandTransport,
} from "../src/index.ts";

test("local desktop client dispatches typed bounded navigator query", async () => {
  const calls: LocalDesktopCommandEnvelope[] = [];
  const transport: LocalDesktopCommandTransport = async (request) => {
    calls.push(request);
    return {
      ok: true,
      data: {
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
        nextCursor: "20",
      },
    };
  };
  const client = createLocalDesktopCommandClient(transport);

  const result = await client.getDocumentNavigator({
    workspaceId: "workspace-1",
    view: "Collection",
    viewKey: "work",
    filter: "arch",
    limit: 20,
  });

  assert.equal(calls.length, 1);
  assert.equal(calls[0]?.commandName, "local_document_navigator");
  assert.deepEqual(calls[0]?.payload, {
    workspaceId: "workspace-1",
    view: "Collection",
    viewKey: "work",
    filter: "arch",
    limit: 20,
  });
  assert.equal(result.state, "Ready");
  assert.equal(result.items[0]?.documentId, "doc-1");
  assert.equal(result.nextCursor, "20");
});

test("navigator client preserves safe command failures without payload text", async () => {
  const client = createLocalDesktopCommandClient(async () => ({
    ok: false,
    errorCode: "DOCUMENT_NAVIGATOR_PROJECTION_UNAVAILABLE",
    retryable: true,
    message: "notes/private.md raw document body",
  }));

  await assert.rejects(
    () =>
      client.getDocumentNavigator({
        workspaceId: "workspace-1",
        view: "Tree",
        limit: 20,
      }),
    (error: unknown) => {
      assert.equal(
        (error as { code?: string }).code,
        "DOCUMENT_NAVIGATOR_PROJECTION_UNAVAILABLE",
      );
      assert.equal(String(error).includes("notes/private.md"), false);
      return true;
    },
  );
});
