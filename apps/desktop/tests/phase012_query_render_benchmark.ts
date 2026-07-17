import { performance } from "node:perf_hooks";
import { renderToStaticMarkup } from "react-dom/server";

import {
  createPersonalLocalDesktopCapabilityProfile,
  type LocalDesktopCommandClient,
} from "@sponzey-cabinet/client-core";
import {
  DocumentSaveCoordinatorState,
  createDocumentNavigatorLoadingModel,
  createPersonalWorkspaceHomeModelFromResult,
} from "@sponzey-cabinet/ui";

import {
  createDesktopDocumentAuthoringController,
} from "../src/desktop_document_authoring_controller.ts";
import {
  createDesktopLinkOverviewSnapshot,
  loadDesktopLinkOverview,
  requestDesktopLinkOverviewLoad,
} from "../src/desktop_link_overview_controller.ts";
import { loadDesktopDocumentNavigator } from "../src/desktop_navigator_controller.ts";
import {
  createDesktopGraphSnapshot,
  loadDesktopGlobalKnowledgeGraph,
  loadDesktopKnowledgeGraph,
  requestDesktopGraphLoad,
} from "../src/desktop_graph_controller.ts";
import {
  createDesktopCanvasSnapshot,
  loadDesktopCanvas,
  requestDesktopCanvasLoad,
} from "../src/desktop_canvas_controller.ts";
import {
  createDesktopAssetSnapshot,
  loadDesktopWorkspaceAssets,
  requestDesktopWorkspaceAssetLoad,
} from "../src/desktop_asset_controller.ts";
import { createDesktopDocumentAuthoringWorkbenchElement } from "../src/react_document_authoring_workbench.ts";
import { createDesktopDocumentNavigatorElement } from "../src/react_document_navigator.ts";
import {
  createDesktopAttachmentsElement,
  createDesktopCanvasElement,
  createDesktopKnowledgeGraphElement,
} from "../src/react_exploration_surfaces.ts";

const WARMUP_COUNT = 30;
const SAMPLE_COUNT = 200;

const home = createPersonalWorkspaceHomeModelFromResult(
  createPersonalLocalDesktopCapabilityProfile(),
  {
    workspaceId: "workspace-performance",
    state: "Ready",
    healthStatus: "Healthy",
    backupStatus: "Fresh",
    recentDocuments: [],
    favorites: [],
    tags: [],
    recentChanges: [],
    unfinishedItems: [],
  },
);

const authoringCallbacks = {
  onHome() {}, onMode() {}, onBodyChange() {}, onSave() {}, onRetry() {}, onDiscard() {}, onCancel() {},
  onOpenLinkedDocument() {},
};
const navigatorCallbacks = {
  onCreateDocument() {}, onHome() {}, onView() {}, onFilter() {}, onRetry() {}, onOpenDocument() {},
};
const explorationCallbacks = {
  onHome() {}, onSearch() {}, onGraph() {}, onCanvas() {}, onAssets() {}, onOpenDocument() {}, onOpenAsset() {},
  onGraphQuery() {}, onGraphNodeSelect() {}, onGraphRetry() {}, onGraphReindex() {},
  onCanvasCreate() {}, onCanvasRetry() {}, onCanvasRecover() {}, onCanvasAddNote() {}, onCanvasAutoArrange() {}, onCanvasApplyArrange() {},
  onCanvasCancelArrange() {}, onCanvasZoom() {}, onCanvasPan() {}, onCanvasRemoveNode() {}, onCanvasAddDocument() {},
  onCanvasAddAsset() {}, onCanvasConnect() {}, onCanvasRemoveEdge() {}, onCanvasNodeSelect() {}, onCanvasEdgeSelect() {},
  onCanvasDragStart() {}, onCanvasDragEnd() {}, onCanvasResizeStart() {}, onCanvasResizeEnd() {},
  onAssetSelect() {}, onAssetRetry() {}, onAssetImport() {}, onAssetWorkspace() {}, onAssetLink() {}, onAssetUnlink() {}, onAssetCancel() {},
  canPlaceDocument: true, canPlaceAsset: true,
} as never;

const documentSnapshot = {
  workspaceId: "workspace-performance",
  documentId: "doc-target",
  title: "Performance document",
  path: "fixture/document",
  body: "# Performance\n\nBounded preview content.",
  revision: 0,
  persistedRevision: 0,
  expectedVersionId: "version-current",
  saveState: DocumentSaveCoordinatorState.Clean,
} as const;

