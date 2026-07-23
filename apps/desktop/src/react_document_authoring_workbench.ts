import React, { useEffect, useRef } from "react";
import {
  ArrowLeft,
  Bold,
  Heading1,
  History,
  Italic,
  Link2,
  List,
  ListChecks,
  Paperclip,
  RotateCcw,
  Search,
  Table2,
  X,
} from "lucide-react";

import {
  DocumentSaveCoordinatorState,
} from "@sponzey-cabinet/ui";
import {
  applyWysiwygPatchToSyncSession,
  applyWysiwygMarkdownChecklistItemToggle,
  applyWysiwygMarkdownBlockTextEdit,
  applyWysiwygMarkdownTableCellEdit,
  createWysiwygPlainTextSyncSession,
  createWysiwygMarkdownPresentationModel,
  type WysiwygMarkdownBlock,
  type WysiwygMarkdownBlockquoteBlock,
  type WysiwygMarkdownChecklistBlock,
  type WysiwygMarkdownCodeBlock,
  type WysiwygMarkdownHeadingBlock,
  type WysiwygMarkdownInlineNode,
  type WysiwygMarkdownParagraphBlock,
  type WysiwygMarkdownTableBlock,
} from "@sponzey-cabinet/editor";

import type { DesktopDocumentAuthoringSnapshot } from "./desktop_document_authoring_controller.ts";
import type { DesktopLinkOverviewSnapshot } from "./desktop_link_overview_controller.ts";
import type {
  DocumentAttachmentDiffView,
  DocumentDiffView,
  DocumentDiffHunkView,
  DocumentTitleDeltaView,
} from "@sponzey-cabinet/client-core";
import {
  mountCodeMirrorDocumentEditor,
  type CodeMirrorDocumentEditor,
} from "./codemirror_document_editor.ts";
import { createWorkspaceShellModel, type WorkspaceShellRouteKind } from "./workspace_shell_contract.ts";
import {
  createWorkspaceShellElement,
  type WorkspaceShellDocumentShortcut,
} from "./react_workspace_shell.ts";
import { formatHistoryRangeKoKr, KO_KR_MESSAGES } from "./ko_kr_catalog.ts";
import { mapUserFacingError } from "./user_facing_error_presenter.ts";
import type { DesktopAssetSurfaceSnapshot } from "./desktop_asset_controller.ts";
import type { DesktopGraphQueryState, DesktopGraphSurfaceSnapshot } from "./desktop_graph_controller.ts";
import type { DocumentAssetLibraryState } from "./document_asset_library_state.ts";
import {
  createDocumentAttachmentPanelElement,
  type DocumentAttachmentPanelCallbacks,
} from "./react_document_attachment_panel.ts";
import {
  createDocumentInspectorState,
  type DocumentInspectorState,
  type DocumentInspectorTab,
} from "./document_inspector_state.ts";
import {
  createDocumentHistoryCompareSelection,
  type DocumentHistoryCompareSelectionState,
} from "./document_history_compare_selection.ts";
import type { DocumentRestorePresentationState } from "./document_restore_presentation.ts";
import {
  createDocumentDiffHunkWindow,
  nextDocumentDiffHunkWindow,
  previousDocumentDiffHunkWindow,
} from "./document_diff_hunk_window.ts";
import {
  createDocumentHistoryWindow,
  historyIdentityChangeRequiresReset,
  nextDocumentHistoryWindow,
  previousDocumentHistoryWindow,
  reconcileDocumentHistoryWindow,
  type DocumentHistoryFocusRequest,
} from "./document_history_window.ts";
import { presentGraphNodes } from "./graph_display_presenter.ts";
import { ReactTopologyVisualHost } from "./react_topology_visual_host.ts";
import { createTopologyRendererModel } from "./topology_visual_orchestrator.ts";
import { filterTopologyVisualGraph } from "./topology_visual_filter.ts";
import type { DesktopGraphCameraPreference } from "./desktop_graph_preference.ts";
import { presentTopologyEmptyState } from "./topology_empty_state_presenter.ts";

const shellRoutes: readonly WorkspaceShellRouteKind[] = ["Home", "Search", "Document", "Graph", "Canvas", "Assets", "Backup"];

export type DocumentFormattingCommand = "heading" | "bold" | "italic" | "link" | "list" | "checklist" | "table";

const formattingCommands = [
  { command: "heading", action: "format-heading", label: "제목", Icon: Heading1 },
  { command: "bold", action: "format-bold", label: "굵게", Icon: Bold },
  { command: "italic", action: "format-italic", label: "기울임", Icon: Italic },
  { command: "link", action: "format-link", label: "링크", Icon: Link2 },
  { command: "list", action: "format-list", label: "목록", Icon: List },
  { command: "checklist", action: "format-checklist", label: "체크리스트", Icon: ListChecks },
  { command: "table", action: "format-table", label: "표", Icon: Table2 },
] as const;

export interface DesktopDocumentAuthoringWorkbenchCallbacks extends DocumentAttachmentPanelCallbacks {
  readonly onHome: () => void;
  readonly onReturnToSearch?: () => void;
  readonly onBodyChange: (body: string) => void;
  readonly onOpenPlainTextEditor?: () => void;
  readonly onClosePlainTextEditor?: () => void;
  readonly onFormatCommand?: (command: DocumentFormattingCommand) => void;
  readonly onSave: () => void;
  readonly onRetry: () => void;
  readonly onDiscard: () => void;
  readonly onCancel: () => void;
  readonly onLoadHistory?: () => void;
  readonly onLoadMoreHistory?: () => void;
  readonly onToggleHistoryCompareSelection?: (versionId: string, versionLabel: string) => void;
  readonly onCompareSelectedVersions?: () => void;
  readonly onCompareVersion?: (versionId: string) => void;
  readonly onCloseDiff?: () => void;
  readonly onCancelBackgroundDiff?: () => void;
  readonly onRetryBackgroundDiff?: () => void;
  readonly onPreviewRestore?: (versionId: string) => void;
  readonly onRequestRestoreConfirmation?: () => void;
  readonly onCancelRestoreConfirmation?: () => void;
  readonly onApplyRestore?: () => void;
  readonly onRefreshRestorePreview?: () => void;
  readonly onContinueRestoreRecovery?: () => void;
  readonly onSearchOpen?: () => void;
  readonly onSearch?: (query?: string) => void;
  readonly onGraph?: () => void;
  readonly onLocalGraphNodeSelect?: (nodeId: string) => void;
  readonly onLocalGraphQuery?: (patch: Partial<DesktopGraphQueryState>) => void;
  readonly onOpenLocalGraphAsset?: (assetId: string) => void;
  readonly onLocalGraphVisualSearch?: (query: string) => void;
  readonly onLocalGraphCameraPreferenceChanged?: (camera: DesktopGraphCameraPreference) => void;
  readonly onLocalGraphIncludeExternalChange?: (include: boolean) => void;
  readonly onLocalGraphRetry?: () => void;
  readonly onLocalGraphRepair?: () => void;
  readonly onCanvas?: () => void;
  readonly onAssets?: () => void;
  readonly onBackup?: () => void;
  readonly onCreateDocument?: () => void;
  readonly onOpenLinkedDocument?: (documentId: string) => void;
  readonly onInspectorTab?: (tab: DocumentInspectorTab) => void;
}

export interface DesktopDocumentAuthoringWorkbenchOptions {
  readonly documentShortcuts?: readonly WorkspaceShellDocumentShortcut[];
  readonly history?: DesktopDocumentHistoryWorkbenchState;
  readonly links?: DesktopLinkOverviewSnapshot;
  readonly assets?: DesktopAssetSurfaceSnapshot;
  readonly assetLibrary?: DocumentAssetLibraryState;
  readonly inspector?: DocumentInspectorState;
  readonly graph?: DesktopGraphSurfaceSnapshot;
  readonly graphVisualSearch?: string;
  readonly graphCameraPreference?: DesktopGraphCameraPreference;
  readonly graphIncludeExternal?: boolean;
  readonly plainTextEditorOpen?: boolean;
}

export interface DesktopDocumentHistoryEntryView {
  readonly versionId: string;
  readonly versionLabel: string;
  readonly createdAtLabel: string;
  readonly summaryLabel: string;
}

export interface DesktopDocumentRestorePreviewView {
  readonly targetVersionId: string;
  readonly expectedCurrentVersionId: string;
  readonly changedLineCount: number;
  readonly canRestore: boolean;
}

export interface DesktopDocumentHistoryWorkbenchState {
  readonly status: "Idle" | "Loading" | "LoadingMore" | "Empty" | "Ready" | "PreviewReady" | "Blocked" | "Applying" | "Applied" | "Failed";
  readonly entries: readonly DesktopDocumentHistoryEntryView[];
  readonly nextCursor?: string;
  readonly loadMoreErrorCode?: string;
  readonly comparison?: DocumentHistoryCompareSelectionState;
  readonly diff?: DesktopDocumentDiffWorkbenchState;
  readonly preview?: DesktopDocumentRestorePreviewView;
  readonly restore?: DocumentRestorePresentationState;
  readonly errorCode?: string;
}

