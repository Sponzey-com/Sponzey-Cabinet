import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { renderToStaticMarkup } from "react-dom/server";

import { DocumentSaveCoordinatorState } from "@sponzey-cabinet/ui";

import { createDesktopDocumentAuthoringWorkbenchElement } from "../src/react_document_authoring_workbench.ts";
import {
  applyDesktopAssetResult,
  createDesktopAssetSnapshot,
  requestDesktopAssetLoad,
} from "../src/desktop_asset_controller.ts";

test("React authoring workbench renders split source preview table and semantic controls", () => {
  const assetLoading = requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1");
  const assets = applyDesktopAssetResult(assetLoading, assetLoading.generation, {
    queryName: "list-document-assets",
    workspaceId: "workspace-1",
    documentId: "doc-1",
    assets: [{ assetId: "internal-asset-id", label: "설계 자료", fileName: "architecture.pdf", mediaType: "application/pdf", byteSize: 2048, status: "available" }],
  });
  const markup = renderToStaticMarkup(
    createDesktopDocumentAuthoringWorkbenchElement(snapshot(), {
      ...callbacks(),
      onAssetImport() {},
      onAssetSelect() {},
      onAssetPreview() {},
      onAssetPreviewClose() {},
      onAssetOpen() {},
      onAssetUnlink() {},
      onAssetRetry() {},
    }, {
      viewMode: "split",
      assets,
      inspector: { tab: "attachments", unlink: { status: "Closed" } },
    }),
  );

  assert.match(markup, /data-cabinet-authoring-state="Dirty"/);
  assert.match(markup, /data-design-reference="penpot-20260713"/);
  assert.match(markup, />Cabinet</);
  assert.match(markup, /내 지식 지도/);
  assert.match(markup, />연결<\/button>/);
  assert.match(markup, /첨부 파일/);
  assert.match(markup, /role="tablist"/);
  assert.match(markup, /role="tab"[^>]*aria-selected="true"[^>]*data-action="select-document-inspector-attachments"/);
  assert.match(markup, /architecture\.pdf/);
  assert.match(markup, /data-action="import-document-asset"(?![^>]*disabled)/);
  assert.match(markup, /data-action="select-document-asset"/);
  assert.match(markup, /data-action="open-document-asset-externally"/);
  assert.match(markup, /기본 앱으로 열기/);
  assert.doesNotMatch(markup.replace(/\sdata-asset-id="[^"]*"/g, ""), /internal-asset-id/);
  assert.match(markup, /aria-label="편집 화면"/);
  assert.match(markup, /data-editor-mode="source"/);
  assert.match(markup, /data-editor-mode="split"/);
  assert.match(markup, /data-editor-mode="preview"/);
  assert.match(markup, /aria-pressed="true"[^>]*>나란히/);
  assert.match(markup, /aria-label="Markdown 원문"/);
  assert.match(markup, /data-codemirror-host="pending"/);
  assert.match(markup, /aria-label="Markdown 미리보기"/);
  assert.match(markup, /<table/);
  assert.match(markup, /<th[^>]*>Item/);
  assert.match(markup, /<td[^>]*>Grid/);
  assert.match(markup, /저장되지 않음/);
  assert.match(markup, /data-action="navigate-graph"[^>]*disabled/);
  assert.match(markup, /class="sidebar-current-document"[^>]*aria-current="page"/);
  assert.doesNotMatch(markup, /data-action="(?:open-settings|toggle-theme|open-ai)"/);
  assert.match(markup, /data-action="open-authoring-graph"[^>]*disabled/);
  assert.doesNotMatch(markup, /연결된 문서가 없습니다|이력 불러오기/);
  assert.equal((markup.match(/class="desktop-sidebar"/g) ?? []).length, 1);
  assert.equal((markup.match(/class="desktop-topbar"/g) ?? []).length, 1);
  assert.equal((markup.match(/<main/g) ?? []).length, 1);
  assert.doesNotMatch(markup, /로컬 우선 원칙|검색 인덱스|백업과 복원/);
  assert.doesNotMatch(markup, /server|tenant|billing|admin-console/i);
  assertNoUnidentifiedInteractiveControls(markup);
});

function assertNoUnidentifiedInteractiveControls(markup: string): void {
  const controls = markup.match(/<(?:button|input|select|textarea|a)\b[^>]*>/g) ?? [];
  assert.ok(controls.length > 0);
  assert.deepEqual(controls.filter((control) => !control.includes("data-action=")), []);
}

test("React authoring workbench supports source preview modes and escapes unsafe source", () => {
  const unsafe = {
    ...snapshot(),
    body: "# Safe\n\n<script>globalThis.compromised=true</script>",
  };
  const source = renderToStaticMarkup(
    createDesktopDocumentAuthoringWorkbenchElement(unsafe, callbacks(), { viewMode: "source" }),
  );
  const preview = renderToStaticMarkup(
    createDesktopDocumentAuthoringWorkbenchElement(unsafe, callbacks(), { viewMode: "preview" }),
  );

  assert.match(source, /aria-label="Markdown 원문"/);
  assert.doesNotMatch(source, /aria-label="Markdown 미리보기"/);
  assert.match(preview, /aria-label="Markdown 미리보기"/);
  assert.doesNotMatch(preview, /data-codemirror-host/);
  assert.doesNotMatch(preview, /<script>/);
  assert.doesNotMatch(preview, /globalThis\.compromised/);
});

test("React authoring attachment panel presents empty, importing, preview, and mutation actions safely", () => {
  const loading = requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1");
  const ready = applyDesktopAssetResult(loading, loading.generation, {
    queryName: "list-document-assets",
    workspaceId: "workspace-1",
    documentId: "doc-1",
    assets: [{ assetId: "asset-secret", label: "회의 자료", fileName: "meeting.txt", mediaType: "text/plain", byteSize: 12, status: "available" }],
  });
  const callbacksWithAssets = {
    ...callbacks(),
    onAssetImport() {}, onAssetRetry() {}, onAssetCancel() {}, onAssetSelect() {},
    onAssetPreview() {}, onAssetPreviewClose() {}, onAssetUnlink() {}, onOpenLibrary() {},
  };
  const empty = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacksWithAssets, {
    assets: { ...loading, state: "Empty", page: { queryName: "list-document-assets", workspaceId: "workspace-1", documentId: "doc-1", assets: [] } },
    inspector: { tab: "attachments", unlink: { status: "Closed" } },
  }));
  const importing = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacksWithAssets, {
    assets: { ...ready, importState: "Importing", importOperationId: "internal-operation" },
    inspector: { tab: "attachments", unlink: { status: "Closed" } },
  }));
  const preview = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacksWithAssets, {
    assets: {
      ...ready,
      selectedAssetId: "asset-secret",
      detailState: "Ready",
      detail: { assetId: "asset-secret", fileName: "meeting.txt", mediaType: "text/plain", byteSize: 12, version: 1, previewCapability: "text", extractionStatus: "not_requested", referenceCount: 1, linkedDocumentIds: ["doc-1"] },
      previewState: "Ready",
      preview: { assetId: "asset-secret", capability: "text", mediaType: "text/plain", presentation: "text", content: "safe preview" },
    },
    inspector: { tab: "attachments", unlink: { status: "Confirming", fileName: "meeting.txt" } },
  }));
  const submitting = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacksWithAssets, {
    assets: { ...ready, selectedAssetId: "asset-secret", detailState: "Ready", mutationState: "Unlinking" },
    inspector: { tab: "attachments", unlink: { status: "Submitting", fileName: "meeting.txt" } },
  }));
  const failedUnlink = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacksWithAssets, {
    assets: { ...ready, selectedAssetId: "asset-secret", detailState: "Ready", mutationState: "Failed" },
    inspector: { tab: "attachments", unlink: { status: "Failed", fileName: "meeting.txt" } },
  }));

  assert.match(empty, /이 문서에 첨부된 파일이 없습니다/);
  assert.match(importing, /파일을 안전하게 저장하고 있습니다/);
  assert.match(importing, /data-action="cancel-document-asset-import"/);
  assert.match(preview, /role="dialog"/);
  assert.match(preview, /safe preview/);
  assert.match(preview, /data-action="unlink-document-asset"/);
  assert.match(preview, /문서 연결만 해제하며 파일은 보관함에 남습니다/);
  assert.match(preview, /data-action="confirm-document-asset-unlink"/);
  assert.match(preview, /data-action="cancel-document-asset-unlink"/);
  assert.match(submitting, /data-document-asset-unlink-state="Submitting"/);
  assert.match(submitting, /data-action="confirm-document-asset-unlink"[^>]*disabled/);
  assert.match(failedUnlink, /data-document-asset-unlink-state="Failed"/);
  assert.match(failedUnlink, /role="alert"/);
  assert.match(failedUnlink, />다시 시도<\/button>/);
  assert.match(preview, /data-action="close-document-asset-preview"/);
  const visible = preview
    .replace(/\sdata-asset-id="[^"]*"/g, "")
    .replace(/\sdata-document-id="[^"]*"/g, "");
  assert.doesNotMatch(visible, /asset-secret|internal-operation|version-/);
});

