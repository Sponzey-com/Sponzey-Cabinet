import React from "react";
import { X } from "lucide-react";

import type { DocumentNavigatorView } from "@sponzey-cabinet/client-core";
import type { DocumentNavigatorModel } from "@sponzey-cabinet/ui";
import { createWorkspaceShellModel, type WorkspaceShellRouteKind } from "./workspace_shell_contract.ts";
import {
  createWorkspaceShellElement,
  type WorkspaceShellDocumentShortcut,
} from "./react_workspace_shell.ts";
import { KO_KR_MESSAGES } from "./ko_kr_catalog.ts";
import { mapUserFacingError } from "./user_facing_error_presenter.ts";
import { presentDocumentLocation } from "./document_location_presenter.ts";
import {
  createDesktopSearchResultWindow,
  selectDesktopSearchResultWindow,
  transitionDesktopSearchResultWindow,
  type DesktopSearchResultWindow,
} from "./desktop_search_result_window.ts";
import { presentSearchResultSnippet } from "./search_result_snippet_presenter.ts";
import { presentGlobalSearchOverlay } from "./global_search_overlay_presenter.ts";
import { presentGlobalSearchResultMetadata } from "./global_search_result_metadata_presenter.ts";
import { handleModalKeyboard } from "./modal_keyboard_policy.ts";
import { presentAssetMetadata } from "./asset_display_presenter.ts";

const shellRoutes: readonly WorkspaceShellRouteKind[] = ["Home", "Search", "Document", "Graph", "Canvas", "Assets", "Backup"];
const NAVIGATOR_ICON_PROPS = Object.freeze({ size: 17, strokeWidth: 2, "aria-hidden": true });

export interface DesktopDocumentNavigatorCallbacks {
  readonly onCreateDocument: () => void;
  readonly onHome: () => void;
  readonly onDocument: () => void;
  readonly onView: (view: DocumentNavigatorView, viewKey?: string) => void;
  readonly onSearchOpen?: () => void;
  readonly onFilter: (filter: string) => void;
  readonly onSearchEscape?: (query: string) => void;
  readonly onRetry: () => void;
  readonly onOpenDocument: (documentId: string) => void;
  readonly onOpenAsset?: (assetId: string) => void;
  readonly onPreviousResults?: () => void;
  readonly onNextResults?: () => void;
  readonly onGraph?: () => void;
  readonly onCanvas?: () => void;
  readonly onAssets?: () => void;
  readonly onBackup?: () => void;
}

export interface DesktopDocumentNavigatorOptions {
  readonly resultWindow?: DesktopSearchResultWindow;
  readonly documentShortcuts?: readonly WorkspaceShellDocumentShortcut[];
  readonly searchMetrics?: {
    readonly durationMs?: number;
  };
  readonly assetResults?: readonly DesktopGlobalSearchAssetResult[];
}

