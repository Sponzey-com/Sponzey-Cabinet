import assert from "node:assert/strict";
import test from "node:test";

import { codeMirrorDocumentContentAttributes } from "../src/codemirror_document_editor.ts";

test("CodeMirror document content exposes a stable action identity and accessible name", () => {
  assert.deepEqual(codeMirrorDocumentContentAttributes(), {
    "data-action": "edit-document-body",
    "aria-label": "문서 본문 편집",
  });
});