test("React authoring inspector renders only the selected context panel", () => {
  const common = { links: { state: "Idle", workspaceId: "workspace-1", documentId: "doc-1", generation: 0 } as const };
  const links = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks(), {
    ...common,
    inspector: { tab: "links", unlink: { status: "Closed" } },
  }));
  const history = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks(), {
    ...common,
    history: { status: "Idle", entries: [] },
    inspector: { tab: "history", unlink: { status: "Closed" } },
  }));

  assert.match(links, /연결된 문서가 없습니다/);
  assert.doesNotMatch(links, /이력 불러오기|이 문서에 첨부된 파일/);
  assert.match(history, /이력 불러오기/);
  assert.doesNotMatch(history, /연결된 문서가 없습니다|이 문서에 첨부된 파일/);
});

test("React authoring restore states expose safe missing conflict and recovery actions", () => {
  const restoreCallbacks = {
    ...callbacks(),
    onRefreshRestorePreview() {},
    onContinueRestoreRecovery() {},
  };
  const common = {
    entries: [],
    inspector: { tab: "history", unlink: { status: "Closed" } } as const,
  };
  const blocked = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(
    snapshot(),
    restoreCallbacks,
    {
      history: {
        status: "Failed",
        entries: [],
        restore: {
          status: "BlockedMissingAsset",
          targetVersionId: "opaque-version-token",
          expectedCurrentVersionId: "opaque-current-token",
          targetVersionLabel: "버전 3",
          changedLineCount: 0,
          missingAssetLabels: ["회의 자료"],
          canRestore: false,
          diff: restoreDiff(),
        },
      },
      inspector: common.inspector,
    },
  ));
  const conflict = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(
    snapshot(),
    restoreCallbacks,
    {
      history: {
        status: "Failed",
        entries: [],
        restore: { status: "Conflict", targetVersionId: "opaque-version-token", targetVersionLabel: "버전 3" },
      },
      inspector: common.inspector,
    },
  ));
  const recovery = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(
    snapshot(),
    restoreCallbacks,
    {
      history: {
        status: "Failed",
        entries: [],
        restore: {
          status: "RecoveryRequired",
          operationId: "opaque-operation-token",
          targetVersionId: "opaque-version-token",
          expectedCurrentVersionId: "opaque-current-token",
          targetVersionLabel: "버전 3",
          changedLineCount: 2,
          missingAssetLabels: [],
          canRestore: true,
          diff: restoreDiff(),
        },
      },
      inspector: common.inspector,
    },
  ));

  assert.match(blocked, /파일을 찾을 수 없어 복원할 수 없습니다/);
  assert.match(blocked, /회의 자료/);
  assert.match(blocked, /data-action="apply-restore"[^>]*disabled/);
  assert.match(conflict, /data-action="refresh-restore-preview"/);
  assert.match(conflict, /미리보기 새로고침/);
  assert.match(recovery, /data-action="continue-restore-recovery"/);
  assert.match(recovery, /복구 계속/);
  assert.doesNotMatch(`${blocked}${conflict}${recovery}`, /opaque-(?:version|current|operation)-token/);
});

