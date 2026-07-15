import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  EditorCollaborationAdapterError,
  createCollaborativeEditInputFromEditorTransaction,
  createPresenceInputFromEditorSelection,
} from "../src/index.ts";

test("editor collaboration adapter maps a single text change to plain edit DTO", () => {
  const input = createCollaborativeEditInputFromEditorTransaction("workspace-1", {
    documentId: "doc-1",
    actorUserId: "user-1",
    operationId: "op-1",
    baseRevision: 3,
    currentRevision: 3,
    changes: [{ start: 4, end: 7, insertedText: "next" }],
  });

  assert.deepEqual(input, {
    workspaceId: "workspace-1",
    documentId: "doc-1",
    actorUserId: "user-1",
    operationId: "op-1",
    baseRevision: 3,
    currentRevision: 3,
    startOffset: 4,
    endOffset: 7,
    insertedText: "next",
  });
});

test("editor collaboration adapter rejects multi-change draft with stable error code", () => {
  assert.throws(
    () =>
      createCollaborativeEditInputFromEditorTransaction("workspace-1", {
        documentId: "doc-1",
        actorUserId: "user-1",
        operationId: "op-1",
        baseRevision: 3,
        currentRevision: 3,
        changes: [
          { start: 0, end: 1, insertedText: "a" },
          { start: 2, end: 3, insertedText: "b" },
        ],
      }),
    (error) =>
      error instanceof EditorCollaborationAdapterError &&
      error.code === "EDITOR_COLLABORATION_MULTI_CHANGE_UNSUPPORTED",
  );
});

test("editor presence adapter excludes selected text body and token fields", () => {
  const input = createPresenceInputFromEditorSelection("workspace-1", {
    documentId: "doc-1",
    actorUserId: "user-1",
    cursorStart: 5,
    cursorEnd: 5,
    selectedText: "selected text must not leave adapter",
    documentBody: "document body must not leave adapter",
    token: "token must not leave adapter",
  });

  assert.deepEqual(input, {
    workspaceId: "workspace-1",
    documentId: "doc-1",
    actorUserId: "user-1",
    cursorStart: 5,
    cursorEnd: 5,
  });
  assert.equal(Object.keys(input).includes("selectedText"), false);
  assert.equal(Object.keys(input).includes("documentBody"), false);
  assert.equal(Object.keys(input).includes("token"), false);
});

test("editor collaboration adapter does not import CodeMirror runtime types", async () => {
  const source = await readFile(new URL("../src/index.ts", import.meta.url), "utf8");

  assert.doesNotMatch(source, /@codemirror\/state|@codemirror\/view/);
  assert.doesNotMatch(source, /import\s+.*Transaction/);
});