export type DesktopDocumentDiffWorkbenchState =
  | {
      readonly status: "Accepted" | "Running" | "Cancelled" | "Expired";
      readonly targetVersionId: string;
      readonly targetVersionLabel: string;
    }
  | {
      readonly status: "Loading";
      readonly targetVersionId: string;
      readonly targetVersionLabel: string;
    }
  | {
      readonly status: "Ready";
      readonly targetVersionId: string;
      readonly targetVersionLabel: string;
      readonly addedCount: number;
      readonly removedCount: number;
      readonly attachmentDiff: DocumentAttachmentDiffView;
      readonly titleDelta: DocumentTitleDeltaView;
      readonly hunks: readonly DocumentDiffHunkView[];
    }
  | {
      readonly status: "TooLarge";
      readonly targetVersionId: string;
      readonly targetVersionLabel: string;
      readonly limitReason: "bytes" | "lines" | "hunks";
      readonly attachmentDiff: DocumentAttachmentDiffView;
    }
  | {
      readonly status: "Failed";
      readonly targetVersionId: string;
      readonly targetVersionLabel: string;
      readonly errorCode: string;
      readonly canRetry?: boolean;
    };

interface WorkbenchProps {
  readonly snapshot: DesktopDocumentAuthoringSnapshot;
  readonly callbacks: DesktopDocumentAuthoringWorkbenchCallbacks;
  readonly history: DesktopDocumentHistoryWorkbenchState;
  readonly links?: DesktopLinkOverviewSnapshot;
  readonly assets?: DesktopAssetSurfaceSnapshot;
  readonly assetLibrary?: DocumentAssetLibraryState;
  readonly inspector: DocumentInspectorState;
  readonly graph?: DesktopGraphSurfaceSnapshot;
  readonly graphVisualSearch: string;
  readonly graphCameraPreference?: DesktopGraphCameraPreference;
  readonly graphIncludeExternal: boolean;
  readonly plainTextEditorOpen: boolean;
  readonly documentShortcuts?: readonly WorkspaceShellDocumentShortcut[];
}

export function createDesktopDocumentAuthoringWorkbenchElement(
  snapshot: DesktopDocumentAuthoringSnapshot,
  callbacks: DesktopDocumentAuthoringWorkbenchCallbacks,
  options: DesktopDocumentAuthoringWorkbenchOptions = {},
): React.ReactElement {
  return React.createElement(DesktopDocumentAuthoringWorkbench, {
    snapshot,
    callbacks,
    history: options.history ?? { status: "Idle", entries: [] },
    links: options.links,
    assets: options.assets,
    assetLibrary: options.assetLibrary,
    inspector: options.inspector ?? createDocumentInspectorState(),
    graph: options.graph,
    graphVisualSearch: options.graphVisualSearch ?? "",
    graphCameraPreference: options.graphCameraPreference,
    graphIncludeExternal: options.graphIncludeExternal ?? false,
    plainTextEditorOpen: options.plainTextEditorOpen ?? false,
    documentShortcuts: options.documentShortcuts,
  });
}

function DesktopDocumentAuthoringWorkbench({
  snapshot,
  callbacks,
  history,
  links,
  assets,
  assetLibrary,
  inspector,
  graph,
  graphVisualSearch,
  graphCameraPreference,
  graphIncludeExternal,
  plainTextEditorOpen,
  documentShortcuts,
}: WorkbenchProps): React.ReactElement {
  const e = React.createElement;
  const body = snapshot.body ?? "";
  const wysiwyg = createWysiwygMarkdownPresentationModel({ source: body });
  const restoreConfirmation = history.restore?.status === "Confirming" ? history.restore : undefined;

  const topbar = e(
      React.Fragment,
      null,
      e(
        "div",
        { className: "authoring-navigation" },
        callbacks.onReturnToSearch
          ? e(
              "button",
              {
                type: "button",
                className: "authoring-search-return",
                "data-action": "return-search-results",
                "aria-label": "검색 결과로 돌아가기",
                title: "검색 결과로 돌아가기",
                onClick: callbacks.onReturnToSearch,
              },
              e(ArrowLeft, { size: 15, "aria-hidden": true }),
              e("span", null, "검색 결과"),
            )
          : null,
        e(
          "button",
          { type: "button", className: "authoring-breadcrumb", "data-action": "authoring-home", onClick: callbacks.onHome },
          "프로젝트 / Cabinet",
        ),
      ),
      e(
        "form",
        {
          className: "topbar-search authoring-global-search",
          role: "search",
          onSubmit: (event: React.FormEvent<HTMLFormElement>) => {
            event.preventDefault();
            const query = new FormData(event.currentTarget).get("workspace-search");
            callbacks.onSearch(typeof query === "string" ? query.trim() : "");
          },
        },
        e(
          "button",
          {
            type: "button",
            className: "topbar-search-submit",
            "data-action": "submit-workspace-search",
            "aria-label": KO_KR_MESSAGES.message("shell.searchPrompt"),
            onClick: (event: React.MouseEvent<HTMLButtonElement>) => {
              const query = new FormData(event.currentTarget.form ?? undefined).get("workspace-search");
              callbacks.onSearch(typeof query === "string" ? query.trim() : "");
            },
          },
          e(Search, { size: 15, "aria-hidden": true }),
        ),
        e("input", {
          type: "search",
          name: "workspace-search",
          "data-action": "workspace-search-input",
          placeholder: KO_KR_MESSAGES.message("shell.searchPlaceholder"),
          "aria-label": KO_KR_MESSAGES.message("shell.searchPrompt"),
        }),
      ),
      e(
        "div",
        { className: "authoring-document-heading" },
        e("strong", { className: "authoring-title-display" }, snapshot.title ?? "제목 없는 문서"),
      ),
      e(
        "div",
        { className: "desktop-actions" },
        e(
          "span",
          { className: "save-status", "aria-live": "polite" },
          saveStateLabel(snapshot.saveState),
        ),
        e(
          "button",
          {
            type: "button",
            className: "primary",
            "data-action": "save-document",
            onClick: callbacks.onSave,
            disabled: ![
              DocumentSaveCoordinatorState.Dirty,
              DocumentSaveCoordinatorState.SaveFailed,
            ].includes(snapshot.saveState),
          },
          "저장",
        ),
      ),
    );
  const main = e(
      "main",
      { className: "authoring-main" },
      e(
        "div",
        { className: "authoring-toolbar" },
        e(
          "div",
          { className: "formatting-toolbar", role: "toolbar", "aria-label": "문서 서식" },
          formattingCommands.map(({ command, action, label, Icon }) =>
            e(
              "button",
              {
                key: action,
                type: "button",
                className: "formatting-command",
                "data-action": action,
                "aria-label": label,
                title: label,
                onClick: callbacks.onFormatCommand ? () => callbacks.onFormatCommand?.(command) : undefined,
                disabled: !callbacks.onFormatCommand,
              },
              e(Icon, { size: 17, "aria-hidden": true }),
            ),
          ),
        ),
        e(
          "div",
          { className: "editor-mode-control", role: "group", "aria-label": "편집 화면" },
          e(
            "button",
            {
              type: "button",
              "data-action": "open-plain-text-editor",
              "aria-label": "Markdown 원문 편집",
              onClick: callbacks.onOpenPlainTextEditor,
              disabled: !callbacks.onOpenPlainTextEditor,
            },
            "원문 편집",
          ),
        ),
        e(
          "span",
          { className: "revision-status" },
          saveStateLabel(snapshot.saveState),
        ),
      ),
      e(
        "div",
        { className: "authoring-layout" },
        e(
          "div",
          { className: history.diff || restoreConfirmation ? "authoring-workspace mode-compare" : "authoring-workspace mode-wysiwyg" },
          history.diff
            ? renderDocumentDiff(history.diff, callbacks)
            : restoreConfirmation
              ? renderRestoreConfirmation(restoreConfirmation, callbacks)
            : e(
                "section",
                {
                  className: "wysiwyg-document-surface",
                  "data-editor-surface": "wysiwyg",
                  "aria-label": "WYSIWYG 문서 편집",
                },
                wysiwyg.blocks.length > 0
                  ? wysiwyg.blocks.map((block) =>
                    renderWysiwygBlock(block, body, snapshot.documentId, snapshot.revision, callbacks)
                  )
                  : e("p", { className: "wysiwyg-empty-state", role: "status" }, "빈 문서"),
              ),
        ),
        e(
          "aside",
          { className: "authoring-context-column", "aria-label": "문서 정보" },
          renderAuthoringKnowledgeMap(graph, graphVisualSearch, graphCameraPreference, graphIncludeExternal, callbacks),
          renderDocumentInspector(inspector, links, assets, assetLibrary, history, callbacks),
        ),
      ),
    );
  return createWorkspaceShellElement({
    model: createWorkspaceShellModel({ route: "Document", availableActions: shellRoutes, messages: KO_KR_MESSAGES }),
    messages: KO_KR_MESSAGES,
    routeActions: { Home: callbacks.onHome, Search: callbacks.onSearch, Graph: callbacks.onGraph, Canvas: callbacks.onCanvas, Assets: callbacks.onAssets, Backup: callbacks.onBackup },
    onCreateDocument: callbacks.onCreateDocument,
    onSearchOpen: callbacks.onSearchOpen,
    onSearch: callbacks.onSearch,
    rootClassName: "authoring-shell",
    rootAttributes: {
      "data-cabinet-authoring-state": snapshot.saveState,
      "data-document-id": snapshot.documentId ?? "",
      "data-document-revision": String(snapshot.revision),
      "data-persisted-revision": String(snapshot.persistedRevision),
    },
    topbarContent: topbar,
    globalLayer: e(
      React.Fragment,
      null,
      renderRecovery(snapshot, callbacks),
      plainTextEditorOpen ? renderPlainTextEditorDialog(snapshot, callbacks) : null,
    ),
    documentShortcuts,
    content: main,
  });
}

