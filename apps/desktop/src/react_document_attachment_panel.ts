import React from "react";
import { CircleAlert, CircleCheck, ExternalLink, Eye, FileText, LoaderCircle, Paperclip, Plus, RotateCcw, Trash2, X } from "lucide-react";

import type { DesktopAssetSurfaceSnapshot } from "./desktop_asset_controller.ts";
import { presentAssetMetadata } from "./asset_display_presenter.ts";
import { handleModalKeyboard } from "./modal_keyboard_policy.ts";
import {
  browserModalFocusEnvironment,
  createFocusRestoringModalAction,
} from "./modal_focus_restoration.ts";
import type { DocumentAttachmentUnlinkState } from "./document_inspector_state.ts";
import type { DocumentAssetLibraryState } from "./document_asset_library_state.ts";

export interface DocumentAttachmentPanelCallbacks {
  readonly onAssetImport?: () => void;
  readonly onAssetRetry?: () => void;
  readonly onAssetCancel?: () => void;
  readonly onAssetRepair?: (operationId: string) => void;
  readonly onAssetSelect?: (assetId: string) => void;
  readonly onAssetPreview?: () => void;
  readonly onAssetPreviewClose?: () => void;
  readonly onAssetOpen?: () => void;
  readonly onAssetUnlinkRequest?: () => void;
  readonly onAssetUnlinkCancel?: () => void;
  readonly onAssetUnlinkConfirm?: () => void;
  readonly onOpenLibrary?: () => void;
  readonly onAssetLibraryClose?: () => void;
  readonly onAssetLibraryRetry?: () => void;
  readonly onAssetLibrarySelect?: (assetId: string) => void;
  readonly onAssetLibraryLink?: () => void;
  readonly onAssetLibraryLoadMore?: () => void;
}

const ICON_PROPS = Object.freeze({ size: 14, strokeWidth: 2, "aria-hidden": true });