export interface DesktopGlobalSearchAssetResult {
  readonly assetId: string;
  readonly label?: string;
  readonly fileName?: string;
  readonly mediaType: string;
  readonly byteSize: number;
  readonly status?: string;
  readonly linkedDocumentTitle?: string;
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
  options: DesktopDocumentNavigatorOptions = {},
): React.ReactElement {
  const e = React.createElement;
  const resultWindow = transitionDesktopSearchResultWindow(
    options.resultWindow ?? createDesktopSearchResultWindow(model.generation, model.items.length),
    { type: "Reconcile", generation: model.generation, total: model.items.length },
  );
  const overlay = presentGlobalSearchOverlay(model);
  return createWorkspaceShellElement({
    model: createWorkspaceShellModel({ route: "Search", availableActions: shellRoutes, messages: KO_KR_MESSAGES }),
    messages: KO_KR_MESSAGES,
    routeActions: { Home: callbacks.onHome, Document: callbacks.onDocument, Graph: callbacks.onGraph, Canvas: callbacks.onCanvas, Assets: callbacks.onAssets, Backup: callbacks.onBackup },
    rootAttributes: {
      "data-cabinet-navigator-state": model.displayState,
      "data-global-search-overlay": "open",
    },
    rootClassName: "navigator-shell",
    onCreateDocument: callbacks.onCreateDocument,
    onSearchOpen: callbacks.onSearchOpen,
    onSearch: (query) => callbacks.onFilter(query ?? ""),
    onSearchEscape: callbacks.onSearchEscape,
    searchValue: model.filter,
    searchAriaLabel: "문서 검색어",
    searchActionId: "navigator-search-field",
    documentShortcuts: options.documentShortcuts,
    mainClassName: "desktop-main search-main",
    content: e(
        "section",
        {
          className: "global-search-overlay",
          role: "dialog",
          "aria-modal": true,
          "aria-labelledby": "navigator-title",
          "data-global-search-state": overlay.state,
          onKeyDown: callbacks.onSearchEscape
            ? (event: React.KeyboardEvent<HTMLElement>) => {
                handleModalKeyboard(event, () => callbacks.onSearchEscape?.(overlay.query));
              }
            : undefined,
        },
        e(
          "div",
          { className: "search-heading global-search-heading" },
          e(
            "div",
            null,
            e("h1", { id: "navigator-title" }, overlay.title),
            e("p", null, overlay.description),
          ),
          e(
            "button",
            {
              type: "button",
              className: "global-search-close",
              "data-action": "close-global-search",
              "aria-label": overlay.closeLabel,
              onClick: callbacks.onSearchEscape ? () => callbacks.onSearchEscape?.(overlay.query) : undefined,
              disabled: !callbacks.onSearchEscape,
            },
            e(X, NAVIGATOR_ICON_PROPS),
          ),
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
            renderNavigatorState(model, callbacks, resultWindow, options.searchMetrics, options.assetResults ?? model.assetResults),
          ),
        ),
    ),
  });
}

function renderNavigatorState(
  model: DocumentNavigatorModel,
  callbacks: DesktopDocumentNavigatorCallbacks,
  resultWindow: DesktopSearchResultWindow,
  searchMetrics?: DesktopDocumentNavigatorOptions["searchMetrics"],
  assetResults: readonly DesktopGlobalSearchAssetResult[] = [],
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
    return e(
      "section",
      { className: "navigator-results empty", "aria-label": "문서 검색 결과" },
      e(
        "div",
        { className: "global-search-empty", "aria-live": "polite" },
        e("strong", null, "검색 결과가 없습니다"),
        e("span", null, model.filter ? "다른 검색어를 입력해 다시 확인하세요." : "문서 제목, 본문, 첨부 파일 이름으로 검색할 수 있습니다."),
      ),
      renderGlobalSearchFooter(model, searchMetrics, 0),
    );
  }
  const window = selectDesktopSearchResultWindow(resultWindow, model.items);
  return e(
    "section",
    { className: "navigator-results", "aria-label": "문서 검색 결과" },
    e(
      "section",
      {
        className: "search-result-group",
        "data-search-result-group": "document",
        "aria-labelledby": "navigator-document-results-title",
      },
      e("h2", { id: "navigator-document-results-title" }, "문서"),
      e(
        "p",
        { className: "search-result-group-summary", "aria-live": "polite" },
        presentGlobalSearchResultMetadata({ documentCount: model.items.length }),
      ),
      model.items.length > resultWindow.pageSize
        ? e(
          "div",
          { className: "search-result-pagination", "aria-label": "검색 결과 범위" },
          e("span", { "aria-live": "polite" }, `${window.start}-${window.end} / ${model.items.length}`),
          e(
            "div",
            null,
            e("button", { type: "button", "data-action": "previous-search-results", onClick: callbacks.onPreviousResults, disabled: !window.canPrevious || !callbacks.onPreviousResults }, "이전"),
            e("button", { type: "button", "data-action": "next-search-results", onClick: callbacks.onNextResults, disabled: !window.canNext || !callbacks.onNextResults }, "다음"),
          ),
        )
        : null,
      e(
        "ul",
        { className: "item-list" },
        window.items.map((item, index) =>
          e(
            "li",
            { key: item.documentId },
            e(
              "button",
              { type: "button", className: "document-row search-result-row", "data-action": "open-navigator-document", "data-document-id": item.documentId, onClick: () => callbacks.onOpenDocument(item.documentId) },
              e("i", { className: `document-color accent-${index % 4}` }),
              e(
                "span",
                { className: "document-row-copy" },
                e("strong", null, item.title),
                presentSearchResultSnippet(item.snippet)
                  ? e("span", { className: "search-result-snippet" }, presentSearchResultSnippet(item.snippet))
                  : null,
                e("small", null, [presentDocumentLocation(item.path), item.favorite ? "즐겨찾기" : "", ...item.collections, ...item.tags].filter(Boolean).join(" · ")),
              ),
              e("span", { className: "document-arrow", "aria-hidden": "true" }, "›"),
            ),
          ),
        ),
      ),
      renderAssetResultGroup(assetResults, callbacks),
      renderGlobalSearchFooter(model, searchMetrics, assetResults.length),
    ),
  );
}

