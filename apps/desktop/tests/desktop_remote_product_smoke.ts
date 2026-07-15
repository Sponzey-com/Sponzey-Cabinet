import assert from "node:assert/strict";

import { createPlatformCapabilityMatrix } from "@sponzey-cabinet/client-core";

import {
  beginDesktopRemoteConnection,
  connectDesktopRemoteWorkspace,
  createDesktopRemoteWorkspaceApiClient,
  createDesktopWorkspaceSelectorModel,
  readDesktopRemoteCurrentDocument,
  readDesktopRemoteKnowledgeGraph,
  saveDesktopDocumentEdit,
  selectDesktopRemoteWorkspace,
  type DesktopLocalWorkspaceRepository,
  type DesktopDocumentEditCommand,
  type DesktopLocalWorkspaceSelection,
} from "../src/index.ts";

const desktopSmokeBodyFixture = "desktop smoke body should not be logged";
const invalidTokenFixture = "desktop-invalid-token-should-not-log";

async function main() {
  const serverBaseUrl = requireArg("--server-base-url");
  const sessionToken = requireArg("--session-token");

  await runDesktopRemoteSelfHostFlow(serverBaseUrl, sessionToken);
  await runDesktopRemoteErrorFlow(serverBaseUrl);
  verifyDesktopCapabilityMatrix();

  console.log("desktop_remote_product_smoke=passed");
}

async function runDesktopRemoteSelfHostFlow(serverBaseUrl: string, sessionToken: string) {
  console.log("desktop_remote_product_step_start=remote_self_host_flow");
  const localRepository = new FailingLocalRepository();
  const remoteClient = createDesktopRemoteWorkspaceApiClient();
  const selected = selectDesktopRemoteWorkspace(
    createDesktopWorkspaceSelectorModel({
      supportsLocalWorkspace: true,
      supportsRemoteWorkspace: true,
    }),
    {
      workspaceId: "workspace-1",
      displayName: "Self-host Team Cabinet",
      serverBaseUrl,
      sessionToken,
    },
  );
  const connecting = beginDesktopRemoteConnection(selected);
  const connected = await connectDesktopRemoteWorkspace(connecting, remoteClient);

  assert.equal(selected.displayState, "RemoteWorkspaceSelected");
  assert.equal(connecting.displayState, "RemoteConnecting");
  assert.equal(connected.displayState, "RemoteConnected");

  const loaded = await readDesktopRemoteCurrentDocument(
    connected,
    {
      workspaceId: "workspace-1",
      documentId: "doc-allowed",
    },
    remoteClient,
  );
  assert.equal(loaded.status, "loaded-remote");
  assert.equal(loaded.document?.documentId, "doc-allowed");
  assert.equal(loaded.document?.permissionDecision.effect, "allow");

  console.log("desktop_remote_product_step_start=remote_graph_flow");
  const graph = await readDesktopRemoteKnowledgeGraph(
    connected,
    {
      workspaceId: "workspace-1",
      documentId: "doc-allowed",
    },
    remoteClient,
  );
  assert.equal(graph.status, "loaded-remote");
  assert.equal(graph.graph?.centerDocumentId, "doc-allowed");
  assert.equal(graph.graph?.status, "clean");
  assert.ok(graph.graph?.nodes.some((node) => node.id === "doc-visible"));
  assert.ok(!graph.graph?.nodes.some((node) => node.id === "doc-hidden"));
  assert.equal(graph.graph?.stats.filteredCount, 1);
  assert.ok((graph.graph?.performance?.observedMs ?? 301) <= 300);
  console.log("desktop_remote_product_step_passed=remote_graph_flow");

  const saved = await saveDesktopDocumentEdit(
    connected,
    {
      workspaceId: "workspace-1",
      documentId: "doc-allowed",
      title: "Desktop Smoke",
      path: "docs/desktop-smoke.md",
      body: desktopSmokeBodyFixture,
      expectedVersionId: loaded.document?.versionId ?? "version-3",
    },
    localRepository,
    remoteClient,
  );
  assert.equal(saved.status, "saved-remote");
  assert.deepEqual(localRepository.calls, []);
  console.log("desktop_remote_product_step_passed=remote_self_host_flow");
}

