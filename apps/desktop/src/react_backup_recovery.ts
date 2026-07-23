import React from "react";
import type { DesktopBackupRecoverySnapshot } from "./desktop_backup_recovery_controller.ts";
import { createWorkspaceShellModel, WORKSPACE_SHELL_PRIMARY_ROUTES } from "./workspace_shell_contract.ts";
import {
  createWorkspaceShellElement,
  type WorkspaceShellDocumentShortcut,
} from "./react_workspace_shell.ts";
import { KO_KR_MESSAGES } from "./ko_kr_catalog.ts";
import { formatBytesKoKr } from "./ko_kr_catalog.ts";
import { mapUserFacingError } from "./user_facing_error_presenter.ts";
import { createKoKrBackupDateFormatter, presentBackupCreatedAt, presentBackupManifest, presentBackupRestorePreflight, presentBackupSafetySummary } from "./backup_manifest_presenter.ts";
import { handleModalKeyboard } from "./modal_keyboard_policy.ts";
import { browserModalFocusEnvironment, createFocusRestoringModalAction } from "./modal_focus_restoration.ts";

const backupDateFormatter = createKoKrBackupDateFormatter();

export interface DesktopBackupRecoveryCallbacks {
  readonly onHome: () => void;
  readonly onSearchOpen?: () => void;
  readonly onSearch: (query?: string) => void;
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
  readonly onReloadCatalog: () => void;
  readonly onLoadMoreCatalog: () => void;
  readonly onSelectCatalogPackage: (packageId: string) => void;
}

export interface DesktopBackupRecoveryOptions {
  readonly documentShortcuts?: readonly WorkspaceShellDocumentShortcut[];
}

export function createDesktopBackupRecoveryElement(
  snapshot: DesktopBackupRecoverySnapshot,
  callbacks: DesktopBackupRecoveryCallbacks,
  options: DesktopBackupRecoveryOptions = {},
): React.ReactElement {
  const e = React.createElement;
  const observationAttributes = backupObservationAttributes(snapshot);
  const busy = snapshot.state === "Creating" || snapshot.state === "Previewing" || snapshot.state === "Applying";
  const error = snapshot.errorCode ? mapUserFacingError({ stableCode: snapshot.errorCode, retryable: snapshot.retryable === true, operationContext: "backup" }) : undefined;
  const dismissRestore = createFocusRestoringModalAction(callbacks.onCancelRestore, browserModalFocusEnvironment("preview-backup-restore"));
  const confirmRestore = createFocusRestoringModalAction(callbacks.onConfirmRestore, browserModalFocusEnvironment("preview-backup-restore"));
  const safetySummary = presentBackupSafetySummary(snapshot.catalogRecords, backupDateFormatter);
  const content = e("main", {
    className: "backup-recovery-surface",
    ...observationAttributes,
  },
    e("header", { className: "surface-header" },
      e("button", { type: "button", "data-action": "navigate-home", onClick: callbacks.onHome, "aria-label": "작업 공간으로 돌아가기" }, "←"),
      e("div", null, e("p", { className: "eyebrow" }, "데이터 보호"), e("h1", null, "백업과 복원")),
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
      snapshot.state === "Failed" && snapshot.manifest
        ? e("button", { type: "button", "data-action": "retry-backup-restore-preview", onClick: callbacks.onPreviewRestore }, "복원 가능 여부 다시 확인")
        : null,
      snapshot.state === "Creating"
        ? e("button", { type: "button", "data-action": "cancel-backup", onClick: callbacks.onCancelBackup }, "백업 취소")
        : null,
      snapshot.state === "Applying" && snapshot.restoreOperationState === "Staging"
        ? e("button", { type: "button", "data-action": "cancel-backup-restore", onClick: callbacks.onCancelRestore }, "복원 취소")
        : null,
    ),
    e("section", { className: "backup-safety-panel", "data-backup-safety-state": safetySummary.state, "aria-label": "백업 안전 상태" },
      e("div", null,
        e("strong", null, safetySummary.statusLabel),
        e("p", null, safetySummary.detailLabel),
      ),
      e("span", null, safetySummary.locationLabel),
      e("p", null, safetySummary.contentLabel),
    ),
    catalogSection(snapshot, callbacks),
    snapshot.manifest ? e(React.Fragment, null,
      manifestTable(snapshot),
      snapshot.state === "Ready" ? e("div", { className: "backup-primary-actions" }, e("button", { type: "button", "data-action": "preview-backup-restore", onClick: callbacks.onPreviewRestore, disabled: busy }, "복원 가능 여부 확인")) : null,
    ) : e("section", { className: "empty-state" }, e("h2", null, "선택한 백업이 없습니다"), e("p", null, "먼저 백업을 만든 뒤 복원 가능 여부를 확인하세요.")),
    snapshot.state === "CleanupRequired" ? e("section", { className: "recovery-actions", "aria-label": "복구 작업" },
      e("h2", null, "정리가 필요합니다"),
      e("button", { type: "button", "data-action": "retry-backup-recovery", onClick: callbacks.onRecover }, "복구 다시 시도"),
      e("button", { type: "button", "data-action": "cancel-backup-restore", onClick: callbacks.onCancelRestore }, "작업 취소"),
    ) : null,
    snapshot.state === "RecoveryRequired" ? e("section", { className: "recovery-actions", "aria-label": "복구 작업" },
      e("h2", null, "복구가 필요합니다"),
      e("p", null, "이전 작업 공간으로 되돌리는 중 문제가 발생했습니다. 앱의 복구 절차를 다시 실행하세요."),
      e("button", { type: "button", "data-action": "retry-backup-recovery", onClick: callbacks.onRecover }, "복구 다시 시도"),
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
      restorePreflightSummary(snapshot),
      e("p", null, "검증이 끝난 뒤 현재 작업 공간을 교체합니다. 다시 열기에 성공할 때까지 되돌리기용 백업을 보존합니다."),
      e("button", { type: "button", "data-action": "cancel-backup-restore", onClick: dismissRestore }, "취소"),
      e("button", { type: "button", "data-action": "confirm-backup-restore", onClick: confirmRestore, autoFocus: true }, "복원 시작"),
    ) : null,
  );
  return createWorkspaceShellElement({
    model: createWorkspaceShellModel({ route: "Backup", availableActions: WORKSPACE_SHELL_PRIMARY_ROUTES, messages: KO_KR_MESSAGES }),
    messages: KO_KR_MESSAGES,
    routeActions: { Home: callbacks.onHome, Document: callbacks.onDocument, Graph: callbacks.onGraph, Canvas: callbacks.onCanvas, Assets: callbacks.onAssets },
    onCreateDocument: callbacks.onCreateDocument,
    onSearchOpen: callbacks.onSearchOpen,
    onSearch: callbacks.onSearch,
    documentShortcuts: options.documentShortcuts,
    rootClassName: "backup-shell",
    rootAttributes: observationAttributes,
    content,
  });
}

