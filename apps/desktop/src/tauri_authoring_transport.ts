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
  RenameLocalDocumentCommand,
  RenameLocalDocumentResult,
  RestoreDocumentVersionCommand,
  RestoreDocumentVersionResult,
  RestorePreviewQuery,
  RestorePreviewResult,
  SaveDocumentRevisionCommand,
  SaveDocumentRevisionResult,
} from "@sponzey-cabinet/client-core";

import type { TauriInvoke } from "./tauri_home_transport.ts";
import { mapNativeDocumentDiffPayload } from "./tauri_document_diff_transport.ts";

const mutationCommands = new Set(["create_document", "save_document_revision"]);
const queryCommands = new Set([
  "get_current_document",
  "get_document_history",
  "get_document_version",
]);
const legacyAuthoringCommands = new Set([
  "rename_document",
  "preview_document_restore",
  "restore_document_version",
]);

export function createTauriDocumentAuthoringTransport(
  invoke: TauriInvoke,
): LocalDesktopCommandTransport {
  return async <TData>(
    envelope: LocalDesktopCommandEnvelope,
  ): Promise<LocalDesktopCommandResponse<TData>> => {
    const request = toNativeRequest(envelope);
    const nativeCommand = nativeCommandFor(envelope.commandName);
    if (!request || !nativeCommand) return bridgeFailure();

    try {
      const response = await invoke(nativeCommand, { request });
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

function nativeCommandFor(commandName: string): string | undefined {
  if (mutationCommands.has(commandName)) return "execute_desktop_document_mutation";
  if (queryCommands.has(commandName)) return "execute_desktop_document_query";
  if (legacyAuthoringCommands.has(commandName)) return "execute_desktop_document_authoring";
  return undefined;
}

function triggersProjection(commandName: string): boolean {
  return commandName === "create_document" ||
    commandName === "rename_document" ||
    commandName === "save_document_revision" ||
    commandName === "restore_document_version";
}

function toNativeRequest(
  envelope: LocalDesktopCommandEnvelope,
): Record<string, unknown> | undefined {
  const payload = envelope.payload;
  if (envelope.commandName === "create_document" && isCreateCommand(payload)) {
    return {
      kind: "create",
      operationId: payload.operationId,
      workspaceId: payload.workspaceId,
      documentId: payload.documentId,
      body: payload.body,
      author: payload.author,
      summary: payload.summary,
    };
  }
  if (envelope.commandName === "save_document_revision" && isSaveRevisionCommand(payload)) {
    return {
      kind: "update",
      operationId: payload.operationId,
      workspaceId: payload.workspaceId,
      documentId: payload.documentId,
      expectedCurrentVersionId: payload.expectedVersionId,
      body: payload.body,
      author: payload.author,
      summary: payload.summary,
    };
  }
  if (envelope.commandName === "get_current_document" && isCurrentQuery(payload)) {
    return {
      kind: "current",
      workspaceId: payload.workspaceId,
      documentId: payload.documentId,
    };
  }
  if (envelope.commandName === "get_document_history" && isHistoryQuery(payload)) {
    return {
      kind: "history",
      workspaceId: payload.workspaceId,
      documentId: payload.documentId,
      cursor: payload.cursor,
      limit: payload.limit,
    };
  }
  if (envelope.commandName === "get_document_version" && isVersionQuery(payload)) {
    return {
      kind: "version",
      workspaceId: payload.workspaceId,
      documentId: payload.documentId,
      versionToken: payload.versionId,
    };
  }
  if (envelope.commandName === "rename_document" && isRenameCommand(payload)) {
    return {
      kind: "rename",
      workspaceId: payload.workspaceId,
      documentId: payload.documentId,
      currentVersionId: payload.currentVersionId,
      title: payload.title,
      path: payload.path,
    };
  }
  if (envelope.commandName === "preview_document_restore" && isRestorePreviewQuery(payload)) {
    return {
      kind: "preview_restore",
      workspaceId: payload.workspaceId,
      documentId: payload.documentId,
      targetVersionId: payload.targetVersionId,
    };
  }
  if (envelope.commandName === "restore_document_version" && isRestoreCommand(payload)) {
    return {
      kind: "restore",
      operationId: payload.operationId,
      workspaceId: payload.workspaceId,
      documentId: payload.documentId,
      targetVersionId: payload.targetVersionId,
      expectedCurrentVersionId: payload.expectedCurrentVersionId,
      author: payload.author,
      summary: payload.summary,
    };
  }
  return undefined;
}

type AuthoringResponseData =
  | CreateLocalDocumentResult
  | CurrentDocumentView
  | SaveDocumentRevisionResult
  | DocumentHistoryPage
  | DocumentVersionView
  | RestorePreviewResult
  | RestoreDocumentVersionResult
  | RenameLocalDocumentResult;

function mapNativeResponse(
  envelope: LocalDesktopCommandEnvelope,
  response: unknown,
): LocalDesktopCommandResponse<AuthoringResponseData> {
  if (!isRecord(response) || typeof response.ok !== "boolean") return bridgeFailure();
  if (!response.ok) return mapNativeFailure(response);
  if (!isRecord(response.data)) return bridgeFailure();

  const payload = envelope.payload;
  const data = response.data;
  if (
    envelope.commandName === "create_document" &&
    isCreateCommand(payload) &&
    data.kind === "created" &&
    typeof data.documentId === "string" &&
    typeof data.currentVersionId === "string"
  ) {
    return {
      ok: true,
      data: {
        workspaceId: payload.workspaceId,
        documentId: data.documentId,
        currentVersionId: data.currentVersionId,
      },
    };
  }
  if (
    envelope.commandName === "save_document_revision" &&
    isSaveRevisionCommand(payload) &&
    data.kind === "updated" &&
    typeof data.documentId === "string" &&
    typeof data.currentVersionId === "string"
  ) {
    return {
      ok: true,
      data: {
        status: "saved-local",
        workspaceId: payload.workspaceId,
        documentId: data.documentId,
        currentVersionId: data.currentVersionId,
        versionAppended: true,
        revision: payload.revision,
      },
    };
  }
  if (
    envelope.commandName === "get_current_document" &&
    isCurrentQuery(payload) &&
    isNativeCurrentData(data)
  ) {
    return {
      ok: true,
      data: {
        workspaceId: payload.workspaceId,
        documentId: payload.documentId,
        title: data.title,
        body: data.body,
        versionId: data.currentVersionToken,
      },
    };
  }
  if (
    envelope.commandName === "get_document_history" &&
    isHistoryQuery(payload) &&
    isNativeHistoryData(data)
  ) {
    return {
      ok: true,
      data: {
        workspaceId: payload.workspaceId,
        documentId: payload.documentId,
        entries: data.entries.map((entry) => ({
          versionId: entry.versionToken,
          revisionNumber: entry.revisionNumber,
          summary: entry.summary,
          author: entry.author,
          createdAt: entry.createdAtEpochMs === undefined
            ? ""
            : new Date(entry.createdAtEpochMs).toISOString(),
        })),
        nextCursor: data.nextCursor,
      },
    };
  }
  if (
    envelope.commandName === "get_document_version" &&
    isVersionQuery(payload) &&
    isNativeVersionData(data)
  ) {
    return {
      ok: true,
      data: {
        workspaceId: payload.workspaceId,
        documentId: payload.documentId,
        versionId: data.versionToken,
        body: data.body,
      },
    };
  }
  if (
    envelope.commandName === "rename_document" &&
    isRenameCommand(payload) &&
    data.kind === "renamed" &&
    typeof data.documentId === "string" &&
    typeof data.currentVersionId === "string" &&
    typeof data.title === "string" &&
    typeof data.path === "string"
  ) {
    return {
      ok: true,
      data: {
        workspaceId: payload.workspaceId,
        documentId: data.documentId,
        currentVersionId: data.currentVersionId,
        title: data.title,
        path: data.path,
      },
    };
  }
  if (
    envelope.commandName === "preview_document_restore" &&
    isRestorePreviewQuery(payload) &&
    isNativeRestorePreviewData(data)
  ) {
    const diff = mapNativeDocumentDiffPayload(
      data.restoreDiff,
      payload.workspaceId,
      payload.documentId,
    );
    if (!diff) return bridgeFailure();
    return {
      ok: true,
      data: {
        workspaceId: payload.workspaceId,
        documentId: data.documentId,
        targetVersionId: data.targetVersionId,
        expectedCurrentVersionId: data.expectedCurrentVersionId,
        canRestore: data.canRestore,
        missingAssetLabels: data.missingAssetLabels,
        diff,
        lines: data.lines,
      },
    };
  }
  if (
    envelope.commandName === "restore_document_version" &&
    isRestoreCommand(payload) &&
    isNativeRestoredData(data)
  ) {
    return {
      ok: true,
      data: {
        workspaceId: payload.workspaceId,
        documentId: data.documentId,
        restoredVersionId: data.restoredVersionId,
        currentVersionId: data.currentVersionId,
        revisionNumber: data.revisionNumber,
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
    "operationId",
    "workspaceId",
    "documentId",
    "body",
    "author",
    "summary",
  ]);
}

function isSaveRevisionCommand(
  value: Record<string, unknown>,
): value is SaveDocumentRevisionCommand & Record<string, unknown> {
  return hasStringFields(value, [
    "operationId",
    "workspaceId",
    "documentId",
    "body",
    "expectedVersionId",
    "author",
    "summary",
  ]) && typeof value.revision === "number" && Number.isInteger(value.revision) &&
    value.revision >= 0;
}

function isCurrentQuery(
  value: Record<string, unknown>,
): value is CurrentDocumentQuery & Record<string, unknown> {
  return hasStringFields(value, ["workspaceId", "documentId"]);
}

function isHistoryQuery(
  value: Record<string, unknown>,
): value is DocumentHistoryQuery & Record<string, unknown> {
  return hasStringFields(value, ["workspaceId", "documentId"]) &&
    (value.cursor === undefined || typeof value.cursor === "string") &&
    typeof value.limit === "number" && Number.isInteger(value.limit) && value.limit > 0 &&
    value.limit <= 100;
}

function isVersionQuery(
  value: Record<string, unknown>,
): value is DocumentVersionQuery & Record<string, unknown> {
  return hasStringFields(value, ["workspaceId", "documentId", "versionId"]);
}

function isRenameCommand(
  value: Record<string, unknown>,
): value is RenameLocalDocumentCommand & Record<string, unknown> {
  return hasStringFields(value, [
    "workspaceId",
    "documentId",
    "currentVersionId",
    "title",
    "path",
  ]);
}

function isRestorePreviewQuery(
  value: Record<string, unknown>,
): value is RestorePreviewQuery & Record<string, unknown> {
  return hasStringFields(value, [
    "workspaceId",
    "documentId",
    "targetVersionId",
  ]);
}

function isRestoreCommand(
  value: Record<string, unknown>,
): value is RestoreDocumentVersionCommand & Record<string, unknown> {
  return hasStringFields(value, [
    "workspaceId",
    "documentId",
    "operationId",
    "targetVersionId",
    "expectedCurrentVersionId",
    "author",
    "summary",
  ]);
}

function isNativeCurrentData(
  value: Record<string, unknown>,
): value is Record<string, unknown> & {
  currentVersionToken: string;
  title: string;
  body: string;
} {
  return value.kind === "current" && typeof value.currentVersionToken === "string" &&
    typeof value.title === "string" && typeof value.body === "string";
}

type NativeHistoryEntry = {
  versionToken: string;
  revisionNumber: number;
  summary: string;
  author: string;
  createdAtEpochMs?: number;
};

function isNativeHistoryData(
  value: Record<string, unknown>,
): value is Record<string, unknown> & {
  entries: NativeHistoryEntry[];
  nextCursor?: string;
} {
  return value.kind === "history" && Array.isArray(value.entries) &&
    value.entries.every(isNativeHistoryEntry) &&
    (value.nextCursor === undefined || typeof value.nextCursor === "string");
}

function isNativeHistoryEntry(value: unknown): value is NativeHistoryEntry {
  return isRecord(value) && typeof value.versionToken === "string" &&
    typeof value.revisionNumber === "number" && Number.isInteger(value.revisionNumber) &&
    typeof value.summary === "string" && typeof value.author === "string" &&
    (value.createdAtEpochMs === undefined ||
      (typeof value.createdAtEpochMs === "number" && Number.isSafeInteger(value.createdAtEpochMs) &&
        value.createdAtEpochMs >= 0));
}

function isNativeVersionData(
  value: Record<string, unknown>,
): value is Record<string, unknown> & { versionToken: string; body: string } {
  return value.kind === "version" && typeof value.versionToken === "string" &&
    typeof value.body === "string";
}

function isNativeRestorePreviewData(
  value: Record<string, unknown>,
): value is Record<string, unknown> & {
  documentId: string;
  targetVersionId: string;
  expectedCurrentVersionId: string;
  canRestore: boolean;
  missingAssetLabels: string[];
  restoreDiff: unknown;
  lines: RestorePreviewResult["lines"];
} {
  return value.kind === "restore_preview" && typeof value.documentId === "string" &&
    typeof value.targetVersionId === "string" &&
    typeof value.expectedCurrentVersionId === "string" &&
    typeof value.canRestore === "boolean" && Array.isArray(value.missingAssetLabels) &&
    value.missingAssetLabels.every((label) => typeof label === "string") &&
    value.restoreDiff !== undefined &&
    Array.isArray(value.lines);
}

function isNativeRestoredData(
  value: Record<string, unknown>,
): value is Record<string, unknown> & {
  documentId: string;
  restoredVersionId: string;
  currentVersionId: string;
  revisionNumber: number;
} {
  return value.kind === "restored" && typeof value.documentId === "string" &&
    typeof value.restoredVersionId === "string" && typeof value.currentVersionId === "string" &&
    typeof value.revisionNumber === "number" && Number.isSafeInteger(value.revisionNumber) &&
    value.revisionNumber > 0;
}

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
