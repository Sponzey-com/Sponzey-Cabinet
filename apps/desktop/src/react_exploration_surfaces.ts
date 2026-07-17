import React, { useMemo, useState } from "react";

import type { PersonalWorkspaceHomeModel } from "@sponzey-cabinet/ui";
import type { DesktopAssetSurfaceSnapshot } from "./desktop_asset_controller.ts";
import type { DesktopCanvasSurfaceSnapshot } from "./desktop_canvas_controller.ts";
import type { DesktopGraphQueryState, DesktopGraphSurfaceSnapshot } from "./desktop_graph_controller.ts";
import { createCanvasWorldTransform, projectDesktopCanvasViewport } from "./canvas_viewport_projection.ts";
import { createWorkspaceShellModel, type WorkspaceShellRouteKind } from "./workspace_shell_contract.ts";
import { createWorkspaceShellElement } from "./react_workspace_shell.ts";
import { KO_KR_MESSAGES } from "./ko_kr_catalog.ts";
import { filterGraphDisplayNodes, presentGraphNodes } from "./graph_display_presenter.ts";
import type { DisplayReference } from "./display_reference_resolver.ts";
import { presentAssetMetadata, presentLinkedDocuments } from "./asset_display_presenter.ts";
import { mapUserFacingError } from "./user_facing_error_presenter.ts";
import { handleModalKeyboard } from "./modal_keyboard_policy.ts";
import { browserModalFocusEnvironment, createFocusRestoringModalAction } from "./modal_focus_restoration.ts";

export interface DesktopExplorationCallbacks {
  readonly onHome: () => void;
  readonly onSearch: () => void;
  readonly onGraph: () => void;
  readonly onCanvas: () => void;
  readonly onAssets: () => void;
  readonly onDocument: () => void;
  readonly onBackup: () => void;
  readonly onCreateDocument: () => void;
  readonly onOpenDocument: (documentId: string) => void;
  readonly onOpenAsset: (assetId: string) => void;
}

type ExplorationSurface = "graph" | "canvas" | "assets";

export function createDesktopKnowledgeGraphElement(
  model: PersonalWorkspaceHomeModel,
  snapshot: DesktopGraphSurfaceSnapshot,
  callbacks: DesktopExplorationCallbacks & {
    readonly onGraphQuery: (patch: Partial<DesktopGraphQueryState>) => void;
    readonly onGraphNodeSelect: (nodeId: string) => void;
    readonly onGraphRetry: () => void;
    readonly onGraphReindex: () => void;
  },
): React.ReactElement {
  return React.createElement(DesktopKnowledgeGraph, { model, snapshot, callbacks });
}

export function createDesktopCanvasElement(
  model: PersonalWorkspaceHomeModel,
  snapshot: DesktopCanvasSurfaceSnapshot,
  callbacks: DesktopExplorationCallbacks & {
    readonly onCanvasCreate: () => void;
    readonly onCanvasRetry: () => void;
    readonly onCanvasRecover: () => void;
    readonly onCanvasAddNote: () => void;
    readonly onCanvasAutoArrange: () => void;
    readonly onCanvasApplyArrange: () => void;
    readonly onCanvasCancelArrange: () => void;
    readonly onCanvasZoom: (zoomPercent: number) => void;
    readonly onCanvasPan: (deltaX: number, deltaY: number) => void;
    readonly onCanvasRemoveNode: (nodeId: string) => void;
    readonly onCanvasAddDocument: (documentId: string) => void;
    readonly onCanvasAddAsset: (assetId: string) => void;
    readonly documentPlacementOptions?: readonly CanvasPlacementOption[];
    readonly assetPlacementOptions?: readonly CanvasPlacementOption[];
    readonly selectedDocumentPlacementId?: string;
    readonly selectedAssetPlacementId?: string;
    readonly onDocumentPlacementSelect?: (documentId: string) => void;
    readonly onAssetPlacementSelect?: (assetId: string) => void;
    readonly onCanvasConnect: () => void;
    readonly onCanvasRemoveEdge: () => void;
    readonly canvasArchiveConfirmationOpen: boolean;
    readonly canvasRenameDialogOpen: boolean;
    readonly canvasRenameDraft: string;
    readonly onCanvasArchiveRequest: () => void;
    readonly onCanvasArchiveCancel: () => void;
    readonly onCanvasRenameRequest: () => void;
    readonly onCanvasRenameDraftChange: (title: string) => void;
    readonly onCanvasRenameCancel: () => void;
    readonly onCanvasRename: (title: string) => void;
    readonly onCanvasArchive: () => void;
    readonly onCanvasNodeSelect: (nodeId: string) => void;
    readonly onCanvasEdgeSelect: (edgeId: string) => void;
    readonly onCanvasDragStart: (nodeId: string, clientX: number, clientY: number) => void;
    readonly onCanvasDragEnd: (nodeId: string, clientX: number, clientY: number) => void;
    readonly onCanvasResizeStart: (nodeId: string, clientX: number, clientY: number) => void;
    readonly onCanvasResizeEnd: (nodeId: string, clientX: number, clientY: number) => void;
    readonly canPlaceDocument: boolean;
    readonly canPlaceAsset: boolean;
  },
): React.ReactElement {
  return React.createElement(DesktopCanvas, { model, snapshot, callbacks });
}

export interface CanvasPlacementOption {
  readonly identity: string;
  readonly label: string;
}

export function createDesktopAttachmentsElement(
  model: PersonalWorkspaceHomeModel,
  snapshot: DesktopAssetSurfaceSnapshot,
  callbacks: DesktopExplorationCallbacks & {
    readonly onAssetSelect: (assetId: string) => void;
    readonly onAssetRetry: () => void;
    readonly onAssetImport: () => void;
    readonly onAssetWorkspace: () => void;
    readonly onAssetLink: () => void;
    readonly onAssetUnlink: () => void;
    readonly onAssetCancel: () => void;
    readonly onAssetPreview: () => void;
    readonly onAssetPreviewClose: () => void;
  },
): React.ReactElement {
  return React.createElement(DesktopAttachments, { model, snapshot, callbacks });
}

export function createLinkedDocumentActionsElement(
  documentIds: readonly string[],
  onOpenDocument: (documentId: string) => void,
  references: readonly DisplayReference[] = [],
): React.ReactElement {
  return linkedDocumentActions(documentIds, onOpenDocument, references);
}

