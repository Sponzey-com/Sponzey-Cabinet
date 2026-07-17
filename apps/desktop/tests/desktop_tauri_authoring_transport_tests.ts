import assert from "node:assert/strict";
import test from "node:test";

import type { LocalDesktopCommandEnvelope } from "@sponzey-cabinet/client-core";

import { createTauriDocumentAuthoringTransport } from "../src/tauri_authoring_transport.ts";
import { createTauriDesktopTransport } from "../src/tauri_desktop_transport.ts";

test("authoring transport maps create current and revision save to tagged native requests", async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const transport = createTauriDocumentAuthoringTransport(async (command, args) => {
    calls.push({ command, args });
    const request = args?.request as { kind?: string };
    if (command === "execute_desktop_document_query") {
      return {
        ok: true,
        retryable: false,
        repairRequired: false,
        data: {
          kind: "current",
          currentVersionToken: "v1",
          revisionNumber: 1,
          title: "Source",
          body: "body one",
          hasMore: false,
        },
      };
    }
    return {
      ok: true,
      retryable: false,
      repairRequired: false,
      data: {
        kind: request.kind === "create" ? "created" : "updated",
        documentId: "doc-1",
        currentVersionId: request.kind === "create" ? "v1" : "v2",
      },
    };
  });

  const created = await transport(createEnvelope());
  const current = await transport(currentEnvelope());
  const saved = await transport(saveEnvelope());

  assert.equal(created.ok, true);
  assert.equal(current.ok && (current.data as { versionId: string }).versionId, "v1");
  assert.equal(saved.ok && (saved.data as { revision: number }).revision, 7);
  assert.deepEqual(calls.map((call) => call.command), [
    "execute_desktop_document_mutation",
    "run_desktop_projection_worker",
    "execute_desktop_document_query",
    "execute_desktop_document_mutation",
    "run_desktop_projection_worker",
  ]);
  assert.equal(calls[1]?.args, undefined);
  assert.equal(calls[4]?.args, undefined);
  assert.deepEqual(calls[0]?.args, {
    request: {
      kind: "create",
      operationId: "operation-create-1",
      workspaceId: "workspace-1",
      documentId: "doc-1",
      body: "body one",
      author: "local-user",
      summary: "Created",
    },
  });
  assert.deepEqual(calls[3]?.args, {
    request: {
      kind: "update",
      operationId: "operation-save-1",
      workspaceId: "workspace-1",
      documentId: "doc-1",
      body: "body two",
      expectedCurrentVersionId: "v1",
      author: "local-user",
      summary: "Updated",
    },
  });
});

test("authoring transport preserves safe native failure and rejects malformed response", async () => {
  const nativeFailure = createTauriDocumentAuthoringTransport(async () => ({
    ok: false,
    errorCode: "DOCUMENT_AUTHORING_POINTER_UPDATE_FAILED",
    retryable: true,
    repairRequired: true,
    message: "body two notes/source.md",
  }));
  const malformed = createTauriDocumentAuthoringTransport(async () => ({
    ok: true,
    data: { kind: "current", body: "missing identity" },
  }));

  assert.deepEqual(await nativeFailure(saveEnvelope()), {
    ok: false,
    errorCode: "DOCUMENT_AUTHORING_POINTER_UPDATE_FAILED",
    retryable: true,
    repairRequired: true,
  });
  assert.deepEqual(await malformed(currentEnvelope()), {
    ok: false,
    errorCode: "COMMAND_BRIDGE_FAILED",
    retryable: false,
    repairRequired: false,
  });
});

test("authoring transport maps rename metadata without body and verifies renamed response", async () => {
  let nativeRequest: unknown;
  const transport = createTauriDocumentAuthoringTransport(async (_command, args) => {
    if (args?.request) nativeRequest = args.request;
    return { ok: true, retryable: false, repairRequired: false, data: { kind: "renamed", documentId: "doc-1", currentVersionId: "v1", title: "새 제목", path: "notes/source.md" } };
  });
  const result = await transport({ commandName: "rename_document", payload: { workspaceId: "workspace-1", documentId: "doc-1", currentVersionId: "v1", title: "새 제목", path: "notes/source.md" } } as LocalDesktopCommandEnvelope);
  assert.deepEqual(nativeRequest, { kind: "rename", workspaceId: "workspace-1", documentId: "doc-1", currentVersionId: "v1", title: "새 제목", path: "notes/source.md" });
  assert.equal(result.ok && (result.data as { title: string }).title, "새 제목");
});

