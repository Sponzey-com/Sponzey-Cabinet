import React from "react";

import type { PersonalWorkspaceHomeModel } from "@sponzey-cabinet/ui";
import { createWorkspaceShellModel, type WorkspaceShellRouteKind } from "./workspace_shell_contract.ts";
import {
  createWorkspaceShellElement,
  type WorkspaceShellDocumentShortcut,
} from "./react_workspace_shell.ts";
import { KO_KR_MESSAGES } from "./ko_kr_catalog.ts";
import { mapUserFacingError } from "./user_facing_error_presenter.ts";
import type { DesktopGraphSurfaceSnapshot } from "./desktop_graph_controller.ts";
import { presentGraphNodes, type GraphDisplayNode } from "./graph_display_presenter.ts";

export interface DesktopWorkspaceHomeElementOptions {
  readonly documentShortcuts?: readonly WorkspaceShellDocumentShortcut[];
  readonly onRetry?: () => void;
  readonly onCreateDocument?: () => void;
  readonly onOpenSearchOverlay?: () => void;
  readonly onOpenNavigator?: (query?: string) => void;
  readonly onResumeDocument?: () => void;
  readonly onOpenGraph?: () => void;
  readonly onOpenCanvas?: () => void;
  readonly onOpenAssets?: () => void;
  readonly onOpenBackup?: () => void;
  readonly onOpenDocument?: (documentId: string) => void;
  readonly knowledgeGraph?: DesktopGraphSurfaceSnapshot;
  readonly onRetryKnowledgeGraph?: () => void;
}

const shellRoutes: readonly WorkspaceShellRouteKind[] = ["Home", "Search", "Document", "Graph", "Canvas", "Assets", "Backup"];

export function createDesktopWorkspaceHomeElement(
  model: PersonalWorkspaceHomeModel,
  options: DesktopWorkspaceHomeElementOptions = {},
): React.ReactElement {
  const e = React.createElement;
  return createWorkspaceShellElement({
    model: createWorkspaceShellModel({ route: "Home", availableActions: shellRoutes, messages: KO_KR_MESSAGES }),
    messages: KO_KR_MESSAGES,
    routeActions: { Search: options.onOpenNavigator, Document: options.onResumeDocument, Graph: options.onOpenGraph, Canvas: options.onOpenCanvas, Assets: options.onOpenAssets, Backup: options.onOpenBackup },
    rootAttributes: { "data-cabinet-home-state": model.displayState },
    onCreateDocument: options.onCreateDocument,
    onSearchOpen: options.onOpenSearchOverlay,
    onSearch: options.onOpenNavigator,
    documentShortcuts: options.documentShortcuts,
    savedStatus: model.displayState === "Degraded" ? "확인 필요" : "모든 변경 저장됨",
    mainClassName: "desktop-main home-main",
    content: e(
      React.Fragment,
      null,
      renderStateBanner(model, options),
      model.displayState !== "Failed" && model.displayState !== "Loading"
        ? e(
            "div",
            { className: "home-dashboard" },
            e(
              "div",
              { className: "home-primary-column" },
              e(
                "section",
                { className: "home-welcome", "aria-labelledby": "home-title" },
                e("h1", { id: "home-title" }, "좋은 오후예요"),
                e("p", null, "필요한 문서를 바로 열거나, 관계를 따라 탐색하세요."),
              ),
              renderContinueSection(model, options),
              renderRecentDocuments(model, options),
              renderWorkspaceDetails(model),
            ),
            e(
              "aside",
              { className: "home-secondary-column", "aria-label": "작업 공간 요약" },
              renderKnowledgeMap(model, options),
              renderToday(model),
            ),
          )
        : null,
    ),
  });
}

