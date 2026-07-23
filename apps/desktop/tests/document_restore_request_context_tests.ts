import assert from "node:assert/strict";
import test from "node:test";

import {
  createDocumentRestoreRequestContext,
  isCurrentDocumentRestoreRequest,
} from "../src/document_restore_request_context.ts";

test("restore request context accepts only the same operation and document", () => {
  const request = createDocumentRestoreRequestContext(4, 7, "doc-current");

  assert.equal(isCurrentDocumentRestoreRequest(request, request), true);
  assert.equal(
    isCurrentDocumentRestoreRequest(
      request,
      createDocumentRestoreRequestContext(5, 7, "doc-current"),
    ),
    false,
  );
  assert.equal(
    isCurrentDocumentRestoreRequest(
      request,
      createDocumentRestoreRequestContext(4, 8, "doc-current"),
    ),
    false,
  );
  assert.equal(
    isCurrentDocumentRestoreRequest(
      request,
      createDocumentRestoreRequestContext(4, 7, "doc-new"),
    ),
    false,
  );
});
