import React from "react";
import type { DesktopBackupRecoverySnapshot } from "./desktop_backup_recovery_controller.ts";
import { createWorkspaceShellModel, type WorkspaceShellRouteKind } from "./workspace_shell_contract.ts";
import { createWorkspaceShellElement } from "./react_workspace_shell.ts";
import { KO_KR_MESSAGES } from "./ko_kr_catalog.ts";
import { formatBytesKoKr } from "./ko_kr_catalog.ts";
import { mapUserFacingError } from "./user_facing_error_presenter.ts";
import { createKoKrBackupDateFormatter, presentBackupCreatedAt, presentBackupManifest } from "./backup_manifest_presenter.ts";
import { handleModalKeyboard } from "./modal_keyboard_policy.ts";
import { browserModalFocusEnvironment, createFocusRestoringModalAction } from "./modal_focus_restoration.ts";

const shellRoutes: readonly WorkspaceShellRouteKind[] = ["Home", "Search", "Document", "Graph", "Canvas", "Assets", "Backup"];
const backupDateFormatter = createKoKrBackupDateFormatter();

export interface DesktopBackupRecoveryCallbacks {
  readonly onHome: () => void;
  readonly onSearch: () => void;
  readonly onDocument: () => void;
  readonly onGraph: () => void;
  readonly onCanvas: () => void;
  readonly onAssets: () => void;
  readonly onCreateDocument: () => void;
  readonly onCreateBackup: () => void;
  readonly onCancelBackup: () => void;
  readonly onPreviewRestore: () => void;
  readonly onConfirmRestore: () => void;
  readonly onCancelRestore: () => void;
  readonly onRecover: () => void;
}

export function createDesktopBackupRecoveryElement(
  snapshot: DesktopBackupRecoverySnapshot,
  callbacks: DesktopBackupRecoveryCallbacks,
): React.ReactElement {
  const e = React.createElement;
  const observationAttributes = backupObservationAttributes(snapshot);
  const busy = snapshot.state === "Creating" || snapshot.state === "Previewing" || snapshot.state === "Applying";
  const error = snapshot.errorCode ? mapUserFacingError({ stableCode: snapshot.errorCode, retryable: snapshot.retryable === true, operationContext: "backup" }) : undefined;
  const dismissRestore = createFocusRestoringModalAction(callbacks.onCancelRestore, browserModalFocusEnvironment("preview-backup-restore"));
  const confirmRestore = createFocusRestoringModalAction(callbacks.onConfirmRestore, browserModalFocusEnvironment("preview-backup-restore"));
  const content = e("main", {
    className: "backup-recovery-surface",
    ...observationAttributes,
  },
    e("header", { className: "surface-header" },
      e("button", { type: "button", "data-action": "navigate-home", onClick: callbacks.onHome, "aria-label": "작업 공간으로 돌아가기" }, "←"),
      e("div", null, e("p", { className: "eyebrow" }, "데이터 보호"), e("h1", null, "백업 및 복원")),
      e("button", { type: "button", "data-action": "create-backup", onClick: callbacks.onCreateBackup, disabled: busy }, "백업 만들기"),
    ),
    e("section", { className: "backup-status-band", "aria-live": "polite", "aria-label": "백업 작업 상태" },
      e("strong", null, stateLabel(snapshot.state)),
      snapshot.state === "Creating" && snapshot.operationProgress
        ? e("span", { className: "backup-progress", "aria-label": "백업 진행률" },
            `${snapshot.operationProgress.totalUnits}개 중 ${snapshot.operationProgress.completedUnits}개`)
        : null,
      error ? e("span", { className: "error-message" }, error.message) : null,
      snapshot.state === "Failed" && error?.recoveryAction === "retry"
        ? e("button", { type: "button", "data-action": "retry-backup-recovery", onClick: callbacks.onCreateBackup }, error.recoveryLabel)
        : null,
      snapshot.state === "Creating"
        ? e("button", { type: "button", "data-action": "cancel-backup", onClick: callbacks.onCancelBackup }, "백업 취소")
        : null,
      snapshot.state === "Applying" && snapshot.restoreOperationState === "Staging"
        ? e("button", { type: "button", "data-action": "cancel-backup-restore", onClick: callbacks.onCancelRestore }, "복원 취소")
        : null,
    ),
    snapshot.manifest ? e(React.Fragment, null,
      manifestTable(snapshot),
      snapshot.state === "Ready" ? e("div", { className: "backup-primary-actions" }, e("button", { type: "button", "data-action": "preview-backup-restore", onClick: callbacks.onPreviewRestore, disabled: busy }, "복원 가능 여부 확인")) : null,
    ) : e("section", { className: "empty-state" }, e("h2", null, "선택한 백업이 없습니다"), e("p", null, "먼저 백업을 만든 뒤 복원 가능 여부를 확인하세요.")),
    snapshot.state === "CleanupRequired" ? e("section", { className: "recovery-actions", "aria-label": "복구 작업" },
      e("h2", null, "정리가 필요합니다"),
      e("button", { type: "button", "data-action": "retry-backup-recovery", onClick: callbacks.onRecover }, "복구 다시 시도"),
      e("button", { type: "button", "data-action": "cancel-backup-restore", onClick: callbacks.onCancelRestore }, "작업 취소"),
    ) : null,
    snapshot.state === "AwaitingConfirmation" ? e("div", {
      role: "dialog",
      "aria-modal": "true",
      "aria-labelledby": "restore-confirm-title",
      className: "restore-confirm-dialog",
      onKeyDown: (event: React.KeyboardEvent<HTMLDivElement>) => {
        handleModalKeyboard(event, dismissRestore);
      },
    },
      e("h2", { id: "restore-confirm-title" }, "복원 확인"),
      e("p", null, "검증이 끝난 뒤 현재 작업 공간을 교체합니다. 다시 열기에 성공할 때까지 되돌리기용 백업을 보존합니다."),
      e("button", { type: "button", "data-action": "cancel-backup-restore", onClick: dismissRestore }, "취소"),
      e("button", { type: "button", "data-action": "confirm-backup-restore", onClick: confirmRestore, autoFocus: true }, "복원 시작"),
    ) : null,
  );
  return createWorkspaceShellElement({
    model: createWorkspaceShellModel({ route: "Backup", availableActions: shellRoutes, messages: KO_KR_MESSAGES }),
    messages: KO_KR_MESSAGES,
    routeActions: { Home: callbacks.onHome, Search: callbacks.onSearch, Document: callbacks.onDocument, Graph: callbacks.onGraph, Canvas: callbacks.onCanvas, Assets: callbacks.onAssets },
    onCreateDocument: callbacks.onCreateDocument,
    onSearch: callbacks.onSearch,
    rootClassName: "backup-shell",
    rootAttributes: observationAttributes,
    content,
  });
}

