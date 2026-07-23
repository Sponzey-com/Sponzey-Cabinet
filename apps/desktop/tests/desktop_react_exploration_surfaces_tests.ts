import assert from "node:assert/strict";
import test from "node:test";

import React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { readFile } from "node:fs/promises";

import { createPersonalLocalDesktopCapabilityProfile } from "@sponzey-cabinet/client-core";
import { createPersonalWorkspaceHomeModelFromResult } from "@sponzey-cabinet/ui";

import {
  createDesktopAttachmentsElement,
  createDesktopCanvasElement,
  createDesktopKnowledgeGraphElement,
  createLinkedDocumentActionsElement,
} from "../src/react_exploration_surfaces.ts";
import { applyAttachmentFileStatus, createAttachmentFileSnapshot } from "../src/attachment_operation_presenter.ts";

const model = createPersonalWorkspaceHomeModelFromResult(
  createPersonalLocalDesktopCapabilityProfile(),
  {
    workspaceId: "workspace-1",
    state: "Ready",
    healthStatus: "Healthy",
    backupStatus: "Fresh",
    recentDocuments: [
      { documentId: "doc-1", title: "Cabinet 제품 지도", path: "projects/cabinet.md" },
      { documentId: "doc-2", title: "로컬 저장소 설계", path: "architecture/storage.md" },
    ],
    favorites: [],
    tags: [{ label: "Rust", documentCount: 2 }, { label: "UX", documentCount: 1 }],
    recentChanges: [],
    unfinishedItems: [],
  },
);

const callbacks = {
  onHome() {},
  onSearch() {},
  onGraph() {},
  onCanvas() {},
  onAssets() {},
  onAssetQueryChange(_query: string) {},
  onAssetMediaFilterChange(_filter: string) {},
  onAssetLoadMore() {},
  onOpenDocument(_documentId: string) {},
  onOpenAsset(_assetId: string) {},
  onGraphQuery() {},
  onGraphNodeSelect() {},
  onGraphRetry() {},
  onGraphReindex() {},
  onCanvasCreate() {},
  onCanvasRetry() {},
  onCanvasRecover() {},
  onCanvasAddNote() {},
  onCanvasAutoArrange() {},
  onCanvasApplyArrange() {},
  onCanvasCancelArrange() {},
  onCanvasZoom(_zoomPercent: number) {},
  onCanvasPan(_deltaX: number, _deltaY: number) {},
  onCanvasRemoveNode(_nodeId: string) {},
  onCanvasAddDocument() {},
  onCanvasAddAsset() {},
  onCanvasConnect() {},
  onCanvasRemoveEdge() {},
  canvasArchiveConfirmationOpen: false,
  canvasRenameDialogOpen: false,
  canvasRenameDraft: "",
  onCanvasArchiveRequest() {},
  onCanvasArchiveCancel() {},
  onCanvasRenameRequest() {},
  onCanvasRenameDraftChange(_title: string) {},
  onCanvasRenameCancel() {},
  onCanvasRename(_title: string) {},
  canvasTextEditDialog: { kind: "Closed" as const },
  onCanvasTextEditRequest(_nodeId: string, _text: string) {},
  onCanvasTextEditDraftChange(_text: string) {},
  onCanvasTextEditCancel() {},
  onCanvasTextEdit(_nodeId: string, _text: string) {},
  onCanvasArchive() {},
  onCanvasNodeSelect(_nodeId: string) {},
  onCanvasEdgeSelect(_edgeId: string) {},
  onCanvasDragStart(_nodeId: string, _clientX: number, _clientY: number) {},
  onCanvasDragEnd(_nodeId: string, _clientX: number, _clientY: number) {},
  onCanvasResizeStart(_nodeId: string, _clientX: number, _clientY: number) {},
  onCanvasResizeEnd(_nodeId: string, _clientX: number, _clientY: number) {},
  canPlaceDocument: true,
  canPlaceAsset: true,
};

