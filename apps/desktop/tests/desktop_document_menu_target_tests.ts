import assert from "node:assert/strict";
import test from "node:test";

import { resolveDesktopDocumentMenuTarget } from "../src/desktop_document_menu_target.ts";

test("document menu resumes the last authoring document before recent documents", () => {
  assert.deepEqual(
    resolveDesktopDocumentMenuTarget("doc-last", ["doc-recent-1", "doc-recent-2"]),
    { kind: "LastDocument", documentId: "doc-last" },
  );
});

test("document menu opens the most recent document when no authoring document exists", () => {
  assert.deepEqual(
    resolveDesktopDocumentMenuTarget(undefined, ["doc-recent-1", "doc-recent-2"]),
    { kind: "RecentDocument", documentId: "doc-recent-1" },
  );
});

test("document menu ignores blank identities and reports an empty workspace", () => {
  assert.deepEqual(
    resolveDesktopDocumentMenuTarget("   ", ["", "  "]),
    { kind: "EmptyWorkspace" },
  );
});
