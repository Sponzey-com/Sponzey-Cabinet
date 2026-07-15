import assert from "node:assert/strict";
import test from "node:test";

import {
  createLocalDesktopCommandClient,
  LocalDesktopCommandClientError,
  type LocalDesktopCommandEnvelope,
  type LocalDesktopCommandTransport,
} from "../src/index.ts";

test("authoring client dispatches explicit create current and revision save commands", async () => {
  const calls: LocalDesktopCommandEnvelope[] = [];
  const transport: LocalDesktopCommandTransport = async (request) => {
    calls.push(request);
    if (request.commandName === "create_document") {
      return {
        ok: true,
        data: {
          workspaceId: "workspace-1",
          documentId: "doc-1",
          currentVersionId: "v1",
        },
      };
    }
    if (request.commandName === "save_document_revision") {
      return {
        ok: true,
        data: {
          status: "saved-local",
          workspaceId: "workspace-1",
          documentId: "doc-1",
          currentVersionId: "v2",
          versionAppended: true,
          revision: 7,
        },
      };
    }
    if (request.commandName === "rename_document") {
      return { ok: true, data: { workspaceId: "workspace-1", documentId: "doc-1", currentVersionId: "v2", title: "새 제목", path: "notes/source.md" } };
    }
    return {
      ok: true,
      data: {
        workspaceId: "workspace-1",
        documentId: "doc-1",
        title: "Source",
        path: "notes/source.md",
        body: "body one",
        versionId: "v1",
      },
    };
  };
  const client = createLocalDesktopCommandClient(transport);

  await client.createDocument(createCommand());
  await client.getCurrentDocument({
    queryName: "get-current-document",
    workspaceId: "workspace-1",
    documentId: "doc-1",
  });
  const saved = await client.saveDocumentRevision(saveCommand());
  const renamed = await client.renameDocument({
    workspaceId: "workspace-1",
    documentId: "doc-1",
    currentVersionId: "v2",
    title: "새 제목",
    path: "notes/source.md",
  });

  assert.deepEqual(calls.map((call) => call.commandName), [
    "create_document",
    "get_current_document",
    "save_document_revision",
    "rename_document",
  ]);
  assert.equal(calls[2]?.payload.revision, 7);
  assert.equal(calls[2]?.payload.nextVersionId, "v2");
  assert.equal(saved.revision, 7);
  assert.equal(saved.currentVersionId, "v2");
  assert.equal(renamed.title, "새 제목");
});

test("authoring client preserves repair metadata without native message or body leakage", async () => {
  const client = createLocalDesktopCommandClient(async () => ({
    ok: false,
    errorCode: "DOCUMENT_AUTHORING_POINTER_UPDATE_FAILED",
    retryable: true,
    repairRequired: true,
    message: "notes/source.md body two",
  }));

  await assert.rejects(
    () => client.saveDocumentRevision(saveCommand()),
    (error: unknown) => {
      assert.ok(error instanceof LocalDesktopCommandClientError);
      assert.equal(error.code, "DOCUMENT_AUTHORING_POINTER_UPDATE_FAILED");
      assert.equal(error.retryable, true);
      assert.equal(error.repairRequired, true);
      assert.equal(String(error).includes("body two"), false);
      assert.equal(String(error).includes("notes/source.md"), false);
      return true;
    },
  );
});

function createCommand() {
  return {
    workspaceId: "workspace-1",
    documentId: "doc-1",
    title: "Source",
    path: "notes/source.md",
    body: "body one",
    versionId: "v1",
    snapshotRef: "snapshot-v1",
    author: "local-user",
    summary: "Created",
  };
}

function saveCommand() {
  return {
    workspaceId: "workspace-1",
    documentId: "doc-1",
    body: "body two",
    expectedVersionId: "v1",
    nextVersionId: "v2",
    snapshotRef: "snapshot-v2",
    author: "local-user",
    summary: "Updated",
    revision: 7,
  };
}