test("Canvas recovery state renders an explicit recovery action without raw path", () => {
  const html = renderToStaticMarkup(createDesktopCanvasElement(model, {
    state: "RecoveryRequired",
    workspaceId: "workspace-1",
    canvasId: "canvas-1",
    generation: 2,
    selectedNodeIds: [],
    errorCode: "CANVAS_RECOVERY_REQUIRED",
    retryable: false,
  }, callbacks));
  assert.match(html, /data-action="recover-canvas"/);
  assert.match(html, /캔버스 복구/);
  assert.doesNotMatch(html, /CANVAS_RECOVERY_REQUIRED|COMMAND_BRIDGE_FAILED/);
  assert.doesNotMatch(html, /\/private|\\Users|file:\/\//);
});

test("Canvas recovery action takes precedence over a failed catalog refresh", () => {
  const html = renderToStaticMarkup(createDesktopCanvasElement(model, {
    state: "RecoveryRequired",
    workspaceId: "workspace-1",
    canvasId: "canvas-1",
    generation: 3,
    selectedNodeIds: [],
    errorCode: "CANVAS_RECOVERY_REQUIRED",
    retryable: false,
  }, {
    ...callbacks,
    canvasCatalog: {
      state: "Failed",
      workspaceId: "workspace-1",
      generation: 3,
      entries: [{ canvasId: "canvas-1", title: "보관 지도", lifecycle: "archived", revision: 4 }],
      errorCode: "canvas_catalog.corrupted",
      retryable: false,
    },
    displayedCanvasId: "canvas-1",
  }));
  assert.match(html, /data-exploration-state="RecoveryRequired"/);
  assert.match(html, /data-action="recover-canvas"/);
  assert.doesNotMatch(html, /캔버스 목록을 불러오지 못했습니다/);
});

test("knowledge graph renders the Penpot 20260721 topology with workspace data", () => {
  const html = renderToStaticMarkup(createDesktopKnowledgeGraphElement(model, {
    state: "Ready",
    workspaceId: "workspace-1",
    generation: 1,
    selectedNodeId: "doc-1",
    query: {
      centerDocumentId: "doc-1",
      depth: 1,
      direction: "both",
      includeUnresolved: true,
      includeAssets: false,
      nodeLimit: 120,
      edgeLimit: 240,
    },
    graph: {
      centerDocumentId: "doc-1",
      status: "clean",
      nodes: [
        { id: "doc-1", kind: "document", label: "그래프 제품 지도", breadcrumbLabel: "제품", availability: "available", canNavigate: true },
        { id: "doc-2", kind: "document", label: "그래프 저장소 설계", breadcrumbLabel: "아키텍처", availability: "available", canNavigate: true },
      ],
      edges: [{ id: "actual-edge", sourceId: "doc-1", targetId: "doc-2", kind: "document_link" }],
      stats: { candidateCount: 2, filteredCount: 0 },
      freshnessRevision: "version-1",
    },
  }, callbacks));

  assert.match(html, /data-exploration-surface="graph"/);
  assert.match(html, /data-exploration-state="Ready"/);
  assert.match(html, /data-exploration-generation="1"/);
  assert.match(html, /data-design-reference="penpot-20260721"/);
  assert.match(html, /지식 지도/);
  assert.match(html, /<strong>그래프 제품 지도<\/strong>/);
  assert.match(html, /<strong>그래프 저장소 설계<\/strong>/);
  assert.doesNotMatch(html, /<strong>Cabinet 제품 지도<\/strong>|<strong>로컬 저장소 설계<\/strong>/);
  assert.doesNotMatch(html, />doc-[12]</);
  assert.match(html, /문서 열기/);
  assert.match(html, /data-action="open-graph-document"/);
  assert.match(html, /data-action="navigate-home"/);
  assert.match(html, /data-action="workspace-search-input"/);
  assert.match(html, /data-action="navigate-canvas"/);
  assert.match(html, /data-action="navigate-assets"/);
  assert.match(html, /data-graph-camera-zoom="100"/);
  assert.match(html, /data-topology-renderer-host="accelerated"/);
  assert.match(html, /data-topology-semantic-list="available"/);
  assert.doesNotMatch(html, /class="graph-edges"/);
  assert.match(html, /data-action="graph-zoom-in"/);
  assert.match(html, /data-action="graph-zoom-out"/);
  assert.match(html, /data-action="graph-fit-view"/);
  assert.match(html, /data-action="graph-reset-layout"/);
  assert.match(html, /data-action="graph-pause-layout"/);
  assert.match(html, /title="화면에 맞춤"/);
  assert.match(html, /title="배치 초기화"/);
  assert.doesNotMatch(html, />⌗</);
  assert.match(html, /data-topology-visual-state="Initializing"/);
  assert.match(html, /data-action="graph-zoom-in"[^>]*disabled/);
  assert.match(html, /data-action="graph-pause-layout"[^>]*disabled/);
  assert.doesNotMatch(html, /data-action="(?:open-settings|toggle-theme|open-ai)"/);
  assert.match(html, />로컬</);
  assert.match(html, />전체</);
  assert.doesNotMatch(html, /title="전역 그래프는 아직 지원되지 않습니다"/);
});

test("knowledge graph consumes the controlled shared visual search", () => {
  const html = renderToStaticMarkup(createDesktopKnowledgeGraphElement(model, {
    state: "Ready", workspaceId: "workspace-1", generation: 1,
    query: { scope: "global", depth: 1, direction: "both", includeUnresolved: true, includeAssets: false, nodeLimit: 120, edgeLimit: 240 },
    graph: {
      status: "clean",
      nodes: [
        { id: "doc-1", kind: "document", label: "제품 개요", availability: "available", canNavigate: true },
        { id: "doc-2", kind: "document", label: "저장소 설계", availability: "available", canNavigate: true },
      ],
      edges: [{ id: "edge-1", sourceId: "doc-1", targetId: "doc-2", kind: "document_link" }], candidateCount: 2,
    },
  }, { ...callbacks, graphVisualSearch: "저장소", onGraphVisualSearch() {} }));

  assert.match(html, /data-action="filter-graph-nodes"[^>]*value="저장소"/);
  assert.match(html, /data-action="filter-graph-nodes"[^>]*aria-label="지도에서 문서 찾기"/);
  assert.match(html, />저장소 설계</);
  assert.doesNotMatch(html, />제품 개요</);
});

test("knowledge graph scope controls dispatch route mode intents", () => {
  const modes: string[] = [];
  const element = createDesktopKnowledgeGraphElement(model, {
    state: "Ready", workspaceId: "workspace-1", generation: 1,
    query: { scope: "global", depth: 1, direction: "both", includeUnresolved: true, includeAssets: false, nodeLimit: 120, edgeLimit: 240 },
    graph: { status: "clean", nodes: [], edges: [], candidateCount: 0 },
  }, {
    ...callbacks,
    onGraphScopeChange(scope) { modes.push(scope); },
  });
  const tree = renderFunctionElement(element);

  clickElement(tree, (props) => props["data-action"] === "graph-scope-local");
  clickElement(tree, (props) => props["data-action"] === "graph-scope-global");

  assert.deepEqual(modes, ["local", "global"]);
});

test("knowledge graph routes attachment activation and detail action to the exact asset", () => {
  const openedAssets: string[] = [];
  const element = createDesktopKnowledgeGraphElement(model, {
    state: "Ready",
    workspaceId: "workspace-1",
    generation: 1,
    selectedNodeId: "asset-secret",
    query: {
      centerDocumentId: "doc-1",
      depth: 1,
      direction: "both",
      includeUnresolved: true,
      includeAssets: true,
      nodeLimit: 120,
      edgeLimit: 240,
    },
    graph: {
      centerDocumentId: "doc-1",
      status: "clean",
      nodes: [
        { id: "doc-1", kind: "document", label: "제품 지도", availability: "available", canNavigate: true },
        { id: "asset-secret", kind: "attachment", label: "설계 자료.pdf", availability: "available", canNavigate: true },
        { id: "missing-secret", kind: "unresolved_link", label: "아직 없는 문서", availability: "missing", canNavigate: false },
      ],
      edges: [{ id: "asset-edge", sourceId: "doc-1", targetId: "asset-secret", kind: "attachment_reference" }],
      stats: { candidateCount: 3, filteredCount: 0 },
      freshnessRevision: "version-1",
    },
  }, {
    ...callbacks,
    onOpenAsset(assetId) { openedAssets.push(assetId); },
  });
  const tree = renderFunctionElement(element);
  const topology = findElement(tree, (props) => Array.isArray(props.semanticNodes));
  assert.ok(topology);
  (topology.props as { readonly onNodeActivated: (nodeId: string) => void }).onNodeActivated("asset-secret");
  clickElement(tree, (props) => props["data-action"] === "open-graph-asset");

  assert.deepEqual(openedAssets, ["asset-secret", "asset-secret"]);
  const html = renderToStaticMarkup(tree)
    .replace(/\sdata-[a-z-]+="[^"]*"/g, "");
  assert.match(html, /설계 자료\.pdf/);
  assert.doesNotMatch(html, />asset-secret</);
});

test("knowledge graph inspector follows the filtered visible node set", () => {
  const html = renderToStaticMarkup(createDesktopKnowledgeGraphElement(model, {
    state: "Ready",
    workspaceId: "workspace-1",
    generation: 1,
    selectedNodeId: "hidden-doc",
    query: {
      centerDocumentId: "visible-doc",
      depth: 1,
      direction: "both",
      includeUnresolved: true,
      includeAssets: false,
      nodeLimit: 120,
      edgeLimit: 240,
    },
    graph: {
      centerDocumentId: "visible-doc",
      status: "clean",
      nodes: [
        { id: "visible-doc", kind: "document", label: "표시 문서", breadcrumbLabel: "Cabinet", availability: "available", canNavigate: true },
        { id: "hidden-doc", kind: "document", label: "숨겨진 문서", breadcrumbLabel: "Cabinet", availability: "available", canNavigate: true },
      ],
      edges: [{ id: "edge-1", sourceId: "visible-doc", targetId: "hidden-doc", kind: "document_link" }],
      stats: { candidateCount: 2, filteredCount: 0 },
      freshnessRevision: "version-1",
    },
  }, {
    ...callbacks,
    graphVisualSearch: "표시",
  }));

  assert.match(html, /표시 문서/);
  assert.match(html, /노드를 선택하면 연결 정보를 확인할 수 있습니다/);
  assert.doesNotMatch(html, /숨겨진 문서/);
});

test("knowledge graph inspector counts only visible filtered connections", () => {
  const html = renderToStaticMarkup(createDesktopKnowledgeGraphElement(model, {
    state: "Ready",
    workspaceId: "workspace-1",
    generation: 1,
    selectedNodeId: "visible-doc",
    query: {
      centerDocumentId: "visible-doc",
      depth: 1,
      direction: "both",
      includeUnresolved: true,
      includeAssets: false,
      nodeLimit: 120,
      edgeLimit: 240,
    },
    graph: {
      centerDocumentId: "visible-doc",
      status: "clean",
      nodes: [
        { id: "visible-doc", kind: "document", label: "표시 문서", breadcrumbLabel: "Cabinet", availability: "available", canNavigate: true },
        { id: "hidden-doc", kind: "document", label: "숨겨진 문서", breadcrumbLabel: "Cabinet", availability: "available", canNavigate: true },
      ],
      edges: [{ id: "edge-1", sourceId: "visible-doc", targetId: "hidden-doc", kind: "document_link" }],
      stats: { candidateCount: 2, filteredCount: 0 },
      freshnessRevision: "version-1",
    },
  }, {
    ...callbacks,
    graphVisualSearch: "표시",
  }));

  assert.match(html, /표시 문서/);
  assert.match(html, /<dt>나가는 링크<\/dt><dd>0<\/dd>/);
  assert.match(html, /<dt>들어오는 링크<\/dt><dd>0<\/dd>/);
});

test("knowledge graph does not expose open actions for unresolved and external nodes", () => {
  for (const kind of ["unresolved_link", "external_link"] as const) {
    const html = renderToStaticMarkup(createDesktopKnowledgeGraphElement(model, {
      state: "Ready",
      workspaceId: "workspace-1",
      generation: 1,
      selectedNodeId: `${kind}-secret`,
      query: { centerDocumentId: "doc-1", depth: 1, direction: "both", includeUnresolved: true, includeAssets: true, nodeLimit: 120, edgeLimit: 240 },
      graph: {
        centerDocumentId: "doc-1",
        status: "clean",
        nodes: [{ id: `${kind}-secret`, kind, label: "이동할 수 없는 항목", availability: "available", canNavigate: false }],
        edges: [],
        stats: { candidateCount: 1, filteredCount: 0 },
        freshnessRevision: "version-1",
      },
    }, callbacks));

    assert.doesNotMatch(html, /data-action="open-graph-(?:document|asset)"/);
  }
});

test("ready knowledge graph without nodes renders one non-overlapping empty-state message", () => {
  const html = renderToStaticMarkup(createDesktopKnowledgeGraphElement(model, {
    state: "Ready",
    workspaceId: "workspace-1",
    generation: 1,
    query: {
      centerDocumentId: "doc-1",
      depth: 1,
      direction: "both",
      includeUnresolved: true,
      includeAssets: false,
      nodeLimit: 120,
      edgeLimit: 240,
    },
    graph: {
      centerDocumentId: "doc-1",
      status: "clean",
      nodes: [],
      edges: [],
      stats: { candidateCount: 0, filteredCount: 0 },
      freshnessRevision: "version-1",
    },
  }, callbacks));

  assert.equal((html.match(/class="graph-empty"/g) ?? []).length, 1);
  assert.doesNotMatch(html, /선택할 노드가 없습니다/);
});

test("global knowledge graph renders pagination without local repair action", () => {
  const snapshot = {
    state: "Stale" as const,
    workspaceId: "workspace-1",
    generation: 1,
    selectedNodeId: "doc-1",
    query: { scope: "global" as const, depth: 1 as const, direction: "both" as const, includeUnresolved: true, includeAssets: false, nodeLimit: 120, edgeLimit: 240 },
    graph: { status: "degraded" as const, nodes: [{ id: "doc-1", kind: "document" as const }], edges: [], candidateCount: 1, nextCursor: "projection-50" },
  };

  const html = renderToStaticMarkup(createDesktopKnowledgeGraphElement(model, snapshot, callbacks));

  assert.match(html, /다음 관계 불러오기/);
  assert.doesNotMatch(html, /관계 다시 만들기/);
  assert.match(html, /전체 관계 인덱스 일부가 오래되었습니다/);
});

test("knowledge graph renders actionable stale, repairing, and retryable failed states", () => {
  const base = {
    workspaceId: "workspace-1",
    generation: 1,
    query: { centerDocumentId: "doc-1", depth: 1 as const, direction: "both" as const, includeUnresolved: true, includeAssets: false, nodeLimit: 120, edgeLimit: 240 },
  };
  const stale = renderToStaticMarkup(createDesktopKnowledgeGraphElement(model, { ...base, state: "Stale" }, callbacks));
  const repairing = renderToStaticMarkup(createDesktopKnowledgeGraphElement(model, { ...base, state: "Repairing" }, callbacks));
  const failed = renderToStaticMarkup(createDesktopKnowledgeGraphElement(model, { ...base, state: "Failed", errorCode: "projection.failed", retryable: true }, callbacks));

  assert.match(stale, /data-action="reindex-graph"/);
  assert.match(stale, /관계 다시 만들기/);
  assert.match(repairing, /문서 관계를 다시 만드는 중입니다/);
  assert.doesNotMatch(repairing, /data-action="reindex-graph"/);
  assert.match(failed, /다시 만들기 재시도/);
});

test("canvas renders durable nodes, edges, revision and viewport controls without session fixtures", () => {
  const html = renderToStaticMarkup(createDesktopCanvasElement(model, {
    state: "Ready",
    workspaceId: "workspace-1",
    canvasId: "canvas-1",
    generation: 1,
    selectedNodeIds: [],
    canvas: {
      canvasId: "canvas-1",
      title: "Durable product map",
      revision: 7,
      lifecycle: "updated",
      viewport: { centerX: 100, centerY: 120, zoomPercent: 125 },
      nodes: [
        { nodeId: "document-node", targetKind: "document", targetId: "doc-secret-1", displayLabel: "설계 문서", targetStatus: "available", x: 80, y: 80, width: 320, height: 180 },
        { nodeId: "memo-node", targetKind: "text", targetId: "Persisted memo", displayLabel: "Persisted memo", targetStatus: "available", x: 520, y: 240, width: 240, height: 120 },
      ],
      edges: [{ edgeId: "edge-1", sourceNodeId: "document-node", targetNodeId: "memo-node" }],
    },
  }, callbacks));

  assert.match(html, /data-exploration-surface="canvas"/);
  assert.match(html, /data-exploration-state="Ready"/);
  assert.match(html, /data-exploration-generation="1"/);
  assert.match(html, /Durable product map/);
  assert.doesNotMatch(html, /revision 7/i);
  assert.match(html, /저장됨/);
  assert.match(html, /Persisted memo/);
  assert.match(html, /data-edge-id="edge-1"/);
  assert.match(html, /data-canvas-node-id="document-node"/);
  assert.match(html, /data-canvas-target-kind="document"/);
  assert.match(html, /data-canvas-target-id="doc-secret-1"/);
  assert.match(html, />문서</);
  assert.match(html, /aria-label="캔버스 카드: 설계 문서"/);
  assert.doesNotMatch(html, />doc-secret-1</);
  assert.doesNotMatch(html, /aria-label="[^"]*doc-secret-1/);
  assert.match(html, /data-canvas-node-x="80"/);
  assert.match(html, /data-canvas-node-width="320"/);
  assert.match(html, /125%/);
  assert.match(html, /data-action="add-canvas-note"/);
  assert.match(html, /data-action="edit-canvas-text-card"/);
  assert.match(html, /data-action="auto-arrange-canvas"/);
  assert.match(html, /canvas-minimap/);
  assert.match(html, /canvas-minimap-node/g);
  assert.match(html, /canvas-minimap-viewport/);
  assert.doesNotMatch(html, /class="canvas-minimap"[^>]*><i(?:\s|>)/);
  assert.match(html, /canvas-zoom-controls/);
  assert.doesNotMatch(html, /로컬 세션/);
  assert.doesNotMatch(html, /핵심 원칙을 한 문장으로 정리하세요/);
});

