import type { DesktopAssetImportStatus } from "./tauri_asset_import_transport.ts";

export type AttachmentWireState = DesktopAssetImportStatus["state"];
export type AttachmentFileStage =
  | "Selected" | "Validating" | "Staging" | "Hashing" | "PublishingObject"
  | "PersistingMetadata" | "PreparingRevision" | "Associating" | "Projecting"
  | "Verifying" | "Cancelling" | "Completed" | "Cancelled" | "Failed"
  | "Conflict" | "RecoveryRequired";

export interface AttachmentStagePresentation {
  readonly stage: AttachmentFileStage;
  readonly label: string;
  readonly terminal: boolean;
  readonly canCancel: boolean;
  readonly canRetry: boolean;
  readonly canRepair: boolean;
  readonly canStartNewAttempt: boolean;
  readonly errorCode?: string;
}

export interface AttachmentFileSnapshot extends AttachmentStagePresentation {
  readonly generation: number;
  readonly operationId: string;
  readonly displayName: string;
  readonly byteSize: number;
  readonly progressPercent: number;
  readonly userLabel: string;
  readonly assetId?: string;
}

export class AttachmentStatusPresentationError extends Error {
  readonly code = "ATTACHMENT_STATUS_UNSUPPORTED";

  constructor() {
    super("ATTACHMENT_STATUS_UNSUPPORTED");
    this.name = "AttachmentStatusPresentationError";
  }
}

export function presentAttachmentWireState(state: string): AttachmentStagePresentation {
  switch (state as AttachmentWireState) {
    case "selected": return active("Selected", "파일 선택됨", true);
    case "validating": return active("Validating", "파일 확인 중", true);
    case "staging": return active("Staging", "안전한 임시 저장 중", true);
    case "hashing": return active("Hashing", "파일 식별 중", true);
    case "publishing_object": return active("PublishingObject", "파일 저장 중", true);
    case "persisting_metadata": return active("PersistingMetadata", "파일 정보 저장 중", true);
    case "preparing_revision": return active("PreparingRevision", "문서 버전 준비 중", true);
    case "linking":
    case "associating": return active("Associating", "문서에 연결 중", false);
    case "projecting": return active("Projecting", "문서 관계 갱신 중", false);
    case "verifying": return active("Verifying", "저장 결과 확인 중", false);
    case "cancelling": return active("Cancelling", "취소 정리 중", false);
    case "completed": return terminal("Completed", "첨부 완료");
    case "cancelled": return terminal("Cancelled", "첨부 취소됨");
    case "validation_failed": return newAttemptFailure("ATTACHMENT_VALIDATION_FAILED");
    case "staging_failed": return newAttemptFailure("ATTACHMENT_STAGING_FAILED");
    case "object_publish_failed": return newAttemptFailure("ATTACHMENT_OBJECT_PUBLISH_FAILED");
    case "failed": return newAttemptFailure("ATTACHMENT_OPERATION_FAILED");
    case "metadata_persist_failed": return recovery("ATTACHMENT_METADATA_RECONCILIATION_REQUIRED");
    case "link_failed": return recovery("ATTACHMENT_ASSOCIATION_RECOVERY_REQUIRED");
    case "cleanup_required": return recovery("ATTACHMENT_CLEANUP_REQUIRED");
    case "recovery_required": return recovery("ATTACHMENT_RECOVERY_REQUIRED");
    case "conflict": return Object.freeze({
      stage: "Conflict", label: "문서 변경 충돌", terminal: false,
      canCancel: false, canRetry: true, canRepair: false, canStartNewAttempt: false,
      errorCode: "DOCUMENT_CURRENT_CONFLICT",
    });
    default: throw new AttachmentStatusPresentationError();
  }
}

export function sanitizeAttachmentDisplayName(value: string, maxLength = 120): string {
  const segments = value.replaceAll("\\", "/").split("/");
  const basename = segments.at(-1)?.replace(/[\u0000-\u001f\u007f]/g, "").trim() ?? "";
  if (!basename) return "첨부 파일";
  return basename.slice(0, Math.max(1, maxLength));
}

