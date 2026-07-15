import React from "react";

import type { DocumentNavigatorView } from "@sponzey-cabinet/client-core";
import type { DocumentNavigatorModel } from "@sponzey-cabinet/ui";
import { createWorkspaceShellModel, type WorkspaceShellRouteKind } from "./workspace_shell_contract.ts";
import { createWorkspaceShellElement } from "./react_workspace_shell.ts";
import { KO_KR_MESSAGES } from "./ko_kr_catalog.ts";
import { mapUserFacingError } from "./user_facing_error_presenter.ts";
import { presentDocumentLocation } from "./document_location_presenter.ts";

const shellRoutes: readonly WorkspaceShellRouteKind[] = ["Home", "Search", "Document", "Graph", "Canvas", "Assets", "Backup"];

export interface DesktopDocumentNavigatorCallbacks {
  readonly onCreateDocument: () => void;
  readonly onHome: () => void;
  readonly onView: (view: DocumentNavigatorView, viewKey?: string) => void;
  readonly onFilter: (filter: string) => void;
  readonly onRetry: () => void;
  readonly onOpenDocument: (documentId: string) => void;
  readonly onGraph?: () => void;
  readonly onCanvas?: () => void;
  readonly onAssets?: () => void;
  readonly onBackup?: () => void;
}

const navigatorViews: readonly DocumentNavigatorView[] = [
  "Tree",
  "Collection",
  "Tag",
  "Recent",
  "Favorite",
];

const viewLabels: Readonly<Record<DocumentNavigatorView, string>> = {
  Tree: "전체",
  Collection: "문서",
  Tag: "태그",
  Recent: "최근",
  Favorite: "즐겨찾기",
};

export function createDesktopDocumentNavigatorElement(
  model: DocumentNavigatorModel,
  callbacks: DesktopDocumentNavigatorCallbacks,
): React.ReactElement {
  const e = React.createElement;
  return createWorkspaceShellElement({
    model: createWorkspaceShellModel({ route: "Search", availableActions: shellRoutes, messages: KO_KR_MESSAGES }),
    messages: KO_KR_MESSAGES,
    routeActions: { Home: callbacks.onHome, Document: () => callbacks.onView("Tree"), Graph: callbacks.onGraph, Canvas: callbacks.onCanvas, Assets: callbacks.onAssets, Backup: callbacks.onBackup },
    rootAttributes: { "data-cabinet-navigator-state": model.displayState },
    rootClassName: "navigator-shell",
    onCreateDocument: callbacks.onCreateDocument,
    searchActionId: "navigator-search-field",
    documentShortcuts: model.items.slice(0, 2).map((item) => ({ label: item.title, actionId: "open-sidebar-document", onOpen: () => callbacks.onOpenDocument(item.documentId) })),
    mainClassName: "desktop-main search-main",
    content: e(
      React.Fragment,
      null,
      e(
        "section",
        { className: "search-heading", "aria-labelledby": "navigator-title" },
        e("h1", { id: "navigator-title" }, "검색과 발견"),
        e("p", null, "제목, 본문, 태그와 첨부 내용을 한 번에 찾습니다."),
      ),
      e(
        "label",
        { className: "search-query-field" },
        e("span", { "aria-hidden": "true" }, "⌕"),
        e("input", {
          type: "search",
          "data-action": "navigator-search-field",
          value: model.filter ?? "",
          placeholder: "로컬 저장소와 백업",
          "aria-label": "문서 검색어",
          onChange: (event: React.ChangeEvent<HTMLInputElement>) => callbacks.onFilter(event.currentTarget.value),
        }),
        e("kbd", null, "Enter"),
      ),
      e(
        "div",
        { className: "search-layout" },
        e(
          "div",
          { className: "search-results-column" },
          e(
            "div",
            { className: "navigator-tabs", role: "tablist", "aria-label": "문서 보기 방식" },
            navigatorViews.map((view) =>
              e(
                "button",
                {
                  key: view,
                  type: "button",
                  role: "tab",
                  "data-action": `navigator-view-${view.toLowerCase()}`,
                  "aria-selected": model.view === view,
                  "data-navigator-view": view,
                  className: model.view === view ? "active" : undefined,
                  onClick: () => callbacks.onView(view, defaultViewKey(view)),
                },
                viewLabels[view],
              ),
            ),
          ),
          model.view === "Collection" || model.view === "Tag"
            ? e("p", { className: "navigator-scope" }, `${viewLabels[model.view]}: ${model.viewKey ?? defaultViewKey(model.view)}`)
            : null,
          renderNavigatorState(model, callbacks),
        ),
      ),
    ),
  });
}

function renderNavigatorState(
  model: DocumentNavigatorModel,
  callbacks: DesktopDocumentNavigatorCallbacks,
): React.ReactElement {
  const e = React.createElement;
  if (model.displayState === "Loading" || model.displayState === "Filtering") {
    return e("div", { className: "state-banner", "aria-live": "polite" }, "문서를 불러오는 중입니다");
  }
  if (model.displayState === "Failed") {
    const error = mapUserFacingError({ stableCode: model.error?.code ?? "COMMAND_BRIDGE_FAILED", retryable: model.error?.retryable ?? false, operationContext: "navigator" });
    return e("div", { className: "state-banner failed", role: "alert" }, e("strong", null, error.title), e("span", null, error.message), error.recoveryAction === "retry" ? e("button", { type: "button", "data-action": "retry-navigator", onClick: callbacks.onRetry }, error.recoveryLabel) : null);
  }
  if (model.displayState === "Degraded") {
    return e("div", { className: "state-banner degraded", "aria-live": "polite" }, "문서 목록을 확인해야 합니다");
  }
  if (model.displayState === "EmptyResult") {
    return e("div", { className: "navigator-empty", "aria-live": "polite" }, "조건에 맞는 문서가 없습니다");
  }
  return e(
    "section",
    { className: "navigator-results", "aria-label": "문서 검색 결과" },
    e("h2", null, `${model.items.length}개의 결과`),
    e(
      "ul",
      { className: "item-list" },
      model.items.map((item, index) =>
        e(
          "li",
          { key: item.documentId },
          e(
            "button",
            { type: "button", className: "document-row search-result-row", "data-action": "open-navigator-document", "data-document-id": item.documentId, onClick: () => callbacks.onOpenDocument(item.documentId) },
            e("i", { className: `document-color accent-${index % 4}` }),
            e("span", { className: "document-row-copy" }, e("strong", null, item.title), e("small", null, [presentDocumentLocation(item.path), item.favorite ? "즐겨찾기" : "", ...item.collections, ...item.tags].filter(Boolean).join(" · "))),
            e("span", { className: "document-arrow", "aria-hidden": "true" }, "›"),
          ),
        ),
      ),
    ),
  );
}

function defaultViewKey(view: DocumentNavigatorView): string | undefined {
  if (view === "Collection") return "work";
  if (view === "Tag") return "rust";
  return undefined;
}
