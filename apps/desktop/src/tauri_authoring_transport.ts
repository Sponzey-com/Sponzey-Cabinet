import type {
  CreateLocalDocumentCommand,
  CreateLocalDocumentResult,
  CurrentDocumentQuery,
  CurrentDocumentView,
  DocumentHistoryPage,
  DocumentHistoryQuery,
  DocumentVersionQuery,
  DocumentVersionView,
  LocalDesktopCommandEnvelope,
  LocalDesktopCommandErrorCode,
  LocalDesktopCommandResponse,
  LocalDesktopCommandTransport,
  RestoreDocumentVersionCommand,
  RestoreDocumentVersionResult,
  RestorePreviewQuery,
  RestorePreviewResult,
  RenameLocalDocumentCommand,
  RenameLocalDocumentResult,
  SaveDocumentRevisionCommand,
  SaveDocumentRevisionResult,
} from "@sponzey-cabinet/client-core";

import type { TauriInvoke } from "./tauri_home_transport.ts";

const authoringCommands = new Set([
  "create_document",
  "rename_document",
  "get_current_document",
  "save_document_revision",
  "get_document_history",
  "get_document_version",
  "preview_document_restore",
  "restore_document_version",
]);

export function createTauriDocumentAuthoringTransport(
  invoke: TauriInvoke,
): LocalDesktopCommandTransport {
  return async <TData>(
    envelope: LocalDesktopCommandEnvelope,
  ): Promise<LocalDesktopCommandResponse<TData>> => {
    if (!authoringCommands.has(envelope.commandName)) return bridgeFailure();
    const request = toNativeRequest(envelope);
    if (!request) return bridgeFailure();

    try {
      const response = await invoke("execute_desktop_document_authoring", { request });
      const mapped = mapNativeResponse(envelope, response) as LocalDesktopCommandResponse<TData>;
      if (mapped.ok && triggersProjection(envelope.commandName)) {
        await invoke("run_desktop_projection_worker").catch(() => undefined);
      }
      return mapped;
    } catch {
      return bridgeFailure();
    }
  };
}

function triggersProjection(commandName: string): boolean {
  return commandName === "create_document" ||
    commandName === "rename_document" ||
    commandName === "save_document_revision" ||
    commandName === "restore_document_version";
}

function toNativeRequest(envelope: LocalDesktopCommandEnvelope): Record<string, unknown> | undefined {
  const payload = envelope.payload;
  if (envelope.commandName === "create_document" && isCreateCommand(payload)) {
    return {
      kind: "create",
      workspaceId: payload.workspaceId,
      documentId: payload.documentId,
      path: payload.path,
      body: payload.body,
      versionId: payload.versionId,
      snapshotRef: payload.snapshotRef,
      author: payload.author,
      summary: payload.summary,
    };
  }
  if (envelope.commandName === "get_current_document" && isCurrentQuery(payload)) {
    return {
      kind: "get_current",
      workspaceId: payload.workspaceId,
      documentId: payload.documentId,
    };
  }
  if (envelope.commandName === "rename_document" && isRenameCommand(payload)) {
    return { kind: "rename", workspaceId: payload.workspaceId, documentId: payload.documentId, currentVersionId: payload.currentVersionId, title: payload.title, path: payload.path };
  }
  if (envelope.commandName === "save_document_revision" && isSaveRevisionCommand(payload)) {
    return {
      kind: "update",
      workspaceId: payload.workspaceId,
      documentId: payload.documentId,
      body: payload.body,
      expectedVersionId: payload.expectedVersionId,
      versionId: payload.nextVersionId,
      snapshotRef: payload.snapshotRef,
      author: payload.author,
      summary: payload.summary,
    };
  }
  if (envelope.commandName === "get_document_history" && isHistoryQuery(payload)) {
    return {
      kind: "get_history",
      workspaceId: payload.workspaceId,
      documentId: payload.documentId,
      limit: payload.limit,
    };
  }
  if (envelope.commandName === "get_document_version" && isVersionQuery(payload)) {
    return {
      kind: "get_version",
      workspaceId: payload.workspaceId,
      documentId: payload.documentId,
      versionId: payload.versionId,
    };
  }
  if (envelope.commandName === "preview_document_restore" && isRestorePreviewQuery(payload)) {
    return {
      kind: "preview_restore",
      workspaceId: payload.workspaceId,
      documentId: payload.documentId,
      targetVersionId: payload.targetVersionId,
      expectedCurrentVersionId: payload.expectedCurrentVersionId,
    };
  }
  if (envelope.commandName === "restore_document_version" && isRestoreCommand(payload)) {
    return {
      kind: "restore",
      workspaceId: payload.workspaceId,
      documentId: payload.documentId,
      targetVersionId: payload.targetVersionId,
      expectedCurrentVersionId: payload.expectedCurrentVersionId,
      restoredVersionId: payload.restoredVersionId,
      restoredSnapshotRef: payload.restoredSnapshotRef,
      author: payload.author,
      summary: payload.summary,
    };
  }
  return undefined;
}

