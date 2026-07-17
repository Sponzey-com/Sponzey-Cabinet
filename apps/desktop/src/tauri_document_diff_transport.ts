import type {
  DocumentDiffHunkView,
  DocumentDiffQuery,
  DocumentDiffView,
  LocalDesktopCommandEnvelope,
  LocalDesktopCommandErrorCode,
  LocalDesktopCommandResponse,
  LocalDesktopCommandTransport,
} from "@sponzey-cabinet/client-core";

import type { TauriInvoke } from "./tauri_home_transport.ts";

export function createTauriDocumentDiffTransport(
  invoke: TauriInvoke,
): LocalDesktopCommandTransport {
  return async <TData>(
    envelope: LocalDesktopCommandEnvelope,
  ): Promise<LocalDesktopCommandResponse<TData>> => {
    if (envelope.commandName !== "compare_document_versions" || !isDiffQuery(envelope.payload)) {
      return bridgeFailure();
    }
    const request = toNativeRequest(envelope.payload);
    try {
      const response = await invoke("execute_desktop_document_diff", { request });
      return mapNativeResponse(envelope.payload, response) as LocalDesktopCommandResponse<TData>;
    } catch {
      return bridgeFailure();
    }
  };
}

function toNativeRequest(query: DocumentDiffQuery): Record<string, unknown> {
  if (query.queryName === "compare-current-document-to-version") {
    return {
      kind: "current_to_version",
      workspaceId: query.workspaceId,
      documentId: query.documentId,
      versionToken: query.targetVersionId,
    };
  }
  return {
    kind: "versions",
    workspaceId: query.workspaceId,
    documentId: query.documentId,
    leftVersionToken: query.leftVersionId,
    rightVersionToken: query.rightVersionId,
  };
}

function mapNativeResponse(
  query: DocumentDiffQuery,
  response: unknown,
): LocalDesktopCommandResponse<DocumentDiffView> {
  if (!isRecord(response) || typeof response.ok !== "boolean") return bridgeFailure();
  if (!response.ok) return mapNativeFailure(response);
  const data = mapNativeDocumentDiffPayload(
    response.data,
    query.workspaceId,
    query.documentId,
  );
  if (!data) return bridgeFailure();

  return { ok: true, data };
}

export function mapNativeDocumentDiffPayload(
  value: unknown,
  workspaceId: string,
  documentId: string,
): DocumentDiffView | undefined {
  if (!isNativeDiffData(value)) return undefined;

  const data = value;
  const common = {
    workspaceId,
    documentId,
    leftVersionId: data.leftVersionToken,
    rightVersionId: data.rightVersionToken,
    addedCount: data.addedCount,
    removedCount: data.removedCount,
    attachmentDiff: mapAttachmentDiff(data.attachmentDiff),
  } as const;
  if (data.kind === "too_large") {
    return {
      ...common,
      status: "TooLarge",
      limitReason: data.limitReason,
      titleDelta: { kind: "Unchanged" },
      hunks: [],
    };
  }
  return {
    ...common,
    status: "Complete",
    titleDelta: mapTitleDelta(data.titleDelta),
    hunks: data.hunks.map(mapHunk),
  };
}

function mapHunk(value: NativeDiffHunk): DocumentDiffHunkView {
  return {
    oldStartLine: value.oldStartLine,
    newStartLine: value.newStartLine,
    addedCount: value.addedCount,
    removedCount: value.removedCount,
    lines: value.lines.map((line) => ({
      kind: line.kind === "added" ? "Added" : line.kind === "removed" ? "Removed" : "Unchanged",
      text: line.text,
      ...(line.oldLineNumber === undefined ? {} : { oldLineNumber: line.oldLineNumber }),
      ...(line.newLineNumber === undefined ? {} : { newLineNumber: line.newLineNumber }),
    })),
  };
}

function mapTitleDelta(value: NativeTitleDelta): DocumentDiffView["titleDelta"] {
  return value.kind === "changed"
    ? { kind: "Changed", before: value.before, after: value.after }
    : { kind: "Unchanged" };
}

function mapAttachmentDiff(value: NativeAttachmentDiff): DocumentDiffView["attachmentDiff"] {
  if (value.kind === "legacy_unknown") return { status: "LegacyUnknown" };
  return {
    status: "Known",
    added: value.added.map(({ label, availability }) => ({
      label,
      availability: mapAttachmentAvailability(availability),
    })),
    removed: value.removed.map(({ label, availability }) => ({
      label,
      availability: mapAttachmentAvailability(availability),
    })),
    relabeled: value.relabeled.map(({ beforeLabel, afterLabel, availability }) => ({
      beforeLabel,
      afterLabel,
      availability: mapAttachmentAvailability(availability),
    })),
    unchangedCount: value.unchangedCount,
  };
}

function mapAttachmentAvailability(value: NativeAttachmentAvailability): "Available" | "Missing" {
  return value === "available" ? "Available" : "Missing";
}

function mapNativeFailure(value: Record<string, unknown>): LocalDesktopCommandResponse<never> {
  if (
    typeof value.errorCode !== "string" ||
    typeof value.retryable !== "boolean" ||
    typeof value.repairRequired !== "boolean"
  ) return bridgeFailure();
  return {
    ok: false,
    errorCode: value.errorCode as LocalDesktopCommandErrorCode,
    retryable: value.retryable,
    repairRequired: value.repairRequired,
  };
}