function renderAssetResultGroup(
  assetResults: readonly DesktopGlobalSearchAssetResult[],
  callbacks: DesktopDocumentNavigatorCallbacks,
): React.ReactElement | null {
  if (assetResults.length === 0) return null;
  const e = React.createElement;
  return e(
    "section",
    {
      className: "search-result-group",
      "data-search-result-group": "asset",
      "aria-labelledby": "navigator-asset-results-title",
    },
    e("h2", { id: "navigator-asset-results-title" }, "첨부 파일"),
    e("p", { className: "search-result-group-summary", "aria-live": "polite" }, presentGlobalSearchResultMetadata({ documentCount: 0, assetCount: assetResults.length })),
    e(
      "ul",
      { className: "item-list asset-search-result-list" },
      assetResults.slice(0, 20).map((asset) => {
        const presentation = presentAssetMetadata({
          mediaType: asset.mediaType,
          byteSize: asset.byteSize,
          status: asset.status ?? "available",
        });
        return e(
          "li",
          { key: asset.assetId },
          e(
            "button",
            {
              type: "button",
              className: "document-row search-result-row asset-search-result-row",
              "data-action": "open-search-asset",
              "data-asset-id": asset.assetId,
              disabled: !callbacks.onOpenAsset,
              onClick: callbacks.onOpenAsset ? () => callbacks.onOpenAsset?.(asset.assetId) : undefined,
            },
            e("i", { className: "document-color accent-2" }),
            e(
              "span",
              { className: "document-row-copy" },
              e("strong", null, presentAssetResultLabel(asset)),
              e("small", null, [
                presentation.mediaTypeLabel,
                presentation.sizeLabel,
                asset.linkedDocumentTitle?.trim() ?? "",
              ].filter(Boolean).join(" · ")),
            ),
            e("span", { className: "document-arrow", "aria-hidden": "true" }, "›"),
          ),
        );
      }),
    ),
  );
}

function renderGlobalSearchFooter(
  model: DocumentNavigatorModel,
  searchMetrics?: DesktopDocumentNavigatorOptions["searchMetrics"],
  assetCount = 0,
): React.ReactElement {
  const e = React.createElement;
  return e(
    "footer",
    { className: "global-search-footer", "aria-live": "polite" },
    presentGlobalSearchResultMetadata({
      documentCount: model.items.length,
      assetCount,
      durationMs: searchMetrics?.durationMs ?? model.searchMetrics?.durationMs,
    }),
  );
}

function presentAssetResultLabel(asset: DesktopGlobalSearchAssetResult): string {
  const label = asset.label?.trim() ?? "";
  return label ? label : "첨부 파일";
}

function defaultViewKey(view: DocumentNavigatorView): string | undefined {
  if (view === "Collection") return "work";
  if (view === "Tag") return "rust";
  return undefined;
}