function DesktopKnowledgeGraph({
  model,
  snapshot,
  callbacks,
}: {
  readonly model: PersonalWorkspaceHomeModel;
  readonly snapshot: DesktopGraphSurfaceSnapshot;
  readonly callbacks: DesktopExplorationCallbacks & {
    readonly onGraphQuery: (patch: Partial<DesktopGraphQueryState>) => void;
    readonly onGraphNodeSelect: (nodeId: string) => void;
    readonly onGraphRetry: () => void;
    readonly onGraphReindex: () => void;
  };
}): React.ReactElement {
  const e = React.createElement;
  const [query, setQuery] = useState("");
  const [cameraZoom, setCameraZoom] = useState(100);
  const cameraScale = cameraZoom / 100;
  const graph = snapshot.graph;
  const documentReferences: readonly DisplayReference[] = model.recentDocuments.map((document) => Object.freeze({
    category: "document" as const,
    identity: document.documentId,
    label: document.title,
    breadcrumbLabel: document.path.split("/").slice(0, -1).join(" / "),
    statusLabel: "",
    state: "resolved" as const,
  }));
  const presentedNodes = presentGraphNodes(graph?.nodes ?? [], documentReferences);
  const visibleNodes = filterGraphDisplayNodes(presentedNodes, query);
  const visibleIds = new Set(visibleNodes.map((node) => node.identity));
  const visibleEdges = (graph?.edges ?? []).filter((edge) =>
    visibleIds.has(edge.sourceId) && visibleIds.has(edge.targetId),
  );
  const positions = new Map(visibleNodes.map((node, index) => [
    node.identity,
    node.identity === graph?.centerDocumentId
      ? { x: 500, y: 310 }
      : graphPosition(index, visibleNodes.length),
  ]));
  const selected = presentedNodes.find((node) => node.identity === snapshot.selectedNodeId);
  const incoming = selected ? graph?.edges.filter((edge) => edge.targetId === selected.identity).length ?? 0 : 0;
  const outgoing = selected ? graph?.edges.filter((edge) => edge.sourceId === selected.identity).length ?? 0 : 0;

  return surfaceShell(
    "graph",
    snapshot.state,
    snapshot.generation,
    model,
    callbacks,
    e(
      "main",
      { className: "desktop-main graph-main" },
      e(
        "header",
        { className: "exploration-heading" },
        e("div", null, e("h1", null, "지식 지도"), e("p", null, "문서, 태그와 연결 관계를 한눈에 탐색합니다.")),
        e("span", { className: "surface-count" }, `${graph?.nodes.length ?? 0} 노드 · ${graph?.edges.length ?? 0} 연결`),
      ),
      e(
        "div",
        { className: "graph-filterbar", role: "search" },
        e("label", { className: "graph-search" }, e("span", { "aria-hidden": "true" }, "⌕"), e("input", {
          type: "search",
          "data-action": "filter-graph-nodes",
          value: query,
          placeholder: "지도에서 문서 찾기",
          onChange: (event: React.ChangeEvent<HTMLInputElement>) => setQuery(event.currentTarget.value),
        })),
        e("button", { type: "button", "data-action": "graph-scope-local", className: `filter-chip${snapshot.query.scope !== "global" ? " active" : ""}`, onClick: () => callbacks.onGraphQuery({ scope: "local" }) }, "로컬"),
        e("button", { type: "button", "data-action": "graph-scope-global", className: `filter-chip${snapshot.query.scope === "global" ? " active" : ""}`, onClick: () => callbacks.onGraphQuery({ scope: "global" }) }, "전체"),
        e("button", { type: "button", "data-action": "graph-toggle-depth", disabled: snapshot.query.scope === "global", className: `filter-chip${snapshot.query.depth === 2 ? " active" : ""}`, onClick: () => callbacks.onGraphQuery({ depth: snapshot.query.depth === 1 ? 2 : 1 }) }, `${snapshot.query.depth}단계`),
        e("button", { type: "button", "data-action": "graph-toggle-direction", className: `filter-chip${snapshot.query.direction === "incoming" ? " active" : ""}`, onClick: () => callbacks.onGraphQuery({ direction: snapshot.query.direction === "incoming" ? "both" : "incoming" }) }, "들어오는 링크"),
        e("button", { type: "button", "data-action": "graph-toggle-unresolved", className: `filter-chip${snapshot.query.includeUnresolved ? " active" : ""}`, onClick: () => callbacks.onGraphQuery({ includeUnresolved: !snapshot.query.includeUnresolved }) }, "미해결 링크"),
        e("button", { type: "button", "data-action": "graph-toggle-assets", className: `filter-chip${snapshot.query.includeAssets ? " active" : ""}`, onClick: () => callbacks.onGraphQuery({ includeAssets: !snapshot.query.includeAssets }) }, "첨부 포함"),
      ),
      e(
        "section",
        { className: "graph-stage", "aria-label": "문서 관계 그래프", "data-graph-camera-zoom": cameraZoom },
        e("div", { className: "graph-grid" }),
        e(
          "svg",
          { className: "graph-edges", viewBox: "0 0 1000 620", preserveAspectRatio: "none", "aria-hidden": "true", style: { transform: `scale(${cameraScale})`, transformOrigin: "50% 50%" } },
          visibleEdges.map((edge) => {
            const source = positions.get(edge.sourceId);
            const target = positions.get(edge.targetId);
            return source && target ? e("line", { key: edge.id, x1: source.x, y1: source.y, x2: target.x, y2: target.y, "data-edge-id": edge.id }) : null;
          }),
        ),
        visibleNodes.map((node, index) => {
          const position = positions.get(node.identity) ?? graphPosition(index, visibleNodes.length);
          return e(
            "button",
            {
              key: node.identity,
              type: "button",
              className: `graph-node ${node.identity === graph?.centerDocumentId ? "graph-node-root" : "graph-node-document"} tone-${index % 4}${selected?.identity === node.identity ? " selected" : ""}`,
              "data-action": "select-graph-node",
              "data-graph-node-id": node.identity,
              style: { left: `${(500 + (position.x - 500) * cameraScale) / 10}%`, top: `${(310 + (position.y - 310) * cameraScale) / 6.2}%` },
              onClick: () => callbacks.onGraphNodeSelect(node.identity),
            },
            e("i", null),
            e("strong", null, node.label),
            e("small", null, node.kindLabel),
          );
        }),
        e("div", { className: "graph-legend" }, e("span", null, e("i", { className: "legend-document" }), "문서"), e("span", null, e("i", { className: "legend-tag" }), "미해결/첨부"), e("span", null, e("i", { className: "legend-link" }), "링크")),
        e("div", { className: "graph-zoom-controls" }, e("button", { type: "button", "data-action": "graph-zoom-in", disabled: cameraZoom >= 200, "aria-label": `확대, 현재 ${cameraZoom}%`, onClick: () => setCameraZoom((value) => Math.min(200, value + 25)) }, "+"), e("button", { type: "button", "data-action": "graph-zoom-out", disabled: cameraZoom <= 50, "aria-label": `축소, 현재 ${cameraZoom}%`, onClick: () => setCameraZoom((value) => Math.max(50, value - 25)) }, "−"), e("button", { type: "button", "data-action": "graph-fit-view", "aria-label": "화면에 맞춤", onClick: () => setCameraZoom(100) }, "⌗")),
        snapshot.state === "Loading" ? e("p", { className: "graph-empty", role: "status" }, "지식 지도를 불러오는 중입니다.") : null,
        snapshot.state === "Failed" ? e("div", { className: "graph-empty", role: "alert" }, e("strong", null, "지식 지도를 불러오지 못했습니다."), e("button", { type: "button", "data-action": snapshot.retryable && snapshot.query.scope !== "global" ? "reindex-graph" : "retry-graph", onClick: snapshot.retryable && snapshot.query.scope !== "global" ? callbacks.onGraphReindex : callbacks.onGraphRetry }, snapshot.retryable && snapshot.query.scope !== "global" ? "다시 만들기 재시도" : "다시 시도")) : null,
        snapshot.state === "Empty" ? e("p", { className: "graph-empty" }, "표시할 문서 관계가 없습니다.") : null,
        snapshot.state === "Stale" && snapshot.query.scope === "global"
          ? e("div", { className: "graph-stale", role: "status" }, e("span", null, "전체 관계 인덱스 일부가 오래되었습니다."), e("button", { type: "button", "data-action": "retry-graph", onClick: callbacks.onGraphRetry }, "다시 불러오기"))
          : snapshot.state === "Stale" ? e("div", { className: "graph-stale", role: "status" }, e("span", null, "문서 관계를 갱신해야 합니다."), e("button", { type: "button", "data-action": "reindex-graph", onClick: callbacks.onGraphReindex }, "관계 다시 만들기")) : null,
        snapshot.state === "Repairing" ? e("p", { className: "graph-stale", role: "status" }, "문서 관계를 다시 만드는 중입니다.") : null,
        snapshot.query.scope === "global" && graph?.nextCursor
          ? e("button", { type: "button", className: "graph-next-page", "data-action": "load-next-graph-page", onClick: () => callbacks.onGraphQuery({ globalCursor: graph.nextCursor }) }, "다음 관계 불러오기")
          : null,
        selected
          ? e(
              "aside",
              { className: "graph-detail", "aria-label": "선택한 문서" },
              e("span", { className: "detail-type" }, selected.kindLabel),
              e("h2", null, selected.label),
              e("p", null, selected.breadcrumbLabel || selected.kindLabel),
              e("dl", null, e("div", null, e("dt", null, "들어오는 링크"), e("dd", null, `${incoming}`)), e("div", null, e("dt", null, "나가는 링크"), e("dd", null, `${outgoing}`))),
              selected.kind === "document" && selected.state !== "missing" ? e("button", { type: "button", className: "primary", "data-action": "open-graph-document", onClick: () => callbacks.onOpenDocument(selected.identity) }, "문서 열기") : null,
            )
          : snapshot.state === "Ready" || snapshot.state === "Stale" ? e("p", { className: "graph-empty" }, "선택할 노드가 없습니다.") : null,
      ),
    ),
  );
}

