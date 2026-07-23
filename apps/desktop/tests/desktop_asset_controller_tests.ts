import assert from "node:assert/strict";
import test from "node:test";

import type { DocumentAssetsPage, LocalDesktopCommandClient } from "@sponzey-cabinet/client-core";
import {
  applyDesktopAssetDetailResult,
  applyDesktopAssetResult,
  applyDesktopAssetDragState,
  createDesktopAssetPlacementOptions,
  beginDesktopAssetImport,
  cancelDesktopAssetImport,
  createDesktopAssetSnapshot,
  importDesktopDocumentAssets,
  linkDesktopSelectedAsset,
  loadDesktopAssetDetail,
  loadDesktopAssetPreview,
  loadDesktopDocumentAssets,
  loadDesktopWorkspaceAssets,
  requestDesktopAssetLoad,
  requestDesktopWorkspaceAssetLoad,
  requestDesktopWorkspaceAssetNextPage,
  selectDesktopAsset,
  requestDesktopAssetPreview,
  requestDesktopAssetOpen,
  setDesktopAssetMediaFilter,
  setDesktopAssetQuery,
  visibleDesktopAssets,
  repairDesktopAttachmentProjection,
  openDesktopSelectedAsset,
  closeDesktopAssetPreview,
  unlinkDesktopSelectedAsset,
} from "../src/desktop_asset_controller.ts";
import { applyAttachmentFileStatus, createAttachmentFileSnapshot } from "../src/attachment_operation_presenter.ts";

test("Asset placement options are empty before load and use the durable page after load", () => {
  const idle = createDesktopAssetSnapshot("workspace-1");
  const loading = requestDesktopAssetLoad(idle, undefined);
  const ready = applyDesktopAssetResult(loading, loading.generation, {
    queryName: "list-workspace-assets",
    workspaceId: "workspace-1",
    assets: page().assets,
  });

  assert.deepEqual(createDesktopAssetPlacementOptions(idle), []);
  assert.deepEqual(createDesktopAssetPlacementOptions(ready), [
    { identity: "asset-1", label: "architecture.pdf" },
  ]);
});

test("Asset controller loads document metadata into an explicit Ready snapshot", async () => {
  let received: unknown;
  const client = {
    async getAssetMetadata(query: unknown) {
      received = query;
      return page();
    },
  } as Pick<LocalDesktopCommandClient, "getAssetMetadata">;
  const loading = requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1");

  const ready = await loadDesktopDocumentAssets(client, loading);
  const selected = selectDesktopAsset(ready, "asset-1");

  assert.equal(ready.state, "Ready");
  assert.equal(selected.selectedAssetId, "asset-1");
  assert.deepEqual(received, {
    queryName: "list-document-assets",
    workspaceId: "workspace-1",
    documentId: "doc-1",
  });
});

test("Asset controller loads workspace scope, empty page, and safe failure", async () => {
  const global = requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), undefined);
  const globalReady = await loadDesktopWorkspaceAssets({
    async listWorkspaceAssets() {
      return { workspaceId: "workspace-1", assets: page().assets, nextCursor: "a".repeat(64) };
    },
  }, global);
  const loading = requestDesktopAssetLoad(global, "doc-1");
  const empty = applyDesktopAssetResult(loading, loading.generation, { ...page(), assets: [] });
  const failed = await loadDesktopDocumentAssets({
    async getAssetMetadata() { throw new Error("/Users/private/asset.bin"); },
  }, loading);

  assert.equal(global.state, "Loading");
  assert.equal(globalReady.state, "Ready");
  assert.equal(globalReady.scope, "Workspace");
  assert.equal(globalReady.page?.nextCursor, "a".repeat(64));
  assert.equal(empty.state, "Empty");
  assert.equal(failed.state, "Failed");
  assert.equal(failed.errorCode, "COMMAND_BRIDGE_FAILED");
  assert.equal(JSON.stringify(failed).includes("/Users/private"), false);
});

