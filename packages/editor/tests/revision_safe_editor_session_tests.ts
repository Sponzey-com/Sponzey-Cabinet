import assert from "node:assert/strict";
import test from "node:test";

import {
  applyRevisionSafeEditorContentChange,
  completeRevisionSafeEditorSave,
  createRevisionSafeEditorSession,
  startRevisionSafeEditorSave,
} from "../src/index.ts";

test("revision-safe session increments only for actual body changes", () => {
  const loaded = createRevisionSafeEditorSession({
    documentId: "doc-1",
    body: "version one",
    versionId: "version-1",
  });
  const unchanged = applyRevisionSafeEditorContentChange(loaded, "version one");
  const changed = applyRevisionSafeEditorContentChange(unchanged, "version two");

  assert.equal(loaded.revision, 0);
  assert.equal(loaded.persistedRevision, 0);
  assert.equal(loaded.dirtyState, "clean");
  assert.equal(unchanged, loaded);
  assert.equal(changed.revision, 1);
  assert.equal(changed.persistedRevision, 0);
  assert.equal(changed.dirtyState, "dirty");
});

test("save start snapshots exact revision body and expected version", () => {
  const dirty = applyRevisionSafeEditorContentChange(
    createRevisionSafeEditorSession({
      documentId: "doc-1",
      body: "one",
      versionId: "version-1",
    }),
    "two",
  );
  const started = startRevisionSafeEditorSave(dirty);

  assert.equal(started.started, true);
  assert.equal(started.session.inFlightRevision, 1);
  assert.deepEqual(started.command, {
    kind: "save-document-revision",
    documentId: "doc-1",
    body: "two",
    revision: 1,
    expectedVersionId: "version-1",
  });
  const duplicate = startRevisionSafeEditorSave(started.session);
  assert.equal(duplicate.started, false);
  assert.equal(duplicate.command, undefined);
});

test("matching save completion persists only saved revision and keeps newer edit dirty", () => {
  const revisionOne = applyRevisionSafeEditorContentChange(
    createRevisionSafeEditorSession({ documentId: "doc-1", body: "one", versionId: "v1" }),
    "two",
  );
  const started = startRevisionSafeEditorSave(revisionOne);
  const revisionTwo = applyRevisionSafeEditorContentChange(started.session, "three");
  const completed = completeRevisionSafeEditorSave(revisionTwo, {
    revision: 1,
    status: "succeeded",
    savedVersionId: "v2",
  });

  assert.equal(completed.ignored, false);
  assert.equal(completed.session.persistedRevision, 1);
  assert.equal(completed.session.revision, 2);
  assert.equal(completed.session.currentBody, "three");
  assert.equal(completed.session.expectedVersionId, "v2");
  assert.equal(completed.session.dirtyState, "dirty");
  assert.equal(completed.session.inFlightRevision, undefined);
});

test("stale completion and failure cannot clear current in-flight revision", () => {
  const dirty = applyRevisionSafeEditorContentChange(
    createRevisionSafeEditorSession({ documentId: "doc-1", body: "one", versionId: "v1" }),
    "two",
  );
  const started = startRevisionSafeEditorSave(dirty);
  const staleSuccess = completeRevisionSafeEditorSave(started.session, {
    revision: 0,
    status: "succeeded",
    savedVersionId: "stale-version",
  });
  const staleFailure = completeRevisionSafeEditorSave(started.session, {
    revision: 0,
    status: "failed",
    errorCode: "STALE_FAILURE",
  });
  const matchingFailure = completeRevisionSafeEditorSave(started.session, {
    revision: 1,
    status: "failed",
    errorCode: "STORE_UNAVAILABLE",
  });

  assert.equal(staleSuccess.ignored, true);
  assert.equal(staleSuccess.session, started.session);
  assert.equal(staleFailure.ignored, true);
  assert.equal(matchingFailure.ignored, false);
  assert.equal(matchingFailure.session.inFlightRevision, undefined);
  assert.equal(matchingFailure.session.dirtyState, "dirty");
  assert.equal(matchingFailure.session.errorCode, "STORE_UNAVAILABLE");
  assert.equal(JSON.stringify(matchingFailure).includes("/Users/"), false);
});