function renderStateBanner(
  model: PersonalWorkspaceHomeModel,
  options: DesktopWorkspaceHomeElementOptions,
): React.ReactElement {
  const e = React.createElement;
  if (model.displayState === "Loading") {
    return e("div", { className: "state-banner home-state-banner", "aria-live": "polite" }, "작업 공간을 불러오는 중입니다");
  }
  if (model.displayState === "Failed") {
    const error = mapUserFacingError({
      stableCode: model.error?.code ?? "COMMAND_BRIDGE_FAILED",
      retryable: model.error?.retryable ?? false,
      operationContext: "workspace_home",
    });
    return e(
      "div",
      { className: "state-banner failed home-state-banner", role: "alert" },
      e("strong", null, error.title),
      e("span", null, error.message),
      error.recoveryAction === "retry"
        ? e("button", { type: "button", "data-action": "retry-workspace-home", onClick: options.onRetry }, error.recoveryLabel)
        : null,
    );
  }
  if (model.displayState === "Degraded") {
    return e("div", { className: "state-banner degraded home-state-banner", "aria-live": "polite" }, "읽기 전용 복구 모드입니다");
  }
  return e("div", { className: "state-banner visually-hidden", "aria-live": "polite" }, "작업 공간을 사용할 수 있습니다");
}

function renderContinueSection(
  model: PersonalWorkspaceHomeModel,
  options: DesktopWorkspaceHomeElementOptions,
): React.ReactElement {
  const e = React.createElement;
  const documents = model.recentDocuments.slice(0, 1);
  return e(
    "section",
    { className: "dashboard-section continue-section", "aria-labelledby": "continue-title" },
    sectionHeading("continue-title", "이어서 작업하기", "모두 보기", "home-open-all-documents", options.onOpenNavigator),
    documents.length > 0
      ? e(
          "div",
          { className: "continue-grid" },
          documents.map((document, index) =>
            e(
              "button",
              {
                key: document.documentId,
                type: "button",
                  className: `continue-card accent-${index % 3}`,
                "data-action": "open-recent-document",
                "data-document-id": document.documentId,
                "data-document-title": document.title,
                onClick: options.onOpenDocument
                  ? () => options.onOpenDocument?.(document.documentId)
                  : undefined,
              },
              e("i", { "aria-hidden": "true" }),
              e("strong", null, document.title),
              e("span", null, continueDocumentLabel(index)),
              e("small", null, index === 0 ? "방금 전" : index === 1 ? "어제" : "최근"),
            ),
          ),
        )
      : e(
          "button",
          {
            type: "button",
            className: "empty-action continue-empty",
            "data-action": "new-document",
            onClick: options.onCreateDocument,
            disabled: !options.onCreateDocument,
          },
          "첫 문서 만들기",
          e("span", { className: "visually-hidden" }, "새 문서 만들기"),
        ),
  );
}

function renderRecentDocuments(
  model: PersonalWorkspaceHomeModel,
  options: DesktopWorkspaceHomeElementOptions,
): React.ReactElement {
  const e = React.createElement;
  return e(
    "section",
    { className: "dashboard-section recent-documents", "aria-labelledby": "recent-title" },
    sectionHeading("recent-title", "최근 문서"),
    model.recentDocuments.length === 0
      ? e("p", { className: "empty-label" }, "아직 작성한 문서가 없습니다.")
      : e(
          "ul",
          { className: "recent-document-list" },
          model.recentDocuments.slice(0, 5).map((document, index) =>
            e(
              "li",
              { key: document.documentId },
              e(
                "button",
                {
                  type: "button",
                  className: "document-row",
                  "data-action": "open-recent-document",
                  "data-document-id": document.documentId,
                  "data-document-title": document.title,
                  onClick: options.onOpenDocument
                    ? () => options.onOpenDocument?.(document.documentId)
                    : undefined,
                },
                e("i", { className: `document-color accent-${index % 4}`, "aria-hidden": "true" }),
              e("span", { className: "document-row-copy" }, e("strong", null, document.title), e("small", null, recentDocumentLabel(index))),
                e("span", { className: "document-arrow", "aria-hidden": "true" }, "›"),
              ),
            ),
          ),
        ),
  );
}

