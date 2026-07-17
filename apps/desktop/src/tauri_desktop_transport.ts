import type { LocalDesktopCommandTransport } from "@sponzey-cabinet/client-core";

import { createTauriWorkspaceHomeTransport, type TauriInvoke } from "./tauri_home_transport.ts";
import { createTauriDocumentNavigatorTransport } from "./tauri_navigator_transport.ts";
import { createTauriDocumentAuthoringTransport } from "./tauri_authoring_transport.ts";
import { createTauriDiscoveryTransport } from "./tauri_discovery_transport.ts";
import { createTauriDocumentDiffTransport } from "./tauri_document_diff_transport.ts";

export function createTauriDesktopTransport(invoke: TauriInvoke): LocalDesktopCommandTransport {
  const home = createTauriWorkspaceHomeTransport(invoke);
  const navigator = createTauriDocumentNavigatorTransport(invoke);
  const authoring = createTauriDocumentAuthoringTransport(invoke);
  const discovery = createTauriDiscoveryTransport(invoke);
  const diff = createTauriDocumentDiffTransport(invoke);
  return async (request) => {
    if (request.commandName === "local_workspace_home") return home(request);
    if (request.commandName === "local_document_navigator") return navigator(request);
    if (request.commandName === "get_graph_projection" || request.commandName === "list_document_assets") return discovery(request);
    if (request.commandName === "compare_document_versions") return diff(request);
    if (
      request.commandName === "create_document" ||
      request.commandName === "rename_document" ||
      request.commandName === "get_current_document" ||
      request.commandName === "save_document_revision" ||
      request.commandName === "get_document_history" ||
      request.commandName === "get_document_version" ||
      request.commandName === "preview_document_restore" ||
      request.commandName === "restore_document_version"
    ) {
      return authoring(request);
    }
    return {
      ok: false,
      errorCode: "COMMAND_BRIDGE_FAILED",
      retryable: false,
    };
  };
}
