import assert from "node:assert/strict";
import test from "node:test";

import {
  DocumentAutosaveEvent,
  DocumentAutosaveState,
  DocumentAutosaveErrorCode,
  transitionDocumentAutosaveState,
} from "../src/index.ts";

test("autosave state machine saves dirty content and returns to saved", () => {
  const dirty = transitionDocumentAutosaveState(DocumentAutosaveState.Idle, {
    type: DocumentAutosaveEvent.ContentChanged,
    dirtyContentRef: "draft-1",
  });
  const saving = transitionDocumentAutosaveState(dirty, {
    type: DocumentAutosaveEvent.DebounceElapsed,
  });
  const saved = transitionDocumentAutosaveState(saving.state, {
    type: DocumentAutosaveEvent.SaveSucceeded,
    savedVersionId: "version-2",
  });

  assert.equal(dirty.state, DocumentAutosaveState.DirtyQueued);
  assert.equal(dirty.dirtyContentRef, "draft-1");
  assert.equal(saving.state, DocumentAutosaveState.Saving);
  assert.equal(saving.dirtyContentRef, "draft-1");
  assert.equal(saved.state, DocumentAutosaveState.Saved);
  assert.equal(saved.savedVersionId, "version-2");
});

test("autosave failure keeps dirty content ref and supports retry", () => {
  const failed = transitionDocumentAutosaveState(
    { state: DocumentAutosaveState.Saving, dirtyContentRef: "draft-2" },
    {
      type: DocumentAutosaveEvent.SaveFailed,
      errorCode: "LOCAL_SAVE_FAILED",
    },
  );
  const retry = transitionDocumentAutosaveState(failed, {
    type: DocumentAutosaveEvent.RetryRequested,
  });

  assert.equal(failed.state, DocumentAutosaveState.SaveFailed);
  assert.equal(failed.dirtyContentRef, "draft-2");
  assert.equal(failed.errorCode, "LOCAL_SAVE_FAILED");
  assert.equal(retry.state, DocumentAutosaveState.Saving);
  assert.equal(retry.dirtyContentRef, "draft-2");
});

test("autosave read only transition pauses writes and rejects invalid save success", () => {
  const paused = transitionDocumentAutosaveState(
    { state: DocumentAutosaveState.DirtyQueued, dirtyContentRef: "draft-3" },
    { type: DocumentAutosaveEvent.ReadOnlyEntered },
  );
  const invalid = transitionDocumentAutosaveState(paused, {
    type: DocumentAutosaveEvent.SaveSucceeded,
    savedVersionId: "version-3",
  });

  assert.equal(paused.state, DocumentAutosaveState.PausedReadOnly);
  assert.equal(paused.dirtyContentRef, "draft-3");
  assert.equal(invalid.errorCode, DocumentAutosaveErrorCode.InvalidTransition);
});