function renderWorkspaceDetails(model: PersonalWorkspaceHomeModel): React.ReactElement {
  const e = React.createElement;
  const summary = model.workspaceSummary;
  return e(
    "section",
    { className: "workspace-detail-strip", "aria-label": "작업 공간 상태" },
    e("span", null, e("strong", null, "문서"), ` ${summaryValue(summary, "Documents", summary.documentCount)}`),
    e("span", null, e("strong", null, "첨부"), ` ${summaryValue(summary, "Assets", summary.assetCount)}`),
    e("span", null, e("strong", null, "Canvas"), ` ${summaryValue(summary, "Canvases", summary.canvasCount)}`),
    e("span", null, e("strong", null, "백업 상태"), ` ${backupStatus(model)}`),
    model.recentChanges.map((change) => e("span", { key: change.documentId, className: "visually-hidden" }, change.summary)),
  );
}

function summaryValue(
  summary: PersonalWorkspaceHomeModel["workspaceSummary"],
  kind: "Documents" | "Assets" | "Canvases",
  value: number,
): string {
  return summary.unavailable.includes(kind) ? "확인 필요" : String(value);
}

function renderKnowledgeMap(
  _model: PersonalWorkspaceHomeModel,
  options: DesktopWorkspaceHomeElementOptions,
): React.ReactElement {
  const e = React.createElement;
  const snapshot = options.knowledgeGraph;
  const content = renderKnowledgeMapContent(snapshot, options);
  return e(
    "section",
    { className: "overview-card knowledge-map-card", "aria-labelledby": "knowledge-map-title" },
    sectionHeading("knowledge-map-title", "내 지식 지도", "전체 화면", "home-open-graph", options.onOpenGraph),
    content,
  );
}

const compactMapPositions = Object.freeze([
  Object.freeze({ x: 50, y: 50 }),
  Object.freeze({ x: 20, y: 24 }),
  Object.freeze({ x: 79, y: 22 }),
  Object.freeze({ x: 84, y: 72 }),
  Object.freeze({ x: 48, y: 82 }),
  Object.freeze({ x: 14, y: 67 }),
]);

function renderKnowledgeMapContent(
  snapshot: DesktopGraphSurfaceSnapshot | undefined,
  options: DesktopWorkspaceHomeElementOptions,
): React.ReactElement {
  const e = React.createElement;
  if (!snapshot || snapshot.state === "Idle" || snapshot.state === "Loading" || snapshot.state === "Repairing") {
    return e("div", { className: "knowledge-map map-status", role: "status" }, "지식 지도를 불러오는 중입니다.");
  }
  if (snapshot.state === "Failed") {
    return e(
      "div",
      { className: "knowledge-map map-status", role: "alert" },
      e("strong", null, "지식 지도를 불러오지 못했습니다."),
      options.onRetryKnowledgeGraph
        ? e("button", { type: "button", className: "secondary", "data-action": "retry-home-knowledge-map", onClick: options.onRetryKnowledgeGraph }, "다시 시도")
        : null,
    );
  }
  const graph = snapshot.graph;
  if (!graph || graph.nodes.length === 0) {
    return e("div", { className: "knowledge-map map-status", role: "status" }, "아직 표시할 문서 관계가 없습니다.");
  }

  const compact = selectCompactKnowledgeMap(presentGraphNodes(graph.nodes), graph.edges);
  const positionByIdentity = new Map(compact.nodes.map((node, index) => [node.identity, compactMapPositions[index]]));
  return e(
    "div",
    {
      className: "knowledge-map",
      "aria-label": "문서 관계 미리보기",
      "data-knowledge-map-source": "authoritative-graph",
    },
    e(
      "svg",
      { className: "knowledge-map-edges", viewBox: "0 0 100 100", preserveAspectRatio: "none", "aria-hidden": "true" },
      compact.edges.map((edge) => {
        const source = positionByIdentity.get(edge.sourceId);
        const target = positionByIdentity.get(edge.targetId);
        return source && target
          ? e("line", { key: edge.id, x1: source.x, y1: source.y, x2: target.x, y2: target.y })
          : null;
      }),
    ),
    compact.nodes.map((node, index) => renderKnowledgeMapNode(node, index, options)),
    e("p", { className: "knowledge-map-summary" }, `${graph.nodes.length}개 항목 · ${graph.edges.length}개 연결`),
    snapshot.state === "Stale"
      ? e("p", { className: "knowledge-map-stale", role: "status" }, "일부 관계를 갱신하는 중입니다.")
      : null,
  );
}