test("canvas text card dialog dispatches edit intent and blocks archived mutation", () => {
  const actions: string[] = [];
  const editingCallbacks = {
    ...callbacks,
    canvasTextEditDialog: { kind: "Editing" as const, nodeId: "memo-node", originalText: "Old memo", draft: "New memo" },
    onCanvasTextEditRequest(nodeId: string, text: string) { actions.push(`open:${nodeId}:${text}`); },
    onCanvasTextEditDraftChange(text: string) { actions.push(`draft:${text}`); },
    onCanvasTextEditCancel() { actions.push("cancel"); },
    onCanvasTextEdit(nodeId: string, text: string) { actions.push(`save:${nodeId}:${text}`); },
  };
  const snapshot = {
    state: "Ready" as const,
    workspaceId: "workspace-1",
    canvasId: "canvas-1",
    generation: 1,
    selectedNodeIds: [],
    canvas: {
      ...durableCanvas(),
      nodes: [{ nodeId: "memo-node", targetKind: "text" as const, targetId: "Persisted memo", displayLabel: "Persisted memo", targetStatus: "available" as const, x: 80, y: 80, width: 320, height: 180 }],
    },
  };
  const tree = renderFunctionElement(createDesktopCanvasElement(model, snapshot, editingCallbacks));

  clickElement(tree, (props) => props["data-action"] === "edit-canvas-text-card");
  const input = findElement(tree, (props) => props["data-action"] === "edit-canvas-text");
  assert.ok(input);
  (input.props as { readonly onChange: (event: { currentTarget: { value: string } }) => void })
    .onChange({ currentTarget: { value: "Changed memo" } });
  clickElement(tree, (props) => props["data-action"] === "confirm-canvas-text-edit");
  clickElement(tree, (props) => props["data-action"] === "cancel-canvas-text-edit");
  assert.deepEqual(actions, ["open:memo-node:Persisted memo", "draft:Changed memo", "save:memo-node:New memo", "cancel"]);

  const archived = renderToStaticMarkup(createDesktopCanvasElement(model, {
    ...snapshot,
    canvas: { ...snapshot.canvas, lifecycle: "archived" },
  }, callbacks));
  assert.match(archived, /data-action="edit-canvas-text-card"[^>]*disabled/);
});