function renderDocumentInspector(
  inspector: DocumentInspectorState,
  links: DesktopLinkOverviewSnapshot | undefined,
  assets: DesktopAssetSurfaceSnapshot | undefined,
  assetLibrary: DocumentAssetLibraryState | undefined,
  history: DesktopDocumentHistoryWorkbenchState,
  callbacks: DesktopDocumentAuthoringWorkbenchCallbacks,
): React.ReactElement {
  const e = React.createElement;
  const tabs = [
    { id: "links" as const, label: "연결", icon: Link2 },
    { id: "attachments" as const, label: "첨부 파일", icon: Paperclip },
    { id: "history" as const, label: "이력", icon: History },
  ];
  const content = inspector.tab === "links"
    ? renderConnectedDocuments(links, callbacks)
    : inspector.tab === "attachments"
      ? assets
        ? createDocumentAttachmentPanelElement(assets, callbacks, inspector.unlink, assetLibrary)
        : e("p", { className: "document-inspector-empty", role: "status" }, "첨부 파일을 준비하는 중입니다")
      : renderHistoryRestorePanel(history, callbacks);
  return e(
    "section",
    { className: "document-inspector", "data-document-inspector-tab": inspector.tab, "aria-label": "문서 세부 정보" },
    e(
      "div",
      { className: "document-inspector-tabs", role: "tablist", "aria-label": "문서 세부 정보" },
      tabs.map((tab) => e(
        "button",
        {
          key: tab.id,
          id: `document-inspector-tab-${tab.id}`,
          type: "button",
          role: "tab",
          "aria-selected": inspector.tab === tab.id,
          "aria-controls": `document-inspector-panel-${tab.id}`,
          tabIndex: inspector.tab === tab.id ? 0 : -1,
          "data-action": `select-document-inspector-${tab.id}`,
          onClick: () => callbacks.onInspectorTab?.(tab.id),
          disabled: !callbacks.onInspectorTab,
        },
        e(tab.icon, { size: 14, strokeWidth: 2, "aria-hidden": true }),
        tab.label,
      )),
    ),
    e(
      "div",
      {
        id: `document-inspector-panel-${inspector.tab}`,
        className: "document-inspector-panel",
        role: "tabpanel",
        "aria-labelledby": `document-inspector-tab-${inspector.tab}`,
      },
      content,
    ),
  );
}

function renderAuthoringKnowledgeMap(
  snapshot: DesktopGraphSurfaceSnapshot | undefined,
  visualSearch: string,
  cameraPreference: DesktopGraphCameraPreference | undefined,
  includeExternal: boolean,
  callbacks: DesktopDocumentAuthoringWorkbenchCallbacks,
): React.ReactElement {
  const e = React.createElement;
  const graph = snapshot?.graph;
  const nodes = presentGraphNodes(graph?.nodes ?? []);
  const visibleGraph = filterTopologyVisualGraph(nodes, graph?.edges ?? [], visualSearch, { includeExternal });
  const visibleNodes = visibleGraph.nodes;
  const emptyState = presentTopologyEmptyState({
    sourceNodeCount: graph?.nodes.length ?? 0,
    sourceEdgeCount: graph?.edges.length ?? 0,
    visibleNodeCount: visibleNodes.length,
    visualFilterActive: Boolean(visualSearch.trim()) || (graph?.stats?.filteredCount ?? 0) > 0,
  });
  const rendererModel = createTopologyRendererModel(
    visibleNodes,
    visibleGraph.edges,
    snapshot?.selectedNodeId,
    graph?.centerDocumentId,
  );
  const hasGraph = graph !== undefined && visibleNodes.length > 0;
  const state = snapshot?.state ?? "Idle";
  const selected = visibleNodes.find((node) => node.identity === snapshot?.selectedNodeId);
  const incoming = selected ? graph?.edges.filter((edge) => edge.targetId === selected.identity).length ?? 0 : 0;
  const outgoing = selected ? graph?.edges.filter((edge) => edge.sourceId === selected.identity).length ?? 0 : 0;
  return e(
    "section",
    {
      className: "overview-card authoring-map-card",
      "aria-labelledby": "authoring-map-title",
      "data-authoring-local-graph-state": state,
    },
    e(
      "div",
      { className: "section-heading" },
      e("h2", { id: "authoring-map-title" }, "이 문서의 지식 지도"),
      e("button", { type: "button", className: "text-action", "data-action": "open-authoring-graph", disabled: !callbacks.onGraph, onClick: callbacks.onGraph }, "전체 화면"),
    ),
    snapshot
      ? e(
          "div",
          { className: "authoring-local-graph-filters", "aria-label": "문서 관계 범위" },
          e("input", {
            type: "search",
            className: "authoring-local-graph-search",
            "data-action": "search-authoring-graph",
            "aria-label": "이 문서의 지식 지도 검색",
            placeholder: "관계에서 찾기",
            value: visualSearch,
            onChange: (event: React.ChangeEvent<HTMLInputElement>) => callbacks.onLocalGraphVisualSearch?.(event.currentTarget.value),
            disabled: !callbacks.onLocalGraphVisualSearch,
          }),
          ([1, 2] as const).map((depth) => e("button", {
            key: `depth-${depth}`,
            type: "button",
            "data-action": `authoring-graph-depth-${depth}`,
            "aria-pressed": snapshot.query.depth === depth,
            title: `${depth}단계 관계`,
            onClick: () => callbacks.onLocalGraphQuery?.({ depth }),
            disabled: !callbacks.onLocalGraphQuery,
          }, `${depth}단계`)),
          (["both", "incoming", "outgoing"] as const).map((direction) => e("button", {
            key: direction,
            type: "button",
            "data-action": `authoring-graph-direction-${direction}`,
            "aria-pressed": snapshot.query.direction === direction,
            title: direction === "both" ? "모든 방향" : direction === "incoming" ? "들어오는 관계" : "나가는 관계",
            onClick: () => callbacks.onLocalGraphQuery?.({ direction }),
            disabled: !callbacks.onLocalGraphQuery,
          }, direction === "both" ? "전체" : direction === "incoming" ? "들어옴" : "나감")),
          e("button", {
            type: "button",
            "data-action": "authoring-graph-toggle-unresolved",
            "aria-pressed": snapshot.query.includeUnresolved,
            title: "미해결 링크 표시",
            onClick: () => callbacks.onLocalGraphQuery?.({ includeUnresolved: !snapshot.query.includeUnresolved }),
            disabled: !callbacks.onLocalGraphQuery,
          }, "미해결"),
          e("button", {
            type: "button",
            "data-action": "authoring-graph-toggle-assets",
            "aria-pressed": snapshot.query.includeAssets,
            title: "첨부 파일 표시",
            onClick: () => callbacks.onLocalGraphQuery?.({ includeAssets: !snapshot.query.includeAssets }),
            disabled: !callbacks.onLocalGraphQuery,
          }, "첨부"),
          e("button", {
            type: "button",
            "data-action": "authoring-graph-toggle-external",
            "aria-pressed": includeExternal,
            title: "외부 링크 표시",
            onClick: () => callbacks.onLocalGraphIncludeExternalChange?.(!includeExternal),
            disabled: !callbacks.onLocalGraphIncludeExternalChange,
          }, "외부"),
        )
      : null,
    hasGraph
      ? e(
          React.Fragment,
          null,
          e(
            "div",
            { className: "authoring-local-graph-stage", "aria-label": "현재 문서 관계 지도" },
            e(ReactTopologyVisualHost, {
              model: rendererModel,
              semanticNodes: visibleNodes,
              onNodeSelected: (nodeId: string) => callbacks.onLocalGraphNodeSelect?.(nodeId),
              onNodeActivated: (nodeId: string) => {
                const node = visibleNodes.find((candidate) => candidate.identity === nodeId);
                if (node?.kind === "document" && node.canNavigate) callbacks.onOpenLinkedDocument?.(nodeId);
                if (node?.kind === "attachment" && node.canNavigate) callbacks.onOpenLocalGraphAsset?.(nodeId);
              },
              cameraPreference,
              onCameraPreferenceChanged: callbacks.onLocalGraphCameraPreferenceChanged,
            }),
            (state === "Ready" || state === "Empty") && emptyState
              ? e("p", { className: "authoring-local-graph-overlay", role: "status", "data-topology-empty-kind": emptyState.kind }, emptyState.message)
              : null,
            state === "Stale"
              ? e("div", { className: "authoring-local-graph-overlay", role: "status" }, e("span", null, "문서 관계를 갱신해야 합니다."), e("button", { type: "button", "data-action": "repair-authoring-graph", disabled: !callbacks.onLocalGraphRepair, onClick: callbacks.onLocalGraphRepair }, "관계 다시 만들기"))
              : state === "Repairing"
                ? e("p", { className: "authoring-local-graph-overlay", role: "status" }, "문서 관계를 다시 만드는 중입니다.")
                : state === "Loading"
                  ? e("p", { className: "authoring-local-graph-overlay", role: "status" }, "최신 문서 관계를 불러오는 중입니다.")
                  : null,
          ),
          selected
            ? e(
                "section",
                { className: "authoring-local-graph-detail", "aria-label": "선택한 관계 항목" },
                e("div", null, e("span", null, selected.kindLabel), e("strong", null, selected.label)),
                e("dl", null,
                  e("div", null, e("dt", null, "들어오는 연결"), e("dd", null, String(incoming))),
                  e("div", null, e("dt", null, "나가는 연결"), e("dd", null, String(outgoing))),
                ),
                selected.kind === "document" && selected.canNavigate
                  ? e("div", { className: "authoring-local-graph-detail-actions" },
                      e("button", { type: "button", "data-action": "open-authoring-graph-document", onClick: () => callbacks.onOpenLinkedDocument?.(selected.identity), disabled: !callbacks.onOpenLinkedDocument }, "문서 열기"),
                      selected.identity !== graph?.centerDocumentId
                        ? e("button", { type: "button", "data-action": "recenter-authoring-graph", onClick: () => callbacks.onLocalGraphQuery?.({ scope: "local", centerDocumentId: selected.identity }), disabled: !callbacks.onLocalGraphQuery }, "이 문서 중심으로")
                        : null,
                    )
                  : selected.kind === "attachment" && selected.canNavigate
                    ? e("button", { type: "button", "data-action": "open-authoring-graph-asset", onClick: () => callbacks.onOpenLocalGraphAsset?.(selected.identity), disabled: !callbacks.onOpenLocalGraphAsset }, "첨부 파일 열기")
                    : null,
              )
            : null,
        )
      : state === "Loading" || state === "Idle"
        ? e("p", { className: "authoring-local-graph-message", role: "status" }, "문서 관계를 불러오는 중입니다.")
        : state === "Repairing"
          ? e("p", { className: "authoring-local-graph-message", role: "status" }, "문서 관계를 다시 만드는 중입니다.")
          : state === "Failed"
            ? e("div", { className: "authoring-local-graph-message", role: "alert" }, e("span", null, "문서 관계를 불러오지 못했습니다."), e("button", { type: "button", "data-action": snapshot?.retryable ? "repair-authoring-graph" : "retry-authoring-graph", onClick: snapshot?.retryable ? callbacks.onLocalGraphRepair : callbacks.onLocalGraphRetry, disabled: snapshot?.retryable ? !callbacks.onLocalGraphRepair : !callbacks.onLocalGraphRetry }, snapshot?.retryable ? "관계 다시 만들기" : "다시 시도"))
            : e("p", { className: "authoring-local-graph-message" }, "이 문서에 연결된 항목이 없습니다."),
  );
}

