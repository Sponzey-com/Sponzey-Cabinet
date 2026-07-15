import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

test("desktop CodeMirror adapter owns Markdown runtime mount update and destroy while app owns shortcuts", async () => {
  const source = await readFile(
    new URL("../src/codemirror_document_editor.ts", import.meta.url),
    "utf8",
  );

  assert.match(source, /from "@codemirror\/state"/);
  assert.match(source, /from "@codemirror\/view"/);
  assert.match(source, /from "@codemirror\/lang-markdown"/);
  assert.doesNotMatch(source, /key:\s*"Mod-s"/);
  assert.match(source, /EditorView\.updateListener/);
  assert.match(source, /setDocument/);
  assert.match(source, /destroy/);
  assert.doesNotMatch(source, /process\.env|console\.|localStorage|sessionStorage/);
});
