import assert from "node:assert/strict";
import test from "node:test";

import {
  LocalDesktopCommandClientError,
  type CurrentDocumentQuery,
  type CurrentDocumentView,
  type SaveDocumentRevisionCommand,
  type SaveDocumentRevisionResult,
} from "@sponzey-cabinet/client-core";
import { DocumentSaveCoordinatorState } from "@sponzey-cabinet/ui";

import { createDesktopDocumentAuthoringController } from "../src/desktop_document_authoring_controller.ts";

test("authoring controller opens edits and saves only at the 800ms boundary", async () => {
  const saves: SaveDocumentRevisionCommand[] = [];
  const controller = controllerWith({
    async saveDocumentRevision(command) {
      saves.push(command);
      return saveResult(command, "v2");
    },
  });

  await controller.open(identity());
  controller.changeContent("body two");
  await controller.autosaveElapsed(799);
  assert.equal(saves.length, 0);
  await controller.autosaveElapsed(800);

  assert.equal(saves.length, 1);
  assert.equal(saves[0]?.body, "body two");
  assert.equal(saves[0]?.expectedVersionId, "v1");
  assert.equal(saves[0]?.operationId, "save-operation-1");
  assert.equal(saves[0]?.revision, 1);
  assert.equal(controller.snapshot().saveState, DocumentSaveCoordinatorState.Saved);
  assert.equal(controller.snapshot().expectedVersionId, "v2");
});

test("authoring controller keeps one save in flight and queues a newer exact revision", async () => {
  const first = deferred<SaveDocumentRevisionResult>();
  const calls: SaveDocumentRevisionCommand[] = [];
  const controller = controllerWith({
    saveDocumentRevision(command) {
      calls.push(command);
      if (calls.length === 1) return first.promise;
      return Promise.resolve(saveResult(command, "v3"));
    },
  });
  await controller.open(identity());
  controller.changeContent("body two");

  const pending = controller.manualSave();
  controller.changeContent("body three");
  await controller.manualSave();
  assert.equal(calls.length, 1);
  assert.equal(calls[0]?.body, "body two");
  first.resolve(saveResult(calls[0]!, "v2"));
  await pending;

  assert.equal(calls.length, 2);
  assert.equal(calls[1]?.body, "body three");
  assert.equal(calls[1]?.expectedVersionId, "v2");
  assert.equal(calls[0]?.operationId, "save-operation-1");
  assert.equal(calls[1]?.operationId, "save-operation-2");
  assert.equal(calls[1]?.revision, 2);
  assert.equal(controller.snapshot().revision, 2);
  assert.equal(controller.snapshot().persistedRevision, 2);
  assert.equal(controller.snapshot().saveState, DocumentSaveCoordinatorState.Saved);
});

test("authoring controller exposes retry close discard and repair-required read-only recovery", async () => {
  let attempts = 0;
  const retryCommands: SaveDocumentRevisionCommand[] = [];
  const controller = controllerWith({
    async saveDocumentRevision(command) {
      retryCommands.push(command);
      attempts += 1;
      if (attempts === 1) {
        throw new LocalDesktopCommandClientError("DOCUMENT_AUTHORING_STORAGE_UNAVAILABLE", true);
      }
      return saveResult(command, "v2");
    },
  });
  await controller.open(identity());
  controller.changeContent("body retry");
  await controller.manualSave();
  assert.equal(controller.snapshot().saveState, DocumentSaveCoordinatorState.SaveFailed);
  assert.deepEqual(controller.requestClose(), {
    canClose: false,
    choices: ["RetrySave", "Discard", "Cancel"],
  });
  controller.cancelClose();
  await controller.retrySave();
  assert.equal(controller.snapshot().saveState, DocumentSaveCoordinatorState.Saved);
  assert.equal(retryCommands.length, 2);
  assert.equal(retryCommands[0]?.operationId, "save-operation-1");
  assert.equal(retryCommands[1]?.operationId, "save-operation-1");

  const repair = controllerWith({
    async saveDocumentRevision() {
      throw new LocalDesktopCommandClientError(
        "DOCUMENT_AUTHORING_POINTER_UPDATE_FAILED",
        true,
        true,
      );
    },
  });
  await repair.open(identity());
  repair.changeContent("body preserved");
  await repair.manualSave();
  assert.equal(repair.snapshot().saveState, DocumentSaveCoordinatorState.ReadOnlyRecovery);
  assert.equal(repair.snapshot().body, "body preserved");
  assert.equal(repair.snapshot().errorCode, "DOCUMENT_AUTHORING_POINTER_UPDATE_FAILED");
  assert.equal(JSON.stringify(repair.snapshot().errorCode).includes("body preserved"), false);
  assert.deepEqual(repair.requestClose().choices, ["RetrySave", "Discard", "Cancel"]);
  repair.discard();
  assert.equal(repair.snapshot().saveState, DocumentSaveCoordinatorState.NoDocument);
});