async function runDesktopRemoteErrorFlow(serverBaseUrl: string) {
  console.log("desktop_remote_product_step_start=remote_error_flow");
  const unauthorizedClient = createDesktopRemoteWorkspaceApiClient();
  const unauthorizedSelected = selectDesktopRemoteWorkspace(
    createDesktopWorkspaceSelectorModel({
      supportsLocalWorkspace: true,
      supportsRemoteWorkspace: true,
    }),
    {
      workspaceId: "workspace-1",
      displayName: "Self-host Team Cabinet",
      serverBaseUrl,
      sessionToken: invalidTokenFixture,
    },
  );
  const unauthorized = await connectDesktopRemoteWorkspace(
    beginDesktopRemoteConnection(unauthorizedSelected),
    unauthorizedClient,
  );
  assert.equal(unauthorized.displayState, "RemoteError");
  assert.equal(unauthorized.error?.code, "DESKTOP_REMOTE_SESSION_EXPIRED");
  assert.doesNotMatch(unauthorized.error?.message ?? "", /token|secret|desktop-invalid/i);

  const networkClient = createDesktopRemoteWorkspaceApiClient();
  const networkSelected = selectDesktopRemoteWorkspace(
    createDesktopWorkspaceSelectorModel({
      supportsLocalWorkspace: true,
      supportsRemoteWorkspace: true,
    }),
    {
      workspaceId: "workspace-1",
      displayName: "Unavailable Team Cabinet",
      serverBaseUrl: "http://127.0.0.1:9",
      sessionToken: "unprinted-network-token",
    },
  );
  const networkFailure = await connectDesktopRemoteWorkspace(
    beginDesktopRemoteConnection(networkSelected),
    networkClient,
  );
  assert.equal(networkFailure.displayState, "RemoteError");
  assert.equal(networkFailure.error?.code, "DESKTOP_REMOTE_NETWORK_FAILURE");
  assert.doesNotMatch(networkFailure.error?.message ?? "", /token|secret|127\.0\.0\.1/i);
  console.log("desktop_remote_product_step_passed=remote_error_flow");
}

function verifyDesktopCapabilityMatrix() {
  console.log("desktop_remote_product_step_start=capability_matrix");
  const matrix = createPlatformCapabilityMatrix();
  for (const platform of [matrix.windows, matrix.macos, matrix.linux]) {
    assert.equal(platform.supportsRemoteWorkspace, true);
    assert.equal(platform.supportsRemoteEdit, true);
    assert.equal(platform.supportsOfflineRemoteEdit, false);
    assert.equal(platform.supportsRemoteWorkspace, matrix.desktop.supportsRemoteWorkspace);
    assert.equal(platform.supportsRemoteEdit, matrix.desktop.supportsRemoteEdit);
  }
  console.log("desktop_remote_product_step_passed=capability_matrix");
}

class FailingLocalRepository implements DesktopLocalWorkspaceRepository {
  readonly calls: string[] = [];

  async openLocalWorkspace(selection: DesktopLocalWorkspaceSelection): Promise<void> {
    this.calls.push(`open:${selection.workspaceId}`);
    throw new Error("desktop product smoke must not open local workspace");
  }

  async saveLocalDocument(command: DesktopDocumentEditCommand): Promise<void> {
    this.calls.push(`save:${command.documentId}`);
    throw new Error("desktop product smoke must not save remote document locally");
  }
}

function requireArg(name: string): string {
  const index = process.argv.indexOf(name);
  const value = index >= 0 ? process.argv[index + 1] : undefined;
  if (!value?.trim()) {
    throw new Error(`missing required argument: ${name}`);
  }
  return value;
}

main().catch((error) => {
  console.error("desktop_remote_product_smoke=failed");
  console.error(`failure_category=${error instanceof Error ? error.message : "unexpected_failure"}`);
  process.exit(1);
});
