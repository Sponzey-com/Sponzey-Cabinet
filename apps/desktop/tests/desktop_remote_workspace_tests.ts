import assert from "node:assert/strict";
import test from "node:test";

import { CabinetApiClientError, createPlatformCapabilityMatrix } from "@sponzey-cabinet/client-core";

import {
  beginDesktopRemoteConnection,
  connectDesktopRemoteWorkspace,
  readDesktopRemoteCurrentDocument,
  readDesktopRemoteKnowledgeGraph,
  createDesktopWorkspaceSelectorModel,
  saveDesktopDocumentEdit,
  selectDesktopLocalWorkspace,
  selectDesktopRemoteWorkspace,
  type DesktopLocalWorkspaceRepository,
  type DesktopRemoteWorkspaceApiClient,
} from "../src/index.ts";

test("desktop workspace selector keeps local and remote workspace state distinct", async () => {
  const localRepository = new FakeLocalWorkspaceRepository();
  const remoteClient = new FakeRemoteWorkspaceClient();
  const initial = createDesktopWorkspaceSelectorModel({
    supportsLocalWorkspace: true,
    supportsRemoteWorkspace: true,
  });

  const local = await selectDesktopLocalWorkspace(
    initial,
    {
      workspaceId: "local-workspace",
      displayName: "Local notes",
      localPath: "/Users/example/Documents/Cabinet",
    },
    localRepository,
  );
  const remoteSelected = selectDesktopRemoteWorkspace(local, {
    workspaceId: "remote-workspace",
    displayName: "Team Cabinet",
    serverBaseUrl: "https://cabinet.example",
    sessionToken: "token",
  });
  const remoteConnected = await connectDesktopRemoteWorkspace(remoteSelected, remoteClient);

  assert.equal(local.displayState, "LocalWorkspaceSelected");
  assert.equal(local.selectedWorkspace?.kind, "local");
  assert.equal(local.selectedWorkspace?.label, "Local notes");
  assert.equal(remoteSelected.displayState, "RemoteWorkspaceSelected");
  assert.equal(remoteSelected.selectedWorkspace?.kind, "remote");
  assert.equal(remoteSelected.selectedWorkspace?.label, "Team Cabinet");
  assert.equal(remoteConnected.displayState, "RemoteConnected");
  assert.equal(remoteConnected.selectedWorkspace?.kind, "remote");
  assert.deepEqual(localRepository.calls, ["openLocalWorkspace"]);
  assert.deepEqual(remoteClient.calls, ["openRemoteWorkspace"]);
});

test("desktop workspace selector exposes explicit remote connecting display state", () => {
  const remoteSelected = selectDesktopRemoteWorkspace(
    createDesktopWorkspaceSelectorModel({
      supportsLocalWorkspace: true,
      supportsRemoteWorkspace: true,
    }),
    {
      workspaceId: "remote-workspace",
      displayName: "Team Cabinet",
      serverBaseUrl: "https://cabinet.example",
      sessionToken: "token",
    },
  );

  const connecting = beginDesktopRemoteConnection(remoteSelected);

  assert.equal(connecting.displayState, "RemoteConnecting");
  assert.equal(connecting.selectedWorkspace?.kind, "remote");
  assert.equal(connecting.error, undefined);
});

test("desktop remote document save uses server API client and never writes through local repository", async () => {
  const localRepository = new FakeLocalWorkspaceRepository();
  const remoteClient = new FakeRemoteWorkspaceClient();
  const remoteSelected = selectDesktopRemoteWorkspace(
    createDesktopWorkspaceSelectorModel({
      supportsLocalWorkspace: true,
      supportsRemoteWorkspace: true,
    }),
    {
      workspaceId: "remote-workspace",
      displayName: "Team Cabinet",
      serverBaseUrl: "https://cabinet.example",
      sessionToken: "token",
    },
  );
  const remoteConnected = await connectDesktopRemoteWorkspace(remoteSelected, remoteClient);
  const loaded = await readDesktopRemoteCurrentDocument(
    remoteConnected,
    {
      workspaceId: "remote-workspace",
      documentId: "doc-1",
    },
    remoteClient,
  );
  const graph = await readDesktopRemoteKnowledgeGraph(
    remoteConnected,
    {
      workspaceId: "remote-workspace",
      documentId: "doc-1",
    },
    remoteClient,
  );

  const saved = await saveDesktopDocumentEdit(
    remoteConnected,
    {
      workspaceId: "remote-workspace",
      documentId: "doc-1",
      title: "Updated",
      path: "docs/updated.md",
      body: "# Updated",
      expectedVersionId: "version-1",
    },
    localRepository,
    remoteClient,
  );

  assert.equal(loaded.status, "loaded-remote");
  assert.equal(loaded.document?.documentId, "doc-1");
  assert.equal(graph.status, "loaded-remote");
  assert.equal(graph.graph?.centerDocumentId, "doc-1");
  assert.equal(graph.graph?.status, "clean");
  assert.deepEqual(
    graph.graph?.nodes.map((node) => node.id),
    ["doc-1", "doc-2"],
  );
  assert.equal(saved.status, "saved-remote");
  assert.deepEqual(localRepository.calls, []);
  assert.deepEqual(remoteClient.calls, [
    "openRemoteWorkspace",
    "readRemoteCurrentDocument",
    "readRemoteKnowledgeGraph",
    "saveRemoteDocument",
  ]);
  assert.deepEqual(remoteClient.savedBodies, ["# Updated"]);
});