test("Asset controller appends a bounded workspace page by opaque cursor without duplicates", async () => {
  const workspaceLoading = requestDesktopWorkspaceAssetLoad(
    requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1"),
  );
  const first = applyDesktopAssetResult(workspaceLoading, workspaceLoading.generation, {
    queryName: "list-workspace-assets",
    workspaceId: "workspace-1",
    assets: page().assets,
    nextCursor: "opaque-next-page",
  });
  const nextLoading = requestDesktopWorkspaceAssetNextPage(first);
  let request: unknown;
  const appended = await loadDesktopWorkspaceAssets({
    async listWorkspaceAssets(input) {
      request = input;
      return {
        workspaceId: "workspace-1",
        assets: [page().assets[0]!, { ...page().assets[0]!, assetId: "asset-2", fileName: "second.pdf" }],
      };
    },
  }, nextLoading);

  assert.deepEqual(request, { workspaceId: "workspace-1", cursor: "opaque-next-page", limit: 200 });
  assert.deepEqual(appended.page?.assets.map((asset) => asset.assetId), ["asset-1", "asset-2"]);
  assert.equal(appended.page?.nextCursor, undefined);
  assert.strictEqual(requestDesktopWorkspaceAssetNextPage(nextLoading), nextLoading);
});

test("Asset controller links a workspace asset and completes through document readback", async () => {
  const workspaceLoading = requestDesktopWorkspaceAssetLoad(
    requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-2"),
  );
  const workspaceReady = await loadDesktopWorkspaceAssets({
    async listWorkspaceAssets() { return { workspaceId: "workspace-1", assets: page().assets }; },
  }, workspaceLoading);
  const selected = selectDesktopAsset(workspaceReady, "asset-1");
  let linkRequest: unknown;

  const linked = await linkDesktopSelectedAsset(
    { async link(request) { linkRequest = request; return { outcome: "fresh", delta: "linked", revisionNumber: 2 }; } },
    {
      async getCurrentDocument() { return { workspaceId: "workspace-1", documentId: "doc-2", title: "문서", body: "문서", versionId: "version-current" }; },
      async getAssetMetadata() { return { ...page(), documentId: "doc-2" }; },
    },
    selected,
    () => "operation-link-1",
  );

  assert.deepEqual(linkRequest, {
    workspaceId: "workspace-1",
    documentId: "doc-2",
    assetId: "asset-1",
    label: "Architecture",
    operationId: "operation-link-1",
    expectedCurrentVersionToken: "version-current",
  });
  assert.equal(linked.scope, "Document");
  assert.equal(linked.mutationState, "Idle");
  assert.equal(linked.selectedAssetId, "asset-1");
});

test("Asset controller ignores stale completion and invalid selection", () => {
  const first = requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1");
  const second = requestDesktopAssetLoad(first, "doc-2");
  const unchanged = applyDesktopAssetResult(second, first.generation, page());

  assert.strictEqual(unchanged, second);
  assert.strictEqual(selectDesktopAsset(second, "unknown"), second);
});

test("Asset controller owns bounded-page query and media filter state without matching internal identity", () => {
  const loading = requestDesktopWorkspaceAssetLoad(createDesktopAssetSnapshot("workspace-1"));
  const ready = applyDesktopAssetResult(loading, loading.generation, {
    queryName: "list-workspace-assets",
    workspaceId: "workspace-1",
    assets: [
      { assetId: "private-query-token", label: "제품 설계", fileName: "cabinet.pdf", mediaType: "application/pdf", byteSize: 10, status: "available" },
      { assetId: "asset-2", label: "회의 메모", fileName: "notes.txt", mediaType: "text/plain", byteSize: 20, status: "available" },
      { assetId: "asset-3", label: "화면", fileName: "screen.png", mediaType: "image/png", byteSize: 30, status: "available" },
    ],
  });

  const byLabel = setDesktopAssetQuery(ready, "회의");
  assert.deepEqual(visibleDesktopAssets(byLabel).map((asset) => asset.fileName), ["notes.txt"]);
  assert.equal(visibleDesktopAssets(setDesktopAssetQuery(ready, "private-query-token")).length, 0);
  assert.deepEqual(
    visibleDesktopAssets(setDesktopAssetMediaFilter(setDesktopAssetQuery(ready, ""), "pdf")).map((asset) => asset.fileName),
    ["cabinet.pdf"],
  );
});

