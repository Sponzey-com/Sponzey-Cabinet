import React from "react";
import { ExternalLink, Eye, FileText, Paperclip, Plus, Trash2, X } from "lucide-react";

import type { DesktopAssetSurfaceSnapshot } from "./desktop_asset_controller.ts";
import { presentAssetMetadata } from "./asset_display_presenter.ts";
import { handleModalKeyboard } from "./modal_keyboard_policy.ts";
import {
  browserModalFocusEnvironment,
  createFocusRestoringModalAction,
} from "./modal_focus_restoration.ts";
import type { DocumentAttachmentUnlinkState } from "./document_inspector_state.ts";

export interface DocumentAttachmentPanelCallbacks {
  readonly onAssetImport?: () => void;
  readonly onAssetRetry?: () => void;
  readonly onAssetCancel?: () => void;
  readonly onAssetSelect?: (assetId: string) => void;
  readonly onAssetPreview?: () => void;
  readonly onAssetPreviewClose?: () => void;
  readonly onAssetOpen?: () => void;
  readonly onAssetUnlinkRequest?: () => void;
  readonly onAssetUnlinkCancel?: () => void;
  readonly onAssetUnlinkConfirm?: () => void;
  readonly onOpenLibrary?: () => void;
}

const ICON_PROPS = Object.freeze({ size: 14, strokeWidth: 2, "aria-hidden": true });

export function createDocumentAttachmentPanelElement(
  snapshot: DesktopAssetSurfaceSnapshot,
  callbacks: DocumentAttachmentPanelCallbacks,
  unlinkState: DocumentAttachmentUnlinkState = Object.freeze({ status: "Closed" }),
): React.ReactElement {
  const e = React.createElement;
  const assets = snapshot.page?.assets ?? [];
  const selected = assets.find((asset) => asset.assetId === snapshot.selectedAssetId);
  const importBusy = snapshot.importState === "Selecting" || snapshot.importState === "Importing";

  return e(
    "section",
    {
      className: "document-attachment-panel",
      "aria-labelledby": "document-attachments-title",
      "data-document-attachment-state": snapshot.state,
      "data-document-attachment-import-state": snapshot.importState,
      "data-document-attachment-mutation-state": snapshot.mutationState ?? "Idle",
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
    snapshot.importState === "Importing"
      ? e("div", { className: "document-attachment-status", role: "status" }, e("span", null, "파일을 안전하게 저장하고 있습니다."), e("button", { type: "button", "data-action": "cancel-document-asset-import", disabled: !callbacks.onAssetCancel, onClick: callbacks.onAssetCancel }, e(X, ICON_PROPS), "취소"))
      : null,
    snapshot.importState === "Completed"
      ? e("p", { className: "document-attachment-status", role: "status" }, "파일이 문서에 첨부되었습니다.")
      : null,
    snapshot.importState === "Failed"
      ? e("div", { className: "document-attachment-status failed", role: "alert" }, e("span", null, "파일을 첨부하지 못했습니다."), e("button", { type: "button", "data-action": "retry-document-asset-import", disabled: !callbacks.onAssetImport, onClick: callbacks.onAssetImport }, "다시 시도"))
      : null,
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
    e("button", { type: "button", className: "text-action document-attachment-library", "data-action": "open-document-asset-library", disabled: !callbacks.onOpenLibrary, onClick: callbacks.onOpenLibrary }, "전체 파일 보관함"),
    renderPreview(snapshot, callbacks),
    renderUnlinkConfirmation(unlinkState, callbacks),
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
