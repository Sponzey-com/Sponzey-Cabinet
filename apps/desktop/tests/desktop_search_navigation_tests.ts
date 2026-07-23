import assert from "node:assert/strict";
import test from "node:test";

import {
  createDesktopSearchNavigationIntent,
  focusDesktopWorkspaceSearch,
} from "../src/desktop_search_navigation.ts";

test("search navigation intent rejects blank submission without a route effect", () => {
  assert.deepEqual(
    createDesktopSearchNavigationIntent("  \n ", "workspace-1", "Graph"),
    { kind: "NoOp", reason: "EmptyQuery" },
  );
});

test("search navigation intent trims query and preserves the explicit origin", () => {
  assert.deepEqual(
    createDesktopSearchNavigationIntent("  지식 지도  ", "workspace-1", "Canvas"),
    {
      kind: "Navigate",
      route: { kind: "Search", query: "지식 지도" },
      selection: { workspaceId: "workspace-1", originRoute: "Canvas" },
    },
  );
  assert.throws(
    () => createDesktopSearchNavigationIntent("query", " ", "Home"),
    /INVALID_SEARCH_NAVIGATION_CONTEXT/,
  );
});

test("workspace shortcut focuses only the shared enabled search input", () => {
  let focusCount = 0;
  const enabled = { disabled: false, focus() { focusCount += 1; } };
  assert.equal(focusDesktopWorkspaceSearch({ querySelector: () => enabled }), true);
  assert.equal(focusCount, 1);
  assert.equal(focusDesktopWorkspaceSearch({ querySelector: () => null }), false);
  assert.equal(focusDesktopWorkspaceSearch({
    querySelector: () => ({ disabled: true, focus() { focusCount += 1; } }),
  }), false);
  assert.equal(focusCount, 1);
});
