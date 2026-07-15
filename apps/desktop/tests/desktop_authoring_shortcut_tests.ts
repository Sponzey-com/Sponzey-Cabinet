import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import { isMacDocumentSaveShortcut } from "../src/desktop_authoring_shortcut.ts";

test("macOS document save shortcut accepts only Command+S without conflicting modifiers", () => {
  assert.equal(isMacDocumentSaveShortcut({ key: "s", metaKey: true }), true);
  assert.equal(isMacDocumentSaveShortcut({ key: "S", metaKey: true }), true);
  assert.equal(isMacDocumentSaveShortcut({ key: "s", metaKey: false }), false);
  assert.equal(isMacDocumentSaveShortcut({ key: "s", metaKey: true, altKey: true }), false);
  assert.equal(isMacDocumentSaveShortcut({ key: "s", metaKey: true, ctrlKey: true }), false);
  assert.equal(isMacDocumentSaveShortcut({ key: "k", metaKey: true }), false);
});

test("desktop entry owns the only save shortcut and routes it through manual save", async () => {
  const [entry, editor] = await Promise.all([
    readFile(new URL("../src/desktop_entry.ts", import.meta.url), "utf8"),
    readFile(new URL("../src/codemirror_document_editor.ts", import.meta.url), "utf8"),
  ]);

  assert.match(entry, /isMacDocumentSaveShortcut/);
  assert.match(entry, /surface\s*!==\s*"Authoring"/);
  assert.match(entry, /preventDefault\(\)[\s\S]*?runAuthoringSave\("manual"\)/);
  assert.doesNotMatch(editor, /key:\s*"Mod-s"/);
});