test("React authoring restore confirmation renders full diff and explicit actions without internal tokens", () => {
  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(
    snapshot(),
    {
      ...callbacks(),
      onRequestRestoreConfirmation() {},
      onCancelRestoreConfirmation() {},
      onApplyRestore() {},
    },
    {
      history: {
        status: "PreviewReady",
        entries: [],
        restore: {
          status: "Confirming",
          targetVersionId: "opaque-version-token",
          expectedCurrentVersionId: "opaque-current-token",
          targetVersionLabel: "버전 3",
          changedLineCount: 2,
          missingAssetLabels: [],
          canRestore: true,
          diff: restoreDiff(),
        },
      },
      inspector: { tab: "history", unlink: { status: "Closed" } },
    },
  ));

  assert.match(markup, /복원 전 변경 내용 확인/);
  assert.match(markup, /현재 제목/);
  assert.match(markup, /복원할 제목/);
  assert.match(markup, /현재 본문/);
  assert.match(markup, /복원할 본문/);
  assert.match(markup, /추가 자료\.pdf/);
  assert.match(markup, /제거 자료\.pdf/);
  assert.match(markup, /data-action="cancel-restore-confirmation"/);
  assert.match(markup, /data-action="confirm-restore"/);
  assert.doesNotMatch(markup, /opaque-(?:workspace|document|current|version)-token/);
});

