import type {
  AttachAssetCommand,
  CanvasLifecycleStateView,
  CanvasNodeTargetKindView,
  CanvasView,
  KnowledgeGraphEdgeKindView,
  KnowledgeGraphNodeKindView,
  KnowledgeGraphStatusView,
  KnowledgeGraphView,
  SelectedAssetDraft,
} from "@sponzey-cabinet/client-core";
import {
  createAttachAssetClientCommand,
  createClientCapabilities,
  createSelfHostApiClient,
  createSelfHostApiClientConfig,
  withSelfHostSessionToken,
  type CabinetAdminApiClient,
  type CabinetCollaborationApiClient,
  type CabinetHttpTransport,
  type SelfHostApiClientConfig,
} from "@sponzey-cabinet/client-core";
import { createEditorBoundaryDescriptor } from "@sponzey-cabinet/editor";
import {
  createInitialAdminViewModel,
  createShellDescriptor,
  createWorkspaceShellModel,
  type SelfHostAdminViewModel,
} from "@sponzey-cabinet/ui";

const capabilities = createClientCapabilities("web-local");

export interface WebSelectedAsset {
  readonly assetId: string;
  readonly label: string;
  readonly fileName: string;
  readonly mediaType: string;
  readonly byteSize: number;
}

export function mapWebAssetSelection(selection: WebSelectedAsset): SelectedAssetDraft {
  return {
    assetId: selection.assetId,
    label: selection.label,
    fileName: selection.fileName,
    mediaType: selection.mediaType,
    byteSize: selection.byteSize,
  };
}

export function createWebAttachAssetCommand(
  workspaceId: string,
  documentId: string,
  selection: WebSelectedAsset,
): AttachAssetCommand {
  return createAttachAssetClientCommand(workspaceId, documentId, mapWebAssetSelection(selection));
}

export interface WebSelfHostAdminBootstrap {
  readonly serverBaseUrl: string;
  readonly workspaceId: string;
  readonly sessionToken?: string;
}

export function createWebSelfHostApiClientConfig(
  bootstrap: WebSelfHostAdminBootstrap,
): SelfHostApiClientConfig {
  const config = createSelfHostApiClientConfig({
    baseUrl: bootstrap.serverBaseUrl,
  });
  return bootstrap.sessionToken ? withSelfHostSessionToken(config, bootstrap.sessionToken) : config;
}

export function createWebSelfHostAdminClient(
  bootstrap: WebSelfHostAdminBootstrap,
  transport?: CabinetHttpTransport,
): CabinetAdminApiClient {
  const config = createWebSelfHostApiClientConfig(bootstrap);
  return transport
    ? createSelfHostApiClient(config, transport)
    : createSelfHostApiClient(config);
}

export function createWebSelfHostCollaborationClient(
  bootstrap: WebSelfHostAdminBootstrap,
  transport?: CabinetHttpTransport,
): CabinetCollaborationApiClient {
  const config = createWebSelfHostApiClientConfig(bootstrap);
  return transport
    ? createSelfHostApiClient(config, transport)
    : createSelfHostApiClient(config);
}

export interface WebGraphNodeRow {
  readonly id: string;
  readonly kindLabel: string;
  readonly isCenter: boolean;
}

export interface WebGraphEdgeRow {
  readonly id: string;
  readonly sourceId: string;
  readonly targetId: string;
  readonly label: string;
}

export interface WebGraphViewModel {
  readonly centerNodeId: string;
  readonly statusLabel: string;
  readonly summary: string;
  readonly nodeRows: readonly WebGraphNodeRow[];
  readonly edgeRows: readonly WebGraphEdgeRow[];
}

