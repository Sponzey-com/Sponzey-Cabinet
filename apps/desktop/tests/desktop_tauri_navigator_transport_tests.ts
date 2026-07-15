import assert from "node:assert/strict";
import test from "node:test";

import type { LocalDesktopCommandEnvelope } from "@sponzey-cabinet/client-core";

import { createTauriDocumentNavigatorTransport } from "../src/tauri_navigator_transport.ts";

test("navigator transport maps client query to dedicated snake case Tauri request", async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const transport = createTauriDocumentNavigatorTransport(async (command, args) => {
    calls.push({ command, args });
    return {
      ok: true,
      retryable: false,
      data: {
        workspaceId: "workspace-1",
        view: "Tag",
        state: "Ready",
        items: [],
        nextCursor: null,
      },
    };
  });

  const response = await transport({
    commandName: "local_document_navigator",
    payload: {
      workspaceId: "workspace-1",
      view: "Tag",
      viewKey: "rust",
      filter: "arch",
      limit: 20,
      cursor: "40",
    },
  });

  assert.equal(response.ok, true);
  assert.deepEqual(calls, [
    {
      command: "get_desktop_document_navigator",
      args: {
        request: {
          workspace_id: "workspace-1",
          view: "Tag",
          view_key: "rust",
          filter: "arch",
          limit: 20,
          cursor: "40",
        },
      },
    },
  ]);
});

test("navigator transport rejects wrong commands and malformed native responses", async () => {
  const malformed = createTauriDocumentNavigatorTransport(async () => ({
    ok: true,
    data: { state: "Unknown" },
  }));
  const wrongCommand: LocalDesktopCommandEnvelope = {
    commandName: "local_workspace_home",
    payload: {},
  };
  const query: LocalDesktopCommandEnvelope = {
    commandName: "local_document_navigator",
    payload: {
      workspaceId: "workspace-1",
      view: "Tree",
      limit: 20,
    },
  };

  assert.deepEqual(await malformed(wrongCommand), {
    ok: false,
    errorCode: "COMMAND_BRIDGE_FAILED",
    retryable: false,
  });
  assert.deepEqual(await malformed(query), {
    ok: false,
    errorCode: "COMMAND_BRIDGE_FAILED",
    retryable: false,
  });
});