test("canvas renders user save labels and explicit title and filename placement options", () => {
  const placementCallbacks = {
    ...callbacks,
    documentPlacementOptions: [{ identity: "doc-secret", label: "제품 요구사항" }],
    assetPlacementOptions: [{ identity: "asset-secret", label: "화면 설계.png" }],
  };
  const ready = renderToStaticMarkup(createDesktopCanvasElement(model, {
    state: "Ready",
    workspaceId: "workspace-1",
    canvasId: "canvas-1",
    generation: 1,
    selectedNodeIds: [],
    canvas: durableCanvas(),
  }, placementCallbacks));
  const visible = ready
    .replace(/\svalue="[^"]*"/g, "")
    .replace(/\sdata-[a-z-]+="[^"]*"/g, "");

  assert.match(visible, /저장됨/);
  assert.match(visible, /제품 요구사항/);
  assert.match(visible, /화면 설계\.png/);
  assert.doesNotMatch(visible, /revision|doc-secret|asset-secret/i);
  assert.match(ready, /data-action="select-canvas-document-target"/);
  assert.match(ready, /data-action="select-canvas-asset-target"/);

  const empty = renderToStaticMarkup(createDesktopCanvasElement(model, {
    state: "Ready",
    workspaceId: "workspace-1",
    canvasId: "canvas-1",
    generation: 1,
    selectedNodeIds: [],
    canvas: durableCanvas(),
  }, {
    ...callbacks,
    documentPlacementOptions: [],
    assetPlacementOptions: [],
  }));
  assert.match(empty, /data-action="add-canvas-document"[^>]*disabled/);
  assert.match(empty, /data-action="add-canvas-asset"[^>]*disabled/);
});

test("canvas renders current labels and disables navigation for missing targets", () => {
  let opened = 0;
  const tree = renderFunctionElement(createDesktopCanvasElement(model, {
    state: "Ready",
    workspaceId: "workspace-1",
    canvasId: "canvas-1",
    generation: 1,
    selectedNodeIds: [],
    canvas: {
      ...durableCanvas(),
      nodes: [{
        nodeId: "missing-document",
        targetKind: "document" as const,
        targetId: "doc-missing",
        displayLabel: "Deleted document",
        targetStatus: "missing" as const,
        x: 80,
        y: 80,
        width: 320,
        height: 180,
      }],
      edges: [],
    },
  }, { ...callbacks, onOpenDocument() { opened += 1; } }));

  const card = findElement(tree, (props) => props["aria-disabled"] === true);
  assert.ok(card);
  assert.equal(card.props.onDoubleClick, undefined);
  assert.equal(findElement(tree, (props) => props["data-action"] === "open-canvas-document"), undefined);
  assert.equal(opened, 0);
  const html = renderToStaticMarkup(tree);
  assert.match(html, /Deleted document/);
  assert.match(html, /대상을 찾을 수 없음/);
  assert.match(html, /canvas-card-missing/);
});

test("canvas renders explicit create, conflict, recovery and archived states", () => {
  const base = { workspaceId: "workspace-1", canvasId: "canvas-1", generation: 1, selectedNodeIds: [] };
  const missing = renderToStaticMarkup(createDesktopCanvasElement(model, { ...base, state: "Failed", errorCode: "CANVAS_NOT_FOUND", retryable: false }, callbacks));
  const conflict = renderToStaticMarkup(createDesktopCanvasElement(model, { ...base, state: "Conflict", errorCode: "CANVAS_VERSION_CONFLICT", canvas: durableCanvas() }, callbacks));
  const recovery = renderToStaticMarkup(createDesktopCanvasElement(model, { ...base, state: "RecoveryRequired", errorCode: "CANVAS_RECOVERY_REQUIRED" }, callbacks));
  const archived = renderToStaticMarkup(createDesktopCanvasElement(model, { ...base, state: "Ready", canvas: durableCanvas("archived") }, callbacks));

  assert.match(missing, /data-action="create-canvas"/);
  assert.match(conflict, /최신 상태 다시 불러오기/);
  assert.match(recovery, /캔버스 복구가 필요합니다/);
  assert.match(archived, /읽기 전용/);
  assert.match(archived, /data-canvas-lifecycle="archived"/);
  assert.match(archived, /data-canvas-title="Durable product map"/);
  assert.match(archived, /data-action="add-canvas-note"[^>]*disabled/);
  assert.match(archived, /data-action="rename-canvas"[^>]*disabled/);
  assert.match(archived, /data-action="archive-canvas"[^>]*disabled/);
});