test("Asset scope result preserves an available selection and clears stale detail when it disappears", () => {
  const documentLoading = requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1");
  const documentReady = applyDesktopAssetResult(documentLoading, documentLoading.generation, page());
  const selected = {
    ...selectDesktopAsset(documentReady, "asset-1"),
    detailState: "Ready" as const,
    detail: { assetId: "asset-1", fileName: "architecture.pdf", mediaType: "application/pdf", byteSize: 2048, version: 1, previewCapability: "pdf" as const, extractionStatus: "not_requested" as const, referenceCount: 1, linkedDocumentIds: ["doc-1"], linkedDocuments: [{ documentId: "doc-1", title: "Cabinet 제품 지도", state: "available" as const }] },
    previewState: "Ready" as const,
    preview: { assetId: "asset-1", capability: "pdf" as const, mediaType: "application/pdf", presentation: "data_url" as const, content: "data:application/pdf;base64,AA==" },
  };
  const workspaceLoading = requestDesktopWorkspaceAssetLoad(selected);
  const preserved = applyDesktopAssetResult(workspaceLoading, workspaceLoading.generation, {
    queryName: "list-workspace-assets", workspaceId: "workspace-1", assets: page().assets,
  });
  assert.equal(preserved.selectedAssetId, "asset-1");

  const nextLoading = requestDesktopAssetLoad(preserved, "doc-2");
  const changed = applyDesktopAssetResult(nextLoading, nextLoading.generation, {
    queryName: "list-document-assets", workspaceId: "workspace-1", documentId: "doc-2",
    assets: [{ ...page().assets[0]!, assetId: "asset-2", fileName: "other.pdf" }],
  });
  assert.equal(changed.selectedAssetId, "asset-2");
  assert.equal(changed.detail, undefined);
  assert.equal(changed.detailState, "Idle");
  assert.equal(changed.preview, undefined);
  assert.equal(changed.previewState, "Idle");
});

test("Asset controller imports opaque selections and completes only after durable readback", async () => {
  const loading = requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1");
  const ready = applyDesktopAssetResult(loading, loading.generation, { ...page(), assets: [] });
  const selecting = beginDesktopAssetImport(ready);
  const progress: string[] = [];
  const calls: string[] = [];
  let importedRequest: unknown;

  const completed = await importDesktopDocumentAssets(
    {
      async selectFiles() {
        return { cancelled: false, files: [{ handle: "picker:1", fileName: "architecture.pdf", mediaType: "application/pdf", byteSize: 2048 }] };
      },
      async importFile(request) {
        calls.push("import");
        importedRequest = request;
        return { operationId: "operation-1", assetId: "asset-1", state: "completed" };
      },
    },
    {
      async getCurrentDocument() {
        calls.push("current");
        return { versionId: "version-current" };
      },
      async getAssetMetadata() { return page(); },
    } as Pick<LocalDesktopCommandClient, "getAssetMetadata" | "getCurrentDocument">,
    selecting,
    () => "attachment-import-1",
    (snapshot) => progress.push(snapshot.importState),
  );

  assert.equal(completed.importState, "Completed");
  assert.equal(completed.state, "Ready");
  assert.equal(completed.selectedAssetId, "asset-1");
  assert.deepEqual(progress, ["Importing", "Importing"]);
  assert.deepEqual(importedRequest, {
    workspaceId: "workspace-1",
    documentId: "doc-1",
    handle: "picker:1",
    label: "architecture.pdf",
    attachmentOperationId: "attachment-import-1",
    expectedCurrentVersionToken: "version-current",
  });
  assert.deepEqual(calls, ["current", "import"]);
});