type NativeTitleDelta =
  | { kind: "unchanged" }
  | { kind: "changed"; before: string; after: string };

type NativeDiffLine = {
  kind: "unchanged" | "added" | "removed";
  text: string;
  oldLineNumber?: number;
  newLineNumber?: number;
};

type NativeDiffHunk = {
  oldStartLine: number;
  newStartLine: number;
  addedCount: number;
  removedCount: number;
  lines: NativeDiffLine[];
};

type NativeAttachmentDiff =
  | {
      kind: "known";
      added: { label: string; availability: NativeAttachmentAvailability }[];
      removed: { label: string; availability: NativeAttachmentAvailability }[];
      relabeled: {
        beforeLabel: string;
        afterLabel: string;
        availability: NativeAttachmentAvailability;
      }[];
      unchangedCount: number;
    }
  | {
      kind: "legacy_unknown";
      added: [];
      removed: [];
      relabeled: [];
      unchangedCount: 0;
    };

type NativeAttachmentAvailability = "available" | "missing";

type NativeDiffData = {
  kind: "complete" | "too_large";
  leftVersionToken: string;
  rightVersionToken: string;
  limitReason?: "bytes" | "lines" | "hunks";
  addedCount: number;
  removedCount: number;
  attachmentDiff: NativeAttachmentDiff;
  titleDelta?: NativeTitleDelta;
  hunks: NativeDiffHunk[];
};

function isDiffQuery(value: Record<string, unknown>): value is DocumentDiffQuery & Record<string, unknown> {
  if (!hasStrings(value, ["queryName", "workspaceId", "documentId"])) return false;
  if (value.queryName === "compare-current-document-to-version") {
    return typeof value.targetVersionId === "string";
  }
  return value.queryName === "compare-document-versions" &&
    hasStrings(value, ["leftVersionId", "rightVersionId"]);
}

function isNativeDiffData(value: unknown): value is NativeDiffData {
  if (!isRecord(value) || !["complete", "too_large"].includes(String(value.kind))) return false;
  if (!hasStrings(value, ["leftVersionToken", "rightVersionToken"]) ||
    !isCount(value.addedCount) || !isCount(value.removedCount) ||
    !isNativeAttachmentDiff(value.attachmentDiff) || !Array.isArray(value.hunks)) {
    return false;
  }
  if (value.kind === "too_large") {
    return ["bytes", "lines", "hunks"].includes(String(value.limitReason)) && value.hunks.length === 0;
  }
  return isNativeTitleDelta(value.titleDelta) && value.hunks.every(isNativeHunk);
}

function isNativeAttachmentDiff(value: unknown): value is NativeAttachmentDiff {
  if (!isRecord(value) || !Array.isArray(value.added) || !Array.isArray(value.removed) ||
    !Array.isArray(value.relabeled) || !isCount(value.unchangedCount)) return false;
  if (value.kind === "legacy_unknown") {
    return value.added.length === 0 && value.removed.length === 0 &&
      value.relabeled.length === 0 && value.unchangedCount === 0;
  }
  return value.kind === "known" && value.added.every(isNativeAttachmentLabel) &&
    value.removed.every(isNativeAttachmentLabel) && value.relabeled.every(isNativeAttachmentRelabel);
}

function isNativeAttachmentLabel(
  value: unknown,
): value is { label: string; availability: NativeAttachmentAvailability } {
  return isRecord(value) && typeof value.label === "string" &&
    isNativeAttachmentAvailability(value.availability);
}

function isNativeAttachmentRelabel(
  value: unknown,
): value is {
  beforeLabel: string;
  afterLabel: string;
  availability: NativeAttachmentAvailability;
} {
  return isRecord(value) && typeof value.beforeLabel === "string" &&
    typeof value.afterLabel === "string" && isNativeAttachmentAvailability(value.availability);
}

function isNativeAttachmentAvailability(value: unknown): value is NativeAttachmentAvailability {
  return value === "available" || value === "missing";
}

function isNativeTitleDelta(value: unknown): value is NativeTitleDelta {
  if (!isRecord(value)) return false;
  return value.kind === "unchanged" ||
    (value.kind === "changed" && typeof value.before === "string" && typeof value.after === "string");
}

function isNativeHunk(value: unknown): value is NativeDiffHunk {
  return isRecord(value) && isCount(value.oldStartLine) && isCount(value.newStartLine) &&
    isCount(value.addedCount) && isCount(value.removedCount) && Array.isArray(value.lines) &&
    value.lines.every(isNativeLine);
}

function isNativeLine(value: unknown): value is NativeDiffLine {
  return isRecord(value) && ["unchanged", "added", "removed"].includes(String(value.kind)) &&
    typeof value.text === "string" && optionalCount(value.oldLineNumber) && optionalCount(value.newLineNumber);
}

function hasStrings(value: Record<string, unknown>, fields: readonly string[]): boolean {
  return fields.every((field) => typeof value[field] === "string");
}

function isCount(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

function optionalCount(value: unknown): boolean {
  return value === undefined || isCount(value);
}

function bridgeFailure<TData>(): LocalDesktopCommandResponse<TData> {
  return { ok: false, errorCode: "COMMAND_BRIDGE_FAILED", retryable: false, repairRequired: false };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
