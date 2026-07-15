import React, { useEffect, useRef } from "react";

import {
  DocumentSaveCoordinatorState,
  createMarkdownPreviewModel,
  type DocumentEditorViewMode,
  type MarkdownPreviewBlock,
} from "@sponzey-cabinet/ui";

import type { DesktopDocumentAuthoringSnapshot } from "./desktop_document_authoring_controller.ts";
import type { DesktopLinkOverviewSnapshot } from "./desktop_link_overview_controller.ts";
import {
  mountCodeMirrorDocumentEditor,
  type CodeMirrorDocumentEditor,
} from "./codemirror_document_editor.ts";
import { createWorkspaceShellModel, type WorkspaceShellRouteKind } from "./workspace_shell_contract.ts";
import { createWorkspaceShellElement } from "./react_workspace_shell.ts";
import { KO_KR_MESSAGES } from "./ko_kr_catalog.ts";
import { mapUserFacingError } from "./user_facing_error_presenter.ts";

const shellRoutes: readonly WorkspaceShellRouteKind[] = ["Home", "Search", "Document", "Graph", "Canvas", "Assets", "Backup"];

export interface DesktopDocumentAuthoringWorkbenchCallbacks {
  readonly onHome: () => void;
  readonly onMode: (mode: DocumentEditorViewMode) => void;
  readonly onBodyChange: (body: string) => void;
  readonly onSave: () => void;
  readonly onRetry: () => void;
  readonly onDiscard: () => void;
  readonly onCancel: () => void;
  readonly onLoadHistory?: () => void;
  readonly onPreviewRestore?: (versionId: string) => void;
  readonly onApplyRestore?: () => void;
  readonly onSearch?: () => void;
  readonly onGraph?: () => void;
  readonly onCanvas?: () => void;
  readonly onAssets?: () => void;
  readonly onBackup?: () => void;
  readonly onCreateDocument?: () => void;
  readonly onOpenLinkedDocument?: (documentId: string) => void;
}

export interface DesktopDocumentAuthoringWorkbenchOptions {
  readonly viewMode?: DocumentEditorViewMode;
  readonly history?: DesktopDocumentHistoryWorkbenchState;
  readonly links?: DesktopLinkOverviewSnapshot;
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
  readonly status: "Idle" | "Loading" | "Ready" | "PreviewReady" | "Applying" | "Applied" | "Failed";
  readonly entries: readonly DesktopDocumentHistoryEntryView[];
  readonly preview?: DesktopDocumentRestorePreviewView;
  readonly errorCode?: string;
}

interface WorkbenchProps {
  readonly snapshot: DesktopDocumentAuthoringSnapshot;
  readonly callbacks: DesktopDocumentAuthoringWorkbenchCallbacks;
  readonly viewMode: DocumentEditorViewMode;
  readonly history: DesktopDocumentHistoryWorkbenchState;
  readonly links?: DesktopLinkOverviewSnapshot;
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
  });
}

function DesktopDocumentAuthoringWorkbench({
  snapshot,
  callbacks,
  viewMode,
  history,
  links,
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
          { className: `authoring-workspace mode-${viewMode}` },
          showSource
            ? e(CodeMirrorSourceRegion, {
                key: "source",
                body,
                documentId: snapshot.documentId,
                onChange: callbacks.onBodyChange,
              })
            : null,
          showPreview
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
          renderConnectedDocuments(links, callbacks),
          renderHistoryRestorePanel(history, callbacks),
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
    e(
      "ol",
      { className: "history-list" },
      history.entries.map((entry) =>
        e(
          "li",
          { key: entry.versionId },
          e(
            "div",
            { className: "history-entry-display" },
            e("strong", null, entry.versionLabel),
            e("span", null, entry.createdAtLabel),
            e("span", null, entry.summaryLabel),
          ),
          e(
            "button",
            {
              type: "button",
              "data-action": "preview-restore",
              "data-version-id": entry.versionId,
              onClick: () => callbacks.onPreviewRestore?.(entry.versionId),
              disabled: !callbacks.onPreviewRestore,
            },
            "복원 미리보기",
          ),
        ),
      ),
    ),
    history.preview
      ? e(
          "div",
          {
            className: "restore-preview-summary",
            "data-restore-preview-target": history.preview.targetVersionId,
          },
          e("strong", null, "복원 미리보기"),
          e("span", null, `${history.preview.changedLineCount}개 줄 변경`),
          e(
            "button",
            {
              type: "button",
              className: "primary",
              "data-action": "apply-restore",
              onClick: callbacks.onApplyRestore,
              disabled: !callbacks.onApplyRestore || !history.preview.canRestore,
            },
            "이 버전으로 복원",
          ),
        )
      : null,
  );
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
