import assert from "node:assert/strict";
import test from "node:test";

import {
  DesktopAssetImportTransportError,
  createTauriAssetImportTransport,
} from "../src/tauri_asset_import_transport.ts";

test("asset import picker transport returns only opaque descriptors", async () => {
  let commandName = "";
  const client = createTauriAssetImportTransport(async (command) => {
    commandName = command;
    return { ok: true, data: { cancelled: false, files: [{ handle: "picker:1", fileName: "design.pdf", mediaType: "application/pdf", byteSize: 8 }] }, retryable: false };
  });

  const result = await client.selectFiles();

  assert.equal(commandName, "select_desktop_asset_import_files");
  assert.equal(result.files[0]?.handle, "picker:1");
  assert.equal("path" in result.files[0]!, false);
});

test("asset import picker transport preserves cancel and rejects malformed responses", async () => {
  const cancelled = createTauriAssetImportTransport(async () => ({ ok: true, data: { cancelled: true, files: [] }, retryable: false }));
  assert.equal((await cancelled.selectFiles()).cancelled, true);

  const malformed = createTauriAssetImportTransport(async () => ({ ok: true, data: { cancelled: false, files: [{ path: "/private/file" }] } }));
  await assert.rejects(() => malformed.selectFiles(), (error: unknown) => {
    assert.ok(error instanceof DesktopAssetImportTransportError);
    assert.equal(error.code, "COMMAND_BRIDGE_FAILED");
    assert.equal(String(error).includes("/private/file"), false);
    return true;
  });
});

test("asset import transport invokes typed path-free command and validates completion", async () => {
  let received: unknown;
  const client = createTauriAssetImportTransport(async (commandName, request) => {
    received = { commandName, request };
    return { ok: true, operationId: "asset-import-1", assetId: "a".repeat(64), state: "completed", retryable: false };
  });

  const result = await client.importFile({
    workspaceId: "workspace-1",
    documentId: "doc-1",
    handle: "picker:1",
    label: "Design",
  });

  assert.equal(result.state, "completed");
  assert.deepEqual(received, {
    commandName: "import_desktop_asset",
    request: { request: { workspaceId: "workspace-1", documentId: "doc-1", handle: "picker:1", label: "Design" } },
  });
  assert.equal(JSON.stringify(received).includes("path"), false);
});

test("asset import transport rejects malformed and native failure responses safely", async () => {
  const failed = createTauriAssetImportTransport(async () => ({ ok: false, state: "failed", errorCode: "asset_import.read_unavailable", retryable: true }));
  await assert.rejects(
    () => failed.importFile({ workspaceId: "workspace-1", documentId: "doc-1", handle: "picker:1", label: "File" }),
    (error: unknown) => error instanceof DesktopAssetImportTransportError
      && error.code === "asset_import.read_unavailable" && error.retryable,
  );

  const malformed = createTauriAssetImportTransport(async () => ({ ok: true, operationId: "operation", assetId: "/private/source.pdf", state: "completed" }));
  await assert.rejects(
    () => malformed.importFile({ workspaceId: "workspace-1", documentId: "doc-1", handle: "picker:1", label: "File" }),
    (error: unknown) => error instanceof DesktopAssetImportTransportError
      && error.code === "COMMAND_BRIDGE_FAILED" && !String(error).includes("/private"),
  );
});

test("asset lifecycle transport maps detail and unlink without object path or bytes", async () => {
  const commands: string[] = [];
  const client = createTauriAssetImportTransport(async (command) => {
    commands.push(command);
    if (command === "get_desktop_asset_detail") return {
      ok: true,
      data: { assetId: "a".repeat(64), fileName: "spec.pdf", mediaType: "application/pdf", byteSize: 42, version: 1, previewCapability: "pdf", extractionStatus: "not_requested", referenceCount: 1, linkedDocumentIds: ["doc-1"] },
      retryable: false,
    };
    return { ok: true, removed: true, remainingReferences: 0, retryable: false };
  });

  const detail = await client.getDetail({ workspaceId: "workspace-1", assetId: "a".repeat(64) });
  const unlink = await client.unlink({ workspaceId: "workspace-1", documentId: "doc-1", assetId: "a".repeat(64) });

  assert.equal(detail.previewCapability, "pdf");
  assert.equal(unlink.removed, true);
  assert.deepEqual(commands, ["get_desktop_asset_detail", "unlink_desktop_asset"]);
  assert.equal(JSON.stringify({ detail, unlink }).includes("path"), false);
});