test("Asset controller bounds retries while the durable attachment projection becomes visible", async () => {
  const selecting = beginDesktopAssetImport(
    requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1"),
  );
  let readbackCalls = 0;
  let delayCalls = 0;

  const completed = await importDesktopDocumentAssets(
    {
      async selectFiles() {
        return { cancelled: false, files: [{ handle: "picker:1", fileName: "architecture.pdf", mediaType: "application/pdf", byteSize: 2048 }] };
      },
      async importFile() {
        return { operationId: "operation-eventual", state: "completed", retryable: false, repairRequired: false } as const;
      },
    },
    {
      async getCurrentDocument() { return { versionId: "version-current" }; },
      async getAssetMetadata() {
        readbackCalls += 1;
        return readbackCalls === 1 ? { ...page(), assets: [] } : page();
      },
    } as Pick<LocalDesktopCommandClient, "getAssetMetadata" | "getCurrentDocument">,
    selecting,
    () => "attachment-import-eventual",
    () => {},
    undefined,
    {
      attempts: 2,
      intervalMs: 0,
      delay: async () => { delayCalls += 1; },
    },
  );

  assert.equal(completed.importState, "Completed");
  assert.equal(completed.selectedAssetId, "asset-1");
  assert.equal(readbackCalls, 2);
  assert.equal(delayCalls, 1);
});

test("Asset controller imports a prepared drop selection without reopening the picker", async () => {
  const selecting = beginDesktopAssetImport(
    requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1"),
  );
  let pickerCalls = 0;
  let importCalls = 0;
  const result = await importDesktopDocumentAssets(
    {
      async selectFiles() { pickerCalls += 1; return { cancelled: true, files: [] }; },
      async importFile() {
        importCalls += 1;
        return { operationId: "drop-operation-1", assetId: "asset-1", state: "completed" } as const;
      },
    },
    {
      async getCurrentDocument() { return { versionId: "version-drop-current" }; },
      async getAssetMetadata() { return page(); },
    } as Pick<LocalDesktopCommandClient, "getAssetMetadata" | "getCurrentDocument">,
    selecting,
    () => "drop-attachment-operation-1",
    () => {},
    { cancelled: false, files: [{ handle: "drop:1", fileName: "architecture.pdf", mediaType: "application/pdf", byteSize: 2048 }] },
  );

  assert.equal(pickerCalls, 0);
  assert.equal(importCalls, 1);
  assert.equal(result.importState, "Completed");
});

test("Asset controller applies explicit bounded drag presentation state", () => {
  const idle = requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1");
  const entered = applyDesktopAssetDragState(idle, { state: "entered", fileCount: 3 });
  const left = applyDesktopAssetDragState(entered, { state: "left", fileCount: 0 });

  assert.equal(entered.dropState, "Entered");
  assert.equal(entered.dropFileCount, 3);
  assert.equal(left.dropState, "Idle");
  assert.equal(left.dropFileCount, 0);
  const importing = Object.freeze({ ...idle, importState: "Importing" as const });
  assert.strictEqual(applyDesktopAssetDragState(importing, { state: "entered", fileCount: 1 }), importing);
});

test("Asset controller blocks duplicate import and reports readback mismatch safely", async () => {
  const loading = requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1");
  const selecting = beginDesktopAssetImport(loading);
  assert.strictEqual(beginDesktopAssetImport(selecting), selecting);

  const failed = await importDesktopDocumentAssets(
    {
      async selectFiles() { return { cancelled: false, files: [{ handle: "picker:1", fileName: "file.pdf", mediaType: "application/pdf", byteSize: 1 }] }; },
      async importFile() { return { operationId: "operation-1", assetId: "missing-asset", state: "completed" }; },
    },
    {
      async getCurrentDocument() { return { versionId: "version-current" }; },
      async getAssetMetadata() { return { ...page(), assets: [] }; },
    } as Pick<LocalDesktopCommandClient, "getAssetMetadata" | "getCurrentDocument">,
    selecting,
    () => "attachment-import-2",
    () => {},
  );

  assert.equal(failed.importState, "Failed");
  assert.equal(failed.importErrorCode, "ASSET_IMPORT_READBACK_MISMATCH");
});

