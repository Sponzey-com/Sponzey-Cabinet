import assert from "node:assert/strict";
import test from "node:test";

import type { DocumentAssetsPage, LocalDesktopCommandClient } from "@sponzey-cabinet/client-core";
import {
  applyDesktopAssetDetailResult,
  applyDesktopAssetResult,
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
  selectDesktopAsset,
  requestDesktopAssetPreview,
  requestDesktopAssetOpen,
  openDesktopSelectedAsset,
  closeDesktopAssetPreview,
  unlinkDesktopSelectedAsset,
} from "../src/desktop_asset_controller.ts";

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

test("Asset controller loads native detail and confirms unlink through list readback", async () => {
  const loading = requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1");
  const ready = applyDesktopAssetResult(loading, loading.generation, page());
  const selected = selectDesktopAsset(ready, "asset-1");
  const lifecycle = {
    async selectFiles() { return { cancelled: true, files: [] }; },
    async importFile() { throw new Error("unused"); },
    async getDetail() {
      return { assetId: "asset-1", fileName: "architecture.pdf", mediaType: "application/pdf", byteSize: 2048, version: 1, previewCapability: "pdf", extractionStatus: "not_requested", referenceCount: 1, linkedDocumentIds: ["doc-1"] } as const;
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
  const snapshot = Object.freeze({
    ...requestDesktopAssetLoad(createDesktopAssetSnapshot("workspace-1"), "doc-1"),
    importState: "Importing" as const,
    importOperationId: "operation-1",
  });
  const client = {
    async cancelImport() { return { operationId: "operation-1", state: "cancelled" } as const; },
  } as never;

  const cancelled = await cancelDesktopAssetImport(client, snapshot);

  assert.equal(cancelled.importState, "Cancelled");
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