test("React authoring blocks too-large restore while preserving safe attachment labels", () => {
  const diff = restoreDiff();
  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(
    snapshot(),
    { ...callbacks(), onRequestRestoreConfirmation() {}, onApplyRestore() {} },
    {
      history: {
        status: "PreviewReady",
        entries: [],
        restore: {
          status: "BlockedLargeDiff",
          targetVersionId: "opaque-version-token",
          expectedCurrentVersionId: "opaque-current-token",
          targetVersionLabel: "버전 3",
          changedLineCount: 20_000,
          missingAssetLabels: [],
          canRestore: true,
          diff: { ...diff, status: "TooLarge", limitReason: "hunks" },
        },
      },
      inspector: { tab: "history", unlink: { status: "Closed" } },
    },
  ));

  assert.match(markup, /전체 변경 내용을 확인할 수 없어 복원할 수 없습니다/);
  assert.match(markup, /추가 자료\.pdf/);
  assert.match(markup, /data-action="review-restore"[^>]*disabled/);
  assert.doesNotMatch(markup, /data-action="confirm-restore"/);
  assert.doesNotMatch(markup, /opaque-(?:workspace|document|current|version)-token/);
});

test("React authoring workbench exposes retry and read-only recovery controls without raw error", () => {
  const failed = renderToStaticMarkup(
    createDesktopDocumentAuthoringWorkbenchElement(
      {
        ...snapshot(),
        saveState: DocumentSaveCoordinatorState.SaveFailed,
        errorCode: "DOCUMENT_AUTHORING_STORAGE_UNAVAILABLE",
        retryable: true,
      },
      callbacks(),
    ),
  );
  const readOnly = renderToStaticMarkup(
    createDesktopDocumentAuthoringWorkbenchElement(
      {
        ...snapshot(),
        saveState: DocumentSaveCoordinatorState.ReadOnlyRecovery,
        errorCode: "DOCUMENT_AUTHORING_POINTER_UPDATE_FAILED",
        repairRequired: true,
      },
      callbacks(),
    ),
  );
  const closeBlocked = renderToStaticMarkup(
    createDesktopDocumentAuthoringWorkbenchElement(
      {
        ...snapshot(),
        saveState: DocumentSaveCoordinatorState.CloseBlocked,
      },
      callbacks(),
    ),
  );

  assert.match(failed, /role="alert"/);
  assert.match(failed, /data-workspace-global-host[^>]*>[\s\S]*role="alert"/);
  assert.match(failed, /다시 시도/);
  assert.match(readOnly, /읽기 전용 복구/);
  assert.match(readOnly, /변경 취소/);
  assert.match(closeBlocked, /저장하지 않은 변경/);
  assert.match(closeBlocked, /저장 다시 시도/);
  assert.match(closeBlocked, /변경 취소/);
  assert.match(closeBlocked, /계속 편집/);
  assert.doesNotMatch(`${failed}${readOnly}`, /DOCUMENT_AUTHORING_STORAGE_UNAVAILABLE|DOCUMENT_AUTHORING_POINTER_UPDATE_FAILED/);
  assert.equal(
    readOnly.includes("body one") && readOnly.includes("DOCUMENT_AUTHORING"),
    false,
  );
});

test("React authoring maps link and history failures without raw codes or English controls", () => {
  const linkMarkup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks(), {
    links: { state: "Failed", workspaceId: "workspace-1", documentId: "doc-1", generation: 2, errorCode: "LINK_OVERVIEW_UNAVAILABLE", retryable: false },
    inspector: { tab: "links", unlink: { status: "Closed" } },
  }));
  const historyMarkup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks(), {
    history: { status: "Failed", entries: [], errorCode: "DOCUMENT_HISTORY_QUERY_FAILED" },
    inspector: { tab: "history", unlink: { status: "Closed" } },
  }));

  assert.match(linkMarkup, /연결된 문서를 불러오지 못했습니다/);
  assert.match(historyMarkup, /문서 이력/);
  assert.match(historyMarkup, /이력 불러오기/);
  assert.match(historyMarkup, /문서 이력을 불러오지 못했습니다/);
  assert.doesNotMatch(`${linkMarkup}${historyMarkup}`, /LINK_OVERVIEW_UNAVAILABLE|DOCUMENT_HISTORY_QUERY_FAILED|Load history|History/);
});