test("canvas controls dispatch durable create, mutation, zoom and remove callbacks", () => {
  const actions: string[] = [];
  const interactiveCallbacks = {
    ...callbacks,
    onCanvasCreate() { actions.push("create"); },
    onCanvasAddNote() { actions.push("add-note"); },
    onCanvasAutoArrange() { actions.push("arrange"); },
    onCanvasApplyArrange() { actions.push("apply-arrange"); },
    onCanvasCancelArrange() { actions.push("cancel-arrange"); },
    onCanvasZoom(zoomPercent: number) { actions.push(`zoom:${zoomPercent}`); },
    onCanvasPan(deltaX: number, deltaY: number) { actions.push(`pan:${deltaX}:${deltaY}`); },
    onCanvasRemoveNode(nodeId: string) { actions.push(`remove:${nodeId}`); },
    onCanvasAddDocument(documentId: string) { actions.push(`add-document:${documentId}`); },
    onCanvasAddAsset(assetId: string) { actions.push(`add-asset:${assetId}`); },
    documentPlacementOptions: [{ identity: "doc-choice", label: "선택 문서" }],
    assetPlacementOptions: [{ identity: "asset-choice", label: "선택 파일.png" }],
    onCanvasConnect() { actions.push("connect"); },
    onCanvasRemoveEdge() { actions.push("remove-edge"); },
    onCanvasArchiveRequest() { actions.push("archive-request"); },
    onCanvasRenameRequest() { actions.push("rename-request"); },
    onCanvasNodeSelect(nodeId: string) { actions.push(`select:${nodeId}`); },
    onCanvasEdgeSelect(edgeId: string) { actions.push(`select-edge:${edgeId}`); },
    onCanvasDragStart(nodeId: string, clientX: number, clientY: number) { actions.push(`drag-start:${nodeId}:${clientX}:${clientY}`); },
    onCanvasDragEnd(nodeId: string, clientX: number, clientY: number) { actions.push(`drag-end:${nodeId}:${clientX}:${clientY}`); },
    onCanvasResizeStart(nodeId: string, clientX: number, clientY: number) { actions.push(`resize-start:${nodeId}:${clientX}:${clientY}`); },
    onCanvasResizeEnd(nodeId: string, clientX: number, clientY: number) { actions.push(`resize-end:${nodeId}:${clientX}:${clientY}`); },
    onOpenDocument(documentId: string) { actions.push(`open-document:${documentId}`); },
  };
  const readySnapshot = {
    state: "Ready" as const,
    workspaceId: "workspace-1",
    canvasId: "canvas-1",
    generation: 1,
    selectedNodeIds: ["memo-node", "document-node"],
    selectedEdgeId: "edge-1",
    canvas: {
      ...durableCanvas(),
      nodes: [
        { nodeId: "memo-node", targetKind: "text" as const, targetId: "Memo", displayLabel: "메모", targetStatus: "available" as const, x: 80, y: 80, width: 320, height: 180 },
        { nodeId: "document-node", targetKind: "document" as const, targetId: "doc-1", displayLabel: "문서 하나", targetStatus: "available" as const, x: 440, y: 80, width: 320, height: 180 },
      ],
      edges: [{ edgeId: "edge-1", sourceNodeId: "memo-node", targetNodeId: "document-node" }],
    },
  };
  const readyTree = renderFunctionElement(createDesktopCanvasElement(model, readySnapshot, interactiveCallbacks));
  clickElement(readyTree, (props) => props["data-action"] === "add-canvas-note");
  clickElement(readyTree, (props) => props["data-action"] === "auto-arrange-canvas");
  clickElement(readyTree, (props) => props["aria-label"] === "확대");
  clickElement(readyTree, (props) => props["aria-label"] === "오른쪽으로 이동");
  clickElement(readyTree, (props) => props["aria-label"] === "카드 제거");
  clickElement(readyTree, (props) => props["data-action"] === "add-canvas-document");
  clickElement(readyTree, (props) => props["data-action"] === "add-canvas-asset");
  clickElement(readyTree, (props) => props["data-action"] === "connect-canvas-nodes");
  clickElement(readyTree, (props) => props["data-action"] === "remove-canvas-edge");
  clickElement(readyTree, (props) => props["data-action"] === "rename-canvas");
  clickElement(readyTree, (props) => props["data-action"] === "archive-canvas");
  clickElement(readyTree, (props) => props["data-edge-id"] === "edge-1");
  const card = findElement(readyTree, (props) => props.draggable === true && props["aria-pressed"] === true);
  assert.ok(card);
  const cardProps = card.props as { readonly tabIndex: number; readonly role: string; readonly onClick: () => void; readonly onKeyDown: (event: { key: string; preventDefault(): void }) => void; readonly onDragStart: (event: { clientX: number; clientY: number }) => void; readonly onDragEnd: (event: { clientX: number; clientY: number }) => void };
  assert.equal(cardProps.tabIndex, 0);
  assert.equal(cardProps.role, "group");
  cardProps.onClick();
  cardProps.onKeyDown({ key: "Enter", preventDefault() {} });
  cardProps.onDragStart({ clientX: 10, clientY: 20 });
  cardProps.onDragEnd({ clientX: 40, clientY: 60 });
  const resizeHandle = findElement(readyTree, (props) => props["aria-label"] === "카드 크기 조절");
  assert.ok(resizeHandle);
  const resizeProps = resizeHandle.props as { readonly onPointerDown: (event: { clientX: number; clientY: number; stopPropagation(): void }) => void; readonly onPointerUp: (event: { clientX: number; clientY: number; stopPropagation(): void }) => void; readonly onKeyDown: (event: { key: string; preventDefault(): void; stopPropagation(): void }) => void };
  resizeProps.onPointerDown({ clientX: 50, clientY: 60, stopPropagation() {} });
  resizeProps.onPointerUp({ clientX: 90, clientY: 100, stopPropagation() {} });
  resizeProps.onKeyDown({ key: "ArrowRight", preventDefault() {}, stopPropagation() {} });
  clickElement(readyTree, (props) => props["data-action"] === "open-canvas-document");

  const missingTree = renderFunctionElement(createDesktopCanvasElement(model, {
    state: "Failed",
    workspaceId: "workspace-1",
    canvasId: "canvas-1",
    generation: 1,
    selectedNodeIds: [],
    errorCode: "CANVAS_NOT_FOUND",
    retryable: false,
  }, interactiveCallbacks));
  clickElement(missingTree, (props) => props["data-action"] === "create-canvas");

  const previewTree = renderFunctionElement(createDesktopCanvasElement(model, {
    ...readySnapshot,
    state: "ArrangePreview",
    arrangeBaseCanvas: readySnapshot.canvas,
  }, interactiveCallbacks));
  clickElement(previewTree, (props) => props["data-action"] === "apply-canvas-arrange");
  clickElement(previewTree, (props) => props["data-action"] === "cancel-canvas-arrange");

  assert.deepEqual(actions, [
    "add-note", "arrange", "zoom:110", "pan:120:0", "remove:memo-node",
    "add-document:doc-choice", "add-asset:asset-choice", "connect", "remove-edge", "rename-request", "archive-request", "select-edge:edge-1", "select:memo-node", "select:memo-node",
    "drag-start:memo-node:10:20", "drag-end:memo-node:40:60",
    "resize-start:memo-node:50:60", "resize-end:memo-node:90:100", "resize-start:memo-node:0:0", "resize-end:memo-node:16:0", "open-document:doc-1",
    "create", "apply-arrange", "cancel-arrange",
  ]);
});

