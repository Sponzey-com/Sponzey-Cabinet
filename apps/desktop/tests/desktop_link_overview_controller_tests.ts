import assert from "node:assert/strict";
import test from "node:test";

import {
  createDesktopLinkOverviewSnapshot,
  loadDesktopLinkOverview,
  requestDesktopLinkOverviewLoad,
} from "../src/desktop_link_overview_controller.ts";

test("link overview controller transitions through loading to bounded ready results", async () => {
  const idle = createDesktopLinkOverviewSnapshot("workspace-1", "doc-target");
  const loading = requestDesktopLinkOverviewLoad(idle);
  const ready = await loadDesktopLinkOverview({
    async getLinkOverview(query) {
      assert.equal(query.queryName, "get-link-overview");
      return {
        ...query,
        backlinks: Array.from({ length: 50 }, (_, index) => ({
          workspaceId: query.workspaceId,
          sourceDocumentId: `source-${index}`,
          targetDocumentId: query.documentId,
          sourceTitle: `Source ${index}`,
          sourcePath: `fixture/${index}`,
        })),
        unresolvedLinks: [],
        orphanDocuments: [],
      };
    },
  }, loading);

  assert.equal(loading.state, "Loading");
  assert.equal(ready.state, "Ready");
  assert.equal(ready.panel?.backlinks.length, 50);
  assert.equal(ready.panel?.backlinks[0]?.sourceDocumentId, "source-0");
});

test("link overview controller maps empty failures and ignores stale generations", async () => {
  const loading = requestDesktopLinkOverviewLoad(createDesktopLinkOverviewSnapshot("workspace-1", "doc-target"));
  const empty = await loadDesktopLinkOverview({
    async getLinkOverview(query) {
      return { ...query, backlinks: [], unresolvedLinks: [], orphanDocuments: [] };
    },
  }, loading);
  assert.equal(empty.state, "Empty");

  const failed = await loadDesktopLinkOverview({
    async getLinkOverview() {
      throw new Error("raw filesystem failure");
    },
  }, loading);
  assert.equal(failed.state, "Failed");
  assert.equal(failed.errorCode, "COMMAND_BRIDGE_FAILED");
  assert.equal(failed.retryable, false);
  assert.doesNotMatch(JSON.stringify(failed), /raw filesystem failure/);
});
