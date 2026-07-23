import assert from "node:assert/strict";
import test from "node:test";

import {
  applyDocumentAssetLibraryLoad,
  beginDocumentAssetLibraryLink,
  closeDocumentAssetLibrary,
  completeDocumentAssetLibraryLink,
  createDocumentAssetLibraryState,
  requestDocumentAssetLibraryOpen,
  requestDocumentAssetLibraryMore,
  selectDocumentAssetLibraryItem,
} from "../src/document_asset_library_state.ts";
import { applyDesktopAssetResult } from "../src/desktop_asset_controller.ts";

test("document asset library isolates workspace query from the document attachment snapshot", () => {
  const initial = createDocumentAssetLibraryState("workspace-1");
  const loading = requestDocumentAssetLibraryOpen(initial, "doc-1");
  const workspaceReady = applyDesktopAssetResult(loading.assets, loading.assets.generation, {
    queryName: "list-workspace-assets",
    workspaceId: "workspace-1",
    assets: [{ assetId: "asset-private", label: "Architecture", fileName: "architecture.pdf", mediaType: "application/pdf", byteSize: 20, status: "available" }],
  });
  const ready = applyDocumentAssetLibraryLoad(loading, loading.generation, workspaceReady);
  const selected = selectDocumentAssetLibraryItem(ready, "asset-private");

  assert.equal(loading.status, "Loading");
  assert.equal(loading.assets.scope, "Workspace");
  assert.equal(loading.assets.documentId, "doc-1");
  assert.equal(ready.status, "Ready");
  assert.equal(ready.assets.selectedAssetId, undefined);
  assert.equal(selected.assets.selectedAssetId, "asset-private");
  assert.equal(closeDocumentAssetLibrary(selected).status, "Closed");
});

test("document asset library completes only a matching durable document readback", () => {
  const loading = requestDocumentAssetLibraryOpen(createDocumentAssetLibraryState("workspace-1"), "doc-1");
  const workspaceReady = applyDesktopAssetResult(loading.assets, loading.assets.generation, {
    queryName: "list-workspace-assets",
    workspaceId: "workspace-1",
    assets: [{ assetId: "asset-1", label: "Architecture", fileName: "architecture.pdf", mediaType: "application/pdf", byteSize: 20, status: "available" }],
  });
  const ready = selectDocumentAssetLibraryItem(
    applyDocumentAssetLibraryLoad(loading, loading.generation, workspaceReady),
    "asset-1",
  );
  const linking = beginDocumentAssetLibraryLink(ready);
  const documentReadback = Object.freeze({
    ...linking.assets,
    scope: "Document" as const,
    documentId: "doc-1",
    mutationState: "Idle" as const,
  });
  const completed = completeDocumentAssetLibraryLink(linking, linking.generation, documentReadback);

  assert.equal(linking.status, "Linking");
  assert.equal(completed.library.status, "Closed");
  assert.strictEqual(completed.documentAssets, documentReadback);

  const failed = completeDocumentAssetLibraryLink(linking, linking.generation, Object.freeze({ ...linking.assets, mutationState: "Failed" as const }));
  assert.equal(failed.library.status, "Failed");
  assert.equal(failed.documentAssets, undefined);
});

test("document asset library ignores stale load completion", () => {
  const first = requestDocumentAssetLibraryOpen(createDocumentAssetLibraryState("workspace-1"), "doc-1");
  const second = requestDocumentAssetLibraryOpen(first, "doc-2");
  assert.strictEqual(applyDocumentAssetLibraryLoad(second, first.generation, first.assets), second);
});

test("document asset library preserves selection while loading a next page", () => {
  const loading = requestDocumentAssetLibraryOpen(createDocumentAssetLibraryState("workspace-1"), "doc-1");
  const workspaceReady = applyDesktopAssetResult(loading.assets, loading.assets.generation, {
    queryName: "list-workspace-assets",
    workspaceId: "workspace-1",
    assets: [{ assetId: "asset-1", label: "First", fileName: "first.pdf", mediaType: "application/pdf", byteSize: 20, status: "available" }],
    nextCursor: "opaque-next-page",
  });
  const selected = selectDocumentAssetLibraryItem(applyDocumentAssetLibraryLoad(loading, loading.generation, workspaceReady), "asset-1");
  const more = requestDocumentAssetLibraryMore(selected);

  assert.equal(more.status, "LoadingMore");
  assert.equal(more.assets.selectedAssetId, "asset-1");
  assert.strictEqual(requestDocumentAssetLibraryMore(more), more);
});
