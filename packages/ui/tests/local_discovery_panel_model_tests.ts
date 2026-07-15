import assert from "node:assert/strict";
import test from "node:test";

import type {
  DocumentAssetsPage,
  LinkOverviewView,
  SearchResultsPage,
} from "../../client-core/src/index.ts";
import {
  createDiscoveryQueryPolicy,
  createIndexFreshnessActionModel,
  createLocalDiscoveryPanelModel,
} from "../src/index.ts";

test("local discovery panel hashes search query and keeps search result metadata only", () => {
  const model = createLocalDiscoveryPanelModel({
    search: searchResultsPage("secret project query"),
    links: linkOverview(),
    assets: documentAssetsPage(),
    indexFreshness: "Fresh",
    filters: [
      { kind: "tag", label: "Project", active: true },
      { kind: "asset", label: "Has asset", active: false },
    ],
    recentSearches: ["secret project query", "public query"],
  });
  const serialized = JSON.stringify(model);

  assert.equal(model.mode, "local-discovery");
  assert.equal(model.search.queryName, "search-documents");
  assert.equal(model.search.state, "ResultsReady");
  assert.equal(model.search.queryHash.startsWith("uihash:"), true);
  assert.equal(model.search.resultCount, 2);
  assert.deepEqual(
    model.search.filters.map((filter) => [filter.kind, filter.active]),
    [
      ["tag", true],
      ["asset", false],
    ],
  );
  assert.deepEqual(
    model.search.results[0]?.actions.map((action) => action.id),
    ["open-document", "ask-ai"],
  );
  assert.equal(model.search.recentSearches.length, 2);
  assert.equal(model.search.recentSearches[0]?.queryHash.startsWith("uihash:"), true);
  assert.equal(serialized.includes("secret project query"), false);
  assert.equal(serialized.includes("phase006-raw-document-body-should-not-log"), false);
  assert.equal(serialized.includes("team-invite"), false);
});

test("local discovery panel exposes no result state with filter metadata", () => {
  const model = createLocalDiscoveryPanelModel({
    search: { ...searchResultsPage("empty query"), results: [] },
    links: linkOverview(),
    assets: documentAssetsPage(),
    indexFreshness: "Fresh",
    filters: [{ kind: "status", label: "Draft", active: true }],
    recentSearches: [],
  });

  assert.equal(model.search.state, "NoResults");
  assert.equal(model.search.resultCount, 0);
  assert.deepEqual(model.search.filters.map((filter) => filter.kind), ["status"]);
  assert.deepEqual(model.search.recentSearches, []);
});

test("local discovery panel separates backlinks unresolved links and asset metadata", () => {
  const model = createLocalDiscoveryPanelModel({
    search: searchResultsPage("asset query"),
    links: linkOverview(),
    assets: documentAssetsPage(),
    indexFreshness: "Stale",
  });
  const serialized = JSON.stringify(model);

  assert.equal(model.links.backlinkCount, 1);
  assert.equal(model.links.unresolvedCount, 1);
  assert.equal(model.links.orphanCount, 1);
  assert.equal(model.assets.assets.length, 1);
  assert.equal(model.assets.assets[0]?.assetId, "asset-1");
  assert.equal(model.assets.assets[0]?.referencedDocumentCount, 2);
  assert.equal(model.assets.assets[0]?.previewState, "ready");
  assert.equal(model.assets.assets[0]?.ocrState, "indexed");
  assert.equal(model.assets.assets[0]?.indexState, "Fresh");
  assert.equal(serialized.includes("asset binary content should not leak"), false);
  assert.equal(serialized.includes("document body should not leak"), false);
});

test("index freshness model exposes reindex action without raw scan fallback", () => {
  const fresh = createIndexFreshnessActionModel("Fresh");
  const stale = createIndexFreshnessActionModel("Stale");
  const rebuilding = createIndexFreshnessActionModel("Rebuilding");
  const failed = createIndexFreshnessActionModel("RebuildFailed");

  assert.deepEqual(fresh.actions, []);
  assert.deepEqual(stale.actions.map((action) => action.id), ["rebuild-index"]);
  assert.deepEqual(rebuilding.actions, []);
  assert.deepEqual(failed.actions.map((action) => action.id), ["rebuild-index"]);
  assert.equal(JSON.stringify(stale).includes("raw-scan"), false);
});

