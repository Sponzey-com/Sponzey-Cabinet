import assert from "node:assert/strict";
import test from "node:test";

import type { LocalDesktopCommandEnvelope } from "@sponzey-cabinet/client-core";

import { createTauriDesktopTransport } from "../src/tauri_desktop_transport.ts";
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

test("navigator transport maps asset search query to dedicated snake case Tauri request", async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const transport = createTauriDocumentNavigatorTransport(async (command, args) => {
    calls.push({ command, args });
    return {
      ok: true,
      retryable: false,
      data: {
        queryName: "search-assets",
        workspaceId: "workspace-1",
        text: "명세서",
        results: [
          {
            assetId: "asset-1",
            fileName: "source.pdf",
            mediaType: "application/pdf",
            byteSize: 1024,
            score: 3,
          },
        ],
      },
    };
  });

  const response = await transport({
    commandName: "search_assets",
    payload: {
      queryName: "search-assets",
      workspaceId: "workspace-1",
      text: "명세서",
      limit: 20,
    },
  });

  assert.equal(response.ok, true);
  assert.deepEqual(calls, [
    {
      command: "search_desktop_assets",
      args: {
        request: {
          workspace_id: "workspace-1",
          text: "명세서",
          limit: 20,
        },
      },
    },
  ]);
});

test("desktop composite transport routes asset search through the navigator boundary", async () => {
  const commands: string[] = [];
  const transport = createTauriDesktopTransport(async (command) => {
    commands.push(command);
    return {
      ok: true,
      retryable: false,
      data: {
        queryName: "search-assets",
        workspaceId: "workspace-1",
        text: "spec",
        results: [],
      },
    };
  });

  const response = await transport({
    commandName: "search_assets",
    payload: {
      queryName: "search-assets",
      workspaceId: "workspace-1",
      text: "spec",
      limit: 20,
    },
  });

  assert.equal(response.ok, true);
  assert.deepEqual(commands, ["search_desktop_assets"]);
});

test("navigator transport maps document search query to dedicated snake case Tauri request", async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const transport = createTauriDocumentNavigatorTransport(async (command, args) => {
    calls.push({ command, args });
    return {
      ok: true,
      retryable: false,
      data: {
        queryName: "search-documents",
        workspaceId: "workspace-1",
        text: "packaged workflow",
        results: [
          {
            documentId: "doc-1",
            title: "Packaged Workflow",
            path: "notes/doc-1.md",
            snippet: "Packaged Workflow",
            score: 5,
          },
        ],
      },
    };
  });

  const response = await transport({
    commandName: "search_documents",
    payload: {
      queryName: "search-documents",
      workspaceId: "workspace-1",
      text: "packaged workflow",
      limit: 20,
    },
  });

  assert.equal(response.ok, true);
  assert.deepEqual(calls, [
    {
      command: "search_desktop_documents",
      args: {
        request: {
          workspace_id: "workspace-1",
          text: "packaged workflow",
          limit: 20,
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