test("canvas archive confirmation dispatches explicit cancel and confirm actions", () => {
  const actions: string[] = [];
  const snapshot = {
    state: "Ready" as const,
    workspaceId: "workspace-1",
    canvasId: "canvas-1",
    generation: 1,
    selectedNodeIds: [],
    canvas: durableCanvas(),
  };
  const confirmationCallbacks = {
    ...callbacks,
    canvasArchiveConfirmationOpen: true,
    onCanvasArchiveCancel() { actions.push("cancel"); },
    onCanvasArchive() { actions.push("confirm"); },
  };
  const tree = renderFunctionElement(createDesktopCanvasElement(model, snapshot, confirmationCallbacks));
  clickElement(tree, (props) => props["data-action"] === "cancel-canvas-archive");
  clickElement(tree, (props) => props["data-action"] === "confirm-canvas-archive");
  assert.deepEqual(actions, ["cancel", "confirm"]);
});

test("canvas rename dialog guards invalid titles and dispatches trimmed title, cancel, and Escape", () => {
  const actions: string[] = [];
  const snapshot = {
    state: "Ready" as const,
    workspaceId: "workspace-1",
    canvasId: "canvas-1",
    generation: 1,
    selectedNodeIds: [],
    canvas: durableCanvas(),
  };
  const renameCallbacks = {
    ...callbacks,
    canvasRenameDialogOpen: true,
    canvasRenameDraft: "  새 캔버스  ",
    onCanvasRenameDraftChange(title: string) { actions.push(`draft:${title}`); },
    onCanvasRenameCancel() { actions.push("cancel"); },
    onCanvasRename(title: string) { actions.push(`confirm:${title}`); },
  };
  const tree = renderFunctionElement(createDesktopCanvasElement(model, snapshot, renameCallbacks));
  const input = findElement(tree, (props) => props["aria-label"] === "새 캔버스 이름");
  assert.ok(input);
  (input.props as { readonly onChange: (event: { currentTarget: { value: string } }) => void }).onChange({ currentTarget: { value: "다음 이름" } });
  clickElement(tree, (props) => props["data-action"] === "cancel-canvas-rename");
  clickElement(tree, (props) => props["data-action"] === "confirm-canvas-rename");
  const dialog = findElement(tree, (props) => props["aria-label"] === "캔버스 이름 변경");
  assert.ok(dialog);
  (dialog.props as { readonly onKeyDown: (event: { key: string; preventDefault(): void }) => void }).onKeyDown({ key: "Escape", preventDefault() {} });
  assert.deepEqual(actions, ["draft:다음 이름", "cancel", "confirm:새 캔버스", "cancel"]);

  for (const invalidDraft of ["   ", durableCanvas().title, `  ${durableCanvas().title}  `]) {
    const invalidTree = renderFunctionElement(createDesktopCanvasElement(model, snapshot, {
      ...renameCallbacks,
      canvasRenameDraft: invalidDraft,
    }));
    const confirm = findElement(invalidTree, (props) => props["data-action"] === "confirm-canvas-rename");
    assert.ok(confirm);
    assert.equal((confirm.props as { readonly disabled: boolean }).disabled, true);
  }
});

test("canvas non-button interactions expose visible focus styles", async () => {
  const css = await readFile(new URL("../public/styles.css", import.meta.url), "utf8");
  assert.match(css, /\.canvas-card:focus-visible/);
  assert.match(css, /\.canvas-links path:focus-visible/);
  assert.match(css, /\.canvas-card-resize:focus-visible/);
});