function renderConnectedDocuments(
  links: DesktopLinkOverviewSnapshot | undefined,
  callbacks: DesktopDocumentAuthoringWorkbenchCallbacks,
): React.ReactElement {
  const e = React.createElement;
  const backlinks = links?.panel?.backlinks ?? [];
  return e(
    "section",
    {
      className: "overview-card connected-documents",
      "aria-labelledby": "connected-title",
      "data-link-overview-state": links?.state ?? "Idle",
    },
    e("h2", { id: "connected-title" }, "연결된 문서"),
    links?.state === "Loading"
      ? e("p", { className: "empty-label" }, "연결된 문서를 불러오는 중입니다")
      : links?.state === "Failed"
        ? e("p", { className: "state-text failed", role: "alert" }, "연결된 문서를 불러오지 못했습니다")
        : backlinks.length === 0
          ? e("p", { className: "empty-label" }, "연결된 문서가 없습니다")
          : e(
              "ol",
              { className: "connected-document-list" },
              backlinks.map((backlink) => e(
                "li",
                { key: backlink.sourceDocumentId },
                e(
                  "button",
                  {
                    type: "button",
                    "data-action": "open-linked-authoring-document",
                    "data-linked-document-id": backlink.sourceDocumentId,
                    onClick: () => callbacks.onOpenLinkedDocument?.(backlink.sourceDocumentId),
                    disabled: !callbacks.onOpenLinkedDocument,
                  },
                  e("strong", null, backlink.sourceTitle),
                  e("small", null, backlink.sourcePath),
                ),
              )),
            ),
  );
}

function renderHistoryRestorePanel(
  history: DesktopDocumentHistoryWorkbenchState,
  callbacks: DesktopDocumentAuthoringWorkbenchCallbacks,
): React.ReactElement {
  const e = React.createElement;
  const comparison = history.comparison ?? createDocumentHistoryCompareSelection();
  return e(
    "aside",
    {
      className: "history-restore-panel",
      "data-history-restore-state": history.status,
      "aria-label": "문서 이력과 복원",
    },
    e("div", { className: "panel-heading" }, e("strong", null, "문서 이력")),
    e(
      "button",
      {
        type: "button",
        "data-action": "load-history",
        onClick: callbacks.onLoadHistory,
        disabled: !callbacks.onLoadHistory || history.status === "Loading",
      },
      "이력 불러오기",
    ),
    history.errorCode
      ? e("p", { className: "state-text failed", role: "alert" }, "문서 이력을 불러오지 못했습니다")
      : null,
    history.status === "Empty"
      ? e("p", { className: "state-text" }, KO_KR_MESSAGES.message("history.empty"))
      : null,
    e(DocumentHistoryWindowList, {
      entries: history.entries,
      comparison,
      callbacks,
      diffLoading: history.diff?.status === "Loading",
    }),
    e(
      "button",
      {
        type: "button",
        className: "history-compare-selected",
        "data-action": "compare-selected-versions",
        onClick: callbacks.onCompareSelectedVersions,
        disabled: !callbacks.onCompareSelectedVersions || comparison.status !== "TwoSelected" || history.diff?.status === "Loading",
      },
      KO_KR_MESSAGES.message("history.compareSelected"),
    ),
    history.loadMoreErrorCode
      ? e(
          "p",
          { className: "history-load-more-error state-text failed", role: "alert" },
          KO_KR_MESSAGES.message("history.loadMoreFailed"),
        )
      : null,
    history.nextCursor
      ? e(
          "button",
          {
            type: "button",
            className: "history-load-more",
            "data-action": "load-more-history",
            onClick: callbacks.onLoadMoreHistory,
            disabled: !callbacks.onLoadMoreHistory || history.status === "LoadingMore",
          },
          history.status === "LoadingMore"
            ? KO_KR_MESSAGES.message("history.loadingMore")
            : history.loadMoreErrorCode
              ? KO_KR_MESSAGES.message("action.retry")
              : KO_KR_MESSAGES.message("history.loadMore"),
        )
      : null,
    renderRestorePresentation(history.restore, callbacks),
  );
}