test("React authoring workbench renders real backlink identities and bounded link state", () => {
  const markup = renderToStaticMarkup(
    createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks(), {
      links: {
        state: "Ready",
        workspaceId: "workspace-1",
        documentId: "doc-1",
        generation: 1,
        panel: {
          mode: "links",
          queryName: "get-link-overview",
          workspaceId: "workspace-1",
          documentId: "doc-1",
          backlinks: [{
            workspaceId: "workspace-1",
            sourceDocumentId: "doc-linked",
            targetDocumentId: "doc-1",
            sourceTitle: "Linked architecture",
            sourcePath: "fixture/linked",
          }],
          unresolvedLinks: [],
          orphanDocuments: [],
        },
      },
    }),
  );

  assert.match(markup, /data-link-overview-state="Ready"/);
  assert.match(markup, /data-linked-document-id="doc-linked"/);
  assert.match(markup, /Linked architecture/);
  assert.doesNotMatch(markup, /연결된 문서가 없습니다/);
});

test("React authoring presents the derived title without a separate metadata editor", () => {
  const focused = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks(), {
  }));
  assert.match(focused, /class="authoring-title-display"/);
  assert.match(focused, />Source</);
  assert.doesNotMatch(focused, /data-action="edit-document-title"/);
  assert.doesNotMatch(focused, /authoring-title-input/);
  assert.doesNotMatch(focused, /notes\/source\.md/);
});

test("React authoring history renders user labels without visible version identity", () => {
  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks(), {
    history: {
      status: "Ready",
      entries: [{
        versionId: "internal-version-secret",
        versionLabel: "버전 1",
        createdAtLabel: "2026. 7. 15. 오후 3:00",
        summaryLabel: "문서 저장",
      }],
    },
    inspector: { tab: "history", unlink: { status: "Closed" } },
  }));
  const visible = markup.replace(/\sdata-version-id="[^"]*"/g, "");

  assert.match(visible, /버전 1/);
  assert.match(visible, /2026\. 7\. 15/);
  assert.match(visible, /문서 저장/);
  assert.doesNotMatch(visible, /internal-version-secret/);
});

test("React authoring history keeps entries during cursor load-more and exposes bounded retry", () => {
  const base = {
    entries: [{
      versionId: "internal-version-secret",
      versionLabel: "버전 51",
      createdAtLabel: "2026. 7. 16.",
      summaryLabel: "문서 저장",
    }],
    nextCursor: "opaque-cursor-secret",
  };
  const callbacksWithMore = { ...callbacks(), onLoadMoreHistory() {} };
  const ready = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(
    snapshot(),
    callbacksWithMore,
    { history: { status: "Ready", ...base }, inspector: { tab: "history", unlink: { status: "Closed" } } },
  ));
  const loading = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(
    snapshot(),
    callbacksWithMore,
    { history: { status: "LoadingMore", ...base }, inspector: { tab: "history", unlink: { status: "Closed" } } },
  ));
  const failed = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(
    snapshot(),
    callbacksWithMore,
    {
      history: { status: "Ready", ...base, loadMoreErrorCode: "DOCUMENT_QUERY_STORAGE_UNAVAILABLE" },
      inspector: { tab: "history", unlink: { status: "Closed" } },
    },
  ));

  assert.match(ready, /data-action="load-more-history"(?![^>]*disabled)/);
  assert.match(ready, /이전 이력 더 보기/);
  assert.match(loading, /data-action="load-more-history"[^>]*disabled/);
  assert.match(loading, /불러오는 중/);
  assert.match(failed, /이전 이력을 불러오지 못했습니다/);
  assert.match(failed, /다시 시도/);
  for (const markup of [ready, loading, failed]) {
    const visible = markup.replace(/\sdata-version-id="[^"]*"/g, "");
    assert.match(visible, /버전 51/);
    assert.doesNotMatch(visible, /opaque-cursor-secret|internal-version-secret/);
  }
});