export function createWebGraphViewModel(graph: KnowledgeGraphView): WebGraphViewModel {
  return {
    centerNodeId: graph.centerDocumentId,
    statusLabel: formatGraphStatusLabel(graph.status),
    summary: [
      formatCount(graph.nodes.length, "node"),
      formatCount(graph.edges.length, "edge"),
      `${graph.stats.filteredCount} filtered`,
    ].join(", "),
    nodeRows: graph.nodes.map((node) => ({
      id: node.id,
      kindLabel: formatGraphNodeKindLabel(node.kind),
      isCenter: node.id === graph.centerDocumentId,
    })),
    edgeRows: graph.edges.map((edge) => ({
      id: edge.id,
      sourceId: edge.sourceId,
      targetId: edge.targetId,
      label: formatGraphEdgeKindLabel(edge.kind),
    })),
  };
}

export interface WebCanvasNodeRow {
  readonly id: string;
  readonly targetLabel: string;
  readonly positionLabel: string;
}

export interface WebCanvasEdgeRow {
  readonly id: string;
  readonly sourceId: string;
  readonly targetId: string;
}

export interface WebCanvasViewModel {
  readonly canvasId: string;
  readonly statusLabel: string;
  readonly summary: string;
  readonly nodeRows: readonly WebCanvasNodeRow[];
  readonly edgeRows: readonly WebCanvasEdgeRow[];
  readonly embedReference?: string;
}

export function createWebCanvasViewModel(canvas: CanvasView): WebCanvasViewModel {
  return {
    canvasId: canvas.canvasId,
    statusLabel: formatCanvasStateLabel(canvas.state),
    summary: [
      formatCount(canvas.nodes.length, "node"),
      formatCount(canvas.edges.length, "edge"),
    ].join(", "),
    nodeRows: canvas.nodes.map((node) => ({
      id: node.id,
      targetLabel: formatCanvasNodeTargetLabel(node.targetKind),
      positionLabel: `${node.x},${node.y}`,
    })),
    edgeRows: canvas.edges.map((edge) => ({
      id: edge.id,
      sourceId: edge.sourceId,
      targetId: edge.targetId,
    })),
    embedReference: canvas.embedReference,
  };
}

export function createWebSelfHostAdminModel(
  bootstrap: WebSelfHostAdminBootstrap,
): SelfHostAdminViewModel {
  return createInitialAdminViewModel({
    serverBaseUrl: bootstrap.serverBaseUrl,
    workspaceId: bootstrap.workspaceId,
  });
}

export const webShell = {
  shell: createShellDescriptor(capabilities),
  workspace: createWorkspaceShellModel(capabilities),
  editor: createEditorBoundaryDescriptor(capabilities),
};

function formatGraphStatusLabel(status: KnowledgeGraphStatusView): string {
  switch (status) {
    case "clean":
      return "Clean";
    case "reindex_requested":
      return "Reindex Requested";
    case "reindexing":
      return "Reindexing";
    case "degraded":
      return "Degraded";
  }
}

function formatGraphNodeKindLabel(kind: KnowledgeGraphNodeKindView): string {
  switch (kind) {
    case "document":
      return "Document";
    case "unresolved_link":
      return "Unresolved Link";
    case "attachment":
      return "Attachment";
    case "external_link":
      return "External Link";
  }
}

function formatGraphEdgeKindLabel(kind: KnowledgeGraphEdgeKindView): string {
  switch (kind) {
    case "document_link":
      return "Document Link";
    case "attachment_reference":
      return "Attachment Reference";
    case "external_reference":
      return "External Reference";
    case "canvas_relation":
      return "Canvas Relation";
  }
}

function formatCanvasStateLabel(state: CanvasLifecycleStateView): string {
  switch (state) {
    case "draft":
      return "Draft";
    case "saved":
      return "Saved";
    case "embedded":
      return "Embedded";
    case "updated":
      return "Updated";
    case "archived":
      return "Archived";
  }
}

function formatCanvasNodeTargetLabel(kind: CanvasNodeTargetKindView): string {
  switch (kind) {
    case "document":
      return "Document";
    case "attachment":
      return "Attachment";
    case "external_link":
      return "External Link";
    case "text_card":
      return "Text Card";
  }
}

function formatCount(count: number, label: string): string {
  return `${count} ${count === 1 ? label : `${label}s`}`;
}
