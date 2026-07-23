import type {
  LocalDesktopCommandEnvelope,
  LocalDesktopCommandResponse,
  LocalDesktopCommandTransport,
  WorkspaceHomeQuery,
  WorkspaceHomeResult,
} from "@sponzey-cabinet/client-core";

export type TauriInvoke = (
  command: string,
  args?: Record<string, unknown>,
) => Promise<unknown>;

export function createTauriWorkspaceHomeTransport(
  invoke: TauriInvoke,
): LocalDesktopCommandTransport {
  return async <TData>(
    envelope: LocalDesktopCommandEnvelope,
  ): Promise<LocalDesktopCommandResponse<TData>> => {
    if (envelope.commandName !== "local_workspace_home" || !isWorkspaceHomeQuery(envelope.payload)) {
      return bridgeFailure();
    }

    try {
      const query = envelope.payload;
      const response = await invoke("get_desktop_workspace_home", {
        request: {
          command_name: envelope.commandName,
          payload: {
            kind: "workspace_home",
            workspace_id: query.workspaceId,
            recent_documents: query.recentDocuments,
            favorites: query.favorites,
            tags: query.tags,
            recent_changes: query.recentChanges,
            unfinished_items: query.unfinishedItems,
          },
        },
      });
      return isWorkspaceHomeCommandResponse(response)
        ? (response as LocalDesktopCommandResponse<TData>)
        : bridgeFailure();
    } catch {
      return bridgeFailure();
    }
  };
}

export function getGlobalTauriInvoke(): TauriInvoke | undefined {
  const tauri = (globalThis as unknown as {
    readonly __TAURI__?: { readonly core?: { readonly invoke?: TauriInvoke } };
  }).__TAURI__;
  return typeof tauri?.core?.invoke === "function" ? tauri.core.invoke.bind(tauri.core) : undefined;
}

function bridgeFailure<TData>(): LocalDesktopCommandResponse<TData> {
  return {
    ok: false,
    errorCode: "COMMAND_BRIDGE_FAILED",
    retryable: false,
  };
}

function isWorkspaceHomeQuery(value: Record<string, unknown>): value is WorkspaceHomeQuery & Record<string, unknown> {
  return (
    typeof value.workspaceId === "string" &&
    [
      value.recentDocuments,
      value.favorites,
      value.tags,
      value.recentChanges,
      value.unfinishedItems,
    ].every((limit) => typeof limit === "number" && Number.isInteger(limit))
  );
}

function isWorkspaceHomeCommandResponse(
  value: unknown,
): value is LocalDesktopCommandResponse<WorkspaceHomeResult> {
  if (!isRecord(value) || typeof value.ok !== "boolean") {
    return false;
  }
  if (value.ok) {
    return isWorkspaceHomeResult(value.data);
  }
  return (
    typeof value.errorCode === "string" &&
    typeof value.retryable === "boolean"
  );
}

function isWorkspaceHomeResult(value: unknown): value is WorkspaceHomeResult {
  return (
    isRecord(value) &&
    typeof value.workspaceId === "string" &&
    ["Ready", "Empty", "Degraded"].includes(String(value.state)) &&
    Array.isArray(value.recentDocuments) &&
    Array.isArray(value.favorites) &&
    Array.isArray(value.tags) &&
    Array.isArray(value.recentChanges) &&
    Array.isArray(value.unfinishedItems) &&
    typeof value.backupStatus === "string" &&
    typeof value.healthStatus === "string" &&
    isNonNegativeInteger(value.documentCount) &&
    isNonNegativeInteger(value.assetCount) &&
    isNonNegativeInteger(value.canvasCount) &&
    isSummaryUnavailable(value.summaryUnavailable)
  );
}

function isSummaryUnavailable(value: unknown): boolean {
  if (!Array.isArray(value)) return false;
  const allowed = new Set(["Documents", "Assets", "Canvases"]);
  return value.every((kind) => typeof kind === "string" && allowed.has(kind)) && new Set(value).size === value.length;
}

function isNonNegativeInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