function DesktopCanvas({
  model,
  snapshot,
  callbacks,
}: {
  readonly model: PersonalWorkspaceHomeModel;
  readonly snapshot: DesktopCanvasSurfaceSnapshot;
  readonly callbacks: DesktopExplorationCallbacks & {
    readonly onCanvasCreate: () => void;
    readonly onCanvasRetry: () => void;
    readonly onCanvasRecover: () => void;
    readonly onCanvasAddNote: () => void;
    readonly onCanvasAutoArrange: () => void;
    readonly onCanvasApplyArrange: () => void;
    readonly onCanvasCancelArrange: () => void;
    readonly onCanvasZoom: (zoomPercent: number) => void;
    readonly onCanvasPan: (deltaX: number, deltaY: number) => void;
    readonly onCanvasRemoveNode: (nodeId: string) => void;
    readonly onCanvasAddDocument: (documentId: string) => void;
    readonly onCanvasAddAsset: (assetId: string) => void;
    readonly documentPlacementOptions?: readonly CanvasPlacementOption[];
    readonly assetPlacementOptions?: readonly CanvasPlacementOption[];
    readonly selectedDocumentPlacementId?: string;
    readonly selectedAssetPlacementId?: string;
    readonly onDocumentPlacementSelect?: (documentId: string) => void;
    readonly onAssetPlacementSelect?: (assetId: string) => void;
    readonly onCanvasConnect: () => void;
    readonly onCanvasRemoveEdge: () => void;
    readonly canvasArchiveConfirmationOpen: boolean;
    readonly canvasRenameDialogOpen: boolean;
    readonly canvasRenameDraft: string;
    readonly onCanvasArchiveRequest: () => void;
    readonly onCanvasArchiveCancel: () => void;
    readonly onCanvasRenameRequest: () => void;
    readonly onCanvasRenameDraftChange: (title: string) => void;
    readonly onCanvasRenameCancel: () => void;
    readonly onCanvasRename: (title: string) => void;
    readonly onCanvasArchive: () => void;
    readonly onCanvasNodeSelect: (nodeId: string) => void;
    readonly onCanvasEdgeSelect: (edgeId: string) => void;
    readonly onCanvasDragStart: (nodeId: string, clientX: number, clientY: number) => void;
    readonly onCanvasDragEnd: (nodeId: string, clientX: number, clientY: number) => void;
    readonly onCanvasResizeStart: (nodeId: string, clientX: number, clientY: number) => void;
    readonly onCanvasResizeEnd: (nodeId: string, clientX: number, clientY: number) => void;
    readonly canPlaceDocument: boolean;
    readonly canPlaceAsset: boolean;
  };
}): React.ReactElement {
  const e = React.createElement;
  const cancelRename = createFocusRestoringModalAction(callbacks.onCanvasRenameCancel, browserModalFocusEnvironment("rename-canvas"));
  const confirmRename = createFocusRestoringModalAction(() => callbacks.onCanvasRename(renameDraft.trim()), browserModalFocusEnvironment("rename-canvas"));
  const cancelArchive = createFocusRestoringModalAction(callbacks.onCanvasArchiveCancel, browserModalFocusEnvironment("archive-canvas"));
  const confirmArchive = createFocusRestoringModalAction(callbacks.onCanvasArchive, browserModalFocusEnvironment("archive-canvas"));
  const documentPlacementOptions = Array.isArray(callbacks.documentPlacementOptions)
    ? callbacks.documentPlacementOptions
    : [];
  const assetPlacementOptions = Array.isArray(callbacks.assetPlacementOptions)
    ? callbacks.assetPlacementOptions
    : [];
  const selectedDocumentId = documentPlacementOptions.some((option) => option.identity === callbacks.selectedDocumentPlacementId)
    ? callbacks.selectedDocumentPlacementId ?? ""
    : documentPlacementOptions[0]?.identity ?? "";
  const selectedAssetId = assetPlacementOptions.some((option) => option.identity === callbacks.selectedAssetPlacementId)
    ? callbacks.selectedAssetPlacementId ?? ""
    : assetPlacementOptions[0]?.identity ?? "";
  const canvas = snapshot.canvas;
  const renameDialogOpen = callbacks.canvasRenameDialogOpen === true;
  const renameDraft = typeof callbacks.canvasRenameDraft === "string"
    ? callbacks.canvasRenameDraft
    : canvas?.title ?? "";
  const archived = canvas?.lifecycle === "archived";
  const mutationDisabled = snapshot.state !== "Ready" || archived;
  const zoom = canvas?.viewport.zoomPercent ?? 100;
  const projection = canvas ? projectDesktopCanvasViewport(canvas, {
    width: 1_200,
    height: 720,
    overscan: 120,
    nodeLimit: 250,
    edgeLimit: 500,
  }) : undefined;
  const nodesById = new Map(projection?.nodes.map((node) => [node.nodeId, node]) ?? []);
  const targetKindLabels = Object.freeze({ document: "문서", attachment: "첨부 파일", external: "외부 링크", text: "메모" });
  const saveLabel = archived
    ? "읽기 전용"
    : snapshot.state === "PreviewingArrange" ? "정렬 미리보기 계산 중"
      : snapshot.state === "ArrangePreview" ? "정렬 미리보기"
    : snapshot.state === "Mutating" ? "저장 중"
      : canvas ? "저장됨" : "저장되지 않음";
  const edgeElements = projection?.edges.map((edge) => {
    const source = nodesById.get(edge.sourceNodeId);
    const target = nodesById.get(edge.targetNodeId);
    if (!source || !target) return null;
    const sx = source.x + source.width / 2;
    const sy = source.y + source.height / 2;
    const tx = target.x + target.width / 2;
    const ty = target.y + target.height / 2;
    const mid = (sx + tx) / 2;
    return e("path", {
      key: edge.edgeId,
      className: snapshot.selectedEdgeId === edge.edgeId ? "selected" : undefined,
      "data-action": "select-canvas-edge",
      "data-edge-id": edge.edgeId,
      d: `M${sx} ${sy} C${mid} ${sy} ${mid} ${ty} ${tx} ${ty}`,
      tabIndex: 0,
      onClick: () => callbacks.onCanvasEdgeSelect(edge.edgeId),
      onKeyDown: (event: React.KeyboardEvent<SVGPathElement>) => { if (event.key === "Enter" || event.key === " ") callbacks.onCanvasEdgeSelect(edge.edgeId); },
    });
  });
  const nodeElements = projection?.nodes.map((node) => {
    const style = { left: node.x, top: node.y, width: node.width, minHeight: node.height } as React.CSSProperties;
    const targetStatus = node.targetStatus;
    const content = [
      e("span", { key: "type", className: "canvas-card-type" }, targetKindLabels[node.targetKind]),
      node.targetKind === "text" ? e("p", { key: "value" }, node.displayLabel) : e("strong", { key: "value" }, node.displayLabel),
      node.targetStatus === "missing" ? e("span", { key: "missing", className: "canvas-card-status" }, "대상을 찾을 수 없음") : null,
      node.targetKind === "document" && targetStatus === "available" ? e("button", { key: "open-document", type: "button", className: "canvas-card-open", "data-action": "open-canvas-document", onClick: (event?: React.MouseEvent) => { event?.stopPropagation(); callbacks.onOpenDocument(node.targetId); } }, "문서 열기") : null,
      node.targetKind === "attachment" && targetStatus === "available" ? e("button", { key: "open-asset", type: "button", className: "canvas-card-open", "data-action": "open-canvas-asset", onClick: (event?: React.MouseEvent) => { event?.stopPropagation(); callbacks.onOpenAsset(node.targetId); } }, "파일 열기") : null,
      e("button", { key: "remove", type: "button", className: "canvas-card-remove", "data-action": "remove-canvas-node", disabled: mutationDisabled, "aria-label": "카드 제거", onClick: (event?: React.MouseEvent) => { event?.stopPropagation(); callbacks.onCanvasRemoveNode(node.nodeId); } }, "×"),
      snapshot.selectedNodeIds.includes(node.nodeId) ? e("span", {
        key: "resize",
        className: "canvas-card-resize",
        role: "button",
        "data-action": "resize-canvas-node",
        tabIndex: mutationDisabled ? -1 : 0,
        "aria-label": "카드 크기 조절",
        "aria-disabled": mutationDisabled,
        onPointerDown: (event: React.PointerEvent<HTMLSpanElement>) => {
          event.stopPropagation();
          if (!mutationDisabled) callbacks.onCanvasResizeStart(node.nodeId, event.clientX, event.clientY);
        },
        onPointerUp: (event: React.PointerEvent<HTMLSpanElement>) => {
          event.stopPropagation();
          if (!mutationDisabled) callbacks.onCanvasResizeEnd(node.nodeId, event.clientX, event.clientY);
        },
        onKeyDown: (event: React.KeyboardEvent<HTMLSpanElement>) => {
          const delta = event.key === "ArrowRight" ? [16, 0] : event.key === "ArrowLeft" ? [-16, 0] : event.key === "ArrowDown" ? [0, 16] : event.key === "ArrowUp" ? [0, -16] : undefined;
          if (!delta || mutationDisabled) return;
          event.preventDefault();
          event.stopPropagation();
          callbacks.onCanvasResizeStart(node.nodeId, 0, 0);
          callbacks.onCanvasResizeEnd(node.nodeId, delta[0], delta[1]);
        },
      }) : null,
    ];
    const interaction = {
      style,
      "data-action": "select-canvas-node",
      "data-canvas-node-id": node.nodeId,
      "data-canvas-target-kind": node.targetKind,
      "data-canvas-target-id": node.targetId,
      "data-canvas-node-x": node.x,
      "data-canvas-node-y": node.y,
      "data-canvas-node-width": node.width,
      "data-canvas-node-height": node.height,
      role: "group",
      tabIndex: 0,
      "aria-label": `캔버스 카드: ${node.displayLabel}`,
      draggable: !mutationDisabled,
      "aria-pressed": snapshot.selectedNodeIds.includes(node.nodeId),
      onClick: () => callbacks.onCanvasNodeSelect(node.nodeId),
      onKeyDown: (event: React.KeyboardEvent<HTMLElement>) => {
        if (event.key !== "Enter" && event.key !== " ") return;
        event.preventDefault();
        callbacks.onCanvasNodeSelect(node.nodeId);
      },
      onDragStart: (event: React.DragEvent<HTMLElement>) => callbacks.onCanvasDragStart(node.nodeId, event.clientX, event.clientY),
      onDragEnd: (event: React.DragEvent<HTMLElement>) => callbacks.onCanvasDragEnd(node.nodeId, event.clientX, event.clientY),
    };
    const selectedClass = snapshot.selectedNodeIds.includes(node.nodeId) ? " selected" : "";
    const missingClass = targetStatus === "missing" ? " canvas-card-missing" : "";
    if (node.targetKind === "document") return e("article", { key: node.nodeId, className: `canvas-card canvas-document${selectedClass}${missingClass}`, ...interaction, "aria-disabled": targetStatus === "missing", onDoubleClick: targetStatus === "available" ? () => callbacks.onOpenDocument(node.targetId) : undefined }, ...content);
    if (node.targetKind === "attachment") return e("article", { key: node.nodeId, className: `canvas-card canvas-asset${selectedClass}${missingClass}`, ...interaction, "aria-disabled": targetStatus === "missing", onDoubleClick: targetStatus === "available" ? () => callbacks.onOpenAsset(node.targetId) : undefined }, ...content);
    return e("article", { key: node.nodeId, className: `canvas-card canvas-note${selectedClass}`, ...interaction }, ...content);
  });

  return surfaceShell(
    "canvas",
    snapshot.state,
    snapshot.generation,
    model,
    callbacks,
    e(
      "main",
      { className: "desktop-main canvas-main", "data-canvas-lifecycle": canvas?.lifecycle ?? "missing", "data-canvas-title": canvas?.title ?? "" },
      e(
        "header",
        { className: "canvas-header" },
        e("div", null, e("span", { className: "canvas-breadcrumb" }, "캔버스 /"), e("h1", null, canvas?.title ?? "캔버스")),
        e("span", { className: "canvas-save-state", "data-canvas-state": snapshot.state }, e("i", null), saveLabel),
      ),
      e(
        "div",
        { className: "canvas-toolbar", role: "toolbar", "aria-label": "캔버스 도구" },
        e("span", { className: "active canvas-mode-indicator", "aria-current": "true", title: "현재 선택 모드" }, "↖", e("span", null, "선택")),
        e("select", { "data-action": "select-canvas-document-target", "aria-label": "배치할 문서", value: selectedDocumentId, disabled: mutationDisabled || documentPlacementOptions.length === 0, onChange: (event: React.ChangeEvent<HTMLSelectElement>) => callbacks.onDocumentPlacementSelect?.(event.currentTarget.value) }, documentPlacementOptions.map((option) => e("option", { key: option.identity, value: option.identity }, option.label))),
        e("button", { type: "button", "data-action": "add-canvas-document", onClick: () => selectedDocumentId && callbacks.onCanvasAddDocument(selectedDocumentId), disabled: mutationDisabled || !selectedDocumentId }, "▤", e("span", null, "문서 배치")),
        e("button", { type: "button", "data-action": "add-canvas-note", disabled: mutationDisabled, onClick: callbacks.onCanvasAddNote }, "T", e("span", null, "메모")),
        e("select", { "data-action": "select-canvas-asset-target", "aria-label": "배치할 파일", value: selectedAssetId, disabled: mutationDisabled || assetPlacementOptions.length === 0, onChange: (event: React.ChangeEvent<HTMLSelectElement>) => callbacks.onAssetPlacementSelect?.(event.currentTarget.value) }, assetPlacementOptions.map((option) => e("option", { key: option.identity, value: option.identity }, option.label))),
        e("button", { type: "button", "data-action": "add-canvas-asset", onClick: () => selectedAssetId && callbacks.onCanvasAddAsset(selectedAssetId), disabled: mutationDisabled || !selectedAssetId }, "▱", e("span", null, "파일 배치")),
        e("button", { type: "button", "data-action": "connect-canvas-nodes", disabled: mutationDisabled || snapshot.selectedNodeIds.length !== 2, onClick: callbacks.onCanvasConnect, title: "두 카드를 선택해 연결합니다" }, "⌁", e("span", null, "연결")),
        e("button", { type: "button", "data-action": "remove-canvas-edge", disabled: mutationDisabled || !snapshot.selectedEdgeId, onClick: callbacks.onCanvasRemoveEdge }, "×", e("span", null, "연결 삭제")),
        e("span", { className: "toolbar-divider" }),
        snapshot.state === "ArrangePreview"
          ? e(
              "span",
              { className: "canvas-arrange-actions", role: "group", "aria-label": "자동 정렬 미리보기 작업" },
              e("button", { type: "button", className: "primary canvas-arrange-apply", "data-action": "apply-canvas-arrange", "aria-label": "자동 정렬 배치 적용", onClick: callbacks.onCanvasApplyArrange }, "✓", e("span", null, "배치 적용")),
              e("button", { type: "button", className: "canvas-arrange-cancel", "data-action": "cancel-canvas-arrange", "aria-label": "자동 정렬 취소", onClick: callbacks.onCanvasCancelArrange }, "×", e("span", null, "취소")),
            )
          : e("button", { type: "button", "data-action": "auto-arrange-canvas", disabled: mutationDisabled, onClick: callbacks.onCanvasAutoArrange }, "⌘", e("span", null, "자동 정렬")),
        e("span", { className: "toolbar-divider" }),
        e("button", { type: "button", "data-action": "rename-canvas", disabled: mutationDisabled, onClick: callbacks.onCanvasRenameRequest }, "✎", e("span", null, "이름 변경")),
        e("button", { type: "button", "data-action": "archive-canvas", disabled: mutationDisabled, onClick: callbacks.onCanvasArchiveRequest }, "□", e("span", null, "보관")),
      ),
      e(
        "section",
        { className: "canvas-stage", "data-canvas-revision": canvas?.revision },
        canvas ? e("div", { className: "canvas-world", style: { transform: createCanvasWorldTransform(canvas.viewport) } },
          e("svg", { className: "canvas-links", viewBox: "0 0 1200 720", preserveAspectRatio: "none", role: "group", "aria-label": "캔버스 연결" }, edgeElements),
          nodeElements,
        ) : null,
        canvas && canvas.nodes.length === 0 ? e("div", { className: "canvas-empty" }, e("strong", null, "비어 있는 캔버스입니다"), e("p", null, "메모를 추가하거나 문서와 파일을 배치하세요.")) : null,
        snapshot.state === "Conflict" || !canvas ? canvasStateMessage(snapshot, callbacks) : null,
        projection ? e("output", { className: "canvas-viewport-status", "aria-live": "polite" }, projection.truncated
          ? `${projection.nodes.length}/${projection.totalNodeCount} 카드 · ${projection.edges.length}/${projection.totalEdgeCount} 연결 표시`
          : `${projection.nodes.length} 카드 · ${projection.edges.length} 연결`) : null,
        e("div", { className: "canvas-minimap", "aria-label": "캔버스 미니맵" }, e("i", null), e("i", null), e("i", null)),
        e("div", { className: "canvas-zoom-controls" },
          e("button", { type: "button", "data-action": "pan-canvas-left", disabled: mutationDisabled, onClick: () => callbacks.onCanvasPan(-120, 0), "aria-label": "왼쪽으로 이동" }, "←"),
          e("button", { type: "button", "data-action": "pan-canvas-up", disabled: mutationDisabled, onClick: () => callbacks.onCanvasPan(0, -120), "aria-label": "위로 이동" }, "↑"),
          e("button", { type: "button", "data-action": "pan-canvas-down", disabled: mutationDisabled, onClick: () => callbacks.onCanvasPan(0, 120), "aria-label": "아래로 이동" }, "↓"),
          e("button", { type: "button", "data-action": "pan-canvas-right", disabled: mutationDisabled, onClick: () => callbacks.onCanvasPan(120, 0), "aria-label": "오른쪽으로 이동" }, "→"),
          e("button", { type: "button", "data-action": "zoom-canvas-out", disabled: mutationDisabled, onClick: () => callbacks.onCanvasZoom(Math.max(25, zoom - 10)), "aria-label": "축소" }, "−"),
          e("output", null, `${zoom}%`),
          e("button", { type: "button", "data-action": "zoom-canvas-in", disabled: mutationDisabled, onClick: () => callbacks.onCanvasZoom(Math.min(400, zoom + 10)), "aria-label": "확대" }, "+")
        ),
      ),
      renameDialogOpen ? e(
        "div",
        {
          role: "dialog",
          "aria-modal": "true",
          "aria-label": "캔버스 이름 변경",
          className: "canvas-rename-dialog",
          onKeyDown: (event: React.KeyboardEvent<HTMLDivElement>) => handleModalKeyboard(event, cancelRename),
        },
        e("label", null, "새 이름", e("input", {
          type: "text",
          "data-action": "edit-canvas-title",
          value: renameDraft,
          autoFocus: true,
          "aria-label": "새 캔버스 이름",
          onChange: (event: React.ChangeEvent<HTMLInputElement>) => callbacks.onCanvasRenameDraftChange(event.currentTarget.value),
        })),
        e("button", { type: "button", "data-action": "cancel-canvas-rename", onClick: cancelRename }, "취소"),
        e("button", {
          type: "button",
          "data-action": "confirm-canvas-rename",
          disabled: renameDraft.trim().length === 0 || renameDraft.trim() === canvas?.title,
          onClick: confirmRename,
        }, "변경"),
      ) : null,
      callbacks.canvasArchiveConfirmationOpen ? e("div", { role: "dialog", "aria-modal": "true", "aria-label": "캔버스 보관 확인", className: "canvas-archive-dialog", onKeyDown: (event: React.KeyboardEvent<HTMLDivElement>) => handleModalKeyboard(event, cancelArchive) }, e("p", null, "이 캔버스를 읽기 전용으로 보관합니다."), e("button", { type: "button", "data-action": "cancel-canvas-archive", onClick: cancelArchive }, "취소"), e("button", { type: "button", "data-action": "confirm-canvas-archive", onClick: confirmArchive }, "보관")) : null,
    ),
  );
}