function mapNativeResponse(
  envelope: LocalDesktopCommandEnvelope,
  response: unknown,
): LocalDesktopCommandResponse<
  | CreateLocalDocumentResult
  | CurrentDocumentView
  | SaveDocumentRevisionResult
  | DocumentHistoryPage
  | DocumentVersionView
  | RestorePreviewResult
  | RestoreDocumentVersionResult
  | RenameLocalDocumentResult
> {
  if (!isRecord(response) || typeof response.ok !== "boolean") return bridgeFailure();
  if (!response.ok) return mapNativeFailure(response);
  if (!isNativeData(response.data)) return bridgeFailure();

  const payload = envelope.payload;
  if (
    envelope.commandName === "rename_document" &&
    isRenameCommand(payload) &&
    response.data.kind === "renamed" &&
    typeof response.data.currentVersionId === "string" &&
    typeof response.data.title === "string" &&
    typeof response.data.path === "string"
  ) {
    return { ok: true, data: { workspaceId: payload.workspaceId, documentId: response.data.documentId, currentVersionId: response.data.currentVersionId, title: response.data.title, path: response.data.path } };
  }
  if (
    envelope.commandName === "create_document" &&
    isCreateCommand(payload) &&
    response.data.kind === "created"
  ) {
    return {
      ok: true,
      data: {
        workspaceId: payload.workspaceId,
        documentId: response.data.documentId,
        currentVersionId: response.data.currentVersionId,
      },
    };
  }
  if (
    envelope.commandName === "get_current_document" &&
    isCurrentQuery(payload) &&
    isNativeCurrentData(response.data)
  ) {
    return {
      ok: true,
      data: {
        workspaceId: payload.workspaceId,
        documentId: response.data.documentId,
        title: response.data.title,
        path: response.data.path,
        body: response.data.body,
        versionId: response.data.currentVersionId,
      },
    };
  }
  if (
    envelope.commandName === "save_document_revision" &&
    isSaveRevisionCommand(payload) &&
    response.data.kind === "updated"
  ) {
    return {
      ok: true,
      data: {
        status: "saved-local",
        workspaceId: payload.workspaceId,
        documentId: response.data.documentId,
        currentVersionId: response.data.currentVersionId,
        versionAppended: true,
        revision: payload.revision,
      },
    };
  }
  if (
    envelope.commandName === "get_document_history" &&
    isHistoryQuery(payload) &&
    isNativeHistoryData(response.data)
  ) {
    return {
      ok: true,
      data: {
        workspaceId: payload.workspaceId,
        documentId: payload.documentId,
        entries: response.data.entries,
        nextCursor: response.data.nextCursor,
      },
    };
  }
  if (
    envelope.commandName === "get_document_version" &&
    isVersionQuery(payload) &&
    isNativeVersionData(response.data)
  ) {
    return {
      ok: true,
      data: {
        workspaceId: payload.workspaceId,
        documentId: response.data.documentId,
        versionId: response.data.versionId,
        body: response.data.body,
      },
    };
  }
  if (
    envelope.commandName === "preview_document_restore" &&
    isRestorePreviewQuery(payload) &&
    isNativeRestorePreviewData(response.data)
  ) {
    return {
      ok: true,
      data: {
        workspaceId: payload.workspaceId,
        documentId: response.data.documentId,
        targetVersionId: response.data.targetVersionId,
        expectedCurrentVersionId: response.data.expectedCurrentVersionId,
        canRestore: response.data.canRestore,
        lines: response.data.lines,
      },
    };
  }
  if (
    envelope.commandName === "restore_document_version" &&
    isRestoreCommand(payload) &&
    isNativeRestoredData(response.data)
  ) {
    return {
      ok: true,
      data: {
        workspaceId: payload.workspaceId,
        documentId: response.data.documentId,
        restoredVersionId: response.data.restoredVersionId,
        currentVersionId: response.data.currentVersionId,
        finalState: "Completed",
      },
    };
  }
  return bridgeFailure();
}

function mapNativeFailure(
  response: Record<string, unknown>,
): LocalDesktopCommandResponse<never> {
  if (
    typeof response.errorCode !== "string" ||
    typeof response.retryable !== "boolean" ||
    typeof response.repairRequired !== "boolean"
  ) {
    return bridgeFailure();
  }
  return {
    ok: false,
    errorCode: response.errorCode as LocalDesktopCommandErrorCode,
    retryable: response.retryable,
    repairRequired: response.repairRequired,
  };
}

function isCreateCommand(
  value: Record<string, unknown>,
): value is CreateLocalDocumentCommand & Record<string, unknown> {
  return hasStringFields(value, [
    "workspaceId",
    "documentId",
    "path",
    "body",
    "versionId",
    "snapshotRef",
    "author",
    "summary",
  ]);
}