test("authoring controller ignores a stale native completion revision", async () => {
  const controller = controllerWith({
    async saveDocumentRevision(command) {
      return { ...saveResult(command, "stale-version"), revision: command.revision - 1 };
    },
  });
  await controller.open(identity());
  controller.changeContent("body two");

  await controller.manualSave();

  assert.equal(controller.snapshot().revision, 1);
  assert.equal(controller.snapshot().persistedRevision, 0);
  assert.equal(controller.snapshot().expectedVersionId, "v1");
  assert.equal(controller.snapshot().saveState, DocumentSaveCoordinatorState.Saving);
});

test("authoring controller keeps the latest document when concurrent opens complete out of order", async () => {
  const pending = new Map<string, (value: CurrentDocumentView) => void>();
  const controller = createDesktopDocumentAuthoringController({
    client: {
      getCurrentDocument(query) {
        return new Promise<CurrentDocumentView>((resolve) => pending.set(query.documentId, resolve));
      },
      async saveDocumentRevision() {
        throw new Error("save must not run");
      },
    },
    operationIdSource: () => "operation-unused",
  });

  const first = controller.open({ ...identity(), documentId: "doc-first" });
  const second = controller.open({ ...identity(), documentId: "doc-second" });
  pending.get("doc-second")?.({
    ...currentDocument(),
    documentId: "doc-second",
    title: "두 번째 문서",
  });
  await second;
  pending.get("doc-first")?.({
    ...currentDocument(),
    documentId: "doc-first",
    title: "첫 번째 문서",
  });
  const staleResult = await first;

  assert.equal(staleResult.documentId, "doc-second");
  assert.equal(controller.snapshot().documentId, "doc-second");
  assert.equal(controller.snapshot().title, "두 번째 문서");
});

test("authoring controller verifies the durable document before reporting saved", async () => {
  let persisted = currentDocument();
  const controller = controllerWith({
    async getCurrentDocument() {
      return persisted;
    },
    async saveDocumentRevision(command) {
      persisted = {
        ...persisted,
        title: "새 문서 제목",
        body: command.body,
        versionId: "v2",
      };
      return saveResult(command, "v2");
    },
  });
  await controller.open(identity());
  controller.changeContent("# 새 문서 제목\nbody verified");

  await controller.manualSave();

  assert.equal(controller.snapshot().saveState, DocumentSaveCoordinatorState.Saved);
  assert.equal(controller.snapshot().persistedRevision, 1);
  assert.equal(controller.snapshot().expectedVersionId, "v2");
  assert.equal(controller.snapshot().title, "새 문서 제목");
});