function canvasStateMessage(
  snapshot: DesktopCanvasSurfaceSnapshot,
  callbacks: { readonly onCanvasCreate: () => void; readonly onCanvasRetry: () => void; readonly onCanvasRecover: () => void },
): React.ReactElement {
  const e = React.createElement;
  if (snapshot.state === "Loading" || snapshot.state === "Creating" || snapshot.state === "Recovering") {
    const label = snapshot.state === "Creating" ? "캔버스를 만드는 중입니다"
      : snapshot.state === "Recovering" ? "캔버스를 복구하는 중입니다"
        : "캔버스를 여는 중입니다";
    return e("div", { className: "canvas-empty", role: "status" }, e("strong", null, label));
  }
  if (snapshot.errorCode === "CANVAS_NOT_FOUND") {
    return e("div", { className: "canvas-empty" }, e("strong", null, "아직 캔버스가 없습니다"), e("button", { type: "button", "data-action": "create-canvas", onClick: callbacks.onCanvasCreate }, "새 캔버스 만들기"));
  }
  if (snapshot.state === "RecoveryRequired") {
    const error = mapUserFacingError({ stableCode: snapshot.errorCode ?? "CANVAS_RECOVERY_REQUIRED", retryable: snapshot.retryable ?? false, operationContext: "canvas" });
    return e("div", { className: "canvas-empty", role: "alert" }, e("strong", null, error.title), e("p", null, error.message), e("button", { type: "button", "data-action": "recover-canvas", onClick: callbacks.onCanvasRecover }, "캔버스 복구"));
  }
  if (snapshot.state === "Conflict") {
    return e("div", { className: "canvas-empty", role: "alert" }, e("strong", null, "다른 변경이 먼저 저장되었습니다"), e("button", { type: "button", "data-action": "retry-canvas", onClick: callbacks.onCanvasRetry }, "최신 상태 다시 불러오기"));
  }
  const error = mapUserFacingError({ stableCode: snapshot.errorCode ?? "COMMAND_BRIDGE_FAILED", retryable: snapshot.retryable ?? true, operationContext: "canvas" });
  return e("div", { className: "canvas-empty", role: "alert" }, e("strong", null, error.title), e("p", null, error.message), error.recoveryAction === "retry" ? e("button", { type: "button", "data-action": "retry-canvas", onClick: callbacks.onCanvasRetry }, error.recoveryLabel) : null);
}