function DocumentHistoryWindowList({
  entries,
  comparison,
  callbacks,
  diffLoading,
}: {
  readonly entries: readonly DesktopDocumentHistoryEntryView[];
  readonly comparison: DocumentHistoryCompareSelectionState;
  readonly callbacks: DesktopDocumentAuthoringWorkbenchCallbacks;
  readonly diffLoading: boolean;
}): React.ReactElement {
  const e = React.createElement;
  const identities = entries.map((entry) => entry.versionId);
  const identitySignature = identities.join("\u0000");
  const previousIdentities = useRef<readonly string[]>(identities);
  const focusRequest = useRef<DocumentHistoryFocusRequest>("None");
  const firstVisibleAction = useRef<HTMLButtonElement>(null);
  const [window, setWindow] = React.useState(() => createDocumentHistoryWindow(entries.length));

  useEffect(() => {
    const reset = historyIdentityChangeRequiresReset(previousIdentities.current, identities);
    previousIdentities.current = identities;
    setWindow((current) => reconcileDocumentHistoryWindow(current, entries.length, reset));
  }, [identitySignature, entries.length]);

  useEffect(() => {
    if (focusRequest.current !== "FocusFirstVisible") return;
    focusRequest.current = "None";
    firstVisibleAction.current?.focus();
  }, [window.start]);

  const move = (direction: "Previous" | "Next") => {
    const transition = direction === "Previous"
      ? previousDocumentHistoryWindow(window)
      : nextDocumentHistoryWindow(window);
    focusRequest.current = transition.focusRequest;
    setWindow(transition.window);
  };
  const visibleEntries = entries.slice(window.start, window.endExclusive);
  const range = `${window.start + (window.total > 0 ? 1 : 0)}-${window.endExclusive}/${window.total}`;

  return e(
    React.Fragment,
    null,
    e(
      "ol",
      { className: "history-list", "data-history-window": range },
      visibleEntries.map((entry, index) => e(
        "li",
        { key: entry.versionId, "data-history-entry": "visible" },
        e(
          "div",
          { className: "history-entry-display" },
          e("strong", null, entry.versionLabel),
          e("span", null, entry.createdAtLabel),
          e("span", null, entry.summaryLabel),
        ),
        renderHistoryCompareSelectionAction(
          entry,
          comparison,
          callbacks,
          index === 0 ? firstVisibleAction : undefined,
        ),
        e(
          "button",
          {
            type: "button",
            "data-action": "compare-current-version",
            onClick: () => callbacks.onCompareVersion?.(entry.versionId),
            disabled: !callbacks.onCompareVersion || diffLoading,
          },
          "현재 문서와 비교",
        ),
        e(
          "button",
          {
            type: "button",
            "data-action": "preview-restore",
            onClick: () => callbacks.onPreviewRestore?.(entry.versionId),
            disabled: !callbacks.onPreviewRestore,
          },
          "복원 미리보기",
        ),
      )),
    ),
    window.virtualized
      ? e(
          "div",
          { className: "history-window-navigation", "aria-label": "문서 이력 범위 이동" },
          e(
            "p",
            { className: "history-window-range", role: "status", "aria-live": "polite" },
            formatHistoryRangeKoKr(window.start, window.endExclusive, window.total),
          ),
          e(
            "button",
            {
              type: "button",
              "data-action": "previous-history-window",
              disabled: !window.hasPrevious,
              onClick: () => move("Previous"),
            },
            KO_KR_MESSAGES.message("history.previousWindow"),
          ),
          e(
            "button",
            {
              type: "button",
              "data-action": "next-history-window",
              disabled: !window.hasNext,
              onClick: () => move("Next"),
            },
            KO_KR_MESSAGES.message("history.nextWindow"),
          ),
        )
      : null,
  );
}

function renderRestorePresentation(
  restore: DocumentRestorePresentationState | undefined,
  callbacks: DesktopDocumentAuthoringWorkbenchCallbacks,
): React.ReactElement | null {
  const e = React.createElement;
  if (!restore || restore.status === "Idle") return null;
  if (restore.status === "Previewing") {
    return e(
      "div",
      { className: "restore-preview-summary", "data-restore-state": restore.status },
      e("strong", null, KO_KR_MESSAGES.message("restore.preview")),
      e("span", { role: "status" }, KO_KR_MESSAGES.message("restore.previewing")),
    );
  }
  if (restore.status === "Conflict") {
    return e(
      "div",
      { className: "restore-preview-summary conflict", "data-restore-state": restore.status },
      e("strong", null, restore.targetVersionLabel),
      e("p", { role: "alert" }, KO_KR_MESSAGES.message("restore.conflict")),
      e(
        "button",
        {
          type: "button",
          "data-action": "refresh-restore-preview",
          onClick: callbacks.onRefreshRestorePreview,
          disabled: !callbacks.onRefreshRestorePreview,
        },
        KO_KR_MESSAGES.message("restore.refreshPreview"),
      ),
    );
  }
  if (restore.status === "RecoveryRequired") {
    return e(
      "div",
      { className: "restore-preview-summary recovery", "data-restore-state": restore.status },
      e("strong", null, restore.targetVersionLabel),
      e("p", { role: "alert" }, KO_KR_MESSAGES.message("restore.recoveryRequired")),
      e(
        "button",
        {
          type: "button",
          "data-action": "continue-restore-recovery",
          onClick: callbacks.onContinueRestoreRecovery,
          disabled: !callbacks.onContinueRestoreRecovery,
        },
        KO_KR_MESSAGES.message("restore.continueRecovery"),
      ),
    );
  }
  if (restore.status === "BlockedMissingAsset") {
    return e(
      "div",
      { className: "restore-preview-summary blocked", "data-restore-state": restore.status },
      e("strong", null, restore.targetVersionLabel),
      e("p", { role: "alert" }, KO_KR_MESSAGES.message("restore.missingAsset")),
      restore.missingAssetLabels.length > 0
        ? e("ul", null, restore.missingAssetLabels.map((label) => e("li", { key: label }, label)))
        : null,
      e(
        "button",
        { type: "button", className: "primary", "data-action": "apply-restore", disabled: true },
        KO_KR_MESSAGES.message("restore.apply"),
      ),
    );
  }
  if (restore.status === "BlockedLargeDiff") {
    return e(
      "div",
      { className: "restore-preview-summary blocked", "data-restore-state": restore.status },
      e("strong", null, restore.targetVersionLabel),
      e("p", { role: "alert" }, KO_KR_MESSAGES.message("restore.largeDiffBlocked")),
      renderAttachmentDiff(restore.diff.attachmentDiff),
      e(
        "button",
        { type: "button", className: "primary", "data-action": "review-restore", disabled: true },
        KO_KR_MESSAGES.message("restore.review"),
      ),
    );
  }
  if (restore.status === "Failed") {
    return e(
      "div",
      { className: "restore-preview-summary failed", "data-restore-state": restore.status },
      e("strong", null, restore.targetVersionLabel),
      e("p", { role: "alert" }, KO_KR_MESSAGES.message("restore.failed")),
    );
  }
  if (restore.status === "Confirming") {
    return null;
  }
  return e(
    "div",
    { className: "restore-preview-summary", "data-restore-state": restore.status },
    e("strong", null, `${restore.targetVersionLabel} ${KO_KR_MESSAGES.message("restore.preview")}`),
    e("span", null, `${restore.changedLineCount}개 줄 변경`),
    restore.status === "Applied"
      ? e("span", { role: "status" }, KO_KR_MESSAGES.message("restore.completed"))
      : restore.status === "Applying"
        ? e("span", { role: "status" }, KO_KR_MESSAGES.message("restore.applying"))
        : e(
          "button",
          {
            type: "button",
            className: "primary",
            "data-action": "review-restore",
            onClick: callbacks.onRequestRestoreConfirmation,
            disabled: !callbacks.onRequestRestoreConfirmation,
          },
          KO_KR_MESSAGES.message("restore.review"),
        ),
  );
}

function renderRestoreConfirmation(
  restore: Extract<DocumentRestorePresentationState, { status: "Confirming" }>,
  callbacks: DesktopDocumentAuthoringWorkbenchCallbacks,
): React.ReactElement {
  const e = React.createElement;
  return e(
    "section",
    { className: "restore-confirmation", "data-restore-state": restore.status },
    e(
      "div",
      { className: "restore-confirmation-heading" },
      e(
        "div",
        null,
        e("h2", null, KO_KR_MESSAGES.message("restore.confirmHeading")),
        e("strong", null, restore.targetVersionLabel),
      ),
      e(
        "button",
        {
          type: "button",
          "data-action": "cancel-restore-confirmation",
          onClick: callbacks.onCancelRestoreConfirmation,
          disabled: !callbacks.onCancelRestoreConfirmation,
        },
        KO_KR_MESSAGES.message("restore.cancelConfirmation"),
      ),
    ),
    renderRestoreDiffReview(restore.diff),
    e(
      "div",
      { className: "restore-confirmation-actions" },
      e(
        "button",
        {
          type: "button",
          className: "primary",
          "data-action": "confirm-restore",
          onClick: callbacks.onApplyRestore,
          disabled: !callbacks.onApplyRestore,
        },
        KO_KR_MESSAGES.message("restore.confirm"),
      ),
    ),
  );
}

function renderRestoreDiffReview(diff: DocumentDiffView): React.ReactElement {
  const e = React.createElement;
  if (diff.status === "TooLarge") {
    return e(
      "div",
      { className: "restore-diff-review too-large" },
      e("p", { role: "alert" }, "앱에서 바로 확인하기에는 문서가 너무 큽니다"),
      renderAttachmentDiff(diff.attachmentDiff),
    );
  }
  return e(
    "div",
    { className: "restore-diff-review complete" },
    e(
      "div",
      { className: "diff-summary", "aria-label": "복원 변경 요약" },
      e("span", { className: "diff-added-count" }, `추가 ${diff.addedCount}줄`),
      e("span", { className: "diff-removed-count" }, `삭제 ${diff.removedCount}줄`),
    ),
    diff.titleDelta.kind === "Changed"
      ? e(
          "p",
          { className: "diff-title-change" },
          e("span", null, diff.titleDelta.before),
          e("span", { "aria-hidden": "true" }, " → "),
          e("span", null, diff.titleDelta.after),
        )
      : null,
    renderAttachmentDiff(diff.attachmentDiff),
    renderDiffHunks(diff.hunks),
  );
}

