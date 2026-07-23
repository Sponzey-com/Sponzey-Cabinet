import { parseDesktopGraphPreference, type DesktopGraphPreference } from "./desktop_graph_preference.ts";
import type { TauriInvoke } from "./tauri_home_transport.ts";

export interface DesktopGraphPreferencePort {
  load(workspaceId: string): Promise<DesktopGraphPreference>;
  save(workspaceId: string, preference: DesktopGraphPreference): Promise<void>;
}

export class DesktopGraphPreferenceTransportError extends Error {
  readonly code: string;
  readonly retryable: boolean;

  constructor(code: string, retryable: boolean) {
    super(code);
    this.code = code;
    this.retryable = retryable;
  }
}

export function createTauriGraphPreferenceTransport(invoke: TauriInvoke): DesktopGraphPreferencePort {
  return Object.freeze({
    async load(workspaceId) {
      const response = await safeInvoke(invoke, "get_desktop_graph_preference", { request: { workspaceId } });
      if (!isSuccess(response)) throw nativeError(response);
      const parsed = parseDesktopGraphPreference(response.data);
      if (!parsed.valid) throw new DesktopGraphPreferenceTransportError("GRAPH_PREFERENCE_INVALID", false);
      return parsed.preference;
    },
    async save(workspaceId, preference) {
      const parsed = parseDesktopGraphPreference(preference);
      if (!parsed.valid) throw new DesktopGraphPreferenceTransportError("GRAPH_PREFERENCE_INVALID", false);
      const response = await safeInvoke(invoke, "save_desktop_graph_preference", {
        request: { workspaceId, preference: parsed.preference },
      });
      if (!isSuccess(response) || !isRecord(response.data) || response.data.saved !== true) throw nativeError(response);
    },
  });
}

async function safeInvoke(invoke: TauriInvoke, command: string, args: Record<string, unknown>): Promise<unknown> {
  try { return await invoke(command, args); }
  catch { throw new DesktopGraphPreferenceTransportError("COMMAND_BRIDGE_FAILED", false); }
}

function nativeError(value: unknown): DesktopGraphPreferenceTransportError {
  if (isRecord(value) && value.ok === false && typeof value.errorCode === "string" && typeof value.retryable === "boolean") {
    return new DesktopGraphPreferenceTransportError(value.errorCode, value.retryable);
  }
  return new DesktopGraphPreferenceTransportError("COMMAND_BRIDGE_FAILED", false);
}

function isSuccess(value: unknown): value is { readonly ok: true; readonly data: unknown } {
  return isRecord(value) && value.ok === true && "data" in value;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