test("local discovery workflow state exposes stale repairing and failed index states", () => {
  const stale = createLocalDiscoveryPanelModel({
    search: searchResultsPage("stale query"),
    links: linkOverview(),
    assets: documentAssetsPage(),
    indexFreshness: "Stale",
  });
  const repairing = createLocalDiscoveryPanelModel({
    search: searchResultsPage("repair query"),
    links: linkOverview(),
    assets: documentAssetsPage(),
    indexFreshness: "Rebuilding",
  });
  const failed = createLocalDiscoveryPanelModel({
    search: searchResultsPage("failed query"),
    links: linkOverview(),
    assets: documentAssetsPage(),
    indexFreshness: "RebuildFailed",
  });
  const empty = createLocalDiscoveryPanelModel({
    search: { ...searchResultsPage("empty query"), results: [] },
    links: linkOverview(),
    assets: documentAssetsPage(),
    indexFreshness: "Fresh",
  });

  assert.equal(stale.workflowState, "IndexStale");
  assert.equal(repairing.workflowState, "Repairing");
  assert.equal(failed.workflowState, "RepairFailed");
  assert.equal(empty.workflowState, "NoResults");
  assert.equal(JSON.stringify(stale).includes("isStale"), false);
  assert.equal(JSON.stringify(repairing).includes("raw-scan"), false);
});

test("discovery query policy clamps query and graph limits for projection based reads", () => {
  const policy = createDiscoveryQueryPolicy({
    debounceMs: 0,
    cancelPrevious: false,
    pageSize: 1000,
    resultLimit: 1000,
    graphDepthLimit: 12,
  });

  assert.equal(policy.debounceMs, 120);
  assert.equal(policy.cancelPrevious, true);
  assert.equal(policy.pageSize, 100);
  assert.equal(policy.resultLimit, 100);
  assert.equal(policy.graphDepthLimit, 3);
  assert.equal(policy.fullWorkspaceScan, false);
});

function searchResultsPage(text: string): SearchResultsPage {
  return {
    queryName: "search-documents",
    workspaceId: "workspace-1",
    text,
    results: [
      {
        workspaceId: "workspace-1",
        documentId: "doc-1",
        title: "Source",
        path: "docs/source.md",
        snippet: "matched heading",
        score: 0.99,
      },
      {
        workspaceId: "workspace-1",
        documentId: "doc-2",
        title: "Target",
        path: "docs/target.md",
        snippet: "linked note",
        score: 0.75,
      },
    ],
  };
}

function linkOverview(): LinkOverviewView {
  return {
    queryName: "get-link-overview",
    workspaceId: "workspace-1",
    documentId: "doc-1",
    backlinks: [
      {
        workspaceId: "workspace-1",
        sourceDocumentId: "doc-2",
        targetDocumentId: "doc-1",
        sourceTitle: "Target",
        sourcePath: "docs/target.md",
      },
    ],
    unresolvedLinks: [
      {
        workspaceId: "workspace-1",
        sourceDocumentId: "doc-1",
        targetSlug: "missing-target",
      },
    ],
    orphanDocuments: [
      {
        workspaceId: "workspace-1",
        documentId: "doc-3",
        title: "Orphan",
        path: "docs/orphan.md",
      },
    ],
  };
}

function documentAssetsPage(): DocumentAssetsPage {
  return {
    queryName: "list-document-assets",
    workspaceId: "workspace-1",
    documentId: "doc-1",
    assets: [
      {
        assetId: "asset-1",
        label: "Diagram",
        fileName: "diagram.png",
        mediaType: "image/png",
        byteSize: 1200,
        status: "available",
        referencedDocumentCount: 2,
        previewState: "ready",
        ocrState: "indexed",
        indexState: "Fresh",
        content: "asset binary content should not leak",
      } as never,
    ],
  };
}