test("rename resolves only after the projection worker finishes its synchronization attempt", async () => {
  let releaseProjection!: () => void;
  let projectionStarted!: () => void;
  const projectionStartedPromise = new Promise<void>((resolve) => { projectionStarted = resolve; });
  const projectionReleasePromise = new Promise<void>((resolve) => { releaseProjection = resolve; });
  const transport = createTauriDocumentAuthoringTransport(async (command) => {
    if (command === "run_desktop_projection_worker") {
      projectionStarted();
      await projectionReleasePromise;
      return { ok: true };
    }
    return {
      ok: true,
      retryable: false,
      repairRequired: false,
      data: { kind: "renamed", documentId: "doc-1", currentVersionId: "v1", title: "동기화된 제목", path: "notes/source.md" },
    };
  });
  let completed = false;
  const resultPromise = transport({
    commandName: "rename_document",
    payload: { workspaceId: "workspace-1", documentId: "doc-1", currentVersionId: "v1", title: "동기화된 제목", path: "notes/source.md" },
  } as LocalDesktopCommandEnvelope).then((result) => { completed = true; return result; });

  await projectionStartedPromise;
  await Promise.resolve();
  assert.equal(completed, false);
  releaseProjection();
  const result = await resultPromise;
  assert.equal(result.ok && (result.data as { title: string }).title, "동기화된 제목");
});

test("desktop composite transport routes authoring commands and triggers projection without payload", async () => {
  const calls: string[] = [];
  const transport = createTauriDesktopTransport(async (command) => {
    calls.push(command);
    return {
      ok: true,
      retryable: false,
      repairRequired: false,
      data: {
        kind: "created",
        documentId: "doc-1",
        currentVersionId: "v1",
      },
    };
  });

  const result = await transport(createEnvelope());

  assert.equal(result.ok, true);
  assert.deepEqual(calls, [
    "execute_desktop_document_mutation",
    "run_desktop_projection_worker",
  ]);
});

test("projection trigger failure never changes a verified durable save result", async () => {
  const calls: string[] = [];
  const transport = createTauriDocumentAuthoringTransport(async (command) => {
    calls.push(command);
    if (command === "run_desktop_projection_worker") {
      throw new Error("projection unavailable");
    }
    return {
      ok: true,
      retryable: false,
      repairRequired: false,
      data: { kind: "updated", documentId: "doc-1", currentVersionId: "v2" },
    };
  });

  const result = await transport(saveEnvelope());
  await Promise.resolve();

  assert.equal(result.ok, true);
  assert.deepEqual(calls, [
    "execute_desktop_document_mutation",
    "run_desktop_projection_worker",
  ]);
});