test("desktop selector rejects remote workspace when platform capability does not allow it", () => {
  const initial = createDesktopWorkspaceSelectorModel({
    supportsLocalWorkspace: true,
    supportsRemoteWorkspace: false,
  });

  const remoteSelected = selectDesktopRemoteWorkspace(initial, {
    workspaceId: "remote-workspace",
    displayName: "Team Cabinet",
    serverBaseUrl: "https://cabinet.example",
    sessionToken: "token",
  });

  assert.equal(remoteSelected.displayState, "RemoteError");
  assert.equal(remoteSelected.error?.code, "DESKTOP_REMOTE_UNSUPPORTED");
});

test("desktop remote connection maps session expired and network failure to stable display errors", async () => {
  const remoteSelected = selectDesktopRemoteWorkspace(
    createDesktopWorkspaceSelectorModel({
      supportsLocalWorkspace: true,
      supportsRemoteWorkspace: true,
    }),
    {
      workspaceId: "remote-workspace",
      displayName: "Team Cabinet",
      serverBaseUrl: "https://cabinet.example",
      sessionToken: "token",
    },
  );
  const expiredClient = new FakeRemoteWorkspaceClient(
    new CabinetApiClientError("SESSION_EXPIRED", "session expired", 401),
  );
  const networkClient = new FakeRemoteWorkspaceClient(
    new CabinetApiClientError("NETWORK_FAILURE", "network request failed"),
  );
  const unauthorizedClient = new FakeRemoteWorkspaceClient(
    new CabinetApiClientError("UNAUTHORIZED", "unauthorized", 403),
  );

  const expired = await connectDesktopRemoteWorkspace(remoteSelected, expiredClient);
  const networkFailed = await connectDesktopRemoteWorkspace(remoteSelected, networkClient);
  const unauthorized = await connectDesktopRemoteWorkspace(remoteSelected, unauthorizedClient);

  assert.equal(expired.displayState, "RemoteError");
  assert.equal(expired.error?.code, "DESKTOP_REMOTE_SESSION_EXPIRED");
  assert.equal(networkFailed.displayState, "RemoteError");
  assert.equal(networkFailed.error?.code, "DESKTOP_REMOTE_NETWORK_FAILURE");
  assert.equal(unauthorized.displayState, "RemoteError");
  assert.equal(unauthorized.error?.code, "DESKTOP_REMOTE_UNAUTHORIZED");
  assert.doesNotMatch(unauthorized.error?.message ?? "", /token|secret|https:\/\/cabinet\.example/i);
  assert.deepEqual(expiredClient.calls, ["openRemoteWorkspace"]);
  assert.deepEqual(networkClient.calls, ["openRemoteWorkspace"]);
  assert.deepEqual(unauthorizedClient.calls, ["openRemoteWorkspace"]);
});

test("windows macos and linux desktop capabilities share remote workspace contract", () => {
  const matrix = createPlatformCapabilityMatrix();

  for (const platform of [matrix.windows, matrix.macos, matrix.linux]) {
    assert.equal(platform.supportsLocalWorkspace, matrix.desktop.supportsLocalWorkspace);
    assert.equal(platform.supportsRemoteWorkspace, matrix.desktop.supportsRemoteWorkspace);
    assert.equal(platform.supportsRemoteEdit, matrix.desktop.supportsRemoteEdit);
    assert.equal(platform.supportsOfflineRemoteEdit, false);
  }
});

class FakeLocalWorkspaceRepository implements DesktopLocalWorkspaceRepository {
  readonly calls: string[] = [];

  async openLocalWorkspace(): Promise<void> {
    this.calls.push("openLocalWorkspace");
  }

  async saveLocalDocument(): Promise<void> {
    this.calls.push("saveLocalDocument");
  }
}

class FakeRemoteWorkspaceClient implements DesktopRemoteWorkspaceApiClient {
  readonly calls: string[] = [];
  readonly savedBodies: string[] = [];
  private readonly openError?: Error;

  constructor(openError?: Error) {
    this.openError = openError;
  }

  async openRemoteWorkspace(): Promise<void> {
    this.calls.push("openRemoteWorkspace");
    if (this.openError) {
      throw this.openError;
    }
  }

  async readRemoteCurrentDocument(command: { readonly documentId: string }) {
    this.calls.push("readRemoteCurrentDocument");
    return {
      workspaceId: "remote-workspace",
      documentId: command.documentId,
      title: "Remote Document",
      path: "docs/remote.md",
      body: "# Remote",
      versionId: "version-1",
      permissionDecision: {
        effect: "allow" as const,
        reason: "document_acl",
      },
    };
  }

  async readRemoteKnowledgeGraph(command: { readonly documentId: string }) {
    this.calls.push("readRemoteKnowledgeGraph");
    return {
      centerDocumentId: command.documentId,
      status: "clean" as const,
      nodes: [
        { id: command.documentId, kind: "document" as const },
        { id: "doc-2", kind: "document" as const },
      ],
      edges: [
        {
          id: "edge-1",
          sourceId: command.documentId,
          targetId: "doc-2",
          kind: "document_link" as const,
        },
      ],
      stats: {
        candidateCount: 2,
        filteredCount: 0,
      },
    };
  }

  async saveRemoteDocument(command: { readonly body: string }): Promise<{ readonly status: "saved-remote" }> {
    this.calls.push("saveRemoteDocument");
    this.savedBodies.push(command.body);
    return { status: "saved-remote" };
  }
}