const rows = Array.from({ length: 50 }, (_, index) => index);
const graphNodes = rows.map((index) => ({ id: `doc-${index}`, kind: "document" as const }));
const graphEdges = rows.slice(1).map((index) => ({
  id: `edge-${index}`,
  sourceId: "doc-0",
  targetId: `doc-${index}`,
  kind: "document_link" as const,
}));
const canvasNodes = rows.map((index) => ({
  nodeId: `node-${index}`,
  targetKind: "text" as const,
  targetId: `Card ${index}`,
  displayLabel: `Card ${index}`,
  targetStatus: "available" as const,
  x: (index % 10) * 100,
  y: Math.floor(index / 10) * 100,
  width: 320,
  height: 180,
}));
const canvasEdges = rows.slice(1).map((index) => ({
  edgeId: `canvas-edge-${index}`,
  sourceNodeId: `node-${index - 1}`,
  targetNodeId: `node-${index}`,
}));
const assets = rows.map((index) => ({
  assetId: index.toString(16).padStart(64, "0"),
  label: `Asset ${index}`,
  fileName: `asset-${index}`,
  mediaType: "application/octet-stream",
  byteSize: 1_024,
  status: "available" as const,
}));

const benchmarks = [
  {
    queryId: "current_document", standardFixtureCount: 10_000, boundedResultCount: 1,
    marker: 'data-cabinet-authoring-state="Clean"',
    countToken: 'data-cabinet-authoring-state="Clean"',
    execute: async () => {
      const controller = createDesktopDocumentAuthoringController({
        client: {
          async getCurrentDocument() {
            return {
              queryName: "get-current-document" as const,
              workspaceId: "workspace-performance",
              documentId: "doc-target",
              title: "Performance document",
              path: "fixture/document",
              body: documentSnapshot.body,
              versionId: "version-current",
            };
          },
          async saveDocumentRevision() { throw new Error("unused"); },
        },
        operationIdSource: () => "benchmark-save-operation",
        author: "fixture", summary: "fixture",
      });
      const snapshot = await controller.open({ queryName: "get-current-document", workspaceId: "workspace-performance", documentId: "doc-target" });
      return renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot, authoringCallbacks));
    },
  },
  {
    queryId: "history_page", standardFixtureCount: 1_000, boundedResultCount: 50,
    marker: 'data-history-restore-state="Ready"',
    countToken: 'data-history-entry="visible"',
    execute: async () => {
      const page = await Promise.resolve({ entries: rows.map((index) => ({ versionId: `version-${index}`, summary: `Revision ${index}` })) });
      return renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(documentSnapshot, authoringCallbacks, {
        history: { status: "Ready", entries: page.entries },
        inspector: { tab: "history", unlink: { status: "Closed" } },
      }));
    },
  },
  {
    queryId: "search", standardFixtureCount: 10_000, boundedResultCount: 50,
    marker: 'data-cabinet-navigator-state="Ready"',
    countToken: "data-document-id=",
    execute: async () => {
      const loading = createDocumentNavigatorLoadingModel({ workspaceId: "workspace-performance", view: "Tree", generation: 1, filter: "benchmark" });
      const model = await loadDesktopDocumentNavigator({
        async getDocumentNavigator(query) {
          return {
            workspaceId: query.workspaceId, view: query.view, state: "Ready" as const,
            items: rows.map((index) => ({ documentId: `doc-${index}`, title: `Result ${index}`, path: `fixture/${index}`, collections: [], tags: [], favorite: false })),
          };
        },
      }, loading);
      return renderToStaticMarkup(createDesktopDocumentNavigatorElement(model, navigatorCallbacks));
    },
  },
  {
    queryId: "link_overview", standardFixtureCount: 50_000, boundedResultCount: 50,
    marker: 'data-link-overview-state="Ready"',
    countToken: "data-linked-document-id=",
    execute: async () => {
      const loading = requestDesktopLinkOverviewLoad(createDesktopLinkOverviewSnapshot("workspace-performance", "doc-target"));
      const links = await loadDesktopLinkOverview({
        async getLinkOverview(query) {
          return {
            ...query,
            backlinks: rows.map((index) => ({ workspaceId: query.workspaceId, sourceDocumentId: `source-${index}`, targetDocumentId: query.documentId, sourceTitle: `Source ${index}`, sourcePath: `fixture/${index}` })),
            unresolvedLinks: [], orphanDocuments: [],
          };
        },
      }, loading);
      return renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(documentSnapshot, authoringCallbacks, { links }));
    },
  },
  {
    queryId: "local_graph", standardFixtureCount: 50_000, boundedResultCount: 50,
    marker: 'data-exploration-surface="graph"',
    countToken: "data-graph-node-id=",
    execute: async () => {
      const loading = requestDesktopGraphLoad(createDesktopGraphSnapshot("workspace-performance"), { centerDocumentId: "doc-0", scope: "local", depth: 2 });
      const snapshot = await loadDesktopKnowledgeGraph({
        async getKnowledgeGraph() { return { status: "clean" as const, nodes: graphNodes, edges: graphEdges, stats: { candidateCount: 50, filteredCount: 0 } }; },
      }, loading);
      return renderToStaticMarkup(createDesktopKnowledgeGraphElement(home, snapshot, explorationCallbacks));
    },
  },
  {
    queryId: "global_graph", standardFixtureCount: 50_000, boundedResultCount: 50,
    marker: 'data-exploration-surface="graph"',
    countToken: "data-graph-node-id=",
    execute: async () => {
      const loading = requestDesktopGraphLoad(createDesktopGraphSnapshot("workspace-performance"), { scope: "global" });
      const snapshot = await loadDesktopGlobalKnowledgeGraph({
        async getGlobalGraph() { return { status: "clean" as const, nodes: graphNodes, edges: graphEdges, candidateCount: 50 }; },
      }, loading);
      return renderToStaticMarkup(createDesktopKnowledgeGraphElement(home, snapshot, explorationCallbacks));
    },
  },
  {
    queryId: "canvas_viewport", standardFixtureCount: 2_000, boundedResultCount: 50,
    marker: 'data-canvas-revision="1"',
    countToken: 'data-action="select-canvas-node"',
    execute: async () => {
      const loading = requestDesktopCanvasLoad(createDesktopCanvasSnapshot("workspace-performance"), "canvas-performance");
      const snapshot = await loadDesktopCanvas({
        async execute() { return { canvasId: "canvas-performance", title: "Performance canvas", revision: 1, lifecycle: "updated" as const, viewport: { centerX: 600, centerY: 360, zoomPercent: 100 }, nodes: canvasNodes, edges: canvasEdges }; },
      }, loading);
      return renderToStaticMarkup(createDesktopCanvasElement(home, snapshot, explorationCallbacks));
    },
  },
  {
    queryId: "asset_metadata", standardFixtureCount: 10_000, boundedResultCount: 50,
    marker: 'data-exploration-surface="assets"',
    countToken: "data-asset-id=",
    execute: async () => {
      const loading = requestDesktopWorkspaceAssetLoad(createDesktopAssetSnapshot("workspace-performance"));
      const snapshot = await loadDesktopWorkspaceAssets({
        async listWorkspaceAssets() { return { workspaceId: "workspace-performance", assets }; },
      }, loading);
      return renderToStaticMarkup(createDesktopAttachmentsElement(home, snapshot, explorationCallbacks));
    },
  },
] as const;