test("asset import transport accepts immediate operation and reads durable status", async () => {
  const client = createTauriAssetImportTransport(async (command) => command === "import_desktop_asset"
    ? { ok: true, operationId: "operation-1", state: "selected", retryable: false }
    : { ok: true, operationId: "operation-1", state: "completed", retryable: false });

  const started = await client.importFile({ workspaceId: "workspace-1", documentId: "doc-1", handle: "picker:1", label: "File" });
  const status = await client.getImportStatus({ workspaceId: "workspace-1", operationId: started.operationId });

  assert.equal(started.state, "selected");
  assert.equal(status.state, "completed");
});

test("asset transport maps workspace page and existing link through typed commands", async () => {
  const commands: string[] = [];
  const client = createTauriAssetImportTransport(async (command) => {
    commands.push(command);
    if (command === "get_desktop_workspace_assets") return {
      ok: true,
      data: {
        workspaceId: "workspace-1",
        assets: [{ assetId: "a".repeat(64), label: "Spec", fileName: "spec.pdf", mediaType: "application/pdf", byteSize: 42, status: "available" }],
        nextCursor: "b".repeat(64),
      },
      retryable: false,
    };
    return { ok: true, linked: true, referenceCount: 2, retryable: false };
  });

  const page = await client.listWorkspaceAssets({ workspaceId: "workspace-1", limit: 50 });
  const linked = await client.link({ workspaceId: "workspace-1", documentId: "doc-2", assetId: "a".repeat(64), label: "Spec" });

  assert.equal(page.assets[0]?.fileName, "spec.pdf");
  assert.equal(page.nextCursor, "b".repeat(64));
  assert.equal(linked.referenceCount, 2);
  assert.deepEqual(commands, ["get_desktop_workspace_assets", "link_desktop_asset"]);
  assert.equal(JSON.stringify({ page, linked }).includes("path"), false);
});

test("asset transport accepts native null cursor for an empty first-run workspace", async () => {
  const client = createTauriAssetImportTransport(async () => ({
    ok: true,
    data: { workspaceId: "workspace-1", assets: [], nextCursor: null },
    retryable: false,
  }));

  const page = await client.listWorkspaceAssets({ workspaceId: "workspace-1", limit: 200 });

  assert.deepEqual(page.assets, []);
  assert.equal(page.nextCursor, undefined);
});

test("asset preview transport accepts bounded presentation and rejects path-bearing payload", async () => {
  const assetId = "a".repeat(64);
  const client = createTauriAssetImportTransport(async (command) => {
    assert.equal(command, "get_desktop_asset_preview");
    return { ok: true, data: { assetId, capability: "text", mediaType: "text/plain", presentation: "text", content: "preview" }, retryable: false };
  });
  assert.deepEqual(await client.getPreview({ workspaceId: "workspace-1", assetId }), {
    assetId, capability: "text", mediaType: "text/plain", presentation: "text", content: "preview",
  });

  const unsafe = createTauriAssetImportTransport(async () => ({ ok: true, data: { assetId, capability: "text", mediaType: "text/plain", presentation: "text", content: "preview", path: "/private/file" } }));
  await assert.rejects(() => unsafe.getPreview({ workspaceId: "workspace-1", assetId }), (error: unknown) => error instanceof DesktopAssetImportTransportError && !String(error).includes("/private"));
});
