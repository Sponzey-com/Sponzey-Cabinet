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
    attachmentOperationId: "attachment-import-1",
    expectedCurrentVersionToken: "version-current",
  });

  assert.equal(result.state, "completed");
  assert.deepEqual(received, {
    commandName: "import_desktop_asset",
    request: { request: { workspaceId: "workspace-1", documentId: "doc-1", handle: "picker:1", label: "Design", attachmentOperationId: "attachment-import-1", expectedCurrentVersionToken: "version-current" } },
  });
  assert.equal(JSON.stringify(received).includes("path"), false);
});

test("asset import transport rejects malformed and native failure responses safely", async () => {
  const failed = createTauriAssetImportTransport(async () => ({
    ok: false,
    state: "failed",
    errorCode: "asset_import.read_unavailable",
    retryable: true,
    repairRequired: true,
  }));
  await assert.rejects(
    () => failed.importFile({ workspaceId: "workspace-1", documentId: "doc-1", handle: "picker:1", label: "File", attachmentOperationId: "attachment-import-2", expectedCurrentVersionToken: "version-current" }),
    (error: unknown) => error instanceof DesktopAssetImportTransportError
      && error.code === "asset_import.read_unavailable" && error.retryable && error.repairRequired,
  );

  const malformed = createTauriAssetImportTransport(async () => ({ ok: true, operationId: "operation", assetId: "/private/source.pdf", state: "completed" }));
  await assert.rejects(
    () => malformed.importFile({ workspaceId: "workspace-1", documentId: "doc-1", handle: "picker:1", label: "File", attachmentOperationId: "attachment-import-3", expectedCurrentVersionToken: "version-current" }),
    (error: unknown) => error instanceof DesktopAssetImportTransportError
      && error.code === "COMMAND_BRIDGE_FAILED" && !String(error).includes("/private"),
  );
});

test("asset attachment mutation transport preserves native recovery flags", async () => {
  const client = createTauriAssetImportTransport(async () => ({
    ok: false,
    errorCode: "attachment_projection_recovery_required",
    retryable: true,
    repairRequired: true,
  }));

  await assert.rejects(
    () => client.link({
      workspaceId: "workspace-1",
      documentId: "doc-1",
      assetId: "a".repeat(64),
      label: "Spec",
      operationId: "link-1",
      expectedCurrentVersionToken: "current-version",
    }),
    (error: unknown) => error instanceof DesktopAssetImportTransportError
      && error.code === "attachment_projection_recovery_required"
      && error.retryable
      && error.repairRequired,
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
    return { ok: true, outcome: "fresh", delta: "unlinked", revisionNumber: 2, retryable: false, repairRequired: false };
  });

  const detail = await client.getDetail({ workspaceId: "workspace-1", assetId: "a".repeat(64) });
  const unlink = await client.unlink({ workspaceId: "workspace-1", documentId: "doc-1", assetId: "a".repeat(64), operationId: "unlink-1", expectedCurrentVersionToken: "version-current" });

  assert.equal(detail.previewCapability, "pdf");
  assert.equal(unlink.delta, "unlinked");
  assert.deepEqual(commands, ["get_desktop_asset_detail", "unlink_desktop_asset"]);
  assert.equal(JSON.stringify({ detail, unlink }).includes("path"), false);
});

test("asset import transport accepts immediate operation and reads durable status", async () => {
  const client = createTauriAssetImportTransport(async (command) => command === "import_desktop_asset"
    ? { ok: true, operationId: "operation-1", state: "selected", retryable: false }
    : { ok: true, operationId: "operation-1", state: "completed", retryable: false });

  const started = await client.importFile({ workspaceId: "workspace-1", documentId: "doc-1", handle: "picker:1", label: "File", attachmentOperationId: "attachment-import-4", expectedCurrentVersionToken: "version-current" });
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
    return { ok: true, outcome: "fresh", delta: "linked", revisionNumber: 2, retryable: false, repairRequired: false };
  });

  const page = await client.listWorkspaceAssets({ workspaceId: "workspace-1", limit: 50 });
  const linked = await client.link({ workspaceId: "workspace-1", documentId: "doc-2", assetId: "a".repeat(64), label: "Spec", operationId: "link-1", expectedCurrentVersionToken: "version-current" });

  assert.equal(page.assets[0]?.fileName, "spec.pdf");
  assert.equal(page.nextCursor, "b".repeat(64));
  assert.equal(linked.revisionNumber, 2);
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

test("asset external open transport uses a path-free command and rejects unsafe responses", async () => {
  const assetId = "a".repeat(64);
  let received: unknown;
  const client = createTauriAssetImportTransport(async (command, payload) => {
    received = { command, payload };
    return { ok: true, opened: true, retryable: false };
  });

  assert.deepEqual(await client.openExternal({ workspaceId: "workspace-1", assetId }), { opened: true });
  assert.deepEqual(received, {
    command: "open_desktop_asset_externally",
    payload: { request: { workspaceId: "workspace-1", assetId } },
  });
  assert.equal(JSON.stringify(received).includes("path"), false);

  const unsafe = createTauriAssetImportTransport(async () => ({
    ok: true,
    opened: true,
    path: "/private/object.bin",
  }));
  await assert.rejects(
    () => unsafe.openExternal({ workspaceId: "workspace-1", assetId }),
    (error: unknown) => error instanceof DesktopAssetImportTransportError
      && error.code === "COMMAND_BRIDGE_FAILED"
      && !String(error).includes("/private"),
  );
});
