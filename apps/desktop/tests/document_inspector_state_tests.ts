import assert from "node:assert/strict";
import test from "node:test";

import {
  createDocumentInspectorState,
  transitionDocumentInspector,
} from "../src/document_inspector_state.ts";

test("document inspector changes one explicit tab without changing unlink state", () => {
  const initial = createDocumentInspectorState();
  const attachments = transitionDocumentInspector(initial, { type: "SelectTab", tab: "attachments" });
  const history = transitionDocumentInspector(attachments, { type: "SelectTab", tab: "history" });

  assert.equal(initial.tab, "links");
  assert.deepEqual(attachments, { tab: "attachments", unlink: { status: "Closed" } });
  assert.deepEqual(history, { tab: "history", unlink: { status: "Closed" } });
});

test("document inspector requires confirmation before submitting unlink", () => {
  const initial = createDocumentInspectorState("attachments");
  const requested = transitionDocumentInspector(initial, { type: "RequestUnlink", fileName: "design.pdf" });
  const submitting = transitionDocumentInspector(requested, { type: "ConfirmUnlink" });
  const completed = transitionDocumentInspector(submitting, { type: "UnlinkSucceeded" });

  assert.deepEqual(requested.unlink, { status: "Confirming", fileName: "design.pdf" });
  assert.deepEqual(submitting.unlink, { status: "Submitting", fileName: "design.pdf" });
  assert.deepEqual(completed.unlink, { status: "Closed" });
});

test("document inspector preserves safe retry data and ignores invalid transitions", () => {
  const initial = createDocumentInspectorState("attachments");
  assert.equal(transitionDocumentInspector(initial, { type: "ConfirmUnlink" }), initial);
  assert.equal(transitionDocumentInspector(initial, { type: "RequestUnlink", fileName: "  " }), initial);

  const requested = transitionDocumentInspector(initial, { type: "RequestUnlink", fileName: "notes.txt" });
  const cancelled = transitionDocumentInspector(requested, { type: "CancelUnlink" });
  const submitting = transitionDocumentInspector(requested, { type: "ConfirmUnlink" });
  const failed = transitionDocumentInspector(submitting, { type: "UnlinkFailed" });
  const retrying = transitionDocumentInspector(failed, { type: "ConfirmUnlink" });

  assert.deepEqual(cancelled.unlink, { status: "Closed" });
  assert.deepEqual(failed.unlink, { status: "Failed", fileName: "notes.txt" });
  assert.deepEqual(retrying.unlink, { status: "Submitting", fileName: "notes.txt" });
});