async function main(): Promise<void> {
  for (const benchmark of benchmarks) {
    for (let index = 0; index < WARMUP_COUNT; index += 1) await benchmark.execute();
    const samples: number[] = [];
    let errorCount = 0;
    let markerMatched = true;
    let resultCountMatched = true;
    for (let index = 0; index < SAMPLE_COUNT; index += 1) {
      const started = performance.now();
      try {
        const markup = await benchmark.execute();
        markerMatched = markerMatched && markup.includes(benchmark.marker);
        const countMatched = markup.split(benchmark.countToken).length - 1 === benchmark.boundedResultCount;
        resultCountMatched = resultCountMatched && countMatched;
        if (!markup.includes(benchmark.marker) || !countMatched) errorCount += 1;
      } catch {
        errorCount += 1;
      }
      samples.push(performance.now() - started);
    }
    samples.sort((left, right) => left - right);
    const percentile = (value: number) => samples[Math.ceil(samples.length * value) - 1] ?? Number.POSITIVE_INFINITY;
    console.log([
      `query=${benchmark.queryId}`,
      `standard_fixture_count=${benchmark.standardFixtureCount}`,
      `bounded_result_count=${benchmark.boundedResultCount}`,
      `marker_matched=${markerMatched}`,
      `result_count_matched=${resultCountMatched}`,
      `sample_count=${samples.length}`,
      `error_count=${errorCount}`,
      `p50_ms=${percentile(0.5).toFixed(6)}`,
      `p95_ms=${percentile(0.95).toFixed(6)}`,
      `max_ms=${percentile(1).toFixed(6)}`,
    ].join(";"));
  }
}

void main();
