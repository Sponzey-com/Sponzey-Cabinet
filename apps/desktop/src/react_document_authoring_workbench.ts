import React, { useEffect, useRef } from "react";
import { History, Link2, Paperclip, RotateCcw, X } from "lucide-react";

import {
  DocumentSaveCoordinatorState,
  createMarkdownPreviewModel,
  type DocumentEditorViewMode,
  type MarkdownPreviewBlock,
} from "@sponzey-cabinet/ui";

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
import { createWorkspaceShellElement } from "./react_workspace_shell.ts";
import { formatHistoryRangeKoKr, KO_KR_MESSAGES } from "./ko_kr_catalog.ts";
import { mapUserFacingError } from "./user_facing_error_presenter.ts";
import type { DesktopAssetSurfaceSnapshot } from "./desktop_asset_controller.ts";
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

const shellRoutes: readonly WorkspaceShellRouteKind[] = ["Home", "Search", "Document", "Graph", "Canvas", "Assets", "Backup"];

export interface DesktopDocumentAuthoringWorkbenchCallbacks extends DocumentAttachmentPanelCallbacks {
  readonly onHome: () => void;
  readonly onMode: (mode: DocumentEditorViewMode) => void;
  readonly onBodyChange: (body: string) => void;
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
  readonly onSearch?: () => void;
  readonly onGraph?: () => void;
  readonly onCanvas?: () => void;
  readonly onAssets?: () => void;
  readonly onBackup?: () => void;
  readonly onCreateDocument?: () => void;
  readonly onOpenLinkedDocument?: (documentId: string) => void;
  readonly onInspectorTab?: (tab: DocumentInspectorTab) => void;
}

export interface DesktopDocumentAuthoringWorkbenchOptions {
  readonly viewMode?: DocumentEditorViewMode;
  readonly history?: DesktopDocumentHistoryWorkbenchState;
  readonly links?: DesktopLinkOverviewSnapshot;
  readonly assets?: DesktopAssetSurfaceSnapshot;
  readonly inspector?: DocumentInspectorState;
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
  readonly viewMode: DocumentEditorViewMode;
  readonly history: DesktopDocumentHistoryWorkbenchState;
  readonly links?: DesktopLinkOverviewSnapshot;
  readonly assets?: DesktopAssetSurfaceSnapshot;
  readonly inspector: DocumentInspectorState;
}

export function createDesktopDocumentAuthoringWorkbenchElement(
  snapshot: DesktopDocumentAuthoringSnapshot,
  callbacks: DesktopDocumentAuthoringWorkbenchCallbacks,
  options: DesktopDocumentAuthoringWorkbenchOptions = {},
): React.ReactElement {
  return React.createElement(DesktopDocumentAuthoringWorkbench, {
    snapshot,
    callbacks,
    viewMode: options.viewMode ?? "split",
    history: options.history ?? { status: "Idle", entries: [] },
    links: options.links,
    assets: options.assets,
    inspector: options.inspector ?? createDocumentInspectorState(),
  });
}

