import assert from "node:assert/strict";
import test from "node:test";

import {
  createTauriWorkspaceHomeTransport,
  type TauriInvoke,
} from "../src/tauri_home_transport.ts";

test("Tauri home transport maps client query to tagged snake case request", async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke: TauriInvoke = async (command, args) => {
    calls.push({ command, args });
    return {
      ok: true,
      data: {
        workspaceId: "workspace-1",
        state: "Ready",
        recentDocuments: [],
        favorites: [],
        tags: [],
        recentChanges: [],
        unfinishedItems: [],
        backupStatus: "Fresh",
        healthStatus: "Healthy",
      },
      retryable: false,
    };
  };
  const transport = createTauriWorkspaceHomeTransport(invoke);

  const response = await transport({
    commandName: "local_workspace_home",
    payload: {
      workspaceId: "workspace-1",
      recentDocuments: 12,
      favorites: 8,
      tags: 10,
      recentChanges: 14,
      unfinishedItems: 6,
    },
  });

  assert.deepEqual(calls, [
    {
      command: "get_desktop_workspace_home",
      args: {
        request: {
          command_name: "local_workspace_home",
          payload: {
            kind: "workspace_home",
            workspace_id: "workspace-1",
            recent_documents: 12,
            favorites: 8,
            tags: 10,
            recent_changes: 14,
            unfinished_items: 6,
          },
        },
      },
    },
  ]);
  assert.equal(response.ok, true);
});

test("Tauri home transport fails safely for throw, invalid response, and unsupported command", async () => {
  const throwing = createTauriWorkspaceHomeTransport(async () => {
    throw new Error("/Users/private/app-data raw native error");
  });
  const invalid = createTauriWorkspaceHomeTransport(async () => ({ ok: true }));

  const thrown = await throwing(homeEnvelope());
  const malformed = await invalid(homeEnvelope());
  const unsupported = await invalid({ commandName: "get_current_document", payload: {} });

  for (const response of [thrown, malformed, unsupported]) {
    assert.equal(response.ok, false);
    if (!response.ok) {
      assert.equal(response.errorCode, "COMMAND_BRIDGE_FAILED");
      assert.equal(response.retryable, false);
      assert.equal(JSON.stringify(response).includes("/Users/private"), false);
      assert.equal(JSON.stringify(response).includes("raw native error"), false);
    }
  }
});

function homeEnvelope() {
  return {
    commandName: "local_workspace_home" as const,
    payload: {
      workspaceId: "workspace-1",
      recentDocuments: 12,
      favorites: 8,
      tags: 10,
      recentChanges: 14,
      unfinishedItems: 6,
    },
  };
}