test("authoring transport maps history version preview and guarded restore commands", async () => {
  const nativeCalls: Array<{ command: string; kind: string }> = [];
  let historyRequest: unknown;
  let previewRequest: unknown;
  let restoreRequest: unknown;
  const transport = createTauriDocumentAuthoringTransport(async (command, args) => {
    const request = args?.request as { kind?: string };
    nativeCalls.push({ command, kind: String(request.kind) });
    if (request.kind === "history") {
      historyRequest = request;
      return {
        ok: true,
        retryable: false,
        repairRequired: false,
        data: {
          kind: "history",
          entries: [{ revisionNumber: 1, versionToken: "v1", summary: "Created", author: "local-user", createdAtEpochMs: 1783900800000 }],
          nextCursor: "cursor-1",
          hasMore: true,
        },
      };
    }
    if (request.kind === "version") {
      return {
        ok: true,
        retryable: false,
        repairRequired: false,
        data: {
          kind: "version",
          versionToken: "v1",
          revisionNumber: 1,
          title: "Historical",
          body: "historical body",
          hasMore: false,
        },
      };
    }
    if (request.kind === "preview_restore") {
      previewRequest = request;
      return {
        ok: true,
        retryable: false,
        repairRequired: false,
        data: {
          kind: "restore_preview",
          documentId: "doc-1",
          targetVersionId: "v1",
          expectedCurrentVersionId: "v2",
          canRestore: true,
          missingAssetLabels: [],
          lines: [{ kind: "added", text: "sanitized line" }],
          restoreDiff: {
            kind: "complete",
            leftVersionToken: "v2",
            rightVersionToken: "v1",
            addedCount: 1,
            removedCount: 1,
            attachmentDiff: {
              kind: "known",
              added: [],
              removed: [],
              relabeled: [],
              unchangedCount: 0,
            },
            titleDelta: {
              kind: "changed",
              before: "현재 제목",
              after: "과거 제목",
            },
            hunks: [{
              oldStartLine: 1,
              newStartLine: 1,
              addedCount: 1,
              removedCount: 1,
              lines: [
                { kind: "removed", text: "current", oldLineNumber: 1 },
                { kind: "added", text: "historical", newLineNumber: 1 },
              ],
            }],
          },
        },
      };
    }
    restoreRequest = request;
    return {
      ok: true,
      retryable: false,
      repairRequired: false,
      data: {
        kind: "restored",
        documentId: "doc-1",
        restoredVersionId: "v3",
        currentVersionId: "v3",
        revisionNumber: 3,
      },
    };
  });

  const history = await transport(historyEnvelope());
  const version = await transport(versionEnvelope());
  const preview = await transport(previewEnvelope());
  const restored = await transport(restoreEnvelope());

  assert.deepEqual(nativeCalls, [
    { command: "execute_desktop_document_query", kind: "history" },
    { command: "execute_desktop_document_query", kind: "version" },
    { command: "execute_desktop_document_authoring", kind: "preview_restore" },
    { command: "execute_desktop_document_authoring", kind: "restore" },
  ]);
  assert.equal(history.ok && (history.data as { entries: unknown[] }).entries.length, 1);
  assert.equal(history.ok && (history.data as { nextCursor?: string }).nextCursor, "cursor-1");
  assert.deepEqual(historyRequest, {
    kind: "history",
    workspaceId: "workspace-1",
    documentId: "doc-1",
    cursor: "cursor-0",
    limit: 20,
  });
  assert.equal(version.ok && (version.data as { versionId: string }).versionId, "v1");
  assert.equal(preview.ok && (preview.data as { expectedCurrentVersionId: string }).expectedCurrentVersionId, "v2");
  assert.deepEqual(
    preview.ok && (preview.data as { missingAssetLabels: readonly string[] }).missingAssetLabels,
    [],
  );
  assert.deepEqual(previewRequest, {
    kind: "preview_restore",
    workspaceId: "workspace-1",
    documentId: "doc-1",
    targetVersionId: "v1",
  });
  assert.equal(preview.ok && (preview.data as { diff: { status: string } }).diff.status, "Complete");
  assert.deepEqual(restoreRequest, {
    kind: "restore",
    operationId: "operation-restore-1",
    workspaceId: "workspace-1",
    documentId: "doc-1",
    targetVersionId: "v1",
    expectedCurrentVersionId: "v2",
    author: "local-user",
    summary: "Restore v1",
  });
  assert.equal(restored.ok && (restored.data as { currentVersionId: string }).currentVersionId, "v3");
});

function createEnvelope(): LocalDesktopCommandEnvelope {
  return {
    commandName: "create_document",
    payload: {
      operationId: "operation-create-1",
      workspaceId: "workspace-1",
      documentId: "doc-1",
      body: "body one",
      author: "local-user",
      summary: "Created",
    },
  };
}

function currentEnvelope(): LocalDesktopCommandEnvelope {
  return {
    commandName: "get_current_document",
    payload: {
      queryName: "get-current-document",
      workspaceId: "workspace-1",
      documentId: "doc-1",
    },
  };
}

function saveEnvelope(): LocalDesktopCommandEnvelope {
  return {
    commandName: "save_document_revision",
    payload: {
      operationId: "operation-save-1",
      workspaceId: "workspace-1",
      documentId: "doc-1",
      body: "body two",
      expectedVersionId: "v1",
      author: "local-user",
      summary: "Updated",
      revision: 7,
    },
  };
}

function historyEnvelope(): LocalDesktopCommandEnvelope {
  return {
    commandName: "get_document_history",
    payload: {
      queryName: "get-document-history",
      workspaceId: "workspace-1",
      documentId: "doc-1",
      cursor: "cursor-0",
      limit: 20,
    },
  };
}

function versionEnvelope(): LocalDesktopCommandEnvelope {
  return {
    commandName: "get_document_version",
    payload: {
      queryName: "get-document-version",
      workspaceId: "workspace-1",
      documentId: "doc-1",
      versionId: "v1",
    },
  };
}

function previewEnvelope(): LocalDesktopCommandEnvelope {
  return {
    commandName: "preview_document_restore",
    payload: {
      queryName: "preview-document-restore",
      workspaceId: "workspace-1",
      documentId: "doc-1",
      targetVersionId: "v1",
    },
  };
}

function restoreEnvelope(): LocalDesktopCommandEnvelope {
  return {
    commandName: "restore_document_version",
    payload: {
      commandName: "restore-document-version",
      operationId: "operation-restore-1",
      workspaceId: "workspace-1",
      documentId: "doc-1",
      targetVersionId: "v1",
      expectedCurrentVersionId: "v2",
      author: "local-user",
      summary: "Restore v1",
    },
  };
}