export function createDocumentAttachmentPanelElement(
  snapshot: DesktopAssetSurfaceSnapshot,
  callbacks: DocumentAttachmentPanelCallbacks,
  unlinkState: DocumentAttachmentUnlinkState = Object.freeze({ status: "Closed" }),
  libraryState?: DocumentAssetLibraryState,
): React.ReactElement {
  const e = React.createElement;
  const assets = snapshot.page?.assets ?? [];
  const selected = assets.find((asset) => asset.assetId === snapshot.selectedAssetId);
  const importOperations = snapshot.importOperations ?? [];
  const importBusy = snapshot.importState === "Selecting" || snapshot.importState === "Importing";
  const hasCompletedImport = importOperations.some((operation) => operation.stage === "Completed");

  return e(
    "section",
    {
      className: snapshot.dropState === "Entered" ? "document-attachment-panel drop-entered" : "document-attachment-panel",
      "aria-labelledby": "document-attachments-title",
      "data-document-attachment-state": snapshot.state,
      "data-document-attachment-import-state": snapshot.importState,
      "data-document-attachment-mutation-state": snapshot.mutationState ?? "Idle",
      "data-document-attachment-drop-state": snapshot.dropState ?? "Idle",
    },
    e(
      "div",
      { className: "document-attachment-heading" },
      e("div", null, e(Paperclip, ICON_PROPS), e("strong", { id: "document-attachments-title" }, "첨부 파일"), e("span", null, `${assets.length}개`)),
      e(
        "button",
        {
          type: "button",
          className: "icon-text-action",
          "data-action": "import-document-asset",
          disabled: !callbacks.onAssetImport || !snapshot.documentId || importBusy,
          onClick: callbacks.onAssetImport,
        },
        e(Plus, ICON_PROPS),
        importBusy ? "가져오는 중" : "파일 추가",
      ),
    ),
    snapshot.dropState === "Entered"
      ? e(
          "div",
          { className: "document-attachment-drop-target", role: "status" },
          e(Paperclip, ICON_PROPS),
          e("strong", null, "여기에 놓아 첨부"),
          e("span", null, `${snapshot.dropFileCount ?? 0}개 파일`),
        )
      : null,
    snapshot.importState === "Importing"
      ? e(
          "div",
          { className: "document-attachment-status", role: "status" },
          e("span", null, "파일을 안전하게 저장하고 있습니다."),
          importOperations.length === 0
            ? e("button", { type: "button", "data-action": "cancel-document-asset-import", disabled: !callbacks.onAssetCancel, onClick: callbacks.onAssetCancel }, e(X, ICON_PROPS), "취소")
            : null,
        )
      : null,
    snapshot.importState === "Completed"
      ? e("p", { className: "document-attachment-status", role: "status" }, "파일이 문서에 첨부되었습니다.")
      : null,
    snapshot.importState === "Failed"
      ? e("div", { className: "document-attachment-status failed", role: "alert" }, e("span", null, hasCompletedImport ? "일부 파일은 첨부되었고 나머지는 확인이 필요합니다." : "파일을 첨부하지 못했습니다."), e("button", { type: "button", "data-action": "retry-document-asset-import", disabled: !callbacks.onAssetImport, onClick: callbacks.onAssetImport }, "새로 시도"))
      : null,
    createAttachmentOperationListElement(snapshot, {
      onCancel: callbacks.onAssetCancel,
      onRepair: callbacks.onAssetRepair,
      onStartNewAttempt: callbacks.onAssetImport,
      cancelActionId: "cancel-document-asset-import",
      repairActionId: "repair-document-asset-import",
      restartActionId: "retry-document-asset-import",
    }),
    snapshot.state === "Loading"
      ? e("p", { className: "document-attachment-empty", role: "status" }, "첨부 파일을 불러오는 중입니다.")
      : snapshot.state === "Failed"
        ? e("div", { className: "document-attachment-empty", role: "alert" }, e("span", null, "첨부 파일을 불러오지 못했습니다."), e("button", { type: "button", "data-action": "retry-document-assets", disabled: !callbacks.onAssetRetry, onClick: callbacks.onAssetRetry }, "다시 시도"))
        : assets.length === 0
          ? e("div", { className: "document-attachment-empty" }, e(FileText, ICON_PROPS), e("span", null, "이 문서에 첨부된 파일이 없습니다."))
          : e(
              "ul",
              { className: "document-attachment-list", "aria-label": "이 문서의 첨부 파일" },
              assets.map((asset) => {
                const presentation = presentAssetMetadata({ mediaType: asset.mediaType, byteSize: asset.byteSize, status: asset.status });
                return e(
                  "li",
                  { key: asset.assetId },
                  e(
                    "button",
                    {
                      type: "button",
                      className: snapshot.selectedAssetId === asset.assetId ? "selected" : undefined,
                      "data-action": "select-document-asset",
                      "data-asset-id": asset.assetId,
                      "aria-pressed": snapshot.selectedAssetId === asset.assetId,
                      disabled: !callbacks.onAssetSelect,
                      onClick: () => callbacks.onAssetSelect?.(asset.assetId),
                    },
                    e(FileText, ICON_PROPS),
                    e("span", null, e("strong", null, asset.fileName), e("small", null, `${presentation.mediaTypeLabel} · ${presentation.sizeLabel}`)),
                  ),
                );
              }),
            ),
    selected
      ? e(
          "div",
          { className: "document-attachment-actions", "aria-label": `${selected.fileName} 작업` },
          e("span", { className: "document-attachment-label" }, selected.label),
          e("button", { type: "button", "data-action": "preview-document-asset", disabled: !callbacks.onAssetPreview || snapshot.detailState !== "Ready" || snapshot.previewState === "Loading", onClick: callbacks.onAssetPreview }, e(Eye, ICON_PROPS), "미리보기"),
          e(
            "button",
            {
              type: "button",
              "data-action": "open-document-asset-externally",
              disabled: !callbacks.onAssetOpen || snapshot.openState === "Opening",
              onClick: callbacks.onAssetOpen,
            },
            e(ExternalLink, ICON_PROPS),
            snapshot.openState === "Opening" ? "여는 중" : "기본 앱으로 열기",
          ),
          e("button", { type: "button", className: "danger-text-action", "data-action": "unlink-document-asset", disabled: !callbacks.onAssetUnlinkRequest || snapshot.mutationState === "Unlinking", onClick: callbacks.onAssetUnlinkRequest }, e(Trash2, ICON_PROPS), snapshot.mutationState === "Unlinking" ? "해제 중" : "연결 해제"),
        )
      : null,
    snapshot.openState === "Opened"
      ? e("p", { className: "document-attachment-status", role: "status", "data-document-asset-open-state": "Opened" }, "기본 앱에서 파일을 열었습니다.")
      : snapshot.openState === "OpenFailed"
        ? e("p", { className: "document-attachment-status failed", role: "alert", "data-document-asset-open-state": "OpenFailed" }, "파일을 열지 못했습니다. 다시 시도할 수 있습니다.")
        : snapshot.openState === "Opening"
          ? e("p", { className: "document-attachment-status", role: "status", "data-document-asset-open-state": "Opening" }, "기본 앱을 여는 중입니다.")
          : null,
    e("button", { type: "button", className: "text-action document-attachment-library", "data-action": "open-document-asset-library", disabled: !callbacks.onOpenLibrary, onClick: callbacks.onOpenLibrary }, "기존 파일 연결"),
    libraryState && libraryState.status !== "Closed"
      ? e(DocumentAssetLibraryDialog, { state: libraryState, callbacks })
      : null,
    renderPreview(snapshot, callbacks),
    renderUnlinkConfirmation(unlinkState, callbacks),
  );
}