function backupObservationAttributes(snapshot: DesktopBackupRecoverySnapshot): Readonly<Record<string, string | number>> {
  return Object.freeze({
    "data-backup-state": snapshot.state,
    "data-backup-manifest-entry-count": snapshot.manifest?.entries.length ?? 0,
    "data-backup-manifest-classes": snapshot.manifest?.entries.map((entry) => entry.dataClass).join(",") ?? "",
    "data-backup-progress-completed": snapshot.operationProgress?.completedUnits ?? 0,
    "data-backup-progress-total": snapshot.operationProgress?.totalUnits ?? 0,
    "data-restore-operation-state": snapshot.restoreOperationState ?? "Idle",
  });
}

function manifestTable(snapshot: DesktopBackupRecoverySnapshot): React.ReactElement {
  const e = React.createElement;
  const presentation = presentBackupManifest(snapshot.manifest?.entries ?? []);
  const createdAt = presentBackupCreatedAt(snapshot.manifest?.createdAtEpochMs, backupDateFormatter);
  return e("section", { className: "backup-manifest", "aria-labelledby": "backup-manifest-title" },
    e("div", { className: "section-heading" },
      e("div", null, e("h2", { id: "backup-manifest-title" }, "백업 내용"), e("p", null, `생성 ${createdAt}`)),
      e("span", null, `총 ${presentation.totalRecordCount}개 · ${formatBytesKoKr(presentation.totalByteCount)}`),
    ),
    e("table", null,
      e("thead", null, e("tr", null, e("th", { scope: "col" }, "데이터"), e("th", { scope: "col" }, "항목 수"), e("th", { scope: "col" }, "크기"))),
      e("tbody", null, presentation.entries.map((entry) => e("tr", { key: entry.dataClass }, e("th", { scope: "row" }, entry.label), e("td", null, String(entry.recordCount)), e("td", null, formatBytesKoKr(entry.byteCount))))),
    ),
  );
}

function stateLabel(state: DesktopBackupRecoverySnapshot["state"]): string {
  return ({ Idle: "준비됨", Creating: "백업 만드는 중", Ready: "백업 준비됨", Previewing: "백업 검증 중", AwaitingConfirmation: "복원 확인 필요", Applying: "작업 공간 복원 중", Completed: "복원 완료", Cancelled: "복원 취소됨", RolledBack: "이전 상태로 되돌림", Failed: "작업 실패", CleanupRequired: "정리 필요" })[state];
}
