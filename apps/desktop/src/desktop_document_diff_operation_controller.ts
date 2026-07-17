import type { DocumentDiffQuery, DocumentDiffView } from "@sponzey-cabinet/client-core";

export type DesktopDocumentDiffOperationRemoteState =
  | "Accepted" | "Running" | "Completed" | "Cancelled" | "Expired" | "Failed";

export interface DesktopDocumentDiffOperationResult {
  readonly operationToken: string;
  readonly state: DesktopDocumentDiffOperationRemoteState;
  readonly diff?: DocumentDiffView;
  readonly failureCode?: string;
}

export interface DesktopDocumentDiffOperationClient {
  start(query: DocumentDiffQuery): Promise<DesktopDocumentDiffOperationResult>;
  status(input: {
    readonly operationToken: string;
    readonly workspaceId: string;
    readonly documentId: string;
  }): Promise<DesktopDocumentDiffOperationResult>;
  cancel(input: { readonly operationToken: string }): Promise<DesktopDocumentDiffOperationResult>;
}

export type DesktopDocumentDiffOperationState =
  | "Idle" | "Accepted" | "Running" | "Ready" | "Cancelled" | "Expired" | "Failed";

export interface DesktopDocumentDiffOperationSnapshot {
  readonly state: DesktopDocumentDiffOperationState;
  readonly generation: number;
  readonly query?: DocumentDiffQuery;
  readonly operationToken?: string;
  readonly diff?: DocumentDiffView;
  readonly errorCode?: string;
  readonly retryable?: boolean;
}

export function createDesktopDocumentDiffOperationSnapshot(): DesktopDocumentDiffOperationSnapshot {
  return Object.freeze({ state: "Idle", generation: 0 });
}

export async function startDesktopDocumentDiffOperation(
  client: DesktopDocumentDiffOperationClient,
  snapshot: DesktopDocumentDiffOperationSnapshot,
  query: DocumentDiffQuery,
): Promise<DesktopDocumentDiffOperationSnapshot> {
  if (snapshot.state === "Accepted" || snapshot.state === "Running") return snapshot;
  const pending = Object.freeze({
    state: "Idle" as const,
    generation: snapshot.generation + 1,
    query,
  });
  try {
    const result = await client.start(query);
    if (result.state !== "Accepted" || !validToken(result.operationToken) || result.diff !== undefined) {
      return failed(pending, "DOCUMENT_DIFF_OPERATION_INVALID_START", false);
    }
    return Object.freeze({
      ...pending,
      state: "Accepted" as const,
      operationToken: result.operationToken,
    });
  } catch (error) {
    return failed(pending, errorCode(error), retryable(error));
  }
}

export async function pollDesktopDocumentDiffOperation(
  client: DesktopDocumentDiffOperationClient,
  snapshot: DesktopDocumentDiffOperationSnapshot,
): Promise<DesktopDocumentDiffOperationSnapshot> {
  if (!isActive(snapshot) || !snapshot.operationToken || !snapshot.query) return snapshot;
  try {
    const result = await client.status({
      operationToken: snapshot.operationToken,
      workspaceId: snapshot.query.workspaceId,
      documentId: snapshot.query.documentId,
    });
    if (result.operationToken !== snapshot.operationToken) {
      return failed(snapshot, "DOCUMENT_DIFF_OPERATION_ID_MISMATCH", false);
    }
    return mapRemoteResult(snapshot, result);
  } catch (error) {
    return failed(snapshot, errorCode(error), retryable(error));
  }
}