function DocumentAssetLibraryDialog({
  state,
  callbacks,
}: {
  readonly state: DocumentAssetLibraryState;
  readonly callbacks: DocumentAttachmentPanelCallbacks;
}): React.ReactElement {
  const e = React.createElement;
  const [query, setQuery] = React.useState("");
  const normalizedQuery = query.trim().toLocaleLowerCase("ko-KR");
  const assets = state.assets.page?.assets ?? [];
  const visible = assets.filter((asset) => !normalizedQuery
    || asset.fileName.toLocaleLowerCase("ko-KR").includes(normalizedQuery)
    || asset.label.toLocaleLowerCase("ko-KR").includes(normalizedQuery));
  const selected = state.assets.selectedAssetId;
  const resultList = visible.length === 0
    ? e("p", { role: "status", className: "document-asset-library-message" }, assets.length === 0 ? "보관함에 파일이 없습니다." : "일치하는 파일이 없습니다.")
    : e("ul", { className: "document-asset-library-list", "aria-label": "연결할 기존 파일" }, visible.map((asset) => {
        const presentation = presentAssetMetadata({ mediaType: asset.mediaType, byteSize: asset.byteSize, status: asset.status });
        return e("li", { key: asset.assetId }, e("button", {
          type: "button",
          className: selected === asset.assetId ? "selected" : undefined,
          "data-action": "select-existing-document-asset",
          "aria-pressed": selected === asset.assetId,
          onClick: () => callbacks.onAssetLibrarySelect?.(asset.assetId),
        }, e(FileText, ICON_PROPS), e("span", null, e("strong", null, asset.fileName), e("small", null, `${presentation.mediaTypeLabel} · ${presentation.sizeLabel}`))));
      }));
  const closeAndRestore = createFocusRestoringModalAction(
    callbacks.onAssetLibraryClose ?? (() => {}),
    browserModalFocusEnvironment("open-document-asset-library"),
  );
  return e(
    "div",
    {
      role: "dialog",
      "aria-modal": "true",
      "aria-labelledby": "document-asset-library-title",
      className: "asset-preview-dialog document-asset-library-dialog",
      "data-document-asset-library-state": state.status,
      onKeyDown: (event: React.KeyboardEvent<HTMLDivElement>) => handleModalKeyboard(event, closeAndRestore),
    },
    e("div", { className: "document-asset-library-heading" },
      e("strong", { id: "document-asset-library-title" }, "기존 파일 연결"),
      e("button", { type: "button", className: "icon-action", title: "닫기", "aria-label": "기존 파일 연결 닫기", "data-action": "close-document-asset-library", onClick: closeAndRestore }, e(X, ICON_PROPS)),
    ),
    e("label", { className: "document-asset-library-search" },
      e("span", null, "파일명으로 검색"),
      e("input", { type: "search", value: query, placeholder: "파일명으로 검색", "data-action": "search-document-asset-library", onChange: (event: React.ChangeEvent<HTMLInputElement>) => setQuery(event.currentTarget.value) }),
    ),
    state.status === "Loading"
      ? e("p", { role: "status", className: "document-asset-library-message" }, "파일 보관함을 불러오는 중입니다.")
      : e(React.Fragment, null,
          state.status === "Failed"
            ? e("div", { role: "alert", className: "document-asset-library-message compact" }, e("span", null, "파일 보관함을 불러오거나 연결하지 못했습니다."), e("button", { type: "button", "data-action": "retry-document-asset-library", disabled: !callbacks.onAssetLibraryRetry, onClick: callbacks.onAssetLibraryRetry }, "다시 시도"))
            : null,
          resultList,
        ),
    state.status === "LoadingMore"
      ? e("p", { role: "status", className: "document-asset-library-page-status" }, "파일을 더 불러오는 중입니다.")
      : null,
    state.assets.page?.nextCursor
      ? e("button", { type: "button", className: "text-action document-asset-library-more", "data-action": "load-more-document-asset-library", disabled: state.status === "LoadingMore" || !callbacks.onAssetLibraryLoadMore, onClick: callbacks.onAssetLibraryLoadMore }, state.status === "LoadingMore" ? "불러오는 중" : "더 불러오기")
      : null,
    e("div", { className: "document-asset-library-actions" },
      e("button", { type: "button", "data-action": "close-document-asset-library", onClick: closeAndRestore }, "취소"),
      e("button", { type: "button", className: "primary", "data-action": "link-existing-document-asset", disabled: !selected || state.status !== "Ready" || !callbacks.onAssetLibraryLink, onClick: callbacks.onAssetLibraryLink }, state.status === "Linking" ? "연결 중" : "문서에 연결"),
    ),
  );
}

