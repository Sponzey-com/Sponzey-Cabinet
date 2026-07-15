import assert from "node:assert/strict";
import test from "node:test";

import { createTauriProjectionTransport, DesktopProjectionTransportError } from "../src/tauri_projection_transport.ts";

test("projection transport maps explicit identity and validates native responses", async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const transport = createTauriProjectionTransport(async (command, args) => {
    calls.push({ command, args });
    if (command === "get_desktop_projection_freshness") {
      return { ok: true, state: "stale", currentVersionId: "version-2", projections: [{ kind: "Graph", state: "stale" }], retryable: false };
    }
    if (command === "request_desktop_projection_reindex") {
      return { ok: true, enqueuedCount: 3, resetCount: 2, alreadyActiveCount: 1, retryable: false };
    }
    return { ok: true, readyCount: 3, retryScheduledCount: 0, failedCount: 0, retryable: false };
  });

  assert.equal((await transport.getFreshness("workspace-1", "doc-1")).state, "stale");
  assert.equal((await transport.requestReindex("workspace-1", "doc-1")).resetCount, 2);
  assert.equal((await transport.runWorker()).readyCount, 3);
  assert.deepEqual(calls, [
    { command: "get_desktop_projection_freshness", args: { request: { workspaceId: "workspace-1", documentId: "doc-1" } } },
    { command: "request_desktop_projection_reindex", args: { request: { workspaceId: "workspace-1", documentId: "doc-1" } } },
    { command: "run_desktop_projection_worker", args: undefined },
  ]);
});

test("projection transport exposes only stable native failure metadata", async () => {
  const transport = createTauriProjectionTransport(async () => ({
    ok: false,
    errorCode: "projection_freshness.repository_unavailable",
    retryable: true,
    privatePath: "/Users/private/raw",
  }));

  await assert.rejects(
    transport.getFreshness("workspace-1", "doc-1"),
    (error: unknown) => error instanceof DesktopProjectionTransportError
      && error.code === "projection_freshness.repository_unavailable"
      && error.retryable
      && !JSON.stringify(error).includes("/Users/private"),
  );
});

test("projection transport preserves operation identity across start run and status", async () => {
  const calls: string[] = [];
  const transport = createTauriProjectionTransport(async (command, args) => {
    calls.push(`${command}:${JSON.stringify(args ?? {})}`);
    return { ok: true, operationId: "repair-1", state: command.startsWith("start") ? "queued" : "succeeded", attempt: 1, completedUnits: 3, totalUnits: 3, retryable: false };
  });
  const started = await transport.startRepair("workspace-1", "doc-1");
  await transport.runRepair("workspace-1", started.operationId);
  const status = await transport.getRepairStatus("workspace-1", started.operationId);
  assert.equal(status.state, "succeeded");
  assert.match(calls[1] ?? "", /run_desktop_projection_repair_operation.*repair-1/);
  assert.match(calls[2] ?? "", /get_desktop_projection_repair_status.*repair-1/);
});
