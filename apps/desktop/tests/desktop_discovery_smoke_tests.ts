import assert from "node:assert/strict";
import test from "node:test";

import type {
  CanvasView,
  DocumentAssetsPage,
  KnowledgeGraphView,
  LinkOverviewView,
  SearchResultsPage,
} from "../../../packages/client-core/src/index.ts";
import {
  createDesktopCanvasViewportPanel,
  createDesktopGraphPanel,
  createDesktopLocalDiscoveryPanel,
} from "../src/index.ts";

test("desktop local discovery smoke hides raw query and asset content", () => {
  const discovery = createDesktopLocalDiscoveryPanel({
    search: searchResultsPage(),
    links: linkOverview(),
    assets: documentAssetsPage(),
    indexFreshness: "Stale",
    filters: [{ kind: "asset", label: "Has asset", active: true }],
    recentSearches: ["raw query should not appear"],
  });
  const serialized = JSON.stringify(discovery);

  assert.equal(discovery.mode, "local-discovery");
  assert.equal(discovery.search.queryHash.startsWith("uihash:"), true);
  assert.deepEqual(
    discovery.search.results[0]?.actions.map((action) => action.id),
    ["open-document", "ask-ai"],
  );
  assert.equal(discovery.search.filters[0]?.kind, "asset");
  assert.equal(discovery.links.backlinkCount, 1);
  assert.equal(discovery.assets.assets.length, 1);
  assert.equal(discovery.assets.assets[0]?.referencedDocumentCount, 1);
  assert.equal(discovery.assets.assets[0]?.indexState, "Stale");
  assert.deepEqual(discovery.index.actions.map((action) => action.id), ["rebuild-index"]);
  assert.equal(serialized.includes("raw query should not appear"), false);
  assert.equal(serialized.includes("asset binary content should not leak"), false);
  assert.equal(serialized.includes("raw-scan"), false);
});

test("desktop graph smoke uses neighborhood contract", () => {
  const graph = createDesktopGraphPanel(knowledgeGraph(), {
    depthLimit: 2,
    pageSize: 25,
  });

  assert.equal(graph.mode, "graph");
  assert.equal(graph.loadMode, "neighborhood");
  assert.equal(graph.fullWorkspaceScan, false);
  assert.equal(graph.nodeCount, 2);
  assert.equal(graph.edgeCount, 1);
});

test("desktop canvas smoke filters viewport and excludes raw card state", () => {
  const canvas = createDesktopCanvasViewportPanel(canvasView(), {
    viewport: { x: 0, y: 0, width: 200, height: 200 },
    pageSize: 10,
  });
  const serialized = JSON.stringify(canvas);

  assert.equal(canvas.mode, "canvas");
  assert.equal(canvas.loadState, "ViewportReady");
  assert.deepEqual(
    canvas.visibleNodes.map((node) => node.id),
    ["node-1", "node-2"],
  );
  assert.deepEqual(
    canvas.visibleEdges.map((edge) => edge.id),
    ["edge-1"],
  );
  assert.equal(serialized.includes("card text should not leak"), false);
  assert.equal(serialized.includes("canvas_raw_ui_state"), false);
});

function searchResultsPage(): SearchResultsPage {
  return {
    queryName: "search-documents",
    workspaceId: "workspace-1",
    text: "raw query should not appear",
    results: [
      {
        workspaceId: "workspace-1",
        documentId: "doc-1",
        title: "Source",
        path: "docs/source.md",
        snippet: "source snippet",
        score: 0.95,
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
    unresolvedLinks: [],
    orphanDocuments: [],
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
        referencedDocumentCount: 1,
        previewState: "ready",
        ocrState: "not-indexed",
        indexState: "Stale",
        content: "asset binary content should not leak",
      } as never,
    ],
  };
}

function knowledgeGraph(): KnowledgeGraphView {
  return {
    centerDocumentId: "doc-1",
    status: "clean",
    nodes: [
      { id: "doc-1", kind: "document" },
      { id: "doc-2", kind: "document" },
    ],
    edges: [
      { id: "edge-1", sourceId: "doc-1", targetId: "doc-2", kind: "document_link" },
    ],
    stats: { candidateCount: 2, filteredCount: 2 },
  };
}

function canvasView(): CanvasView {
  return {
    canvasId: "canvas-1",
    state: "saved",
    nodes: [
      { id: "node-1", targetKind: "document", x: 10, y: 10 },
      { id: "node-2", targetKind: "attachment", x: 150, y: 150 },
      { id: "node-3", targetKind: "text_card", x: 400, y: 400, text: "card text should not leak" } as never,
    ],
    edges: [
      { id: "edge-1", sourceId: "node-1", targetId: "node-2" },
      { id: "edge-2", sourceId: "node-2", targetId: "node-3" },
    ],
  };
}