function DesktopDocumentAuthoringWorkbench({
  snapshot,
  callbacks,
  viewMode,
  history,
  links,
  assets,
  inspector,
}: WorkbenchProps): React.ReactElement {
  const e = React.createElement;
  const showSource = viewMode === "source" || viewMode === "split";
  const showPreview = viewMode === "preview" || viewMode === "split";
  const body = snapshot.body ?? "";
  const preview = createMarkdownPreviewModel({
    documentId: snapshot.documentId ?? "unloaded",
    versionId: snapshot.expectedVersionId ?? "unloaded",
    source: body,
  });
  const restoreConfirmation = history.restore?.status === "Confirming" ? history.restore : undefined;

  const topbar = e(
      React.Fragment,
      null,
      e(
        "button",
        { type: "button", className: "authoring-breadcrumb", "data-action": "authoring-home", onClick: callbacks.onHome },
        "내 캐비닛 / 프로젝트 / Cabinet",
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
          { className: "editor-mode-control", role: "group", "aria-label": "편집 화면" },
          (["source", "split", "preview"] as const).map((mode) =>
            e(
              "button",
              {
                key: mode,
                type: "button",
                "data-action": `authoring-mode-${mode}`,
                "data-editor-mode": mode,
                "aria-pressed": viewMode === mode,
                onClick: () => callbacks.onMode(mode),
              },
              mode === "source" ? "원문" : mode === "split" ? "나란히" : "미리보기",
            ),
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
          { className: history.diff || restoreConfirmation ? "authoring-workspace mode-compare" : `authoring-workspace mode-${viewMode}` },
          history.diff
            ? renderDocumentDiff(history.diff, callbacks)
            : restoreConfirmation
              ? renderRestoreConfirmation(restoreConfirmation, callbacks)
            : showSource
            ? e(CodeMirrorSourceRegion, {
                key: "source",
                body,
                documentId: snapshot.documentId,
                onChange: callbacks.onBodyChange,
              })
            : null,
          !history.diff && !restoreConfirmation && showPreview
            ? e(
                "section",
                { className: "markdown-preview", "aria-label": "Markdown 미리보기" },
                preview.blocks.map((block, index) => renderPreviewBlock(block, index)),
              )
            : null,
        ),
        e(
          "aside",
          { className: "authoring-context-column", "aria-label": "문서 정보" },
          renderAuthoringKnowledgeMap(callbacks.onGraph),
          renderDocumentInspector(inspector, links, assets, history, callbacks),
        ),
      ),
    );
  return createWorkspaceShellElement({
    model: createWorkspaceShellModel({ route: "Document", availableActions: shellRoutes, messages: KO_KR_MESSAGES }),
    messages: KO_KR_MESSAGES,
    routeActions: { Home: callbacks.onHome, Search: callbacks.onSearch, Graph: callbacks.onGraph, Canvas: callbacks.onCanvas, Assets: callbacks.onAssets, Backup: callbacks.onBackup },
    onCreateDocument: callbacks.onCreateDocument,
    onSearch: callbacks.onSearch,
    rootClassName: "authoring-shell",
    rootAttributes: {
      "data-cabinet-authoring-state": snapshot.saveState,
      "data-document-id": snapshot.documentId ?? "",
      "data-document-revision": String(snapshot.revision),
      "data-persisted-revision": String(snapshot.persistedRevision),
    },
    topbarContent: topbar,
    globalLayer: renderRecovery(snapshot, callbacks),
    documentShortcuts: [{ label: snapshot.title ?? "Untitled", actionId: "current-document" }],
    content: main,
  });
}

function renderDocumentInspector(
  inspector: DocumentInspectorState,
  links: DesktopLinkOverviewSnapshot | undefined,
  assets: DesktopAssetSurfaceSnapshot | undefined,
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
        ? createDocumentAttachmentPanelElement(assets, callbacks, inspector.unlink)
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

function renderAuthoringKnowledgeMap(onOpenGraph?: () => void): React.ReactElement {
  const e = React.createElement;
  return e(
    "section",
    { className: "overview-card authoring-map-card", "aria-labelledby": "authoring-map-title" },
    e("div", { className: "section-heading" }, e("h2", { id: "authoring-map-title" }, "내 지식 지도"), e("button", { type: "button", className: "text-action", "data-action": "open-authoring-graph", disabled: !onOpenGraph, onClick: onOpenGraph }, "전체 화면")),
    e("div", { className: "authoring-map-preview", "aria-hidden": "true" }, e("span", { className: "map-spoke spoke-a" }), e("span", { className: "map-spoke spoke-b" }), e("i", { className: "map-dot dot-center" }), e("i", { className: "map-dot dot-a" }), e("i", { className: "map-dot dot-b" }), e("i", { className: "map-dot dot-c" })),
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

function renderPreviewBlock(block: MarkdownPreviewBlock, index: number): React.ReactElement {
  const e = React.createElement;
  const key = `${block.kind}-${index}`;
  switch (block.kind) {
    case "heading":
      return e(`h${Math.min(6, Math.max(1, block.level))}`, { key, id: block.anchor }, block.text);
    case "paragraph":
      return e("p", { key }, block.text);
    case "table":
      return e(
        "div",
        { key, className: "preview-table-scroll" },
        e(
          "table",
          null,
          e("thead", null, e("tr", null, block.headers.map((header, cell) => e("th", { key: cell }, header)))),
          e(
            "tbody",
            null,
            block.rows.map((row, rowIndex) =>
              e("tr", { key: rowIndex }, row.map((cell, cellIndex) => e("td", { key: cellIndex }, cell))),
            ),
          ),
        ),
      );
    case "checklist":
      return e(
        "ul",
        { key, className: "preview-checklist" },
        block.items.map((item, itemIndex) =>
          e("li", { key: itemIndex }, e("span", { className: "task-checkbox", "aria-hidden": "true" }, item.checked ? "☑" : "☐"), item.text),
        ),
      );
    case "code":
      return e("pre", { key }, e("code", null, `${block.lineCount} lines${block.language ? ` · ${block.language}` : ""}`));
    case "blockquote":
      return e("blockquote", { key }, block.text);
    case "callout":
      return e("aside", { key, className: "preview-callout" }, e("strong", null, block.title), e("p", null, block.text));
  }
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