test("Asset controller preserves immediate native recovery state and stable error", async () => {
  const selecting = beginDesktopAssetImport(
    requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1"),
  );

  const result = await importDesktopDocumentAssets(
    {
      async selectFiles() {
        return { cancelled: false, files: [{ handle: "picker:1", fileName: "recover.pdf", mediaType: "application/pdf", byteSize: 2 }] };
      },
      async importFile() {
        return {
          operationId: "operation-recovery-1",
          assetId: "asset-recovery-1",
          state: "recovery_required",
          errorCode: "asset_graph_reindex.repository_unavailable",
          retryable: true,
          repairRequired: true,
        } as const;
      },
    },
    {
      async getCurrentDocument() { return { versionId: "version-current" }; },
      async getAssetMetadata() { return { ...page(), assets: [] }; },
    } as Pick<LocalDesktopCommandClient, "getAssetMetadata" | "getCurrentDocument">,
    selecting,
    () => "attachment-recovery-1",
    () => {},
  );

  assert.equal(result.importState, "Failed");
  assert.equal(result.importErrorCode, "asset_graph_reindex.repository_unavailable");
  assert.equal(result.importOperations?.[0]?.stage, "RecoveryRequired");
  assert.equal(result.importOperations?.[0]?.errorCode, "asset_graph_reindex.repository_unavailable");
});

test("Asset controller blocks import before current query when operation identity is empty", async () => {
  const loading = requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1");
  const selecting = beginDesktopAssetImport(loading);
  let currentCalls = 0;
  let importCalls = 0;

  const failed = await importDesktopDocumentAssets(
    {
      async selectFiles() { return { cancelled: false, files: [{ handle: "picker:1", fileName: "file.pdf", mediaType: "application/pdf", byteSize: 1 }] }; },
      async importFile() { importCalls += 1; return { operationId: "unexpected", state: "completed" }; },
    },
    {
      async getCurrentDocument() { currentCalls += 1; return { versionId: "version-current" }; },
      async getAssetMetadata() { return page(); },
    } as Pick<LocalDesktopCommandClient, "getAssetMetadata" | "getCurrentDocument">,
    selecting,
    () => " ",
    () => {},
  );

  assert.equal(failed.importState, "Failed");
  assert.equal(failed.importErrorCode, "asset_import.invalid_operation_id");
  assert.equal(currentCalls, 0);
  assert.equal(importCalls, 0);
});

test("Asset controller refreshes current guard for every selected file", async () => {
  const loading = requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1");
  const selecting = beginDesktopAssetImport(loading);
  const expectedTokens = ["version-1", "version-2"];
  const operations = ["attachment-import-1", "attachment-import-2"];
  const requests: Array<{ attachmentOperationId: string; expectedCurrentVersionToken: string }> = [];
  let currentIndex = 0;

  const completed = await importDesktopDocumentAssets(
    {
      async selectFiles() {
        return {
          cancelled: false,
          files: [
            { handle: "picker:1", fileName: "one.pdf", mediaType: "application/pdf", byteSize: 1 },
            { handle: "picker:2", fileName: "two.pdf", mediaType: "application/pdf", byteSize: 2 },
          ],
        };
      },
      async importFile(request) {
        requests.push(request);
        return { operationId: `native-${requests.length}`, assetId: `asset-${requests.length}`, state: "completed" };
      },
    },
    {
      async getCurrentDocument() { return { versionId: expectedTokens[currentIndex++]! }; },
      async getAssetMetadata() {
        return { ...page(), assets: [
          { ...page().assets[0]!, assetId: "asset-1", fileName: "one.pdf" },
          { ...page().assets[0]!, assetId: "asset-2", fileName: "two.pdf" },
        ] };
      },
    } as Pick<LocalDesktopCommandClient, "getAssetMetadata" | "getCurrentDocument">,
    selecting,
    () => operations.shift() ?? "",
    () => {},
  );

  assert.equal(completed.importState, "Completed");
  assert.deepEqual(requests.map(({ attachmentOperationId, expectedCurrentVersionToken }) => ({ attachmentOperationId, expectedCurrentVersionToken })), [
    { attachmentOperationId: "attachment-import-1", expectedCurrentVersionToken: "version-1" },
    { attachmentOperationId: "attachment-import-2", expectedCurrentVersionToken: "version-2" },
  ]);
  assert.equal(currentIndex, 2);
});

