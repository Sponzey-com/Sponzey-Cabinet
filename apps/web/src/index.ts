import type { AttachAssetCommand, SelectedAssetDraft } from "@sponzey-cabinet/client-core";
import {
  createAttachAssetClientCommand,
  createClientCapabilities,
} from "@sponzey-cabinet/client-core";
import { createEditorBoundaryDescriptor } from "@sponzey-cabinet/editor";
import { createShellDescriptor, createWorkspaceShellModel } from "@sponzey-cabinet/ui";

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

export const webShell = {
  shell: createShellDescriptor(capabilities),
  workspace: createWorkspaceShellModel(capabilities),
  editor: createEditorBoundaryDescriptor(capabilities),
};
