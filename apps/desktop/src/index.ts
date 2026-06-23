import type { AttachAssetCommand, SelectedAssetDraft } from "@sponzey-cabinet/client-core";
import {
  createAttachAssetClientCommand,
  createClientCapabilities,
} from "@sponzey-cabinet/client-core";
import { createEditorBoundaryDescriptor } from "@sponzey-cabinet/editor";
import { createShellDescriptor, createWorkspaceShellModel } from "@sponzey-cabinet/ui";

const capabilities = createClientCapabilities("desktop-local");

export interface DesktopSelectedAsset {
  readonly assetId: string;
  readonly label: string;
  readonly fileName: string;
  readonly mediaType: string;
  readonly byteSize: number;
}

export function mapDesktopAssetSelection(selection: DesktopSelectedAsset): SelectedAssetDraft {
  return {
    assetId: selection.assetId,
    label: selection.label,
    fileName: selection.fileName,
    mediaType: selection.mediaType,
    byteSize: selection.byteSize,
  };
}

export function createDesktopAttachAssetCommand(
  workspaceId: string,
  documentId: string,
  selection: DesktopSelectedAsset,
): AttachAssetCommand {
  return createAttachAssetClientCommand(workspaceId, documentId, mapDesktopAssetSelection(selection));
}

export const desktopShell = {
  shell: createShellDescriptor(capabilities),
  workspace: createWorkspaceShellModel(capabilities),
  editor: createEditorBoundaryDescriptor(capabilities),
};