function renderKnowledgeMapNode(
  node: GraphDisplayNode,
  index: number,
  options: DesktopWorkspaceHomeElementOptions,
): React.ReactElement {
  const e = React.createElement;
  const position = compactMapPositions[index];
  const props = {
    className: `map-node compact-map-node kind-${node.kind}${index === 0 ? " center" : ""}`,
    style: { left: `${position.x}%`, top: `${position.y}%` },
    title: node.label,
  };
  const content = e(React.Fragment, null, e("i", { "aria-hidden": "true" }), e("small", null, node.label));
  if (node.kind === "document" && node.canNavigate && options.onOpenDocument) {
    return e(
      "button",
      {
        ...props,
        key: node.identity,
        type: "button",
        "data-action": "open-home-graph-document",
        onClick: () => options.onOpenDocument?.(node.identity),
      },
      content,
    );
  }
  return e("span", { ...props, key: node.identity }, content);
}

function selectCompactKnowledgeMap(
  nodes: readonly GraphDisplayNode[],
  edges: readonly { readonly id: string; readonly sourceId: string; readonly targetId: string }[],
): {
  readonly nodes: readonly GraphDisplayNode[];
  readonly edges: readonly { readonly id: string; readonly sourceId: string; readonly targetId: string }[];
} {
  const degree = new Map<string, number>();
  for (const edge of edges) {
    degree.set(edge.sourceId, (degree.get(edge.sourceId) ?? 0) + 1);
    degree.set(edge.targetId, (degree.get(edge.targetId) ?? 0) + 1);
  }
  const selected = nodes
    .map((node, index) => ({ node, index, degree: degree.get(node.identity) ?? 0 }))
    .sort((left, right) => right.degree - left.degree || left.index - right.index)
    .slice(0, compactMapPositions.length)
    .map((item) => item.node);
  const identities = new Set(selected.map((node) => node.identity));
  return Object.freeze({
    nodes: Object.freeze(selected),
    edges: Object.freeze(edges.filter((edge) => identities.has(edge.sourceId) && identities.has(edge.targetId))),
  });
}

function renderToday(model: PersonalWorkspaceHomeModel): React.ReactElement {
  const e = React.createElement;
  const tasks = model.unfinishedItems.slice(0, 3);
  return e(
    "section",
    { className: "today-section", "aria-labelledby": "today-title" },
    e("h2", { id: "today-title" }, "오늘"),
    e(
      "ul",
      { className: "today-list" },
      e("li", null, e("i", { className: "tone-teal" }), e("strong", null, "백업 확인"), e("span", null, backupStatus(model))),
      ...(tasks.length > 0
        ? tasks.map((task, index) =>
            e("li", { key: task.documentId }, e("i", { className: index % 2 ? "tone-blue" : "tone-amber" }), e("strong", null, task.label), e("span", null, "진행 중")),
          )
        : [e("li", { key: "clear" }, e("i", { className: "tone-blue" }), e("strong", null, "정리된 하루"), e("span", null, "완료"))]),
    ),
  );
}

function sectionHeading(
  id: string,
  title: string,
  action?: string,
  actionId?: string,
  onAction?: () => void,
): React.ReactElement {
  const e = React.createElement;
  return e(
    "div",
    { className: "section-heading" },
    e("h2", { id }, title),
    action ? e("button", { type: "button", className: "text-action", "data-action": actionId, onClick: onAction, disabled: !onAction }, action) : null,
  );
}

function continueDocumentLabel(index: number): string {
  if (index === 0) return "바로 이어서 작업";
  if (index === 1) return "최근에 열었음";
  return "최근 문서";
}

function recentDocumentLabel(index: number): string {
  if (index === 0) return "최근에 열었음";
  return "최근 문서";
}

function backupStatus(model: PersonalWorkspaceHomeModel): string {
  const status = model.sections.find((item) => item.id === "backup-status")?.status;
  const labels: Readonly<Record<string, string>> = Object.freeze({
    Fresh: "최신 백업 있음",
    NeverCreated: "아직 없음",
    Stale: "새 백업 필요",
    Failed: "확인 필요",
    Creating: "만드는 중",
  });
  return status ? labels[status] ?? "확인 필요" : "아직 없음";
}
