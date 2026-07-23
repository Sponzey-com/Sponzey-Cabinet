import assert from "node:assert/strict";
import test from "node:test";

import { presentSearchResultSnippet } from "../src/search_result_snippet_presenter.ts";

test("search snippet presenter normalizes whitespace and bounds text", () => {
  const presented = presentSearchResultSnippet(`  first\n\tsecond   ${"가".repeat(200)}  `);
  assert.ok(presented);
  assert.match(presented, /^first second /);
  assert.equal(presented?.length, 160);
  assert.equal(presentSearchResultSnippet("   "), undefined);
});