test("Asset controller records deterministic partial success and continues after one file fails", async () => {
  const selecting = beginDesktopAssetImport(
    requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1"),
  );
  const operations = ["attachment-1", "attachment-2"];
  const imported: string[] = [];
  const progressQueues: string[][] = [];

  const result = await importDesktopDocumentAssets(
    {
      async selectFiles() {
        return { cancelled: false, files: [
          { handle: "picker:1", fileName: "/private/one.pdf", mediaType: "application/pdf", byteSize: 1 },
          { handle: "picker:2", fileName: "two.pdf", mediaType: "application/pdf", byteSize: 2 },
        ] };
      },
      async importFile(request) {
        imported.push(request.attachmentOperationId);
        if (request.attachmentOperationId === "attachment-1") throw new Error("/private/source.bin");
        return { operationId: request.attachmentOperationId, assetId: "asset-2", state: "completed" };
      },
    },
    {
      async getCurrentDocument() { return { versionId: `version-${imported.length + 1}` }; },
      async getAssetMetadata() {
        return { ...page(), assets: [{ ...page().assets[0]!, assetId: "asset-2", fileName: "two.pdf" }] };
      },
    } as Pick<LocalDesktopCommandClient, "getAssetMetadata" | "getCurrentDocument">,
    selecting,
    () => operations.shift() ?? "",
    (snapshot) => progressQueues.push((snapshot.importOperations ?? []).map((item) => item.stage)),
  );

  assert.deepEqual(imported, ["attachment-1", "attachment-2"]);
  assert.equal(result.importState, "Failed");
  assert.deepEqual(result.importOperations?.map((item) => [item.displayName, item.stage]), [
    ["one.pdf", "Failed"],
    ["two.pdf", "Completed"],
  ]);
  assert.equal(JSON.stringify(result.importOperations).includes("/private"), false);
  assert.ok(progressQueues.some((queue) => queue.includes("Failed")));
});

test("Asset controller preserves same-document queue and resets it when document changes", () => {
  const loading = requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1");
  const withQueue = Object.freeze({
    ...loading,
    importState: "Completed" as const,
    importOperations: Object.freeze([{ stage: "Completed" }]) as never,
  });
  const same = requestDesktopAssetLoad(withQueue, "doc-1");
  const workspace = requestDesktopWorkspaceAssetLoad(withQueue, "doc-1");
  const remounted = requestDesktopAssetLoad(workspace, "doc-1");
  const changed = requestDesktopAssetLoad(withQueue, "doc-2");

  assert.strictEqual(same.importOperations, withQueue.importOperations);
  assert.equal(same.importState, "Completed");
  assert.strictEqual(remounted.importOperations, withQueue.importOperations);
  assert.equal(remounted.importState, "Completed");
  assert.deepEqual(changed.importOperations, []);
  assert.equal(changed.importState, "Idle");
});

test("Asset controller loads native detail and confirms unlink through list readback", async () => {
  const loading = requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1");
  const ready = applyDesktopAssetResult(loading, loading.generation, page());
  const selected = selectDesktopAsset(ready, "asset-1");
  const lifecycle = {
    async selectFiles() { return { cancelled: true, files: [] }; },
    async importFile() { throw new Error("unused"); },
    async getDetail() {
      return { assetId: "asset-1", fileName: "architecture.pdf", mediaType: "application/pdf", byteSize: 2048, version: 1, previewCapability: "pdf", extractionStatus: "not_requested", referenceCount: 1, linkedDocumentIds: ["doc-1"], linkedDocuments: [{ documentId: "doc-1", title: "Cabinet 제품 지도", state: "available" }] } as const;
    },
    async unlink() { return { outcome: "fresh", delta: "unlinked", revisionNumber: 2 }; },
  };

  const detailed = await loadDesktopAssetDetail(lifecycle, selected);
  const unlinked = await unlinkDesktopSelectedAsset(
    lifecycle,
    {
      async getCurrentDocument() { return { workspaceId: "workspace-1", documentId: "doc-1", title: "문서", body: "문서", versionId: "version-current" }; },
      async getAssetMetadata() { return { ...page(), assets: [] }; },
    } as Pick<LocalDesktopCommandClient, "getAssetMetadata" | "getCurrentDocument">,
    detailed,
    () => "operation-unlink-1",
  );

  assert.equal(detailed.detail?.previewCapability, "pdf");
  assert.equal(detailed.detail?.version, 1);
  assert.equal(unlinked.state, "Empty");
  assert.equal(unlinked.mutationState, "Idle");
  assert.equal(unlinked.selectedAssetId, undefined);
});

