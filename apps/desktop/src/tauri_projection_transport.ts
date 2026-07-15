import type { TauriInvoke } from "./tauri_home_transport.ts";

export type DesktopProjectionFreshnessState = "ready" | "stale" | "repairing" | "failed";

export interface DesktopProjectionFreshnessView {
  readonly state: DesktopProjectionFreshnessState;
  readonly currentVersionId: string;
  readonly projections: readonly {
    readonly kind: string;
    readonly state: DesktopProjectionFreshnessState;
  }[];
}

export interface DesktopProjectionReindexResult {
  readonly enqueuedCount: number;
  readonly resetCount: number;
  readonly alreadyActiveCount: number;
}

export interface DesktopProjectionWorkerResult {
  readonly readyCount: number;
  readonly retryScheduledCount: number;
  readonly failedCount: number;
}

export type DesktopProjectionRepairState = "queued" | "running" | "publishing" | "cancel_pending" | "succeeded" | "failed_retryable" | "failed_fatal" | "cancelled";
export interface DesktopProjectionRepairOperationView {
  readonly operationId: string;
  readonly state: DesktopProjectionRepairState;
  readonly attempt: number;
  readonly completedUnits: number;
  readonly totalUnits: number;
}

export interface DesktopProjectionTransport {
  getFreshness(workspaceId: string, documentId: string): Promise<DesktopProjectionFreshnessView>;
  requestReindex(workspaceId: string, documentId: string): Promise<DesktopProjectionReindexResult>;
  runWorker(): Promise<DesktopProjectionWorkerResult>;
  startRepair(workspaceId: string, documentId: string): Promise<DesktopProjectionRepairOperationView>;
  runRepair(workspaceId: string, operationId: string): Promise<DesktopProjectionRepairOperationView>;
  getRepairStatus(workspaceId: string, operationId: string): Promise<DesktopProjectionRepairOperationView>;
}

export class DesktopProjectionTransportError extends Error {
  readonly code: string;
  readonly retryable: boolean;

  constructor(code: string, retryable: boolean) {
    super(code);
    this.name = "DesktopProjectionTransportError";
    this.code = code;
    this.retryable = retryable;
  }
}

export function createTauriProjectionTransport(invoke: TauriInvoke): DesktopProjectionTransport {
  return Object.freeze({
    async getFreshness(workspaceId: string, documentId: string) {
      const response = await invokeIdentity(invoke, "get_desktop_projection_freshness", workspaceId, documentId);
      if (!isFreshnessResponse(response)) throw responseError(response);
      return Object.freeze({
        state: response.state,
        currentVersionId: response.currentVersionId,
        projections: Object.freeze(response.projections.map((item) => Object.freeze({ ...item }))),
      });
    },
    async requestReindex(workspaceId: string, documentId: string) {
      const response = await invokeIdentity(invoke, "request_desktop_projection_reindex", workspaceId, documentId);
      if (!isReindexResponse(response)) throw responseError(response);
      return Object.freeze({
        enqueuedCount: response.enqueuedCount,
        resetCount: response.resetCount,
        alreadyActiveCount: response.alreadyActiveCount,
      });
    },
    async runWorker() {
      let response: unknown;
      try {
        response = await invoke("run_desktop_projection_worker");
      } catch {
        throw new DesktopProjectionTransportError("COMMAND_BRIDGE_FAILED", false);
      }
      if (!isWorkerResponse(response)) throw responseError(response);
      return Object.freeze({
        readyCount: response.readyCount,
        retryScheduledCount: response.retryScheduledCount,
        failedCount: response.failedCount,
      });
    },
    async startRepair(workspaceId: string, documentId: string) {
      return invokeRepair(invoke, "start_desktop_projection_repair", { workspaceId, documentId });
    },
    async runRepair(workspaceId: string, operationId: string) {
      return invokeRepair(invoke, "run_desktop_projection_repair_operation", { workspaceId, operationId });
    },
    async getRepairStatus(workspaceId: string, operationId: string) {
      return invokeRepair(invoke, "get_desktop_projection_repair_status", { workspaceId, operationId });
    },
  });
}

async function invokeRepair(invoke: TauriInvoke, command: string, request: Record<string, string>): Promise<DesktopProjectionRepairOperationView> {
  let response: unknown;
  try { response = await invoke(command, { request }); }
  catch { throw new DesktopProjectionTransportError("COMMAND_BRIDGE_FAILED", false); }
  if (!isRepairResponse(response)) throw responseError(response);
  return Object.freeze({ operationId: response.operationId, state: response.state, attempt: response.attempt, completedUnits: response.completedUnits, totalUnits: response.totalUnits });
}

async function invokeIdentity(
  invoke: TauriInvoke,
  command: string,
  workspaceId: string,
  documentId: string,
): Promise<unknown> {
  try {
    return await invoke(command, { request: { workspaceId, documentId } });
  } catch {
    throw new DesktopProjectionTransportError("COMMAND_BRIDGE_FAILED", false);
  }
}

function responseError(value: unknown): DesktopProjectionTransportError {
  if (isRecord(value) && value.ok === false && typeof value.errorCode === "string" && typeof value.retryable === "boolean") {
    return new DesktopProjectionTransportError(value.errorCode, value.retryable);
  }
  return new DesktopProjectionTransportError("COMMAND_BRIDGE_FAILED", false);
}

function isFreshnessResponse(value: unknown): value is {
  readonly ok: true;
  readonly state: DesktopProjectionFreshnessState;
  readonly currentVersionId: string;
  readonly projections: readonly { readonly kind: string; readonly state: DesktopProjectionFreshnessState }[];
} {
  return isRecord(value) && value.ok === true && isFreshnessState(value.state)
    && typeof value.currentVersionId === "string" && Array.isArray(value.projections)
    && value.projections.every((item) => isRecord(item) && typeof item.kind === "string" && isFreshnessState(item.state));
}

function isReindexResponse(value: unknown): value is { readonly ok: true; readonly enqueuedCount: number; readonly resetCount: number; readonly alreadyActiveCount: number } {
  return isRecord(value) && value.ok === true
    && [value.enqueuedCount, value.resetCount, value.alreadyActiveCount].every(isNonNegativeInteger);
}

function isWorkerResponse(value: unknown): value is { readonly ok: true; readonly readyCount: number; readonly retryScheduledCount: number; readonly failedCount: number } {
  return isRecord(value) && value.ok === true
    && [value.readyCount, value.retryScheduledCount, value.failedCount].every(isNonNegativeInteger);
}

function isRepairResponse(value: unknown): value is { readonly ok: true; readonly operationId: string; readonly state: DesktopProjectionRepairState; readonly attempt: number; readonly completedUnits: number; readonly totalUnits: number } {
  return isRecord(value) && value.ok === true && typeof value.operationId === "string" && isRepairState(value.state)
    && [value.attempt, value.completedUnits, value.totalUnits].every(isNonNegativeInteger)
    && value.totalUnits > 0 && value.completedUnits <= value.totalUnits;
}

function isRepairState(value: unknown): value is DesktopProjectionRepairState {
  return ["queued", "running", "publishing", "cancel_pending", "succeeded", "failed_retryable", "failed_fatal", "cancelled"].includes(String(value));
}

function isFreshnessState(value: unknown): value is DesktopProjectionFreshnessState {
  return value === "ready" || value === "stale" || value === "repairing" || value === "failed";
}

function isNonNegativeInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