export function createAttachmentOperationListElement(
  snapshot: DesktopAssetSurfaceSnapshot,
  callbacks: {
    readonly onCancel?: () => void;
    readonly onRepair?: (operationId: string) => void;
    readonly onStartNewAttempt?: () => void;
    readonly cancelActionId: string;
    readonly repairActionId: string;
    readonly restartActionId: string;
  },
): React.ReactElement | null {
  const operations = snapshot.importOperations ?? [];
  if (operations.length === 0) return null;
  const e = React.createElement;
  return e(
    "ul",
    {
      className: "document-attachment-operation-list",
      "aria-label": "첨부 진행 상태",
      "aria-live": "polite",
    },
    operations.map((operation) => {
      const currentCancellable = operation.operationId === snapshot.importOperationId
        && operation.canCancel
        && Boolean(callbacks.onCancel);
      const StatusIcon = operation.stage === "Completed"
        ? CircleCheck
        : operation.stage === "Failed" || operation.stage === "Conflict" || operation.stage === "RecoveryRequired"
          ? CircleAlert
          : LoaderCircle;
      return e(
        "li",
        {
          key: operation.operationId,
          className: `document-attachment-operation stage-${operation.stage.toLowerCase()}`,
          "data-attachment-operation-stage": operation.stage,
        },
        e(StatusIcon, ICON_PROPS),
        e(
          "span",
          { className: "document-attachment-operation-copy" },
          e("strong", null, operation.displayName),
          e("small", null, operation.userLabel),
        ),
        operation.terminal || operation.canRepair || operation.canRetry
          ? null
          : e("progress", {
              value: operation.progressPercent,
              max: 100,
              "aria-label": `${operation.displayName} 진행률`,
            }),
        currentCancellable
          ? e(
              "button",
              {
                type: "button",
                className: "icon-action",
                title: "첨부 취소",
                "aria-label": `${operation.displayName} 첨부 취소`,
                "data-action": callbacks.cancelActionId,
                onClick: callbacks.onCancel,
              },
              e(X, ICON_PROPS),
            )
          : operation.canRepair && callbacks.onRepair
            ? e(
                "button",
                {
                  type: "button",
                  className: "icon-action",
                  title: "첨부 복구",
                  "aria-label": `${operation.displayName} 첨부 복구`,
                  "data-action": callbacks.repairActionId,
                  onClick: () => callbacks.onRepair?.(operation.operationId),
                },
                e(RotateCcw, ICON_PROPS),
              )
          : (operation.canStartNewAttempt || operation.canRetry) && callbacks.onStartNewAttempt
            ? e(
                "button",
                {
                  type: "button",
                  className: "text-action",
                  "data-action": callbacks.restartActionId,
                  onClick: callbacks.onStartNewAttempt,
                },
                "파일 다시 선택",
              )
            : null,
      );
    }),
  );
}