function restorePreflightSummary(snapshot: DesktopBackupRecoverySnapshot): React.ReactElement | null {
  if (!snapshot.manifest) return null;
  const e = React.createElement;
  const preflight = presentBackupRestorePreflight(snapshot.manifest.schemaVersion, snapshot.manifest.entries);
  return e("section", { className: "restore-preflight-summary", "aria-label": "복원 영향" },
    e("strong", null, preflight.compatibilityLabel),
    e("p", null, `문서와 첨부 등 ${preflight.authoritativeRecordCount}개 · ${formatBytesKoKr(preflight.authoritativeByteCount)}를 백업 시점으로 교체합니다.`),
    e("p", null, `검색과 관계 정보 ${preflight.rebuildableRecordCount}개는 복원 후 다시 구성합니다.`),
  );
}

function backupObservationAttributes(snapshot: DesktopBackupRecoverySnapshot): Readonly<Record<string, string | number>> {
  return Object.freeze({
    "data-backup-state": snapshot.state,
    "data-backup-manifest-entry-count": snapshot.manifest?.entries.length ?? 0,
    "data-backup-manifest-classes": snapshot.manifest?.entries.map((entry) => entry.dataClass).join(",") ?? "",
    "data-backup-progress-completed": snapshot.operationProgress?.completedUnits ?? 0,
    "data-backup-progress-total": snapshot.operationProgress?.totalUnits ?? 0,
    "data-restore-operation-state": snapshot.restoreOperationState ?? "Idle",
    "data-backup-catalog-state": snapshot.catalogState,
    "data-backup-catalog-count": snapshot.catalogRecords.length,
  });
}

function catalogSection(snapshot: DesktopBackupRecoverySnapshot, callbacks: DesktopBackupRecoveryCallbacks): React.ReactElement {
  const e = React.createElement;
  return e("section", { className: "backup-catalog", "aria-labelledby": "backup-catalog-title" },
    e("div", { className: "section-heading" },
      e("div", null, e("h2", { id: "backup-catalog-title" }, "최근 백업"), e("p", null, "이 컴퓨터에 안전하게 저장된 백업입니다.")),
      snapshot.catalogState === "Failed" ? e("button", { type: "button", "data-action": "retry-backup-catalog", onClick: callbacks.onReloadCatalog }, "다시 불러오기") : null,
    ),
    snapshot.catalogState === "Loading" ? e("p", { role: "status" }, "백업 목록을 불러오는 중입니다.") : null,
    snapshot.catalogState === "Empty" || (snapshot.catalogState === "Idle" && snapshot.catalogRecords.length === 0)
      ? e("p", { className: "empty-copy" }, "아직 만든 백업이 없습니다.")
      : null,
    snapshot.catalogRecords.length > 0 ? e("div", { className: "backup-catalog-list", role: "list" }, snapshot.catalogRecords.map((manifest) => {
      const presentation = presentBackupManifest(manifest.entries);
      const selected = snapshot.selectedCatalogPackageId === manifest.packageId;
      return e("button", {
        key: manifest.packageId,
        type: "button",
        role: "listitem",
        "data-action": "select-backup-catalog",
        "aria-pressed": selected,
        className: selected ? "selected" : "",
        onClick: () => callbacks.onSelectCatalogPackage(manifest.packageId),
      },
      e("strong", null, presentBackupCreatedAt(manifest.createdAtEpochMs, backupDateFormatter)),
      e("span", null, `${presentation.totalRecordCount}개 · ${formatBytesKoKr(presentation.totalByteCount)}`));
    })) : null,
    snapshot.catalogNextCursor ? e("button", { type: "button", "data-action": "load-more-backup-catalog", onClick: callbacks.onLoadMoreCatalog }, "더 보기") : null,
  );
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
  return ({ Idle: "준비됨", Creating: "백업 만드는 중", Ready: "백업 준비됨", Previewing: "백업 검증 중", AwaitingConfirmation: "복원 확인 필요", Applying: "작업 공간 복원 중", Completed: "복원 완료", Cancelled: "복원 취소됨", RolledBack: "이전 상태로 되돌림", Failed: "작업 실패", CleanupRequired: "정리 필요", RecoveryRequired: "복구 필요" })[state];
}
