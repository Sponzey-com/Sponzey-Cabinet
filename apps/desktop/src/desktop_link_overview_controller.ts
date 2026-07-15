import {
  LocalDesktopCommandClientError,
  createLinkOverviewQuery,
  type LocalDesktopCommandClient,
} from "@sponzey-cabinet/client-core";
import {
  createLinkPanelViewModel,
  type LinkPanelViewModel,
} from "@sponzey-cabinet/ui";

export type DesktopLinkOverviewState = "Idle" | "Loading" | "Ready" | "Empty" | "Failed";

export interface DesktopLinkOverviewSnapshot {
  readonly state: DesktopLinkOverviewState;
  readonly workspaceId: string;
  readonly documentId: string;
  readonly generation: number;
  readonly panel?: LinkPanelViewModel;
  readonly errorCode?: string;
  readonly retryable?: boolean;
}

export function createDesktopLinkOverviewSnapshot(
  workspaceId: string,
  documentId: string,
): DesktopLinkOverviewSnapshot {
  return Object.freeze({ state: "Idle", workspaceId, documentId, generation: 0 });
}

export function requestDesktopLinkOverviewLoad(
  snapshot: DesktopLinkOverviewSnapshot,
): DesktopLinkOverviewSnapshot {
  return Object.freeze({
    ...snapshot,
    state: "Loading",
    generation: snapshot.generation + 1,
    panel: undefined,
    errorCode: undefined,
    retryable: undefined,
  });
}

export async function loadDesktopLinkOverview(
  client: Pick<LocalDesktopCommandClient, "getLinkOverview">,
  loading: DesktopLinkOverviewSnapshot,
): Promise<DesktopLinkOverviewSnapshot> {
  if (loading.state !== "Loading") return loading;
  try {
    const overview = await client.getLinkOverview(
      createLinkOverviewQuery(loading.workspaceId, loading.documentId),
    );
    const panel = createLinkPanelViewModel(overview);
    const itemCount = panel.backlinks.length
      + panel.unresolvedLinks.length
      + panel.orphanDocuments.length;
    return Object.freeze({
      ...loading,
      state: itemCount === 0 ? "Empty" : "Ready",
      panel,
      errorCode: undefined,
      retryable: undefined,
    });
  } catch (error) {
    const mapped = error instanceof LocalDesktopCommandClientError
      ? { code: error.code, retryable: error.retryable }
      : { code: "COMMAND_BRIDGE_FAILED", retryable: false };
    return Object.freeze({
      ...loading,
      state: "Failed",
      panel: undefined,
      errorCode: mapped.code,
      retryable: mapped.retryable,
    });
  }
}
