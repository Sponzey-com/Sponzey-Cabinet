import assert from "node:assert/strict";
import test from "node:test";

import {
  DesktopCanvasTransportError,
  createTauriCanvasTransport,
} from "../src/tauri_canvas_transport.ts";

const canvasData = {
  canvasId: "canvas-1",
  title: "Product map",
  revision: 3,
  lifecycle: "updated",
  viewport: { centerX: 400, centerY: 300, zoomPercent: 125 },
  nodes: [{ nodeId: "note-1", targetKind: "text", targetId: "Decision", displayLabel: "Decision", targetStatus: "available", x: 80, y: 80, width: 320, height: 180 }],
  edges: [],
};

test("Canvas transport sends discriminated revision request and validates complete safe DTO", async () => {
  let invoked: unknown;
  const client = createTauriCanvasTransport(async (command, args) => {
    invoked = { command, args };
    return { ok: true, data: canvasData, retryable: false, recoveryRequired: false, operationId: "operation-1" };
  });

  const result = await client.execute({
    kind: "update_viewport",
    workspaceId: "workspace-1",
    canvasId: "canvas-1",
    expectedRevision: 2,
    operationId: "operation-1",
    centerX: 400,
    centerY: 300,
    zoomPercent: 125,
  });

  assert.equal(result.revision, 3);
  assert.equal(result.nodes[0]?.targetId, "Decision");
  assert.deepEqual(invoked, {
    command: "execute_desktop_canvas",
    args: { request: {
      kind: "update_viewport",
      workspaceId: "workspace-1",
      canvasId: "canvas-1",
      expectedRevision: 2,
      operationId: "operation-1",
      centerX: 400,
      centerY: 300,
      zoomPercent: 125,
    } },
  });
  assert.equal(JSON.stringify(result).includes("path"), false);
  assert.equal(JSON.stringify(result).includes("bytes"), false);
});

test("Canvas transport maps native conflict and recovery without leaking malformed payload", async () => {
  const conflict = createTauriCanvasTransport(async () => ({
    ok: false,
    data: null,
    errorCode: "CANVAS_VERSION_CONFLICT",
    retryable: false,
    recoveryRequired: false,
  }));
  await assert.rejects(
    () => conflict.execute({ kind: "get", workspaceId: "workspace-1", canvasId: "canvas-1" }),
    (error: unknown) => error instanceof DesktopCanvasTransportError
      && error.code === "CANVAS_VERSION_CONFLICT" && !error.recoveryRequired,
  );

  const recovery = createTauriCanvasTransport(async () => ({
    ok: false,
    data: null,
    errorCode: "CANVAS_RECOVERY_REQUIRED",
    retryable: false,
    recoveryRequired: true,
  }));
  await assert.rejects(
    () => recovery.execute({ kind: "get", workspaceId: "workspace-1", canvasId: "canvas-1" }),
    (error: unknown) => error instanceof DesktopCanvasTransportError && error.recoveryRequired,
  );

  const malformed = createTauriCanvasTransport(async () => ({
    ok: true,
    data: { ...canvasData, revision: 0, path: "/private/canvas" },
    retryable: false,
    recoveryRequired: false,
  }));
  await assert.rejects(
    () => malformed.execute({ kind: "get", workspaceId: "workspace-1", canvasId: "canvas-1" }),
    (error: unknown) => error instanceof DesktopCanvasTransportError
      && error.code === "COMMAND_BRIDGE_FAILED" && !String(error).includes("/private"),
  );
});

test("Canvas transport treats auto arrange preview as a non-mutating revision query", async () => {
  let invoked: unknown;
  const client = createTauriCanvasTransport(async (command, args) => {
    invoked = { command, args };
    return { ok: true, data: canvasData, retryable: false, recoveryRequired: false, operationId: null };
  });
  const preview = await client.execute({
    kind: "preview_auto_arrange",
    workspaceId: "workspace-1",
    canvasId: "canvas-1",
    expectedRevision: 3,
  });
  assert.equal(preview.revision, 3);
  assert.deepEqual(invoked, {
    command: "execute_desktop_canvas",
    args: { request: {
      kind: "preview_auto_arrange",
      workspaceId: "workspace-1",
      canvasId: "canvas-1",
      expectedRevision: 3,
    } },
  });
});

test("Canvas transport sends recovery operation identity and rejects mismatched acknowledgement", async () => {
  const client = createTauriCanvasTransport(async () => ({
    ok: true, data: canvasData, retryable: false, recoveryRequired: false, operationId: "recover-1",
  }));
  const recovered = await client.execute({
    kind: "recover", workspaceId: "workspace-1", canvasId: "canvas-1", operationId: "recover-1",
  });
  assert.equal(recovered.operationId, "recover-1");

  const mismatched = createTauriCanvasTransport(async () => ({
    ok: true, data: canvasData, retryable: false, recoveryRequired: false, operationId: "other",
  }));
  await assert.rejects(
    () => mismatched.execute({ kind: "recover", workspaceId: "workspace-1", canvasId: "canvas-1", operationId: "recover-1" }),
    (error: unknown) => error instanceof DesktopCanvasTransportError && error.code === "COMMAND_BRIDGE_FAILED",
  );
});
