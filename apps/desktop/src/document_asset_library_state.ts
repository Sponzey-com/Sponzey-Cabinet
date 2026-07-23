import {
  createDesktopAssetSnapshot,
  requestDesktopAssetLoad,
  requestDesktopWorkspaceAssetLoad,
  requestDesktopWorkspaceAssetNextPage,
  selectDesktopAsset,
  type DesktopAssetSurfaceSnapshot,
} from "./desktop_asset_controller.ts";

export type DocumentAssetLibraryStatus = "Closed" | "Loading" | "LoadingMore" | "Ready" | "Empty" | "Failed" | "Linking";

export interface DocumentAssetLibraryState {
  readonly status: DocumentAssetLibraryStatus;
  readonly workspaceId: string;
  readonly documentId?: string;
  readonly generation: number;
  readonly assets: DesktopAssetSurfaceSnapshot;
}

export interface DocumentAssetLibraryLinkCompletion {
  readonly library: DocumentAssetLibraryState;
  readonly documentAssets?: DesktopAssetSurfaceSnapshot;
}

export function createDocumentAssetLibraryState(workspaceId: string): DocumentAssetLibraryState {
  return Object.freeze({
    status: "Closed",
    workspaceId,
    generation: 0,
    assets: createDesktopAssetSnapshot(workspaceId),
  });
}

export function requestDocumentAssetLibraryOpen(
  state: DocumentAssetLibraryState,
  documentId: string | undefined,
): DocumentAssetLibraryState {
  const normalizedDocumentId = documentId?.trim();
  if (!normalizedDocumentId) return state;
  const documentScope = requestDesktopAssetLoad(createDesktopAssetSnapshot(state.workspaceId), normalizedDocumentId);
  return Object.freeze({
    status: "Loading",
    workspaceId: state.workspaceId,
    documentId: normalizedDocumentId,
    generation: state.generation + 1,
    assets: requestDesktopWorkspaceAssetLoad(documentScope, normalizedDocumentId),
  });
}

export function applyDocumentAssetLibraryLoad(
  state: DocumentAssetLibraryState,
  generation: number,
  assets: DesktopAssetSurfaceSnapshot,
): DocumentAssetLibraryState {
  if ((state.status !== "Loading" && state.status !== "LoadingMore") || generation !== state.generation) return state;
  const preserveSelection = state.status === "LoadingMore";
  const status = assets.state === "Ready"
    ? "Ready"
    : assets.state === "Empty"
      ? "Empty"
      : "Failed";
  return Object.freeze({
    ...state,
    status,
    assets: Object.freeze({ ...assets, selectedAssetId: preserveSelection ? assets.selectedAssetId : undefined }),
  });
}

export function requestDocumentAssetLibraryMore(state: DocumentAssetLibraryState): DocumentAssetLibraryState {
  if (state.status !== "Ready") return state;
  const assets = requestDesktopWorkspaceAssetNextPage(state.assets);
  if (assets === state.assets) return state;
  return Object.freeze({ ...state, status: "LoadingMore", generation: state.generation + 1, assets });
}

export function selectDocumentAssetLibraryItem(
  state: DocumentAssetLibraryState,
  assetId: string,
): DocumentAssetLibraryState {
  if (state.status !== "Ready") return state;
  const assets = selectDesktopAsset(state.assets, assetId);
  return assets === state.assets ? state : Object.freeze({ ...state, assets });
}

export function beginDocumentAssetLibraryLink(state: DocumentAssetLibraryState): DocumentAssetLibraryState {
  if (state.status !== "Ready" || !state.assets.selectedAssetId) return state;
  return Object.freeze({ ...state, status: "Linking" });
}

export function completeDocumentAssetLibraryLink(
  state: DocumentAssetLibraryState,
  generation: number,
  result: DesktopAssetSurfaceSnapshot,
): DocumentAssetLibraryLinkCompletion {
  if (state.status !== "Linking" || generation !== state.generation) {
    return Object.freeze({ library: state });
  }
  if (result.scope === "Document" && result.mutationState === "Idle" && result.documentId === state.documentId) {
    return Object.freeze({
      library: closeDocumentAssetLibrary(state),
      documentAssets: result,
    });
  }
  return Object.freeze({
    library: Object.freeze({ ...state, status: "Failed", assets: Object.freeze({ ...state.assets, mutationState: "Failed" }) }),
  });
}

export function closeDocumentAssetLibrary(state: DocumentAssetLibraryState): DocumentAssetLibraryState {
  if (state.status === "Closed") return state;
  return Object.freeze({
    status: "Closed",
    workspaceId: state.workspaceId,
    generation: state.generation,
    assets: createDesktopAssetSnapshot(state.workspaceId),
  });
}