test("authoring controller reports a failed save when durable readback is stale", async () => {
  const controller = controllerWith({
    async getCurrentDocument() {
      return currentDocument();
    },
    async saveDocumentRevision(command) {
      return saveResult(command, "v2");
    },
  });
  await controller.open(identity());
  controller.changeContent("body not persisted");

  await controller.manualSave();

  assert.equal(controller.snapshot().saveState, DocumentSaveCoordinatorState.SaveFailed);
  assert.equal(controller.snapshot().persistedRevision, 0);
  assert.equal(controller.snapshot().errorCode, "DOCUMENT_AUTHORING_STORAGE_UNAVAILABLE");
  assert.equal(controller.snapshot().retryable, true);
});

test("authoring controller applies matching metadata readback without replacing a dirty body", async () => {
  const controller = controllerWith({
    async saveDocumentRevision(command) {
      return saveResult(command, "v2");
    },
  });
  await controller.open(identity());
  controller.changeContent("unsaved local draft");

  const applied = controller.applyMetadataReadback({
    ...currentDocument(),
    title: "새 제목",
    path: "notes/renamed.md",
  });

  assert.equal(applied.accepted, true);
  assert.equal(applied.snapshot.title, "새 제목");
  assert.equal(applied.snapshot.path, "notes/renamed.md");
  assert.equal(applied.snapshot.body, "unsaved local draft");
  assert.equal(applied.snapshot.revision, 1);
  assert.equal(applied.snapshot.persistedRevision, 0);
  assert.equal(applied.snapshot.saveState, DocumentSaveCoordinatorState.Dirty);
});

test("authoring controller rejects metadata readback for another identity or version", async () => {
  const controller = controllerWith({
    async saveDocumentRevision(command) {
      return saveResult(command, "v2");
    },
  });
  await controller.open(identity());

  const wrongDocument = controller.applyMetadataReadback({
    ...currentDocument(),
    documentId: "doc-other",
    title: "잘못된 제목",
  });
  const wrongVersion = controller.applyMetadataReadback({
    ...currentDocument(),
    versionId: "v-other",
    title: "잘못된 제목",
  });

  assert.equal(wrongDocument.accepted, false);
  assert.equal(wrongDocument.errorCode, "DOCUMENT_AUTHORING_READBACK_MISMATCH");
  assert.equal(wrongVersion.accepted, false);
  assert.equal(wrongVersion.errorCode, "DOCUMENT_AUTHORING_READBACK_MISMATCH");
  assert.equal(controller.snapshot().title, "Source");
});

function controllerWith(client: {
  getCurrentDocument?: (query: CurrentDocumentQuery) => Promise<CurrentDocumentView>;
  saveDocumentRevision: (command: SaveDocumentRevisionCommand) => Promise<SaveDocumentRevisionResult>;
}) {
  let persisted = currentDocument();
  let operationSequence = 0;
  return createDesktopDocumentAuthoringController({
    client: {
      getCurrentDocument: client.getCurrentDocument ?? (async () => persisted),
      async saveDocumentRevision(command) {
        const result = await client.saveDocumentRevision(command);
        if (!client.getCurrentDocument && result.revision === command.revision) {
          persisted = {
            ...persisted,
            body: command.body,
            versionId: result.currentVersionId,
          };
        }
        return result;
      },
    },
    operationIdSource: () => `save-operation-${++operationSequence}`,
    author: "local-user",
    summary: "Updated",
    autosaveDelayMs: 800,
  });
}

function identity(): CurrentDocumentQuery {
  return {
    queryName: "get-current-document",
    workspaceId: "workspace-1",
    documentId: "doc-1",
  };
}

function currentDocument(): CurrentDocumentView {
  return {
    workspaceId: "workspace-1",
    documentId: "doc-1",
    title: "Source",
    path: "notes/source.md",
    body: "body one",
    versionId: "v1",
  };
}

function saveResult(
  command: SaveDocumentRevisionCommand,
  currentVersionId: string,
): SaveDocumentRevisionResult {
  return {
    status: "saved-local",
    workspaceId: command.workspaceId,
    documentId: command.documentId,
    currentVersionId,
    versionAppended: true,
    revision: command.revision,
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolver) => {
    resolve = resolver;
  });
  return { promise, resolve };
}