test("Asset controller maps durable cancel terminal state", async () => {
  const operation = createAttachmentFileSnapshot({ generation: 1, operationId: "operation-1", fileName: "note.txt", byteSize: 1, state: "staging" });
  const snapshot = Object.freeze({
    ...requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1"),
    importState: "Importing" as const,
    importOperationId: "operation-1",
    importOperations: Object.freeze([operation]),
  });
  const client = {
    async cancelImport() { return { operationId: "operation-1", state: "cancelled" } as const; },
  } as never;

  const cancelled = await cancelDesktopAssetImport(client, snapshot);

  assert.equal(cancelled.importState, "Cancelled");
  assert.equal(cancelled.importOperations?.[0]?.stage, "Cancelled");
});

test("Asset controller repairs projection then verifies attachment readback", async () => {
  const operationBase = createAttachmentFileSnapshot({ generation: 1, operationId: "attachment-op-1", fileName: "repair.pdf", byteSize: 2, state: "projecting" });
  const operation = applyAttachmentFileStatus(operationBase, {
    generation: 1,
    operationId: "attachment-op-1",
    state: "recovery_required",
    errorCode: "asset_graph_reindex.repository_unavailable",
    assetId: "asset-1",
  });
  const snapshot = Object.freeze({
    ...requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1"),
    importState: "Failed" as const,
    importOperationId: "attachment-op-1",
    importOperations: Object.freeze([operation]),
  });
  const calls: string[] = [];
  const progressStages: string[] = [];

  const repaired = await repairDesktopAttachmentProjection(
    {
      async startRepair() { calls.push("start"); return { operationId: "projection-repair-1", state: "queued", attempt: 0, completedUnits: 0, totalUnits: 2 }; },
      async runRepair() { calls.push("run"); return { operationId: "projection-repair-1", state: "succeeded", attempt: 1, completedUnits: 2, totalUnits: 2 }; },
    },
    {
      async getAssetMetadata() { calls.push("readback"); return page(); },
    },
    snapshot,
    "attachment-op-1",
    (progress) => progressStages.push(progress.importOperations?.[0]?.stage ?? "missing"),
  );

  assert.deepEqual(calls, ["start", "run", "readback"]);
  assert.deepEqual(progressStages, ["Projecting", "Verifying"]);
  assert.equal(repaired.importState, "Completed");
  assert.equal(repaired.importOperations?.[0]?.stage, "Completed");
  assert.equal(repaired.importOperations?.[0]?.errorCode, undefined);
});

test("Asset controller keeps recovery required when projection repair does not succeed", async () => {
  const base = createAttachmentFileSnapshot({ generation: 2, operationId: "attachment-op-2", fileName: "repair.pdf", byteSize: 2, state: "projecting" });
  const operation = applyAttachmentFileStatus(base, { generation: 2, operationId: "attachment-op-2", state: "recovery_required" });
  const snapshot = Object.freeze({
    ...requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1"),
    importState: "Failed" as const,
    importOperationId: "attachment-op-2",
    importOperations: Object.freeze([operation]),
  });
  let readbackCalls = 0;

  const failed = await repairDesktopAttachmentProjection(
    {
      async startRepair() { return { operationId: "projection-repair-2", state: "queued", attempt: 0, completedUnits: 0, totalUnits: 2 }; },
      async runRepair() { return { operationId: "projection-repair-2", state: "failed_retryable", attempt: 1, completedUnits: 1, totalUnits: 2 }; },
    },
    { async getAssetMetadata() { readbackCalls += 1; return page(); } },
    snapshot,
    "attachment-op-2",
  );

  assert.equal(readbackCalls, 0);
  assert.equal(failed.importState, "Failed");
  assert.equal(failed.importOperations?.[0]?.stage, "RecoveryRequired");
  assert.equal(failed.importOperations?.[0]?.errorCode, "ATTACHMENT_PROJECTION_RECOVERY_REQUIRED");
});

