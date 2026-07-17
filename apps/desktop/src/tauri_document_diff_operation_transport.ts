import type { DocumentDiffQuery } from "@sponzey-cabinet/client-core";

import type {
  DesktopDocumentDiffOperationClient,
  DesktopDocumentDiffOperationRemoteState,
  DesktopDocumentDiffOperationResult,
} from "./desktop_document_diff_operation_controller.ts";
import { mapNativeDocumentDiffPayload } from "./tauri_document_diff_transport.ts";
import type { TauriInvoke } from "./tauri_home_transport.ts";

const NATIVE_STATES = ["accepted", "running", "completed", "cancelled", "expired", "failed"] as const;
const PROHIBITED_KEYS = new Set(["path", "snapshotRef", "assetId", "documentBody", "content", "bytes"]);

export class DesktopDocumentDiffOperationTransportError extends Error {
  readonly code: string;
  readonly retryable: boolean;

  constructor(code: string, retryable: boolean) {
    super(code);
    this.name = "DesktopDocumentDiffOperationTransportError";
    this.code = code;
    this.retryable = retryable;
  }
}

export function createTauriDocumentDiffOperationTransport(
  invoke: TauriInvoke,
): DesktopDocumentDiffOperationClient {
  return Object.freeze({
    async start(query) {
      const response = await call(invoke, "start_desktop_document_diff_operation", toNativeRequest(query));
      return parseResponse(response, undefined, query.workspaceId, query.documentId);
    },
    async status(input) {
      const response = await call(invoke, "get_desktop_document_diff_operation_status", {
        operationToken: input.operationToken,
      });
      return parseResponse(response, input.operationToken, input.workspaceId, input.documentId);
    },
    async cancel(input) {
      const response = await call(invoke, "cancel_desktop_document_diff_operation", {
        operationToken: input.operationToken,
      });
      return parseResponse(response, input.operationToken);
    },
  });
}

async function call(
  invoke: TauriInvoke,
  command: string,
  request: Record<string, unknown>,
): Promise<Record<string, unknown>> {
  let response: unknown;
  try {
    response = await invoke(command, { request });
  } catch {
    throw bridgeFailure();
  }
  if (!isRecord(response) || hasProhibitedKey(response)) throw bridgeFailure();
  if (response.ok === false && nonEmptyString(response.errorCode) && typeof response.retryable === "boolean") {
    throw new DesktopDocumentDiffOperationTransportError(response.errorCode, response.retryable);
  }
  if (response.ok !== true || response.retryable !== false || !isRecord(response.data)) {
    throw bridgeFailure();
  }
  return response.data;
}

function parseResponse(
  data: Record<string, unknown>,
  expectedToken?: string,
  workspaceId?: string,
  documentId?: string,
): DesktopDocumentDiffOperationResult {
  if (!nonEmptyString(data.operationToken) || !isNativeState(data.state)) throw bridgeFailure();
  if (expectedToken !== undefined && data.operationToken !== expectedToken) throw bridgeFailure();
  const state = mapState(data.state);
  const failureCode = optionalString(data.failureCode);
  if (state === "Completed") {
    if (!workspaceId || !documentId) throw bridgeFailure();
    const diff = mapNativeDocumentDiffPayload(data.diff, workspaceId, documentId);
    if (!diff || diff.status !== "Complete" || failureCode !== undefined) throw bridgeFailure();
    return Object.freeze({ operationToken: data.operationToken, state, diff });
  }
  if (data.diff !== undefined) throw bridgeFailure();
  if (state === "Failed" && failureCode === undefined) throw bridgeFailure();
  if (state !== "Failed" && failureCode !== undefined) throw bridgeFailure();
  return Object.freeze({
    operationToken: data.operationToken,
    state,
    ...(failureCode === undefined ? {} : { failureCode }),
  });
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

function mapState(value: typeof NATIVE_STATES[number]): DesktopDocumentDiffOperationRemoteState {
  return ({
    accepted: "Accepted",
    running: "Running",
    completed: "Completed",
    cancelled: "Cancelled",
    expired: "Expired",
    failed: "Failed",
  } as const)[value];
}

function isNativeState(value: unknown): value is typeof NATIVE_STATES[number] {
  return NATIVE_STATES.includes(value as typeof NATIVE_STATES[number]);
}

function hasProhibitedKey(value: Record<string, unknown>): boolean {
  return Object.entries(value).some(([key, child]) => PROHIBITED_KEYS.has(key)
    || (isRecord(child) && hasProhibitedKey(child))
    || (Array.isArray(child) && child.some((item) => isRecord(item) && hasProhibitedKey(item))));
}

function optionalString(value: unknown): string | undefined {
  if (value === undefined || value === null) return undefined;
  if (!nonEmptyString(value)) throw bridgeFailure();
  return value;
}

function nonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function bridgeFailure(): DesktopDocumentDiffOperationTransportError {
  return new DesktopDocumentDiffOperationTransportError("COMMAND_BRIDGE_FAILED", false);
}
