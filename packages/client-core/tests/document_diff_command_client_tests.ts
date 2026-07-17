import assert from "node:assert/strict";
import test from "node:test";

import { createLocalDesktopCommandClient } from "../src/index.ts";

test("local client dispatches typed document diff query", async () => {
  const calls: unknown[] = [];
  const client = createLocalDesktopCommandClient(async (request) => {
    calls.push(request);
    return {
      ok: true,
      data: {
        workspaceId: "workspace-1",
        documentId: "doc-1",
        status: "TooLarge",
        leftVersionId: "v2",
        rightVersionId: "v1",
        limitReason: "bytes",
        addedCount: 0,
        removedCount: 0,
        attachmentDiff: { status: "LegacyUnknown" },
        titleDelta: { kind: "Unchanged" },
        hunks: [],
      },
    };
  });

  const result = await client.compareDocumentVersions({
    queryName: "compare-current-document-to-version",
    workspaceId: "workspace-1",
    documentId: "doc-1",
    targetVersionId: "v1",
  });

  assert.deepEqual(calls, [{
    commandName: "compare_document_versions",
    payload: {
      queryName: "compare-current-document-to-version",
      workspaceId: "workspace-1",
      documentId: "doc-1",
      targetVersionId: "v1",
    },
  }]);
  assert.equal(result.status, "TooLarge");
});