test("canvas arrange preview keeps its primary apply action visible", async () => {
  const snapshot = {
    state: "ArrangePreview" as const,
    workspaceId: "workspace-1",
    canvasId: "canvas-1",
    generation: 1,
    selectedNodeIds: [],
    canvas: durableCanvas(),
    arrangeBaseCanvas: durableCanvas(),
  };
  const html = renderToStaticMarkup(createDesktopCanvasElement(model, snapshot, callbacks));
  const css = await readFile(new URL("../public/styles.css", import.meta.url), "utf8");

  assert.match(html, /class="canvas-arrange-actions"/);
  assert.match(html, /class="primary canvas-arrange-apply"[^>]*data-action="apply-canvas-arrange"/);
  assert.match(html, /data-action="apply-canvas-arrange"[^>]*aria-label="자동 정렬 배치 적용"/);
  assert.match(css, /\.canvas-toolbar \.canvas-arrange-apply\s*\{[^}]*background:\s*#0F8F83/s);
  assert.match(css, /\.canvas-toolbar \.canvas-arrange-apply\s*\{[^}]*color:\s*#fff/s);
  assert.match(css, /\.canvas-toolbar\s*\{[^}]*overflow-x:\s*auto/s);
  assert.match(css, /\.canvas-toolbar\s*\{[^}]*overflow-y:\s*hidden/s);
  assert.doesNotMatch(css, /\.canvas-toolbar\s*\{[^}]*overflow:\s*hidden/s);
});

test("canvas bounds large durable records to viewport DOM limits", () => {
  const nodes = Array.from({ length: 2_000 }, (_, index) => ({
    nodeId: `node-${index}`,
    targetKind: "text" as const,
    targetId: `Memo ${index}`,
    x: 0,
    y: 0,
    width: 320,
    height: 180,
  }));
  const edges = Array.from({ length: 4_000 }, (_, index) => ({
    edgeId: `edge-${index}`,
    sourceNodeId: `node-${index % 2_000}`,
    targetNodeId: `node-${(index + 1) % 2_000}`,
  }));
  const html = renderToStaticMarkup(createDesktopCanvasElement(model, {
    state: "Ready",
    workspaceId: "workspace-1",
    canvasId: "large-canvas",
    generation: 1,
    selectedNodeIds: [],
    canvas: {
      canvasId: "large-canvas",
      title: "Large Canvas",
      revision: 1,
      lifecycle: "updated",
      viewport: { centerX: 0, centerY: 0, zoomPercent: 100 },
      nodes,
      edges,
    },
  }, callbacks));

  assert.equal((html.match(/class="canvas-card /g) ?? []).length, 250);
  assert.ok((html.match(/data-edge-id=/g) ?? []).length <= 500);
  assert.match(html, /250\/2000 카드/);
});

test("attachments renders durable DTO metadata without a fake browser file import", () => {
  const html = renderToStaticMarkup(createDesktopAttachmentsElement(model, {
    state: "Ready",
    scope: "Document",
    workspaceId: "workspace-1",
    documentId: "doc-1",
    generation: 1,
    selectedAssetId: "asset-1",
    page: {
      queryName: "list-document-assets",
      workspaceId: "workspace-1",
      documentId: "doc-1",
      assets: [{
        assetId: "asset-1",
        label: "Architecture",
        fileName: "architecture.pdf",
        mediaType: "application/pdf",
        byteSize: 2048,
        status: "metadata_only",
      }],
    },
  }, { ...callbacks, onAssetSelect() {}, onAssetRetry() {}, onAssetImport() {}, onAssetWorkspace() {}, onAssetLink() {}, onAssetUnlink() {}, onAssetCancel() {} }));

  assert.match(html, /data-exploration-surface="assets"/);
  assert.match(html, /첨부 파일/);
  assert.match(html, /architecture\.pdf/);
  assert.match(html, /메타데이터만 있음/);
  assert.doesNotMatch(html, />metadata_only</);
  assert.doesNotMatch(html, /type="file"/);
  assert.doesNotMatch(html, /현재 세션/);
  assert.match(html, /이미지/);
  assert.match(html, /PDF/);
  assert.match(html, /연결 문서/);
  assert.match(html, /data-action="import-asset"/);
  assert.match(html, /파일 추가/);
  assert.match(html, /data-action="unlink-asset"/);
  assert.match(html, /data-action="open-asset-library"/);
  assert.match(html, /data-asset-filter="전체 파일"/);
  assert.match(html, /aria-label="파일 형식 필터"/);
  assert.match(html, /data-action="filter-assets-all"[^>]*aria-pressed="true"/);
  assert.match(html, /data-action="filter-assets-pdf"[^>]*aria-pressed="false"/);
  assert.match(html, /현재 문서 파일/);
  assert.match(html, /현재 불러온 파일에서 검색/);
  assert.match(html, /data-action="search-assets"[^>]*aria-label="첨부 파일 목록 검색"/);
});

test("attachments identifies bounded workspace results and exposes pagination without claiming durable search", () => {
  const html = renderToStaticMarkup(createDesktopAttachmentsElement(model, {
    state: "Ready", scope: "Workspace", workspaceId: "workspace-1", generation: 2,
    importState: "Idle", importGeneration: 0, query: "", mediaFilter: "all",
    page: {
      queryName: "list-workspace-assets", workspaceId: "workspace-1", nextCursor: "opaque-cursor",
      assets: [{ assetId: "asset-1", label: "Architecture", fileName: "architecture.pdf", mediaType: "application/pdf", byteSize: 2048, status: "available" }],
    },
  }, { ...callbacks, onAssetSelect() {}, onAssetRetry() {}, onAssetImport() {}, onAssetWorkspace() {}, onAssetLink() {}, onAssetUnlink() {}, onAssetCancel() {}, onAssetPreview() {}, onAssetPreviewClose() {} }));

  assert.match(html, /전체 파일 보관함/);
  assert.match(html, /현재 불러온 1개 파일에서 검색/);
  assert.match(html, /전체 결과 중 일부를 표시/);
  assert.match(html, /data-action="load-more-assets"/);
  assert.doesNotMatch(html, /전체 보관함 검색/);
});

test("attachments inspector follows the filtered visible asset set", () => {
  const html = renderToStaticMarkup(createDesktopAttachmentsElement(model, {
    state: "Ready",
    scope: "Workspace",
    workspaceId: "workspace-1",
    generation: 2,
    importState: "Idle",
    importGeneration: 0,
    query: "visible",
    mediaFilter: "all",
    selectedAssetId: "hidden-asset",
    page: {
      queryName: "list-workspace-assets",
      workspaceId: "workspace-1",
      assets: [
        { assetId: "visible-asset", label: "Visible reference", fileName: "visible.pdf", mediaType: "application/pdf", byteSize: 100, status: "available" },
        { assetId: "hidden-asset", label: "Hidden reference", fileName: "hidden.pdf", mediaType: "application/pdf", byteSize: 200, status: "available" },
      ],
    },
  }, { ...callbacks, onAssetSelect() {}, onAssetRetry() {}, onAssetImport() {}, onAssetWorkspace() {}, onAssetLink() {}, onAssetUnlink() {}, onAssetCancel() {}, onAssetPreview() {}, onAssetPreviewClose() {} }));

  assert.match(html, /visible\.pdf/);
  assert.match(html, /파일을 선택하세요/);
  assert.doesNotMatch(html, /hidden\.pdf/);
  assert.doesNotMatch(html, /Hidden reference/);
});

test("attachments exposes per-file partial outcomes and exact cancel repair and new-selection actions", () => {
  const completed = createAttachmentFileSnapshot({ generation: 3, operationId: "operation-completed-private", fileName: "done.pdf", byteSize: 10, state: "completed" });
  const active = createAttachmentFileSnapshot({ generation: 3, operationId: "operation-active-private", fileName: "active.txt", byteSize: 20, state: "staging" });
  const failed = applyAttachmentFileStatus(
    createAttachmentFileSnapshot({ generation: 3, operationId: "operation-failed-private", fileName: "/private/failed.txt", byteSize: 30, state: "selected" }),
    { generation: 3, operationId: "operation-failed-private", state: "failed", errorCode: "asset_import.private_error" },
  );
  const recovery = applyAttachmentFileStatus(
    createAttachmentFileSnapshot({ generation: 3, operationId: "operation-recovery-private", fileName: "recover.pdf", byteSize: 40, state: "projecting" }),
    { generation: 3, operationId: "operation-recovery-private", state: "recovery_required", errorCode: "asset_projection.private_error" },
  );
  let cancelled = 0;
  let restarted = 0;
  let repaired: string | undefined;
  const element = createDesktopAttachmentsElement(model, {
    ...createAssetSnapshotForOperations(),
    importState: "Failed",
    importOperationId: "operation-active-private",
    importOperations: [completed, active, failed, recovery],
  }, {
    ...callbacks,
    onAssetSelect() {}, onAssetRetry() {}, onAssetImport() { restarted += 1; }, onAssetWorkspace() {},
    onAssetLink() {}, onAssetUnlink() {}, onAssetCancel() { cancelled += 1; },
    onAssetPreview() {}, onAssetPreviewClose() {},
    onAssetRepair(operationId) { repaired = operationId; },
  });
  const tree = renderFunctionElement(element);
  const html = renderToStaticMarkup(tree);

  assert.match(html, /done\.pdf/);
  assert.match(html, /active\.txt/);
  assert.match(html, /failed\.txt/);
  assert.match(html, /recover\.pdf/);
  assert.equal((html.match(/data-action="cancel-asset-import"/g) ?? []).length, 1);
  assert.match(html, /data-action="restart-asset-import"/);
  assert.match(html, /data-action="repair-asset-import"/);
  assert.doesNotMatch(html, /operation-(?:completed|active|failed|recovery)-private|\/private\/|private_error/);

  clickElement(tree, (props) => props["data-action"] === "cancel-asset-import");
  clickElement(tree, (props) => props["data-action"] === "restart-asset-import");
  clickElement(tree, (props) => props["data-action"] === "repair-asset-import");
  assert.equal(cancelled, 1);
  assert.equal(restarted, 1);
  assert.equal(repaired, "operation-recovery-private");
});

test("attachments renders bounded linked document actions and opens the exact document identity", () => {
  const linkedDocumentIds = Array.from({ length: 24 }, (_, index) => `doc-${index + 1}`);
  let openedDocumentId: string | undefined;
  const element = createDesktopAttachmentsElement(model, {
    state: "Ready",
    scope: "Workspace",
    workspaceId: "workspace-1",
    documentId: "doc-current",
    generation: 1,
    selectedAssetId: "asset-1",
    page: {
      queryName: "list-workspace-assets",
      workspaceId: "workspace-1",
      assets: [{
        assetId: "asset-1",
        label: "Architecture",
        fileName: "architecture.pdf",
        mediaType: "application/pdf",
        byteSize: 2048,
        status: "available",
      }],
    },
    detail: {
      assetId: "asset-1",
      fileName: "architecture.pdf",
      mediaType: "application/pdf",
      byteSize: 2048,
      version: 1,
      previewCapability: "pdf",
      extractionStatus: "not_requested",
      referenceCount: linkedDocumentIds.length,
      linkedDocumentIds,
      linkedDocuments: linkedDocumentIds.map((documentId, index) => ({
        documentId,
        title: index === 23 ? "최근 목록 밖 문서" : `현재 제목 ${index + 1}`,
        state: "available" as const,
      })),
    },
  }, {
    ...callbacks,
    onOpenDocument(documentId) { openedDocumentId = documentId; },
    onAssetSelect() {}, onAssetRetry() {}, onAssetImport() {}, onAssetWorkspace() {},
    onAssetLink() {}, onAssetUnlink() {}, onAssetCancel() {},
  });

  const html = renderToStaticMarkup(element);
  assert.equal((html.match(/data-linked-document-id=/g) ?? []).length, 20);
  assert.match(html, /data-linked-document-id="doc-20"/);
  assert.doesNotMatch(html, /data-linked-document-id="doc-21"/);
  assert.match(html, /외 4개/);
  assert.match(html, />현재 제목 1</);
  assert.match(html, />현재 제목 2</);
  assert.doesNotMatch(html, />doc-(?:1|2|20)</);

  clickElement(createLinkedDocumentActionsElement(linkedDocumentIds, (documentId) => {
    openedDocumentId = documentId;
  }, linkedDocumentIds.map((documentId, index) => ({
    category: "document" as const, identity: documentId, label: `현재 제목 ${index + 1}`,
    breadcrumbLabel: "", statusLabel: "", state: "resolved" as const,
  }))), (props) => props["data-linked-document-id"] === "doc-7");
  assert.equal(openedDocumentId, "doc-7");
});

test("attachments uses authoritative linked titles outside recent documents and disables missing targets", () => {
  let openedDocumentId: string | undefined;
  const html = renderToStaticMarkup(createDesktopAttachmentsElement(model, {
    state: "Ready", scope: "Workspace", workspaceId: "workspace-1", generation: 1,
    selectedAssetId: "asset-1",
    page: { queryName: "list-workspace-assets", workspaceId: "workspace-1", assets: [{ assetId: "asset-1", label: "Spec", fileName: "spec.pdf", mediaType: "application/pdf", byteSize: 42, status: "available" }] },
    detail: {
      assetId: "asset-1", fileName: "spec.pdf", mediaType: "application/pdf", byteSize: 42,
      version: 1, previewCapability: "pdf", extractionStatus: "not_requested", referenceCount: 2,
      linkedDocumentIds: ["outside-recent", "deleted-document"],
      linkedDocuments: [
        { documentId: "outside-recent", title: "최근 목록 밖의 현재 제목", state: "available" },
        { documentId: "deleted-document", state: "missing" },
      ],
    },
  }, {
    ...callbacks,
    onOpenDocument(documentId) { openedDocumentId = documentId; },
    onAssetSelect() {}, onAssetRetry() {}, onAssetImport() {}, onAssetWorkspace() {},
    onAssetLink() {}, onAssetUnlink() {}, onAssetCancel() {},
  }));

  assert.match(html, /최근 목록 밖의 현재 제목/);
  assert.match(html, /찾을 수 없는 문서/);
  assert.match(html, /data-linked-document-id="deleted-document"[^>]*disabled/);
  assert.doesNotMatch(html, />outside-recent<|>deleted-document</);
  assert.equal(openedDocumentId, undefined);
});

test("attachments renders an explicit empty linked document state", () => {
  const html = renderToStaticMarkup(createDesktopAttachmentsElement(model, {
    state: "Ready",
    scope: "Workspace",
    workspaceId: "workspace-1",
    generation: 1,
    selectedAssetId: "asset-1",
    page: {
      queryName: "list-workspace-assets",
      workspaceId: "workspace-1",
      assets: [{ assetId: "asset-1", label: "Loose file", fileName: "loose.txt", mediaType: "text/plain", byteSize: 4, status: "available" }],
    },
    detail: {
      assetId: "asset-1", fileName: "loose.txt", mediaType: "text/plain", byteSize: 4,
      version: 1, previewCapability: "text", extractionStatus: "not_requested",
      referenceCount: 0, linkedDocumentIds: [], linkedDocuments: [],
    },
  }, { ...callbacks, onAssetSelect() {}, onAssetRetry() {}, onAssetImport() {}, onAssetWorkspace() {}, onAssetLink() {}, onAssetUnlink() {}, onAssetCancel() {} }));

  assert.match(html, /연결된 문서가 없습니다/);
  assert.doesNotMatch(html, /data-linked-document-id=/);
});

test("attachments renders bounded native text preview with close action", () => {
  const html = renderToStaticMarkup(createDesktopAttachmentsElement(model, {
    state: "Ready", scope: "Document", workspaceId: "workspace-1", documentId: "doc-1", generation: 1,
    selectedAssetId: "asset-1", previewState: "Ready",
    page: { queryName: "list-document-assets", workspaceId: "workspace-1", documentId: "doc-1", assets: [{ assetId: "asset-1", label: "Note", fileName: "note.txt", mediaType: "text/plain", byteSize: 7, status: "available" }] },
    detail: { assetId: "asset-1", fileName: "note.txt", mediaType: "text/plain", byteSize: 7, version: 1, previewCapability: "text", extractionStatus: "not_requested", referenceCount: 1, linkedDocumentIds: ["doc-1"], linkedDocuments: [{ documentId: "doc-1", title: "Cabinet 제품 지도", state: "available" }] },
    preview: { assetId: "asset-1", capability: "text", mediaType: "text/plain", presentation: "text", content: "preview" },
  }, { ...callbacks, onAssetSelect() {}, onAssetRetry() {}, onAssetImport() {}, onAssetWorkspace() {}, onAssetLink() {}, onAssetUnlink() {}, onAssetCancel() {}, onAssetPreview() {}, onAssetPreviewClose() {} }));
  assert.match(html, /data-action="open-asset-preview"/);
  assert.match(html, /data-asset-preview-state="Ready"/);
  assert.match(html, /data-asset-preview-presentation="text"/);
  assert.match(html, /data-action="close-asset-preview"/);
  assert.doesNotMatch(html, /\/private|file:\/\//);
});

function durableCanvas(lifecycle: "draft" | "updated" | "archived" = "updated") {
  return {
    canvasId: "canvas-1",
    title: "Durable product map",
    revision: 7,
    lifecycle,
    viewport: { centerX: 0, centerY: 0, zoomPercent: 100 },
    nodes: [],
    edges: [],
  } as const;
}

function createAssetSnapshotForOperations() {
  return {
    state: "Empty" as const,
    scope: "Document" as const,
    workspaceId: "workspace-1",
    documentId: "doc-1",
    generation: 1,
    importState: "Idle" as const,
    importGeneration: 3,
    query: "",
    mediaFilter: "all" as const,
  };
}

function renderFunctionElement(element: React.ReactElement): React.ReactElement {
  const component = element.type as (props: Record<string, unknown>) => React.ReactElement;
  return component(element.props as Record<string, unknown>);
}

function clickElement(
  tree: React.ReactNode,
  predicate: (props: Record<string, unknown>) => boolean,
): void {
  const found = findElement(tree, predicate);
  assert.ok(found, "interactive element must exist");
  const onClick = (found.props as { readonly onClick?: () => void }).onClick;
  assert.equal(typeof onClick, "function");
  onClick?.();
}

function findElement(
  node: React.ReactNode,
  predicate: (props: Record<string, unknown>) => boolean,
): React.ReactElement | undefined {
  if (Array.isArray(node)) {
    for (const child of node) {
      const found = findElement(child, predicate);
      if (found) return found;
    }
    return undefined;
  }
  if (!React.isValidElement(node)) return undefined;
  const props = node.props as Record<string, unknown>;
  if (predicate(props)) return node;
  return findElement(props.children as React.ReactNode, predicate);
}