function renderPreview(
  snapshot: DesktopAssetSurfaceSnapshot,
  callbacks: DocumentAttachmentPanelCallbacks,
): React.ReactElement | null {
  const e = React.createElement;
  if (!snapshot.previewState || ["Idle", "Loading"].includes(snapshot.previewState)) return null;
  const closeAndRestore = createFocusRestoringModalAction(
    callbacks.onAssetPreviewClose ?? (() => {}),
    browserModalFocusEnvironment("preview-document-asset"),
  );
  const content = snapshot.previewState === "Ready" && snapshot.preview?.presentation === "text"
    ? e("pre", { className: "asset-preview-text" }, snapshot.preview.content)
    : snapshot.previewState === "Ready" && snapshot.preview?.presentation === "data_url" && snapshot.preview.capability === "image"
      ? e("img", { src: snapshot.preview.content, alt: "선택한 첨부 파일 미리보기" })
      : snapshot.previewState === "Ready" && snapshot.preview?.presentation === "data_url"
        ? e("iframe", { src: snapshot.preview.content, title: "선택한 첨부 파일 미리보기" })
        : e("p", { role: snapshot.previewState === "Failed" ? "alert" : "status" }, snapshot.previewState === "Unsupported" ? "이 파일 형식은 미리보기를 지원하지 않습니다." : "미리보기를 불러오지 못했습니다.");
  return e(
    "div",
    { role: "dialog", "aria-modal": "true", "aria-label": "첨부 파일 미리보기", className: "asset-preview-dialog document-asset-preview-dialog", onKeyDown: (event: React.KeyboardEvent<HTMLDivElement>) => handleModalKeyboard(event, closeAndRestore) },
    content,
    snapshot.previewState === "Failed" ? e("button", { type: "button", "data-action": "retry-document-asset-preview", disabled: !callbacks.onAssetPreview, onClick: callbacks.onAssetPreview }, "다시 시도") : null,
    e("button", { type: "button", "data-action": "close-document-asset-preview", disabled: !callbacks.onAssetPreviewClose, onClick: closeAndRestore }, "닫기"),
  );
}

function renderUnlinkConfirmation(
  state: DocumentAttachmentUnlinkState,
  callbacks: DocumentAttachmentPanelCallbacks,
): React.ReactElement | null {
  if (state.status === "Closed") return null;
  const e = React.createElement;
  const submitting = state.status === "Submitting";
  const cancelAndRestore = createFocusRestoringModalAction(
    callbacks.onAssetUnlinkCancel ?? (() => {}),
    browserModalFocusEnvironment("unlink-document-asset"),
  );
  return e(
    "div",
    {
      role: "dialog",
      "aria-modal": "true",
      "aria-labelledby": "document-asset-unlink-title",
      className: "asset-preview-dialog document-asset-unlink-dialog",
      "data-document-asset-unlink-state": state.status,
      onKeyDown: (event: React.KeyboardEvent<HTMLDivElement>) => handleModalKeyboard(event, cancelAndRestore),
    },
    e("strong", { id: "document-asset-unlink-title" }, "첨부 연결을 해제할까요?"),
    e("p", null, e("b", null, state.fileName), "의 문서 연결만 해제하며 파일은 보관함에 남습니다."),
    state.status === "Failed" ? e("p", { role: "alert" }, "연결을 해제하지 못했습니다. 다시 시도할 수 있습니다.") : null,
    e(
      "div",
      { className: "dialog-actions" },
      e("button", { type: "button", "data-action": "cancel-document-asset-unlink", disabled: submitting || !callbacks.onAssetUnlinkCancel, onClick: cancelAndRestore }, "취소"),
      e("button", { type: "button", className: "danger-text-action", "data-action": "confirm-document-asset-unlink", disabled: submitting || !callbacks.onAssetUnlinkConfirm, onClick: callbacks.onAssetUnlinkConfirm }, submitting ? "해제 중" : state.status === "Failed" ? "다시 시도" : "연결 해제"),
    ),
  );
}
