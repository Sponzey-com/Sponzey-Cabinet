import assert from "node:assert/strict";
import test from "node:test";

import {
  DocumentSaveCoordinatorEvent,
  DocumentSaveCoordinatorState,
  createDocumentSaveCoordinator,
  transitionDocumentSaveCoordinator,
} from "../src/index.ts";

test("save coordinator enforces 800ms autosave boundary and manual save", () => {
  const opened = transitionDocumentSaveCoordinator(createDocumentSaveCoordinator(), {
    type: DocumentSaveCoordinatorEvent.DocumentOpened,
    revision: 0,
    versionId: "v1",
  }).snapshot;
  const dirty = transitionDocumentSaveCoordinator(opened, {
    type: DocumentSaveCoordinatorEvent.ContentChanged,
    revision: 1,
    contentRef: "draft-1",
  }).snapshot;
  const early = transitionDocumentSaveCoordinator(dirty, {
    type: DocumentSaveCoordinatorEvent.AutosaveElapsed,
    elapsedMs: 799,
  });
  const due = transitionDocumentSaveCoordinator(dirty, {
    type: DocumentSaveCoordinatorEvent.AutosaveElapsed,
    elapsedMs: 800,
  });
  const manual = transitionDocumentSaveCoordinator(dirty, {
    type: DocumentSaveCoordinatorEvent.SaveRequested,
  });

  assert.equal(opened.state, DocumentSaveCoordinatorState.Clean);
  assert.equal(dirty.state, DocumentSaveCoordinatorState.Dirty);
  assert.equal(early.snapshot.state, DocumentSaveCoordinatorState.Dirty);
  assert.equal(early.sideEffect, undefined);
  assert.equal(due.snapshot.state, DocumentSaveCoordinatorState.SaveQueued);
  assert.equal(due.sideEffect?.type, "StartSave");
  assert.equal(due.sideEffect?.revision, 1);
  assert.equal(manual.snapshot.state, DocumentSaveCoordinatorState.SaveQueued);
  assert.equal(manual.sideEffect?.expectedVersionId, "v1");
});

test("edit during save queues newer revision after matching success", () => {
  const saving = {
    ...createDocumentSaveCoordinator(),
    state: DocumentSaveCoordinatorState.Saving,
    currentRevision: 1,
    persistedRevision: 0,
    dirtyContentRef: "draft-1",
    inFlightRevision: 1,
    expectedVersionId: "v1",
  } as const;
  const edited = transitionDocumentSaveCoordinator(saving, {
    type: DocumentSaveCoordinatorEvent.ContentChanged,
    revision: 2,
    contentRef: "draft-2",
  });
  const duplicateSave = transitionDocumentSaveCoordinator(edited.snapshot, {
    type: DocumentSaveCoordinatorEvent.SaveRequested,
  });
  const completed = transitionDocumentSaveCoordinator(edited.snapshot, {
    type: DocumentSaveCoordinatorEvent.SaveSucceeded,
    revision: 1,
    savedVersionId: "v2",
  });

  assert.equal(edited.snapshot.state, DocumentSaveCoordinatorState.Saving);
  assert.equal(edited.snapshot.currentRevision, 2);
  assert.equal(duplicateSave.sideEffect, undefined);
  assert.equal(completed.snapshot.state, DocumentSaveCoordinatorState.SaveQueued);
  assert.equal(completed.snapshot.persistedRevision, 1);
  assert.equal(completed.snapshot.dirtyContentRef, "draft-2");
  assert.equal(completed.sideEffect?.type, "StartSave");
  assert.equal(completed.sideEffect?.revision, 2);
});