function renderHistoryCompareSelectionAction(
  entry: DesktopDocumentHistoryEntryView,
  comparison: DocumentHistoryCompareSelectionState,
  callbacks: DesktopDocumentAuthoringWorkbenchCallbacks,
  focusRef?: React.RefObject<HTMLButtonElement | null>,
): React.ReactElement {
  const e = React.createElement;
  const selectedIndex = comparison.selections.findIndex((selection) => selection.versionId === entry.versionId);
  const selected = selectedIndex >= 0;
  const label = selected
    ? `${entry.versionLabel} ${selectedIndex + 1}번째 선택됨`
    : `${entry.versionLabel} 비교 대상으로 선택`;
  return e(
    "button",
    {
      type: "button",
      className: selected ? "history-version-selection selected" : "history-version-selection",
      "data-action": "select-history-version",
      ref: focusRef,
      "aria-pressed": selected,
      "aria-label": label,
      onClick: () => callbacks.onToggleHistoryCompareSelection?.(entry.versionId, entry.versionLabel),
      disabled: !callbacks.onToggleHistoryCompareSelection,
    },
    selected ? `${entry.versionLabel} ${selectedIndex + 1}번째 선택됨` : "비교 대상으로 선택",
  );
}

function renderDocumentDiff(
  diff: DesktopDocumentDiffWorkbenchState,
  callbacks: DesktopDocumentAuthoringWorkbenchCallbacks,
): React.ReactElement {
  const e = React.createElement;
  const common = {
    className: `document-diff-workspace ${diff.status.toLowerCase()}`,
    "data-document-diff-state": diff.status,
    "aria-label": `${diff.targetVersionLabel} 비교 결과`,
  };
  const heading = e(
    "div",
    { className: "diff-workspace-heading" },
    e("strong", null, `${diff.targetVersionLabel} 비교 결과`),
    e(
      "button",
      {
        type: "button",
        "data-action": "close-document-diff",
        onClick: callbacks.onCloseDiff,
        disabled: !callbacks.onCloseDiff,
      },
      "편집기로 돌아가기",
    ),
  );
  if (diff.status === "Loading") {
    return e("section", common, heading, e("p", { className: "state-text", "aria-live": "polite" }, "비교 결과를 불러오는 중입니다"));
  }
  if (diff.status === "Accepted" || diff.status === "Running") {
    const messageKey = diff.status === "Accepted" ? "diff.backgroundAccepted" : "diff.backgroundRunning";
    return e(
      "section",
      common,
      heading,
      e("p", { className: "state-text", role: "status", "aria-live": "polite" }, KO_KR_MESSAGES.message(messageKey)),
      e(
        "div",
        { className: "diff-operation-actions" },
        e(
          "button",
          {
            type: "button",
            "data-action": "cancel-background-document-diff",
            "aria-label": KO_KR_MESSAGES.message("diff.backgroundCancel"),
            title: KO_KR_MESSAGES.message("diff.backgroundCancel"),
            onClick: callbacks.onCancelBackgroundDiff,
            disabled: !callbacks.onCancelBackgroundDiff,
          },
          e(X, { size: 16, "aria-hidden": true }),
          KO_KR_MESSAGES.message("diff.backgroundCancel"),
        ),
      ),
    );
  }
  if (diff.status === "Cancelled" || diff.status === "Expired") {
    const messageKey = diff.status === "Cancelled" ? "diff.backgroundCancelled" : "diff.backgroundExpired";
    return e(
      "section",
      common,
      heading,
      e("p", { className: "state-text", role: "status" }, KO_KR_MESSAGES.message(messageKey)),
      renderBackgroundDiffRetry(callbacks),
    );
  }
  if (diff.status === "TooLarge") {
    return e(
      "section",
      common,
      heading,
      e("p", { className: "state-text" }, "앱에서 바로 비교하기에는 문서가 너무 큽니다"),
      renderAttachmentDiff(diff.attachmentDiff),
    );
  }
  if (diff.status === "Failed") {
    return e(
      "section",
      common,
      heading,
      e("p", { className: "state-text failed", role: "alert" }, "비교 결과를 불러오지 못했습니다"),
      diff.canRetry ? renderBackgroundDiffRetry(callbacks) : null,
    );
  }
  return e(
    "section",
    common,
    heading,
    e(
      "div",
      { className: "diff-summary", "aria-label": "변경 요약" },
      e("span", { className: "diff-added-count" }, `추가 ${diff.addedCount}줄`),
      e("span", { className: "diff-removed-count" }, `삭제 ${diff.removedCount}줄`),
    ),
    diff.titleDelta.kind === "Changed"
      ? e(
          "p",
          { className: "diff-title-change" },
          e("span", null, diff.titleDelta.before),
          e("span", { "aria-hidden": "true" }, " → "),
          e("span", null, diff.titleDelta.after),
        )
      : null,
    renderAttachmentDiff(diff.attachmentDiff),
    renderDiffHunks(diff.hunks),
  );
}

function renderBackgroundDiffRetry(callbacks: DesktopDocumentAuthoringWorkbenchCallbacks): React.ReactElement {
  const label = KO_KR_MESSAGES.message("diff.backgroundRetry");
  return React.createElement(
    "div",
    { className: "diff-operation-actions" },
    React.createElement(
      "button",
      {
        type: "button",
        "data-action": "retry-background-document-diff",
        "aria-label": label,
        title: label,
        onClick: callbacks.onRetryBackgroundDiff,
        disabled: !callbacks.onRetryBackgroundDiff,
      },
      React.createElement(RotateCcw, { size: 16, "aria-hidden": true }),
      label,
    ),
  );
}

function renderDiffHunks(hunks: readonly DocumentDiffHunkView[]): React.ReactElement {
  return React.createElement(DocumentDiffHunkWindowView, { hunks });
}

function DocumentDiffHunkWindowView({
  hunks,
}: {
  readonly hunks: readonly DocumentDiffHunkView[];
}): React.ReactElement {
  const e = React.createElement;
  const [hunkWindow, setHunkWindow] = React.useState(() => createDocumentDiffHunkWindow(hunks.length));
  React.useEffect(() => {
    setHunkWindow(createDocumentDiffHunkWindow(hunks.length));
  }, [hunks]);
  const visibleHunks = hunks.slice(hunkWindow.start, hunkWindow.endExclusive);
  return e(
    "div",
    { className: "diff-hunk-window" },
    hunkWindow.total > hunkWindow.size
      ? e(
          "div",
          { className: "diff-hunk-navigation" },
          e(
            "button",
            {
              type: "button",
              "data-action": "previous-diff-hunks",
              "aria-label": "이전 변경 구간 보기",
              disabled: !hunkWindow.hasPrevious,
              onClick: () => setHunkWindow((current) => previousDocumentDiffHunkWindow(current)),
            },
            "이전",
          ),
          e(
            "span",
            { role: "status", "aria-live": "polite" },
            `변경 구간 ${hunkWindow.start + 1}–${hunkWindow.endExclusive} / ${hunkWindow.total}`,
          ),
          e(
            "button",
            {
              type: "button",
              "data-action": "next-diff-hunks",
              "aria-label": "다음 변경 구간 보기",
              disabled: !hunkWindow.hasNext,
              onClick: () => setHunkWindow((current) => nextDocumentDiffHunkWindow(current)),
            },
            "다음",
          ),
        )
      : null,
    e(
      "div",
      { className: "diff-hunks" },
      visibleHunks.map((hunk, visibleIndex) => {
        const hunkIndex = hunkWindow.start + visibleIndex;
        return e(
        "section",
        {
          className: "diff-hunk",
          key: `${hunk.oldStartLine}-${hunk.newStartLine}-${hunkIndex}`,
          "aria-label": `변경 구간 ${hunkIndex + 1}`,
        },
        hunk.lines.map((line, lineIndex) => {
          const kind = line.kind.toLowerCase();
          const lineNumber = line.newLineNumber ?? line.oldLineNumber;
          const label = line.kind === "Added" ? "추가" : line.kind === "Removed" ? "삭제" : "변경 없음";
          return e(
            "div",
            {
              className: `diff-line ${kind}`,
              key: `${line.kind}-${line.oldLineNumber ?? "x"}-${line.newLineNumber ?? "x"}-${lineIndex}`,
            },
            e("span", { className: "diff-line-kind", "aria-label": label }, line.kind === "Added" ? "+" : line.kind === "Removed" ? "−" : " "),
            e("span", { className: "diff-line-number" }, lineNumber ?? ""),
            e("code", null, line.text),
          );
        }),
        );
      }),
    ),
  );
}