test("React authoring virtualizes two hundred history entries without exposing version identities", () => {
  const entries = Array.from({ length: 200 }, (_, index) => ({
    versionId: `opaque-history-secret-${index}`,
    versionLabel: `버전 ${200 - index}`,
    createdAtLabel: "2026. 7. 17.",
    summaryLabel: "문서 저장",
  }));
  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks(), {
    history: { status: "Ready", entries },
    inspector: { tab: "history", unlink: { status: "Closed" } },
  }));

  assert.equal((markup.match(/data-history-entry="visible"/g) ?? []).length, 50);
  assert.match(markup, /data-history-window="1-50\/200"/);
  assert.match(markup, /이력 1-50 \/ 전체 200개/);
  assert.match(markup, /data-action="next-history-window"/);
  assert.match(markup, /data-action="previous-history-window"[^>]*disabled/);
  assert.doesNotMatch(markup, /opaque-history-secret-/);

  const selectedAcrossWindows = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), {
    ...callbacks(),
    onToggleHistoryCompareSelection() {},
    onCompareSelectedVersions() {},
  }, {
    history: {
      status: "Ready",
      entries,
      comparison: {
        status: "TwoSelected",
        selections: [
          { versionId: "opaque-history-secret-0", versionLabel: "버전 200" },
          { versionId: "opaque-history-secret-100", versionLabel: "버전 100" },
        ],
      },
    },
    inspector: { tab: "history", unlink: { status: "Closed" } },
  }));
  assert.match(selectedAcrossWindows, /data-action="compare-selected-versions"(?![^>]*disabled)/);
  assert.doesNotMatch(selectedAcrossWindows, /opaque-history-secret-/);
});

test("React history window consumes explicit focus requests after a window transition", async () => {
  const source = await readFile(new URL("../src/react_document_authoring_workbench.ts", import.meta.url), "utf8");
  assert.match(source, /focusRequest\.current\s*=\s*transition\.focusRequest/);
  assert.match(source, /firstVisibleAction\.current\?\.focus\(\)/);
  assert.match(source, /index\s*===\s*0\s*\?\s*firstVisibleAction/);
});

test("React authoring history enables version-pair compare only for two user-facing selections", () => {
  const entries = [
    { versionId: "opaque-left-secret", versionLabel: "버전 7", createdAtLabel: "오늘", summaryLabel: "문서 저장" },
    { versionId: "opaque-right-secret", versionLabel: "버전 5", createdAtLabel: "어제", summaryLabel: "문서 저장" },
  ];
  const pairCallbacks = {
    ...callbacks(),
    onToggleHistoryCompareSelection() {},
    onCompareSelectedVersions() {},
  };
  const one = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), pairCallbacks, {
    history: {
      status: "Ready",
      entries,
      comparison: {
        status: "OneSelected",
        selections: [{ versionId: "opaque-left-secret", versionLabel: "버전 7" }],
      },
    },
    inspector: { tab: "history", unlink: { status: "Closed" } },
  }));
  const two = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), pairCallbacks, {
    history: {
      status: "Ready",
      entries,
      comparison: {
        status: "TwoSelected",
        selections: [
          { versionId: "opaque-left-secret", versionLabel: "버전 7" },
          { versionId: "opaque-right-secret", versionLabel: "버전 5" },
        ],
      },
    },
    inspector: { tab: "history", unlink: { status: "Closed" } },
  }));

  assert.match(one, /data-action="compare-selected-versions"[^>]*disabled/);
  assert.match(one, /버전 7[^<]*선택됨/);
  assert.match(two, /data-action="compare-selected-versions"(?![^>]*disabled)/);
  assert.match(two, /선택한 두 버전 비교/);
  assert.match(two, /aria-pressed="true"/);
  const visible = two.replace(/\sdata-version-id="[^"]*"/g, "");
  assert.doesNotMatch(visible, /opaque-left-secret|opaque-right-secret/);
});