test("stale completion is ignored while matching failure supports retry", () => {
  const saving = {
    ...createDocumentSaveCoordinator(),
    state: DocumentSaveCoordinatorState.Saving,
    currentRevision: 2,
    persistedRevision: 0,
    dirtyContentRef: "draft-2",
    inFlightRevision: 2,
    expectedVersionId: "v1",
  } as const;
  const stale = transitionDocumentSaveCoordinator(saving, {
    type: DocumentSaveCoordinatorEvent.SaveSucceeded,
    revision: 1,
    savedVersionId: "stale",
  });
  const failed = transitionDocumentSaveCoordinator(saving, {
    type: DocumentSaveCoordinatorEvent.SaveFailed,
    revision: 2,
    errorCode: "STORE_UNAVAILABLE",
  });
  const retry = transitionDocumentSaveCoordinator(failed.snapshot, {
    type: DocumentSaveCoordinatorEvent.RetryRequested,
  });

  assert.equal(stale.ignored, true);
  assert.equal(stale.snapshot, saving);
  assert.equal(failed.snapshot.state, DocumentSaveCoordinatorState.SaveFailed);
  assert.equal(failed.snapshot.dirtyContentRef, "draft-2");
  assert.equal(retry.snapshot.state, DocumentSaveCoordinatorState.SaveQueued);
  assert.equal(retry.sideEffect?.type, "StartSave");
});

test("close guard discard and read-only recovery preserve explicit intent", () => {
  const dirty = {
    ...createDocumentSaveCoordinator(),
    state: DocumentSaveCoordinatorState.Dirty,
    currentRevision: 3,
    persistedRevision: 2,
    dirtyContentRef: "draft-3",
  } as const;
  const blocked = transitionDocumentSaveCoordinator(dirty, {
    type: DocumentSaveCoordinatorEvent.CloseRequested,
  });
  const discarded = transitionDocumentSaveCoordinator(blocked.snapshot, {
    type: DocumentSaveCoordinatorEvent.DiscardConfirmed,
  });
  const readOnly = transitionDocumentSaveCoordinator(dirty, {
    type: DocumentSaveCoordinatorEvent.ReadOnlyEntered,
  });
  const invalidSave = transitionDocumentSaveCoordinator(readOnly.snapshot, {
    type: DocumentSaveCoordinatorEvent.SaveRequested,
  });

  assert.equal(blocked.snapshot.state, DocumentSaveCoordinatorState.CloseBlocked);
  assert.deepEqual(blocked.recoveryChoices, ["RetrySave", "Discard", "Cancel"]);
  assert.equal(discarded.snapshot.state, DocumentSaveCoordinatorState.NoDocument);
  assert.equal(readOnly.snapshot.state, DocumentSaveCoordinatorState.ReadOnlyRecovery);
  assert.equal(readOnly.snapshot.dirtyContentRef, "draft-3");
  assert.equal(invalidSave.errorCode, "DOCUMENT_SAVE_INVALID_TRANSITION");
  assert.equal(JSON.stringify(readOnly).includes("raw document body"), false);
});

test("close cancellation restores the prior recoverable state and read-only can close explicitly", () => {
  const failed = {
    ...createDocumentSaveCoordinator(),
    state: DocumentSaveCoordinatorState.SaveFailed,
    currentRevision: 2,
    persistedRevision: 1,
    dirtyContentRef: "draft-2",
    errorCode: "STORE_UNAVAILABLE",
  } as const;
  const blocked = transitionDocumentSaveCoordinator(failed, {
    type: DocumentSaveCoordinatorEvent.CloseRequested,
  });
  const cancelled = transitionDocumentSaveCoordinator(blocked.snapshot, {
    type: DocumentSaveCoordinatorEvent.CloseCancelled,
  });
  const readOnly = transitionDocumentSaveCoordinator(failed, {
    type: DocumentSaveCoordinatorEvent.ReadOnlyEntered,
  });
  const readOnlyBlocked = transitionDocumentSaveCoordinator(readOnly.snapshot, {
    type: DocumentSaveCoordinatorEvent.CloseRequested,
  });

  assert.equal(cancelled.snapshot.state, DocumentSaveCoordinatorState.SaveFailed);
  assert.equal(cancelled.snapshot.errorCode, "STORE_UNAVAILABLE");
  assert.equal(readOnlyBlocked.snapshot.state, DocumentSaveCoordinatorState.CloseBlocked);
  assert.deepEqual(readOnlyBlocked.recoveryChoices, ["RetrySave", "Discard", "Cancel"]);
});