function DesktopAttachments({
  model,
  snapshot,
  callbacks,
}: {
  readonly model: PersonalWorkspaceHomeModel;
  readonly snapshot: DesktopAssetSurfaceSnapshot;
  readonly callbacks: DesktopExplorationCallbacks & {
    readonly onAssetSelect: (assetId: string) => void;
    readonly onAssetRetry: () => void;
    readonly onAssetImport: () => void;
    readonly onAssetWorkspace: () => void;
    readonly onAssetLink: () => void;
    readonly onAssetUnlink: () => void;
    readonly onAssetCancel: () => void;
    readonly onAssetPreview: () => void;
    readonly onAssetPreviewClose: () => void;
  };
}): React.ReactElement {
  const e = React.createElement;
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState("전체 파일");
  const assets = snapshot.page?.assets ?? [];
  const selected = assets.find((asset) => asset.assetId === snapshot.selectedAssetId);
  const documentReferences: readonly DisplayReference[] = model.recentDocuments.map((document) => Object.freeze({
    category: "document" as const, identity: document.documentId, label: document.title,
    breadcrumbLabel: document.path.split("/").slice(0, -1).join(" / "), statusLabel: "", state: "resolved" as const,
  }));
  const selectedPresentation = selected ? presentAssetMetadata({
    mediaType: selected.mediaType,
    byteSize: selected.byteSize,
    status: selected.status,
    previewCapability: snapshot.detail?.previewCapability,
    extractionStatus: snapshot.detail?.extractionStatus,
  }) : undefined;
  const currentDocumentLabel = documentReferences.find((reference) => reference.identity === snapshot.documentId)?.label
    ?? (snapshot.documentId ? "선택한 문서" : "연결 없음");
  const filtered = useMemo(() => assets.filter((asset) => {
    if (!asset.fileName.toLocaleLowerCase().includes(query.toLocaleLowerCase())) return false;
    if (filter === "이미지") return asset.mediaType.startsWith("image/");
    if (filter === "PDF") return asset.mediaType === "application/pdf";
    if (filter === "문서") return /text|word|document/.test(asset.mediaType);
    if (filter === "기타") return !/image|pdf|text|word|document/.test(asset.mediaType);
    return true;
  }), [assets, filter, query]);

  return surfaceShell(
    "assets",
    snapshot.state,
    snapshot.generation,
    model,
    callbacks,
    e(
      "main",
      {
        className: "desktop-main assets-main",
        "data-asset-scope": snapshot.scope ?? "Unknown",
        "data-asset-import-state": snapshot.importState,
        "data-asset-detail-state": snapshot.detailState ?? "Idle",
        "data-asset-mutation-state": snapshot.mutationState ?? "Idle",
        "data-selected-asset-id": snapshot.selectedAssetId ?? "",
        "data-asset-reference-count": snapshot.detail?.referenceCount ?? -1,
        "data-asset-preview-state": snapshot.previewState ?? "Idle",
        "data-asset-filter": filter,
      },
      e(
      "header",
      { className: "assets-heading" },
      e("div", null, e("h1", null, "첨부 파일"), e("p", null, "문서와 연결된 파일을 한곳에서 확인하고 정리합니다.")),
        e(
          "div",
          { className: "assets-heading-actions" },
          snapshot.documentId ? e("span", { className: "surface-count" }, `${assets.length}개 파일`) : null,
          snapshot.scope === "Document" ? e("button", { type: "button", "data-action": "open-asset-library", onClick: callbacks.onAssetWorkspace }, "전체 보관함") : null,
          e(
            "button",
            {
              type: "button",
              className: "primary asset-import-button",
              "data-action": "import-asset",
              disabled: !snapshot.documentId || snapshot.importState === "Selecting" || snapshot.importState === "Importing",
              onClick: callbacks.onAssetImport,
            },
            snapshot.importState === "Selecting" ? "파일 선택 중" : snapshot.importState === "Importing" ? "가져오는 중" : "+ 파일 추가",
          ),
        ),
      ),
      snapshot.importState === "Completed" ? e("p", { className: "asset-operation-status", role: "status" }, "파일이 이 문서에 저장되었습니다.") : null,
      snapshot.importState === "Importing" && snapshot.importOperationId ? e("div", { className: "asset-operation-status", role: "status" }, e("span", null, "파일을 가져오는 중입니다."), e("button", { type: "button", "data-action": "cancel-asset-import", onClick: callbacks.onAssetCancel }, "취소")) : null,
      snapshot.importState === "Cancelled" ? e("p", { className: "asset-operation-status", role: "status" }, "파일 가져오기를 취소했습니다.") : null,
      snapshot.importState === "Failed" ? e("div", { className: "asset-operation-status failed", role: "alert" }, e("span", null, "파일을 저장하지 못했습니다."), e("button", { type: "button", "data-action": "import-asset", onClick: callbacks.onAssetImport }, "다시 시도")) : null,
      e("div", { className: "asset-controls" }, e("label", { className: "asset-search" }, e("span", { "aria-hidden": "true" }, "⌕"), e("input", { type: "search", "data-action": "search-assets", placeholder: "파일명으로 검색", value: query, onChange: (event: React.ChangeEvent<HTMLInputElement>) => setQuery(event.currentTarget.value) })), e("div", { className: "asset-filters", role: "group", "aria-label": "파일 형식 필터" }, [["전체 파일", "all"], ["이미지", "image"], ["PDF", "pdf"], ["문서", "document"], ["기타", "other"]].map(([label, id]) => e("button", { key: id, type: "button", "data-action": `filter-assets-${id}`, className: filter === label ? "active" : "", "aria-pressed": filter === label, onClick: () => setFilter(label) }, label)))),
      e(
        "div",
        { className: "assets-layout" },
        e(
          "section",
          { className: "asset-library", "aria-label": "파일 목록" },
          e("div", { className: "asset-table-header" }, e("span", null, "파일"), e("span", null, "연결 문서"), e("span", null, "상태"), e("span", null, "크기")),
          filtered.map((asset) => { const presentation = presentAssetMetadata({ mediaType: asset.mediaType, byteSize: asset.byteSize, status: asset.status }); return e("button", { key: asset.assetId, type: "button", "data-action": "select-asset", "data-asset-id": asset.assetId, className: `asset-row${snapshot.selectedAssetId === asset.assetId ? " selected" : ""}`, onClick: () => callbacks.onAssetSelect(asset.assetId) }, e("span", { className: "asset-file" }, e("i", null), e("strong", null, asset.fileName), e("small", null, presentation.mediaTypeLabel)), e("span", null, currentDocumentLabel), e("span", null, presentation.statusLabel), e("span", null, presentation.sizeLabel)); }),
          snapshot.state === "Loading" ? e("div", { className: "asset-empty", role: "status" }, e("strong", null, "첨부 파일을 불러오는 중입니다")) : null,
          snapshot.state === "Failed" ? e("div", { className: "asset-empty", role: "alert" }, e("strong", null, "첨부 파일을 불러오지 못했습니다"), e("button", { type: "button", "data-action": "retry-assets", onClick: callbacks.onAssetRetry }, "다시 시도")) : null,
          snapshot.state === "Empty" ? e("div", { className: "asset-empty" }, e("strong", null, query ? "일치하는 파일이 없습니다" : snapshot.scope === "Workspace" ? "보관함에 파일이 없습니다" : "이 문서에 연결된 파일이 없습니다")) : null,
          snapshot.state === "Ready" && filtered.length === 0 ? e("div", { className: "asset-empty" }, e("strong", null, "일치하는 파일이 없습니다")) : null,
        ),
        e(
          "aside",
          { className: "asset-inspector", "aria-label": "파일 세부 정보" },
          selected
            ? e(React.Fragment, null, e("div", { className: `asset-preview capability-${snapshot.detail?.previewCapability ?? "loading"}` }, e("span", null, selectedPresentation?.previewLabel ?? selectedPresentation?.mediaTypeLabel)), e("h2", null, selected.fileName), e("p", null, selected.label), detailRow("형식", selectedPresentation?.mediaTypeLabel ?? "파일"), detailRow("크기", selectedPresentation?.sizeLabel ?? "0 B"), detailRow("버전", snapshot.detail ? `${snapshot.detail.version}` : "불러오는 중"), detailRow("미리보기", selectedPresentation?.previewLabel ?? "미리보기 확인 필요"), snapshot.detail ? e("button", { type: "button", "data-action": "open-asset-preview", disabled: snapshot.previewState === "Loading", onClick: callbacks.onAssetPreview }, snapshot.previewState === "Loading" ? "미리보기 로딩 중" : "미리보기 열기") : null, detailRow("추출 상태", selectedPresentation?.extractionLabel ?? "추출 상태 확인 필요"), detailRow("연결 문서", snapshot.detail ? `${snapshot.detail.referenceCount}개` : currentDocumentLabel), snapshot.detail ? linkedDocumentActions(snapshot.detail.linkedDocumentIds, callbacks.onOpenDocument, documentReferences) : null, detailRow("상태", selectedPresentation?.statusLabel ?? "상태 확인 필요"), snapshot.scope === "Workspace" && snapshot.documentId ? e("button", { type: "button", className: "asset-unlink-button", "data-action": "link-asset", disabled: snapshot.mutationState === "Linking" || snapshot.detail?.linkedDocumentIds.includes(snapshot.documentId), onClick: callbacks.onAssetLink }, snapshot.detail?.linkedDocumentIds.includes(snapshot.documentId) ? "이미 연결됨" : snapshot.mutationState === "Linking" ? "연결 중" : "이 문서에 연결") : null, snapshot.scope === "Document" ? e("button", { type: "button", className: "asset-unlink-button", "data-action": "unlink-asset", disabled: snapshot.mutationState === "Unlinking", onClick: callbacks.onAssetUnlink }, snapshot.mutationState === "Unlinking" ? "연결 해제 중" : "이 문서에서 연결 해제") : null)
            : e(React.Fragment, null, e("div", { className: "asset-preview empty" }, "파일"), e("h2", null, "파일을 선택하세요"), e("p", null, "선택한 파일의 메타데이터를 표시합니다."), detailRow("연결 문서", currentDocumentLabel), detailRow("상태", "선택되지 않음")),
        ),
      ),
      snapshot.previewState && !["Idle", "Loading"].includes(snapshot.previewState) ? assetPreviewDialog(snapshot, callbacks.onAssetPreview, callbacks.onAssetPreviewClose) : null,
    ),
  );
}