function isCurrentQuery(
  value: Record<string, unknown>,
): value is CurrentDocumentQuery & Record<string, unknown> {
  return hasStringFields(value, ["workspaceId", "documentId"]);
}

function isSaveRevisionCommand(
  value: Record<string, unknown>,
): value is SaveDocumentRevisionCommand & Record<string, unknown> {
  return (
    hasStringFields(value, [
      "workspaceId",
      "documentId",
      "body",
      "expectedVersionId",
      "nextVersionId",
      "snapshotRef",
      "author",
      "summary",
    ]) &&
    typeof value.revision === "number" &&
    Number.isInteger(value.revision) &&
    value.revision >= 0
  );
}

function isRenameCommand(value: Record<string, unknown>): value is RenameLocalDocumentCommand & Record<string, unknown> {
  return hasStringFields(value, ["workspaceId", "documentId", "currentVersionId", "title", "path"]);
}

function isHistoryQuery(
  value: Record<string, unknown>,
): value is DocumentHistoryQuery & Record<string, unknown> {
  return (
    hasStringFields(value, ["workspaceId", "documentId"]) &&
    typeof value.limit === "number" &&
    Number.isInteger(value.limit) &&
    value.limit > 0
  );
}

function isVersionQuery(
  value: Record<string, unknown>,
): value is DocumentVersionQuery & Record<string, unknown> {
  return hasStringFields(value, ["workspaceId", "documentId", "versionId"]);
}

function isRestorePreviewQuery(
  value: Record<string, unknown>,
): value is RestorePreviewQuery & Record<string, unknown> {
  return hasStringFields(value, [
    "workspaceId",
    "documentId",
    "targetVersionId",
    "expectedCurrentVersionId",
  ]);
}

function isRestoreCommand(
  value: Record<string, unknown>,
): value is RestoreDocumentVersionCommand & Record<string, unknown> {
  return hasStringFields(value, [
    "workspaceId",
    "documentId",
    "targetVersionId",
    "expectedCurrentVersionId",
    "restoredVersionId",
    "restoredSnapshotRef",
    "author",
    "summary",
  ]);
}

function isNativeData(value: unknown): value is {
  kind: string;
  documentId: string;
  currentVersionId?: string;
  title?: unknown;
  path?: unknown;
  body?: unknown;
} {
  return (
    isRecord(value) &&
    [
      "created",
      "updated",
      "renamed",
      "current",
      "history",
      "version",
      "restore_preview",
      "restored",
    ].includes(String(value.kind)) &&
    typeof value.documentId === "string"
  );
}

function isNativeCurrentData(
  value: ReturnTypeData,
): value is ReturnTypeData & { title: string; path: string; body: string } {
  return (
    value.kind === "current" &&
    typeof value.currentVersionId === "string" &&
    typeof value.title === "string" &&
    typeof value.path === "string" &&
    typeof value.body === "string"
  );
}

function isNativeHistoryData(value: ReturnTypeData): value is ReturnTypeData & {
  entries: DocumentHistoryPage["entries"];
  nextCursor?: string;
} {
  return value.kind === "history" && Array.isArray(value.entries);
}

function isNativeVersionData(
  value: ReturnTypeData,
): value is ReturnTypeData & { versionId: string; body: string } {
  return value.kind === "version" && typeof value.versionId === "string" && typeof value.body === "string";
}

function isNativeRestorePreviewData(value: ReturnTypeData): value is ReturnTypeData & {
  targetVersionId: string;
  expectedCurrentVersionId: string;
  canRestore: boolean;
  lines: RestorePreviewResult["lines"];
} {
  return (
    value.kind === "restore_preview" &&
    typeof value.targetVersionId === "string" &&
    typeof value.expectedCurrentVersionId === "string" &&
    typeof value.canRestore === "boolean" &&
    Array.isArray(value.lines)
  );
}

function isNativeRestoredData(value: ReturnTypeData): value is ReturnTypeData & {
  restoredVersionId: string;
  currentVersionId: string;
} {
  return (
    value.kind === "restored" &&
    typeof value.restoredVersionId === "string" &&
    typeof value.currentVersionId === "string"
  );
}

type ReturnTypeData = {
  kind: string;
  documentId: string;
  currentVersionId?: string;
  title?: unknown;
  path?: unknown;
  body?: unknown;
  versionId?: unknown;
  targetVersionId?: unknown;
  expectedCurrentVersionId?: unknown;
  canRestore?: unknown;
  lines?: unknown;
  restoredVersionId?: unknown;
  entries?: unknown;
  nextCursor?: unknown;
};

function hasStringFields(value: Record<string, unknown>, fields: readonly string[]): boolean {
  return fields.every((field) => typeof value[field] === "string");
}

function bridgeFailure<TData>(): LocalDesktopCommandResponse<TData> {
  return {
    ok: false,
    errorCode: "COMMAND_BRIDGE_FAILED",
    retryable: false,
    repairRequired: false,
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
