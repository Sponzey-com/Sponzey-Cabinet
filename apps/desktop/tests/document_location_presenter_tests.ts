import assert from "node:assert/strict";
import test from "node:test";

import { presentDocumentLocation } from "../src/document_location_presenter.ts";

test("document location presents parent folders without the stored filename", () => {
  assert.equal(presentDocumentLocation("projects/cabinet/design.md"), "projects / cabinet");
  assert.equal(presentDocumentLocation("notes/architecture.md"), "notes");
});

test("document location falls back for root, empty, and traversal-only paths", () => {
  assert.equal(presentDocumentLocation("readme.md"), "문서");
  assert.equal(presentDocumentLocation(""), "문서");
  assert.equal(presentDocumentLocation("../unsafe.md"), "문서");
});