test("Asset preview controller loads matching identity and clears payload on close or selection", async () => {
  const loading = requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1");
  const selected = selectDesktopAsset(applyDesktopAssetResult(loading, loading.generation, page()), "asset-1");
  const previewing = requestDesktopAssetPreview(selected);
  const ready = await loadDesktopAssetPreview({
    async getPreview() { return { assetId: "asset-1", capability: "text", mediaType: "text/plain", presentation: "text", content: "preview" } as const; },
  }, previewing);
  assert.equal(ready.previewState, "Ready");
  assert.equal(ready.preview?.content, "preview");
  assert.equal(closeDesktopAssetPreview(ready).preview, undefined);
  assert.equal(selectDesktopAsset(ready, "asset-1").previewState, "Idle");
});

test("Asset preview controller rejects identity mismatch and exposes unsupported state", async () => {
  const selected = selectDesktopAsset(applyDesktopAssetResult(requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1"), 1, page()), "asset-1");
  const failed = await loadDesktopAssetPreview({ async getPreview() { return { assetId: "other", capability: "text", mediaType: "text/plain", presentation: "text", content: "x" } as const; } }, requestDesktopAssetPreview(selected));
  assert.equal(failed.previewState, "Failed");
  const unsupported = await loadDesktopAssetPreview({ async getPreview() { return { assetId: "asset-1", capability: "unsupported", mediaType: "application/octet-stream", presentation: "unsupported" } as const; } }, requestDesktopAssetPreview(selected));
  assert.equal(unsupported.previewState, "Unsupported");
});

test("Asset external open controller has explicit terminal and retry states", async () => {
  const selected = selectDesktopAsset(
    applyDesktopAssetResult(requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1"), 1, page()),
    "asset-1",
  );
  const opening = requestDesktopAssetOpen(selected);
  assert.equal(opening.openState, "Opening");
  assert.strictEqual(requestDesktopAssetOpen(opening), opening);

  const opened = await openDesktopSelectedAsset({
    async openExternal(request) {
      assert.deepEqual(request, { workspaceId: "workspace-1", assetId: "asset-1" });
      return { opened: true };
    },
  }, opening);
  assert.equal(opened.openState, "Opened");

  const failed = await openDesktopSelectedAsset({
    async openExternal() { throw new Error("/private/object.bin"); },
  }, requestDesktopAssetOpen(selectDesktopAsset(opened, "asset-1")));
  assert.equal(failed.openState, "OpenFailed");
  assert.equal(failed.openErrorCode, "COMMAND_BRIDGE_FAILED");
  assert.equal(JSON.stringify(failed).includes("/private"), false);
  assert.equal(requestDesktopAssetOpen(failed).openState, "Opening");
});

test("Late asset detail readback preserves the latest external-open state", async () => {
  const selected = selectDesktopAsset(
    applyDesktopAssetResult(requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1"), 1, page()),
    "asset-1",
  );
  const detailed = await loadDesktopAssetDetail({
    async getDetail() {
      return {
        assetId: "asset-1",
        fileName: "architecture.pdf",
        mediaType: "application/pdf",
        byteSize: 2048,
        version: 1,
        status: "metadata_only",
        previewCapability: "pdf",
        extractionStatus: "not_requested",
        referenceCount: 1,
        linkedDocumentIds: ["doc-1"],
        linkedDocuments: [{ documentId: "doc-1", title: "Cabinet 제품 지도", state: "available" }],
      };
    },
  } as never, selected);
  const opened = Object.freeze({
    ...requestDesktopAssetOpen(selected),
    openState: "Opened" as const,
  });

  const merged = applyDesktopAssetDetailResult(opened, detailed);

  assert.equal(merged.detailState, "Ready");
  assert.equal(merged.detail?.assetId, "asset-1");
  assert.equal(merged.openState, "Opened");
  assert.equal(merged.openGeneration, opened.openGeneration);
  const differentSelection = Object.freeze({ ...opened, selectedAssetId: "asset-2" });
  assert.strictEqual(
    applyDesktopAssetDetailResult(differentSelection, detailed),
    differentSelection,
  );
});

function page(): DocumentAssetsPage {
  return {
    queryName: "list-document-assets",
    workspaceId: "workspace-1",
    documentId: "doc-1",
    assets: [{
      assetId: "asset-1",
      label: "Architecture",
      fileName: "architecture.pdf",
      mediaType: "application/pdf",
      byteSize: 2048,
      status: "metadata_only",
    }],
  };
}