test("React authoring history renders compare action and semantic diff without visible tokens", () => {
  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), {
    ...callbacks(),
    onCompareVersion() {},
    onCloseDiff() {},
  }, {
    history: {
      status: "Ready",
      entries: [{
        versionId: "history-token-secret",
        versionLabel: "버전 1",
        createdAtLabel: "2026. 7. 15. 오후 3:00",
        summaryLabel: "문서 저장",
      }],
      diff: {
        status: "Ready",
        targetVersionId: "history-token-secret",
        targetVersionLabel: "버전 1",
        addedCount: 1,
        removedCount: 1,
        attachmentDiff: {
          status: "Known",
          added: [{ label: "새 설계서.pdf", availability: "Available" }],
          removed: [{ label: "이전 설계서.pdf", availability: "Missing" }],
          relabeled: [{ beforeLabel: "초안.pdf", afterLabel: "최종안.pdf", availability: "Available" }],
          unchangedCount: 1,
        },
        titleDelta: { kind: "Changed", before: "현재 제목", after: "이전 제목" },
        hunks: [{
          oldStartLine: 1,
          newStartLine: 1,
          addedCount: 1,
          removedCount: 1,
          lines: [
            { kind: "Removed", text: "현재 본문", oldLineNumber: 2 },
            { kind: "Added", text: "이전 본문", newLineNumber: 2 },
          ],
        }],
      },
    },
    inspector: { tab: "history", unlink: { status: "Closed" } },
  }));
  const visible = markup
    .replace(/\sdata-version-id="[^"]*"/g, "")
    .replace(/\sdata-diff-target="[^"]*"/g, "");

  assert.match(visible, /data-action="compare-current-version"/);
  assert.match(visible, /현재 문서와 비교/);
  assert.match(visible, /class="authoring-workspace mode-compare"/);
  assert.match(visible, /data-action="close-document-diff"/);
  assert.match(visible, /편집기로 돌아가기/);
  assert.match(visible, /버전 1 비교 결과/);
  assert.match(visible, /추가 1줄/);
  assert.match(visible, /삭제 1줄/);
  assert.match(visible, /현재 제목/);
  assert.match(visible, /이전 제목/);
  assert.match(visible, /class="diff-line removed"/);
  assert.match(visible, /class="diff-line added"/);
  assert.match(visible, /현재 본문/);
  assert.match(visible, /이전 본문/);
  assert.match(visible, /첨부 파일 변경/);
  assert.match(visible, /추가됨/);
  assert.match(visible, /새 설계서\.pdf/);
  assert.match(visible, /제거됨/);
  assert.match(visible, /이전 설계서\.pdf/);
  assert.match(visible, /파일을 찾을 수 없음/);
  assert.match(visible, /이름 변경/);
  assert.match(visible, /초안\.pdf/);
  assert.match(visible, /최종안\.pdf/);
  assert.match(visible, /변경 없음 1개/);
  assert.doesNotMatch(visible, /history-token-secret/);
});

test("React authoring diff distinguishes legacy attachment history from no changes", () => {
  const history = {
    status: "Ready" as const,
    entries: [],
    diff: {
      status: "Ready" as const,
      targetVersionId: "legacy-token-secret",
      targetVersionLabel: "버전 1",
      addedCount: 0,
      removedCount: 0,
      attachmentDiff: { status: "LegacyUnknown" as const },
      titleDelta: { kind: "Unchanged" as const },
      hunks: [],
    },
  };
  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(
    snapshot(),
    callbacks(),
    { history },
  )).replace(/\sdata-diff-target="[^"]*"/g, "");

  assert.match(markup, /과거 형식으로 저장된 버전이라 첨부 파일 변경을 확인할 수 없습니다/);
  assert.doesNotMatch(markup, /첨부 파일 변경 없음/);
  assert.doesNotMatch(markup, /legacy-token-secret/);
});

test("React authoring diff exposes loading too-large and safe failure states", () => {
  const baseHistory = {
    status: "Ready" as const,
    entries: [{
      versionId: "v1",
      versionLabel: "버전 1",
      createdAtLabel: "저장 시각 없음",
      summaryLabel: "문서 저장",
    }],
  };
  const loading = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks(), {
    history: { ...baseHistory, diff: { status: "Loading", targetVersionId: "v1", targetVersionLabel: "버전 1" } },
  }));
  const tooLarge = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks(), {
    history: {
      ...baseHistory,
      diff: {
        status: "TooLarge",
        targetVersionId: "v1",
        targetVersionLabel: "버전 1",
        limitReason: "bytes",
        attachmentDiff: {
          status: "Known",
          added: [{ label: "대용량 문서 첨부.pdf", availability: "Available" }],
          removed: [],
          relabeled: [],
          unchangedCount: 0,
        },
      },
    },
  }));
  const failed = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks(), {
    history: { ...baseHistory, diff: { status: "Failed", targetVersionId: "v1", targetVersionLabel: "버전 1", errorCode: "DOCUMENT_DIFF_STORAGE_UNAVAILABLE" } },
  }));

  assert.match(loading, /비교 결과를 불러오는 중/);
  assert.match(tooLarge, /앱에서 바로 비교하기에는 문서가 너무 큽니다/);
  assert.match(tooLarge, /대용량 문서 첨부\.pdf/);
  assert.match(failed, /role="alert"/);
  assert.match(failed, /비교 결과를 불러오지 못했습니다/);
  assert.doesNotMatch(failed, /DOCUMENT_DIFF_STORAGE_UNAVAILABLE/);
});