export function createAttachmentFileSnapshot(input: {
  readonly generation: number;
  readonly operationId: string;
  readonly fileName: string;
  readonly byteSize: number;
  readonly state: AttachmentWireState;
}): AttachmentFileSnapshot {
  const presentation = presentAttachmentWireState(input.state);
  const displayName = sanitizeAttachmentDisplayName(input.fileName);
  return Object.freeze({
    ...presentation,
    generation: input.generation,
    operationId: input.operationId,
    displayName,
    byteSize: Math.max(0, Math.trunc(input.byteSize)),
    progressPercent: presentation.stage === "Completed" ? 100 : 0,
    userLabel: `${displayName} · ${presentation.label}`,
  });
}

export function applyAttachmentFileStatus(
  current: AttachmentFileSnapshot,
  update: {
    readonly generation: number;
    readonly operationId: string;
    readonly state: AttachmentWireState;
    readonly completedBytes?: number;
    readonly totalBytes?: number;
    readonly assetId?: string;
    readonly errorCode?: string;
  },
): AttachmentFileSnapshot {
  if (update.generation !== current.generation || update.operationId !== current.operationId) return current;
  const next = presentAttachmentWireState(update.state);
  if (current.terminal) return current;
  const explicitRepairStart = current.stage === "RecoveryRequired" && next.stage === "Projecting";
  if (!explicitRepairStart && stageRank(next.stage) < stageRank(current.stage)) return current;
  const progressPercent = progress(update.completedBytes, update.totalBytes, next.stage, current.progressPercent);
  const { errorCode: _previousErrorCode, ...currentWithoutError } = current;
  return Object.freeze({
    ...currentWithoutError,
    ...next,
    ...(update.errorCode ? { errorCode: update.errorCode } : {}),
    ...(update.assetId ? { assetId: update.assetId } : {}),
    progressPercent,
    userLabel: `${current.displayName} · ${next.label}`,
  });
}

function active(stage: AttachmentFileStage, label: string, canCancel: boolean): AttachmentStagePresentation {
  return Object.freeze({ stage, label, terminal: false, canCancel, canRetry: false, canRepair: false, canStartNewAttempt: false });
}

function terminal(stage: "Completed" | "Cancelled", label: string): AttachmentStagePresentation {
  return Object.freeze({ stage, label, terminal: true, canCancel: false, canRetry: false, canRepair: false, canStartNewAttempt: false });
}

function newAttemptFailure(errorCode: string): AttachmentStagePresentation {
  return Object.freeze({ stage: "Failed", label: "첨부 실패", terminal: true, canCancel: false, canRetry: false, canRepair: false, canStartNewAttempt: true, errorCode });
}

function recovery(errorCode: string): AttachmentStagePresentation {
  return Object.freeze({ stage: "RecoveryRequired", label: "복구 필요", terminal: false, canCancel: false, canRetry: false, canRepair: true, canStartNewAttempt: false, errorCode });
}

function progress(completed: number | undefined, total: number | undefined, stage: AttachmentFileStage, fallback: number): number {
  if (stage === "Completed") return 100;
  if (completed === undefined || total === undefined || total <= 0 || completed < 0) return fallback;
  return Math.max(fallback, Math.min(99, Math.floor((completed / total) * 100)));
}

function stageRank(stage: AttachmentFileStage): number {
  return ({
    Selected: 0, Validating: 1, Staging: 2, Hashing: 3, PublishingObject: 4,
    PersistingMetadata: 5, PreparingRevision: 6, Associating: 7, Projecting: 8,
    Verifying: 9, Cancelling: 10, Completed: 11, Cancelled: 11, Failed: 11,
    Conflict: 11, RecoveryRequired: 11,
  } satisfies Record<AttachmentFileStage, number>)[stage];
}