export async function cancelDesktopDocumentDiffOperation(
  client: DesktopDocumentDiffOperationClient,
  snapshot: DesktopDocumentDiffOperationSnapshot,
): Promise<DesktopDocumentDiffOperationSnapshot> {
  if (!isActive(snapshot) || !snapshot.operationToken) return snapshot;
  try {
    const result = await client.cancel({ operationToken: snapshot.operationToken });
    if (result.operationToken !== snapshot.operationToken) {
      return failed(snapshot, "DOCUMENT_DIFF_OPERATION_ID_MISMATCH", false);
    }
    if (result.state === "Cancelled" || result.state === "Expired") {
      return terminal(snapshot, result.state);
    }
    return failed(snapshot, "DOCUMENT_DIFF_OPERATION_CANCEL_INVALID_STATE", false);
  } catch (error) {
    return failed(snapshot, errorCode(error), retryable(error));
  }
}

export async function retryDesktopDocumentDiffOperation(
  client: DesktopDocumentDiffOperationClient,
  snapshot: DesktopDocumentDiffOperationSnapshot,
): Promise<DesktopDocumentDiffOperationSnapshot> {
  if (!snapshot.query || !["Cancelled", "Expired", "Failed"].includes(snapshot.state)) {
    return snapshot;
  }
  return startDesktopDocumentDiffOperation(client, snapshot, snapshot.query);
}

export function applyDesktopDocumentDiffOperationCandidate(
  current: DesktopDocumentDiffOperationSnapshot,
  candidate: DesktopDocumentDiffOperationSnapshot,
): DesktopDocumentDiffOperationSnapshot {
  if (candidate.generation !== current.generation) return current;
  if (current.operationToken && candidate.operationToken && current.operationToken !== candidate.operationToken) {
    return current;
  }
  return candidate;
}

function mapRemoteResult(
  snapshot: DesktopDocumentDiffOperationSnapshot,
  result: DesktopDocumentDiffOperationResult,
): DesktopDocumentDiffOperationSnapshot {
  if (result.state === "Accepted" || result.state === "Running") {
    if (result.diff !== undefined || result.failureCode !== undefined) {
      return failed(snapshot, "DOCUMENT_DIFF_OPERATION_INVALID_ACTIVE_STATE", false);
    }
    return Object.freeze({ ...snapshot, state: result.state });
  }
  if (result.state === "Completed") {
    if (!result.diff || result.diff.status !== "Complete") {
      return failed(snapshot, "DOCUMENT_DIFF_OPERATION_RESULT_MISSING", false);
    }
    return Object.freeze({
      state: "Ready" as const,
      generation: snapshot.generation,
      query: snapshot.query,
      diff: result.diff,
    });
  }
  if (result.state === "Cancelled" || result.state === "Expired") {
    return terminal(snapshot, result.state);
  }
  return failed(snapshot, result.failureCode ?? "DOCUMENT_DIFF_OPERATION_FAILED", false);
}

function terminal(
  snapshot: DesktopDocumentDiffOperationSnapshot,
  state: "Cancelled" | "Expired",
): DesktopDocumentDiffOperationSnapshot {
  return Object.freeze({
    state,
    generation: snapshot.generation,
    query: snapshot.query,
  });
}

function failed(
  snapshot: Pick<DesktopDocumentDiffOperationSnapshot, "generation" | "query">,
  code: string,
  isRetryable: boolean,
): DesktopDocumentDiffOperationSnapshot {
  return Object.freeze({
    state: "Failed",
    generation: snapshot.generation,
    query: snapshot.query,
    errorCode: code,
    retryable: isRetryable,
  });
}

function isActive(snapshot: DesktopDocumentDiffOperationSnapshot): boolean {
  return snapshot.state === "Accepted" || snapshot.state === "Running";
}

function validToken(value: string): boolean {
  return value.trim().length > 0 && !Array.from(value).some((character) => /\p{Cc}/u.test(character));
}

function errorCode(error: unknown): string {
  if (typeof error === "object" && error !== null && "code" in error && typeof error.code === "string") {
    return error.code;
  }
  return "COMMAND_BRIDGE_FAILED";
}

function retryable(error: unknown): boolean {
  return typeof error === "object" && error !== null && "retryable" in error && error.retryable === true;
}