test("React authoring diff exposes background progress cancel and retry without operation identity", () => {
  const baseHistory = { status: "Ready" as const, entries: [] };
  const activeCallbacks = {
    ...callbacks(),
    onCancelBackgroundDiff() {},
    onRetryBackgroundDiff() {},
  };
  const accepted = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), activeCallbacks, {
    history: { ...baseHistory, diff: { status: "Accepted", targetVersionId: "version-secret", targetVersionLabel: "버전 8" } },
  }));
  const running = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), activeCallbacks, {
    history: { ...baseHistory, diff: { status: "Running", targetVersionId: "version-secret", targetVersionLabel: "버전 8" } },
  }));
  const cancelled = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), activeCallbacks, {
    history: { ...baseHistory, diff: { status: "Cancelled", targetVersionId: "version-secret", targetVersionLabel: "버전 8" } },
  }));
  const expired = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), activeCallbacks, {
    history: { ...baseHistory, diff: { status: "Expired", targetVersionId: "version-secret", targetVersionLabel: "버전 8" } },
  }));

  assert.match(accepted, /큰 문서 비교를 준비하고 있습니다/);
  assert.match(running, /큰 문서 변경 내용을 비교하고 있습니다/);
  assert.match(running, /data-action="cancel-background-document-diff"/);
  assert.match(running, />비교 취소</);
  assert.match(cancelled, /문서 비교를 취소했습니다/);
  assert.match(cancelled, /data-action="retry-background-document-diff"/);
  assert.match(expired, /앱이 다시 시작되어 문서를 다시 비교해야 합니다/);
  assert.match(expired, /data-action="retry-background-document-diff"/);
  for (const markup of [accepted, running, cancelled, expired]) {
    assert.doesNotMatch(markup, /version-secret|operation-token|snapshotRef|\.md/);
  }
});

test("React authoring renders a bounded first window for one thousand diff hunks", () => {
  const hunks = Array.from({ length: 1_000 }, (_, index) => ({
    oldStartLine: index + 1,
    newStartLine: index + 1,
    addedCount: 1,
    removedCount: 0,
    lines: [{ kind: "Added" as const, text: `사용자 변경 ${index + 1}`, newLineNumber: index + 1 }],
  }));
  const markup = renderToStaticMarkup(createDesktopDocumentAuthoringWorkbenchElement(snapshot(), callbacks(), {
    history: {
      status: "Ready",
      entries: [],
      diff: {
        status: "Ready",
        targetVersionId: "opaque-version-token",
        targetVersionLabel: "버전 1",
        addedCount: 1_000,
        removedCount: 0,
        attachmentDiff: { status: "Known", added: [], removed: [], relabeled: [], unchangedCount: 0 },
        titleDelta: { kind: "Unchanged" },
        hunks,
      },
    },
    inspector: { tab: "history", unlink: { status: "Closed" } },
  }));

  assert.equal((markup.match(/class="diff-hunk"/g) ?? []).length, 50);
  assert.match(markup, /변경 구간 1–50 \/ 1000/);
  assert.match(markup, /data-action="previous-diff-hunks"[^>]*disabled/);
  assert.match(markup, /data-action="next-diff-hunks"/);
  assert.match(markup, /사용자 변경 50/);
  assert.doesNotMatch(markup, /사용자 변경 51/);
  assert.doesNotMatch(markup, /opaque-version-token/);
});

function snapshot() {
  return {
    workspaceId: "workspace-1",
    documentId: "doc-1",
    title: "Source",
    path: "notes/source.md",
    body: [
      "# Source",
      "",
      "| Item | Value |",
      "| :--- | ---: |",
      "| Grid | Ready |",
    ].join("\n"),
    revision: 1,
    persistedRevision: 0,
    expectedVersionId: "v1",
    saveState: DocumentSaveCoordinatorState.Dirty,
  } as const;
}

function callbacks() {
  return {
    onHome() {},
    onMode() {},
    onBodyChange() {},
    onSave() {},
    onRetry() {},
    onDiscard() {},
    onCancel() {},
  };
}

function restoreDiff() {
  return {
    workspaceId: "opaque-workspace-token",
    documentId: "opaque-document-token",
    status: "Complete",
    leftVersionId: "opaque-current-token",
    rightVersionId: "opaque-version-token",
    addedCount: 1,
    removedCount: 1,
    attachmentDiff: {
      status: "Known",
      added: [{ label: "추가 자료.pdf", availability: "Available" }],
      removed: [{ label: "제거 자료.pdf", availability: "Available" }],
      relabeled: [],
      unchangedCount: 0,
    },
    titleDelta: { kind: "Changed", before: "현재 제목", after: "복원할 제목" },
    hunks: [{
      oldStartLine: 1,
      newStartLine: 1,
      addedCount: 1,
      removedCount: 1,
      lines: [
        { kind: "Removed", text: "현재 본문", oldLineNumber: 1 },
        { kind: "Added", text: "복원할 본문", newLineNumber: 1 },
      ],
    }],
  } as const;
}
