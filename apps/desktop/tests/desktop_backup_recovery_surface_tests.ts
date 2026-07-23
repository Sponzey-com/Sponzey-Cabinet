import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";
import { renderToStaticMarkup } from "react-dom/server";
import React from "react";

import { createDesktopBackupRecoverySnapshot } from "../src/desktop_backup_recovery_controller.ts";
import { createDesktopBackupRecoveryElement } from "../src/react_backup_recovery.ts";

const backupSource = await readFile(new URL("../src/react_backup_recovery.ts", import.meta.url), "utf8");

test("backup surface renders accessible confirmation only for validated preview", () => {
  const snapshot = {
    ...createDesktopBackupRecoverySnapshot("workspace-1"),
    state: "AwaitingConfirmation" as const,
    packageId: "package-1",
    manifest: { packageId: "package-1", schemaVersion: 1, entries: [
      { dataClass: "canvas_records" as const, recordCount: 2, byteCount: 20 },
      { dataClass: "asset_associations" as const, recordCount: 3, byteCount: 30 },
    ] },
  };
  const markup = renderToStaticMarkup(createDesktopBackupRecoveryElement(snapshot, callbacks()));

  assert.match(markup, /role="dialog"/);
  assert.match(markup, /aria-modal="true"/);
  assert.match(markup, /복원 확인/);
  assert.match(markup, /data-action="confirm-backup-restore"/);
  assert.match(markup, /data-action="cancel-backup-restore"/);
  assert.match(markup, /data-backup-manifest-entry-count="2"/);
  assert.match(markup, /data-backup-manifest-classes="canvas_records,asset_associations"/);
  assert.match(markup, /캔버스/);
  assert.match(markup, /첨부 연결/);
  assert.doesNotMatch(markup, /checksum|\/Users\//);
});

test("restore confirmation cancel button and Escape share one dismissal callback", () => {
  let cancelled = 0;
  const snapshot = {
    ...createDesktopBackupRecoverySnapshot("workspace-1"),
    state: "AwaitingConfirmation" as const,
    packageId: "package-1",
    manifest: { packageId: "package-1", schemaVersion: 1, entries: [] },
  };
  const tree = createDesktopBackupRecoveryElement(snapshot, { ...callbacks(), onCancelRestore() { cancelled += 1; } });
  const dialog = findElement(tree, (props) => props.role === "dialog");
  assert.ok(dialog);
  const onKeyDown = (dialog.props as { readonly onKeyDown?: (event: { key: string; preventDefault(): void }) => void }).onKeyDown;
  assert.equal(typeof onKeyDown, "function");
  onKeyDown?.({ key: "Enter", preventDefault() {} });
  assert.equal(cancelled, 0);
  onKeyDown?.({ key: "Escape", preventDefault() {} });
  const cancel = findElement(tree, (props) => props["data-action"] === "cancel-backup-restore");
  assert.ok(cancel);
  (cancel.props as { readonly onClick: () => void }).onClick();
  assert.equal(cancelled, 2);
});

test("restore confirmation explains replacement and rebuild impact without internal data", () => {
  const snapshot = {
    ...createDesktopBackupRecoverySnapshot("workspace-1"), state: "AwaitingConfirmation" as const,
    packageId: "internal-package", manifest: { packageId: "internal-package", schemaVersion: 1, entries: [
      { dataClass: "current_documents" as const, recordCount: 2, byteCount: 20 },
      { dataClass: "graph_rebuild_metadata" as const, recordCount: 1, byteCount: 10 },
    ] },
  };
  const markup = renderToStaticMarkup(createDesktopBackupRecoveryElement(snapshot, callbacks()));
  assert.match(markup, /현재 버전과 호환됨/);
  assert.match(markup, /백업 시점으로 교체/);
  assert.match(markup, /복원 후 다시 구성/);
  assert.doesNotMatch(markup, /internal-package/);
});

test("failed restore preview exposes retry without a confirm action", () => {
  const snapshot = {
    ...createDesktopBackupRecoverySnapshot("workspace-1"), state: "Failed" as const,
    errorCode: "BACKUP_PACKAGE_CORRUPTED", packageId: "internal-package",
    manifest: { packageId: "internal-package", schemaVersion: 1, entries: [] },
  };
  const markup = renderToStaticMarkup(createDesktopBackupRecoveryElement(snapshot, callbacks()));
  assert.match(markup, /data-action="retry-backup-restore-preview"/);
  assert.doesNotMatch(markup, /data-action="confirm-backup-restore"/);
});

test("retryable backup failure exposes retry while terminal failure does not", () => {
  const retryable = renderToStaticMarkup(createDesktopBackupRecoveryElement({
    ...createDesktopBackupRecoverySnapshot("workspace-1"), state: "Failed", errorCode: "BACKUP_COMMAND_FAILED", retryable: true,
  }, callbacks()));
  const terminal = renderToStaticMarkup(createDesktopBackupRecoveryElement({
    ...createDesktopBackupRecoverySnapshot("workspace-1"), state: "Failed", errorCode: "BACKUP_COMMAND_FAILED", retryable: false,
  }, callbacks()));
  assert.match(retryable, /data-action="retry-backup-recovery"/);
  assert.match(retryable, />다시 시도</);
  assert.doesNotMatch(terminal, /data-action="retry-backup-recovery"/);
});

test("cleanup-required state exposes retry and cancel without confirm", () => {
  const snapshot = { ...createDesktopBackupRecoverySnapshot("workspace-1"), state: "CleanupRequired" as const, errorCode: "RESTORE_CLEANUP_REQUIRED" };
  const markup = renderToStaticMarkup(createDesktopBackupRecoveryElement(snapshot, callbacks()));
  assert.match(markup, /정리가 필요합니다/);
  assert.match(markup, /복구 다시 시도/);
  assert.match(markup, /작업 취소/);
  assert.match(markup, /data-action="retry-backup-recovery"/);
  assert.doesNotMatch(markup, /RESTORE_CLEANUP_REQUIRED/);
});

test("recovery-required state is never presented as completed", () => {
  const snapshot = { ...createDesktopBackupRecoverySnapshot("workspace-1"), state: "RecoveryRequired" as const, errorCode: "RESTORE_ROLLBACK_FAILED", retryable: true };
  const markup = renderToStaticMarkup(createDesktopBackupRecoveryElement(snapshot, callbacks()));
  assert.match(markup, /복구가 필요합니다/);
  assert.match(markup, /복구 다시 시도/);
  assert.doesNotMatch(markup, /복원 완료/);
  assert.doesNotMatch(markup, /data-action="confirm-backup-restore"/);
});

test("created manifest exposes restore validation action", () => {
  const snapshot = {
    ...createDesktopBackupRecoverySnapshot("workspace-1"),
    state: "Ready" as const,
    packageId: "package-1",
    manifest: { packageId: "package-1", schemaVersion: 1, createdAtEpochMs: 1_784_064_000_000, entries: [
      { dataClass: "current_documents" as const, recordCount: 1, byteCount: 10 },
    ] },
  };
  const markup = renderToStaticMarkup(createDesktopBackupRecoveryElement(snapshot, callbacks()));
  const firstBackupStateElement = markup.match(/<[^>]+data-backup-state="Ready"[^>]*>/)?.[0] ?? "";
  assert.match(markup, /복원 가능 여부 확인/);
  assert.match(markup, /data-action="preview-backup-restore"/);
  assert.match(markup, /data-backup-state="Ready"/);
  assert.match(markup, /data-backup-manifest-entry-count="1"/);
  assert.match(firstBackupStateElement, /data-backup-manifest-entry-count="1"/);
  assert.match(firstBackupStateElement, /data-backup-manifest-classes="current_documents"/);
  assert.doesNotMatch(markup, /package-1/);
  assert.match(markup, /2026/);
  assert.doesNotMatch(markup, /1784064000000/);
});

test("legacy backup manifest labels missing creation time", () => {
  const snapshot = {
    ...createDesktopBackupRecoverySnapshot("workspace-1"),
    state: "Ready" as const,
    manifest: { packageId: "legacy", schemaVersion: 1, entries: [] },
  };
  assert.match(
    renderToStaticMarkup(createDesktopBackupRecoveryElement(snapshot, callbacks())),
    /시각 정보 없음/,
  );
});

test("creating backup exposes durable progress and cancel action", () => {
  const snapshot = {
    ...createDesktopBackupRecoverySnapshot("workspace-1"),
    state: "Creating" as const,
    operationId: "operation-1",
    operationProgress: { completedUnits: 3, totalUnits: 8 },
  };
  const markup = renderToStaticMarkup(createDesktopBackupRecoveryElement(snapshot, callbacks()));
  assert.match(markup, /8개 중 3개/);
  assert.match(markup, /백업 취소/);
  assert.match(markup, /aria-label="백업 진행률"/);
  assert.match(markup, /data-action="cancel-backup"/);
  assert.match(markup, /data-backup-progress-completed="3"/);
  assert.match(markup, /data-backup-progress-total="8"/);
});

test("idle backup surface exposes navigation and creation contracts without identifiers", () => {
  const markup = renderToStaticMarkup(createDesktopBackupRecoveryElement(
    createDesktopBackupRecoverySnapshot("workspace-1"),
    callbacks(),
  ));
  assert.match(markup, /data-action="navigate-home"/);
  assert.match(markup, /data-action="workspace-search-input"/);
  assert.doesNotMatch(markup, /data-action="navigate-search"/);
  assert.match(markup, /data-action="create-backup"/);
  assert.match(markup, />백업과 복원</);
  assert.doesNotMatch(markup, /백업 및 복원/);
  assert.doesNotMatch(markup, /workspace-1/);
  assertNoUnidentifiedInteractiveControls(markup);
});

test("backup surface uses the shared primary navigation contract without injecting Search as a route", () => {
  assert.match(backupSource, /WORKSPACE_SHELL_PRIMARY_ROUTES/);
  assert.doesNotMatch(backupSource, /const shellRoutes[\s\S]*\bSearch\b/);
  assert.doesNotMatch(backupSource, /availableActions:\s*shellRoutes/);
});

test("applying restore exposes cancel only while native operation is staging", () => {
  const staging = {
    ...createDesktopBackupRecoverySnapshot("workspace-1"), state: "Applying" as const,
    operationId: "operation-1", restoreOperationState: "Staging" as const,
  };
  const applying = { ...staging, restoreOperationState: "Applying" as const };
  assert.match(renderToStaticMarkup(createDesktopBackupRecoveryElement(staging, callbacks())), /복원 취소/);
  assert.doesNotMatch(renderToStaticMarkup(createDesktopBackupRecoveryElement(applying, callbacks())), /복원 취소/);
});

test("backup catalog renders safe selectable summaries and load more without package identity", () => {
  const first = { packageId: "internal-package-1", schemaVersion: 1, createdAtEpochMs: 1_784_064_000_000, entries: [
    { dataClass: "current_documents" as const, recordCount: 2, byteCount: 20 },
  ] };
  const markup = renderToStaticMarkup(createDesktopBackupRecoveryElement({
    ...createDesktopBackupRecoverySnapshot("workspace-1"),
    catalogState: "Ready",
    catalogRecords: [first],
    catalogNextCursor: "opaque-cursor",
    selectedCatalogPackageId: first.packageId,
    state: "Ready",
    packageId: first.packageId,
    manifest: first,
  }, callbacks()));
  assert.match(markup, /최근 백업/);
  assert.match(markup, /data-action="select-backup-catalog"/);
  assert.match(markup, /aria-pressed="true"/);
  assert.match(markup, /data-action="load-more-backup-catalog"/);
  assert.doesNotMatch(markup, /internal-package-1|opaque-cursor/);
});

test("backup safety panel summarizes latest local backup without package identity or paths", () => {
  const markup = renderToStaticMarkup(createDesktopBackupRecoveryElement({
    ...createDesktopBackupRecoverySnapshot("workspace-1"),
    catalogState: "Ready",
    catalogRecords: [{
      packageId: "internal-package-safe",
      schemaVersion: 1,
      createdAtEpochMs: 1_784_150_400_000,
      entries: [
        { dataClass: "current_documents" as const, recordCount: 4, byteCount: 40 },
        { dataClass: "asset_metadata" as const, recordCount: 3, byteCount: 30 },
        { dataClass: "canvas_records" as const, recordCount: 2, byteCount: 20 },
      ],
    }],
  }, callbacks()));

  assert.match(markup, /class="backup-safety-panel"/);
  assert.match(markup, /내 지식 공간이 안전합니다/);
  assert.match(markup, /마지막 백업/);
  assert.match(markup, /이 Mac에 저장/);
  assert.match(markup, /문서 4개 · 첨부 3개 · 캔버스 2개/);
  assert.doesNotMatch(markup, /internal-package-safe|1784150400000|\/Users|file:\/\//);
});

function callbacks() {
  return { onHome() {}, onCreateBackup() {}, onCancelBackup() {}, onPreviewRestore() {}, onConfirmRestore() {}, onCancelRestore() {}, onRecover() {}, onReloadCatalog() {}, onLoadMoreCatalog() {}, onSelectCatalogPackage() {} };
}

function findElement(
  node: React.ReactNode,
  predicate: (props: Record<string, unknown>) => boolean,
): React.ReactElement | undefined {
  if (Array.isArray(node)) {
    for (const child of node) {
      const found = findElement(child, predicate);
      if (found) return found;
    }
    return undefined;
  }
  if (!React.isValidElement(node)) return undefined;
  const props = node.props as Record<string, unknown>;
  if (predicate(props)) return node;
  return findElement(props.children as React.ReactNode, predicate);
}

function assertNoUnidentifiedInteractiveControls(markup: string): void {
  const controls = markup.match(/<(?:button|input|select|textarea|a)\b[^>]*>/g) ?? [];
  assert.ok(controls.length > 0);
  assert.deepEqual(controls.filter((control) => !control.includes("data-action=")), []);
}
