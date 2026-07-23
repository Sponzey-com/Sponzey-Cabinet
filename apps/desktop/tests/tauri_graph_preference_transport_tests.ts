import assert from "node:assert/strict";
import test from "node:test";

import { createDefaultDesktopGraphPreference } from "../src/desktop_graph_preference.ts";
import { createTauriGraphPreferenceTransport } from "../src/tauri_graph_preference_transport.ts";

test("graph preference transport loads and saves a path-free typed workspace request", async () => {
  const calls: unknown[] = [];
  const preference = createDefaultDesktopGraphPreference();
  const client = createTauriGraphPreferenceTransport(async (command, args) => {
    calls.push({ command, args });
    if (command === "get_desktop_graph_preference") return { ok: true, data: preference };
    return { ok: true, data: { saved: true } };
  });
  assert.deepEqual(await client.load("workspace-1"), preference);
  await client.save("workspace-1", preference);
  assert.deepEqual(calls, [
    { command: "get_desktop_graph_preference", args: { request: { workspaceId: "workspace-1" } } },
    { command: "save_desktop_graph_preference", args: { request: { workspaceId: "workspace-1", preference } } },
  ]);
  assert.doesNotMatch(JSON.stringify(calls), /path|query|documentId|nodeId/);
});

test("graph preference transport rejects malformed values and native failure", async () => {
  const malformed = createTauriGraphPreferenceTransport(async () => ({ ok: true, data: { schemaVersion: 99 } }));
  await assert.rejects(malformed.load("workspace-1"), /GRAPH_PREFERENCE_INVALID/);
  const failed = createTauriGraphPreferenceTransport(async () => ({ ok: false, errorCode: "GRAPH_PREFERENCE_STORAGE_FAILED", retryable: true }));
  await assert.rejects(failed.load("workspace-1"), /GRAPH_PREFERENCE_STORAGE_FAILED/);
});