function renderAttachmentDiff(diff: DocumentAttachmentDiffView): React.ReactElement {
  const e = React.createElement;
  const headingId = "document-diff-attachments-title";
  if (diff.status === "LegacyUnknown") {
    return e(
      "section",
      { className: "diff-attachments legacy-unknown", "aria-labelledby": headingId },
      e("h3", { id: headingId }, KO_KR_MESSAGES.message("diff.attachmentsHeading")),
      e("p", { className: "diff-attachment-unknown" }, KO_KR_MESSAGES.message("diff.attachmentsLegacyUnknown")),
    );
  }

  const hasChanges = diff.added.length > 0 || diff.removed.length > 0 || diff.relabeled.length > 0;
  return e(
    "section",
    { className: "diff-attachments known", "aria-labelledby": headingId },
    e(
      "div",
      { className: "diff-attachments-heading" },
      e("h3", { id: headingId }, KO_KR_MESSAGES.message("diff.attachmentsHeading")),
      diff.unchangedCount > 0
        ? e("span", null, `${KO_KR_MESSAGES.message("diff.attachmentsUnchanged")} ${diff.unchangedCount}개`)
        : null,
    ),
    !hasChanges
      ? e("p", { className: "diff-attachment-empty" }, KO_KR_MESSAGES.message("diff.attachmentsNone"))
      : e(
          "div",
          { className: "diff-attachment-groups" },
          renderAttachmentLabelGroup("added", KO_KR_MESSAGES.message("diff.attachmentsAdded"), diff.added),
          renderAttachmentLabelGroup("removed", KO_KR_MESSAGES.message("diff.attachmentsRemoved"), diff.removed),
          diff.relabeled.length > 0
            ? e(
                "section",
                { className: "diff-attachment-group relabeled" },
                e("strong", null, KO_KR_MESSAGES.message("diff.attachmentsRelabeled")),
                e(
                  "ul",
                  null,
                  diff.relabeled.map((change, index) => e(
                    "li",
                    { key: `${change.beforeLabel}-${change.afterLabel}-${index}` },
                    e("span", null, change.beforeLabel),
                    e("span", { "aria-hidden": "true" }, " → "),
                    e("span", null, change.afterLabel),
                    renderMissingAttachmentStatus(change.availability),
                  )),
                ),
              )
            : null,
        ),
  );
}

function renderAttachmentLabelGroup(
  kind: "added" | "removed",
  label: string,
  items: readonly {
    readonly label: string;
    readonly availability: "Available" | "Missing";
  }[],
): React.ReactElement | null {
  if (items.length === 0) return null;
  return React.createElement(
    "section",
    { className: `diff-attachment-group ${kind}` },
    React.createElement("strong", null, label),
    React.createElement(
      "ul",
      null,
      items.map((item, index) => React.createElement(
        "li",
        { key: `${item.label}-${index}` },
        React.createElement("span", null, item.label),
        renderMissingAttachmentStatus(item.availability),
      )),
    ),
  );
}

function renderMissingAttachmentStatus(
  availability: "Available" | "Missing",
): React.ReactElement | null {
  return availability === "Missing"
    ? React.createElement(
        "span",
        { className: "diff-attachment-missing" },
        KO_KR_MESSAGES.message("diff.attachmentMissing"),
      )
    : null;
}

function CodeMirrorSourceRegion({
  body,
  documentId,
  onChange,
}: {
  readonly body: string;
  readonly documentId?: string;
  readonly onChange: (body: string) => void;
}): React.ReactElement {
  const host = useRef<HTMLDivElement>(null);
  const editor = useRef<CodeMirrorDocumentEditor>();
  const callbacks = useRef({ onChange });
  callbacks.current = { onChange };

  useEffect(() => {
    if (!host.current) return undefined;
    editor.current = mountCodeMirrorDocumentEditor({
      parent: host.current,
      body,
      onChange: (nextBody) => callbacks.current.onChange(nextBody),
    });
    if (host.current) host.current.dataset.codemirrorHost = "mounted";
    return () => {
      editor.current?.destroy();
      editor.current = undefined;
    };
  }, [documentId]);

  useEffect(() => {
    editor.current?.setDocument(body);
  }, [body]);

  return React.createElement(
    "section",
    { className: "markdown-source", "aria-label": "Markdown 원문" },
    React.createElement("div", {
      ref: host,
      className: "codemirror-host",
      "data-codemirror-host": "pending",
    }),
  );
}

function renderRecovery(
  snapshot: DesktopDocumentAuthoringSnapshot,
  callbacks: DesktopDocumentAuthoringWorkbenchCallbacks,
): React.ReactElement | null {
  const e = React.createElement;
  if (snapshot.saveState === DocumentSaveCoordinatorState.SaveFailed) {
    const error = mapUserFacingError({ stableCode: snapshot.errorCode ?? "DOCUMENT_AUTHORING_STORAGE_UNAVAILABLE", retryable: snapshot.retryable ?? false, operationContext: "authoring" });
    return e(
      "div",
      { className: "state-banner failed", role: "alert" },
      e("strong", null, error.title),
      e("span", null, error.message),
      error.recoveryAction === "retry"
        ? e("button", { type: "button", "data-action": "retry-authoring-save", onClick: callbacks.onRetry }, error.recoveryLabel)
        : null,
      e("button", { type: "button", "data-action": "discard-authoring-changes", onClick: callbacks.onDiscard }, "변경 취소"),
      e("button", { type: "button", "data-action": "cancel-authoring-recovery", onClick: callbacks.onCancel }, "계속 편집"),
    );
  }
  if (snapshot.saveState === DocumentSaveCoordinatorState.ReadOnlyRecovery) {
    const error = mapUserFacingError({ stableCode: snapshot.errorCode ?? "DOCUMENT_AUTHORING_POINTER_UPDATE_FAILED", retryable: false, operationContext: "authoring" });
    return e(
      "div",
      { className: "state-banner degraded", role: "alert" },
      e("strong", null, error.title),
      e("span", null, error.message),
      e("button", { type: "button", "data-action": "discard-authoring-changes", onClick: callbacks.onDiscard }, "변경 취소"),
      e("button", { type: "button", "data-action": "cancel-authoring-recovery", onClick: callbacks.onCancel }, "계속 편집"),
    );
  }
  if (snapshot.saveState === DocumentSaveCoordinatorState.CloseBlocked) {
    return e(
      "div",
      { className: "state-banner degraded", role: "alert" },
      e("strong", null, "저장하지 않은 변경이 있습니다"),
      e("span", null, "닫기 전에 저장하거나 변경 내용을 취소해 주세요."),
      e("button", { type: "button", "data-action": "retry-authoring-save", onClick: callbacks.onRetry }, "저장 다시 시도"),
      e("button", { type: "button", "data-action": "discard-authoring-changes", onClick: callbacks.onDiscard }, "변경 취소"),
      e("button", { type: "button", "data-action": "cancel-authoring-recovery", onClick: callbacks.onCancel }, "계속 편집"),
    );
  }
  return null;
}

function renderPlainTextEditorDialog(
  snapshot: DesktopDocumentAuthoringSnapshot,
  callbacks: DesktopDocumentAuthoringWorkbenchCallbacks,
): React.ReactElement {
  const e = React.createElement;
  return e(
    "div",
    {
      className: "plain-text-editor-backdrop",
      "data-editor-surface": "plain-text",
      role: "dialog",
      "aria-modal": "true",
      "aria-label": "Markdown 원문 편집",
    },
    e(
      "section",
      { className: "plain-text-editor-panel" },
      e(
        "header",
        { className: "plain-text-editor-header" },
        e("strong", null, "원문 편집"),
        e(
          "button",
          {
            type: "button",
            "data-action": "close-plain-text-editor",
            "aria-label": "Markdown 원문 편집 닫기",
            onClick: callbacks.onClosePlainTextEditor,
            disabled: !callbacks.onClosePlainTextEditor,
          },
          e(X, { size: 16, "aria-hidden": true }),
        ),
      ),
      e(CodeMirrorSourceRegion, {
        key: "plain-text-source",
        body: snapshot.body ?? "",
        documentId: snapshot.documentId,
        onChange: callbacks.onBodyChange,
      }),
    ),
  );
}