function assetPreviewDialog(snapshot: DesktopAssetSurfaceSnapshot, retry: () => void, close: () => void): React.ReactElement {
  const e = React.createElement;
  const closeAndRestore = createFocusRestoringModalAction(close, browserModalFocusEnvironment("open-asset-preview"));
  const content = snapshot.previewState === "Ready" && snapshot.preview?.presentation === "text"
    ? e("pre", { className: "asset-preview-text", "data-asset-preview-presentation": "text" }, snapshot.preview.content)
    : snapshot.previewState === "Ready" && snapshot.preview?.presentation === "data_url" && snapshot.preview.capability === "image"
      ? e("img", { src: snapshot.preview.content, alt: "선택한 첨부 파일 미리보기", "data-asset-preview-presentation": "image" })
      : snapshot.previewState === "Ready" && snapshot.preview?.presentation === "data_url"
        ? e("iframe", { src: snapshot.preview.content, title: "선택한 첨부 파일 미리보기", "data-asset-preview-presentation": "pdf" })
        : e("p", { role: snapshot.previewState === "Failed" ? "alert" : "status" }, snapshot.previewState === "Unsupported" ? "이 파일 형식은 미리보기를 지원하지 않습니다." : "미리보기를 불러오지 못했습니다.");
  return e("div", { role: "dialog", "aria-modal": "true", "aria-label": "첨부 파일 미리보기", className: "asset-preview-dialog", onKeyDown: (event: React.KeyboardEvent<HTMLDivElement>) => handleModalKeyboard(event, closeAndRestore) }, content, snapshot.previewState === "Failed" ? e("button", { type: "button", "data-action": "retry-asset-preview", onClick: retry }, "다시 시도") : null, e("button", { type: "button", "data-action": "close-asset-preview", onClick: closeAndRestore }, "닫기"));
}