function renderWysiwygBlock(
  block: WysiwygMarkdownBlock,
  body: string,
  documentId: string,
  revision: number,
  callbacks: DesktopDocumentAuthoringWorkbenchCallbacks,
): React.ReactElement {
  const e = React.createElement;
  const blockProps = {
    key: block.blockId,
    className: `wysiwyg-block wysiwyg-block-${block.blockType}`,
    "data-wysiwyg-block-id": block.blockId,
    "data-wysiwyg-block-type": block.blockType,
  };

  switch (block.blockType) {
    case "heading":
      return e(
        `h${Math.min(6, Math.max(1, block.level))}`,
        {
          ...blockProps,
          ...(hasWysiwygReferenceInline(block.inlines)
            ? { "data-wysiwyg-inline-editing": "plain-text" }
            : createEditableWysiwygBlockProps(block, body, documentId, revision, callbacks)),
        },
        renderWysiwygInlineChildren(block.inlines, callbacks),
      );
    case "paragraph":
      return e("p", {
        ...blockProps,
        ...(hasWysiwygReferenceInline(block.inlines)
          ? { "data-wysiwyg-inline-editing": "plain-text" }
          : createEditableWysiwygBlockProps(block, body, documentId, revision, callbacks)),
      }, renderWysiwygInlineChildren(block.inlines, callbacks));
    case "checklist":
      return e(
        "ul",
        { ...blockProps, className: `${blockProps.className} wysiwyg-checklist` },
        block.items.map((item, itemIndex) =>
          e(
            "li",
            { key: itemIndex, "data-wysiwyg-checklist-state": item.checked ? "checked" : "open" },
            e(
              "button",
              {
                type: "button",
                className: "wysiwyg-task-checkbox",
                "data-action": "toggle-wysiwyg-checklist-item",
                "data-wysiwyg-checklist-index": itemIndex,
                "aria-label": item.checked ? "체크리스트 항목 미완료로 표시" : "체크리스트 항목 완료로 표시",
                onClick: () => toggleWysiwygChecklistItem(block, itemIndex, body, documentId, revision, callbacks),
              },
              item.checked ? "☑" : "☐",
            ),
            e("span", null, item.text),
          ),
        ),
      );
    case "table":
      return e(
        "div",
        { ...blockProps, className: `${blockProps.className} wysiwyg-table-scroll` },
        e(
          "table",
          null,
          e(
            "thead",
            null,
            e(
              "tr",
              null,
              block.headers.map((header, cellIndex) =>
                e("th", { key: cellIndex, "data-wysiwyg-table-align": block.alignments[cellIndex] ?? "default" }, header),
              ),
            ),
          ),
          e(
            "tbody",
            null,
            block.rows.map((row, rowIndex) =>
              e(
                "tr",
                { key: rowIndex },
                row.map((cell, cellIndex) =>
                  e(
                    "td",
                    {
                      key: cellIndex,
                      "data-wysiwyg-table-align": block.alignments[cellIndex] ?? "default",
                      "data-wysiwyg-table-row": rowIndex,
                      "data-wysiwyg-table-cell": cellIndex,
                      contentEditable: true,
                      suppressContentEditableWarning: true,
                      role: "textbox",
                      "aria-label": "표 셀 편집",
                      onBlur: (event: React.FocusEvent<HTMLElement>) =>
                        editWysiwygTableCell(block, rowIndex, cellIndex, event.currentTarget.textContent ?? "", body, documentId, revision, callbacks),
                    },
                    cell,
                  ),
                ),
              ),
            ),
          ),
        ),
      );
    case "code_block":
      return renderWysiwygCodeBlock(block, blockProps, callbacks);
    case "blockquote":
      return renderWysiwygBlockquoteBlock(block, blockProps, callbacks);
    case "fallback":
      return e(
        "div",
        { ...blockProps, "data-wysiwyg-fallback-reason": block.fallbackReason },
        e("p", null, block.displayText),
        e(
          "button",
          {
            type: "button",
            "data-action": "edit-wysiwyg-fallback-in-source",
            "aria-label": "원문에서 편집",
            onClick: callbacks.onOpenPlainTextEditor,
            disabled: !callbacks.onOpenPlainTextEditor,
          },
          "원문에서 편집",
        ),
      );
  }
}

function renderWysiwygCodeBlock(
  block: WysiwygMarkdownCodeBlock,
  blockProps: Record<string, unknown>,
  callbacks: DesktopDocumentAuthoringWorkbenchCallbacks,
): React.ReactElement {
  const e = React.createElement;
  return e(
    "figure",
    {
      ...blockProps,
      "data-wysiwyg-code-language": block.language ?? "plain",
    },
    block.language ? e("figcaption", null, block.language) : null,
    e("pre", null, e("code", null, block.displayText)),
    e(
      "button",
      {
        type: "button",
        "data-action": "edit-wysiwyg-code-source",
        "aria-label": "코드 원문에서 편집",
        onClick: callbacks.onOpenPlainTextEditor,
        disabled: !callbacks.onOpenPlainTextEditor,
      },
      "원문에서 편집",
    ),
  );
}

function renderWysiwygBlockquoteBlock(
  block: WysiwygMarkdownBlockquoteBlock,
  blockProps: Record<string, unknown>,
  callbacks: DesktopDocumentAuthoringWorkbenchCallbacks,
): React.ReactElement {
  const e = React.createElement;
  return e(
    "blockquote",
    {
      ...blockProps,
      ...(block.calloutKind ? { "data-wysiwyg-callout-kind": block.calloutKind } : {}),
    },
    block.displayText.split("\n").map((line, index) => e("p", { key: index }, line)),
    e(
      "button",
      {
        type: "button",
        "data-action": "edit-wysiwyg-quote-source",
        "aria-label": "인용 원문에서 편집",
        onClick: callbacks.onOpenPlainTextEditor,
        disabled: !callbacks.onOpenPlainTextEditor,
      },
      "원문에서 편집",
    ),
  );
}

function renderWysiwygInlineChildren(
  inlines: readonly WysiwygMarkdownInlineNode[],
  callbacks: DesktopDocumentAuthoringWorkbenchCallbacks,
): React.ReactNode {
  const children: React.ReactNode[] = inlines.map((inline, index) => renderWysiwygInlineNode(inline, index));
  if (hasWysiwygReferenceInline(inlines)) {
    children.push(
      React.createElement(
        "button",
        {
          key: "edit-inline-source",
          type: "button",
          className: "wysiwyg-inline-source-action",
          "data-action": "edit-wysiwyg-inline-source",
          "aria-label": "원문에서 편집",
          onClick: callbacks.onOpenPlainTextEditor,
          disabled: !callbacks.onOpenPlainTextEditor,
        },
        "원문에서 편집",
      ),
    );
  }
  return children;
}

function renderWysiwygInlineNode(inline: WysiwygMarkdownInlineNode, index: number): React.ReactNode {
  if (inline.inlineType === "text") return inline.text;
  return React.createElement(
    "span",
    {
      key: index,
      className: `wysiwyg-inline-chip wysiwyg-inline-${inline.inlineType.replace("_", "-")}`,
      "data-wysiwyg-inline-type": inline.inlineType,
    },
    inline.text,
  );
}

function hasWysiwygReferenceInline(inlines: readonly WysiwygMarkdownInlineNode[]): boolean {
  return inlines.some((inline) => inline.inlineType !== "text");
}

function toggleWysiwygChecklistItem(
  block: WysiwygMarkdownChecklistBlock,
  itemIndex: number,
  body: string,
  documentId: string,
  revision: number,
  callbacks: DesktopDocumentAuthoringWorkbenchCallbacks,
): void {
  const expectedSourceText = body.slice(block.sourceRange.start, block.sourceRange.end);
  applyGuardedWysiwygPatch(body, documentId, revision, callbacks, () =>
    applyWysiwygMarkdownChecklistItemToggle({
      body,
      sourceRange: block.sourceRange,
      expectedSourceText,
      itemIndex,
    })
  );
}

function editWysiwygTableCell(
  block: WysiwygMarkdownTableBlock,
  rowIndex: number,
  cellIndex: number,
  replacementText: string,
  body: string,
  documentId: string,
  revision: number,
  callbacks: DesktopDocumentAuthoringWorkbenchCallbacks,
): void {
  if ((block.rows[rowIndex]?.[cellIndex] ?? "") === replacementText) return;
  const expectedSourceText = body.slice(block.sourceRange.start, block.sourceRange.end);
  applyGuardedWysiwygPatch(body, documentId, revision, callbacks, () =>
    applyWysiwygMarkdownTableCellEdit({
      body,
      sourceRange: block.sourceRange,
      expectedSourceText,
      rowIndex,
      cellIndex,
      replacementText,
    })
  );
}

function createEditableWysiwygBlockProps(
  block: WysiwygMarkdownHeadingBlock | WysiwygMarkdownParagraphBlock,
  body: string,
  documentId: string,
  revision: number,
  callbacks: DesktopDocumentAuthoringWorkbenchCallbacks,
): Record<string, unknown> {
  return {
    contentEditable: true,
    suppressContentEditableWarning: true,
    role: "textbox",
    "aria-label": block.blockType === "heading" ? "제목 편집" : "문단 편집",
    onBlur(event: React.FocusEvent<HTMLElement>) {
      const text = event.currentTarget.textContent ?? "";
      const replacementSourceText = block.blockType === "heading"
        ? `${"#".repeat(block.level)} ${text.trim()}`
        : text;
      const expectedSourceText = body.slice(block.sourceRange.start, block.sourceRange.end);
      if (replacementSourceText === expectedSourceText) return;
      applyGuardedWysiwygPatch(body, documentId, revision, callbacks, () =>
        applyWysiwygMarkdownBlockTextEdit({
          body,
          sourceRange: block.sourceRange,
          expectedSourceText,
          replacementSourceText,
        })
      );
    },
  };
}

function applyGuardedWysiwygPatch(
  body: string,
  documentId: string,
  revision: number,
  callbacks: DesktopDocumentAuthoringWorkbenchCallbacks,
  apply: () => ReturnType<typeof applyWysiwygMarkdownBlockTextEdit>,
): void {
  const session = createWysiwygPlainTextSyncSession({ documentId, body, revision });
  const result = applyWysiwygPatchToSyncSession(session, {
    baseRevision: revision,
    apply,
  });
  if (result.status === "Applied") {
    callbacks.onBodyChange(result.session.body);
    return;
  }
  callbacks.onOpenPlainTextEditor?.();
}

function saveStateLabel(state: DesktopDocumentAuthoringSnapshot["saveState"]): string {
  switch (state) {
    case DocumentSaveCoordinatorState.Dirty:
    case DocumentSaveCoordinatorState.SaveQueued:
      return "저장되지 않음";
    case DocumentSaveCoordinatorState.Saving:
      return "저장 중";
    case DocumentSaveCoordinatorState.SaveFailed:
      return "저장 실패";
    case DocumentSaveCoordinatorState.ReadOnlyRecovery:
      return "읽기 전용";
    case DocumentSaveCoordinatorState.Saved:
      return "저장됨";
    default:
      return "로컬";
  }
}