function linkedDocumentActions(
  documentIds: readonly string[],
  onOpenDocument: (documentId: string) => void,
  references: readonly DisplayReference[] = [],
): React.ReactElement {
  const e = React.createElement;
  const visibleDocuments = presentLinkedDocuments(documentIds, references).slice(0, 20);
  return e(
    "div",
    { className: "asset-linked-documents", "aria-label": "연결된 문서" },
    visibleDocuments.length === 0
      ? e("p", { className: "asset-linked-documents-empty" }, "연결된 문서가 없습니다")
      : visibleDocuments.map((document) => e(
        "button",
        {
          key: document.identity,
          type: "button",
          "data-action": "open-linked-document",
          "data-linked-document-id": document.identity,
          onClick: () => onOpenDocument(document.identity),
        },
        document.label,
      )),
    documentIds.length > visibleDocuments.length
      ? e("small", null, `외 ${documentIds.length - visibleDocuments.length}개`)
      : null,
  );
}

function surfaceShell(
  active: ExplorationSurface,
  state: string,
  generation: number,
  model: PersonalWorkspaceHomeModel,
  callbacks: DesktopExplorationCallbacks,
  content: React.ReactElement,
): React.ReactElement {
  const routes: readonly WorkspaceShellRouteKind[] = ["Home", "Search", "Document", "Graph", "Canvas", "Assets", "Backup"];
  const route = ({ graph: "Graph", canvas: "Canvas", assets: "Assets" } as const)[active];
  return createWorkspaceShellElement({
    model: createWorkspaceShellModel({ route, availableActions: routes, messages: KO_KR_MESSAGES }),
    messages: KO_KR_MESSAGES,
    routeActions: { Home: callbacks.onHome, Search: callbacks.onSearch, Document: callbacks.onDocument, Graph: callbacks.onGraph, Canvas: callbacks.onCanvas, Assets: callbacks.onAssets, Backup: callbacks.onBackup },
    rootClassName: `exploration-shell ${active}-shell`,
    rootAttributes: { "data-exploration-surface": active, "data-exploration-state": state, "data-exploration-generation": String(generation) },
    onSearch: callbacks.onSearch,
    onCreateDocument: callbacks.onCreateDocument,
    documentShortcuts: model.recentDocuments.slice(0, 5).map((document) => ({ label: document.title, actionId: "open-sidebar-document", onOpen: () => callbacks.onOpenDocument(document.documentId) })),
    savedStatus: "로컬 작업 공간",
    content,
  });
}

function graphPosition(index: number, count: number): { readonly x: number; readonly y: number } {
  const angle = ((Math.PI * 2) / Math.max(count, 1)) * index - Math.PI / 2;
  return { x: 500 + Math.cos(angle) * 310, y: 310 + Math.sin(angle) * 210 };
}

function detailRow(label: string, value: string): React.ReactElement {
  return React.createElement("div", { className: "asset-detail-row" }, React.createElement("span", null, label), React.createElement("strong", null, value));
}
