export type UserFacingRecoveryAction = "retry" | "recover" | "none";
export type UserFacingErrorContext = "workspace_home" | "navigator" | "canvas" | "authoring" | "backup";

export interface MapUserFacingErrorInput {
  readonly stableCode: string;
  readonly retryable: boolean;
  readonly operationContext: UserFacingErrorContext;
  readonly correlationReference?: string;
}

export interface UserFacingError {
  readonly title: string;
  readonly message: string;
  readonly recoveryAction: UserFacingRecoveryAction;
  readonly recoveryLabel?: string;
  readonly diagnosticReference: string;
  readonly mapping: "known" | "unknown";
}

interface ErrorCopy {
  readonly title: string;
  readonly message: string;
  readonly recoveryAction?: UserFacingRecoveryAction;
}

const ERROR_COPY: Readonly<Record<string, ErrorCopy>> = Object.freeze({
  WORKSPACE_HOME_PROJECTION_UNAVAILABLE: Object.freeze({
    title: "작업 공간을 열 수 없습니다",
    message: "로컬 작업 공간 정보를 불러오지 못했습니다.",
  }),
  DOCUMENT_NAVIGATOR_PROJECTION_UNAVAILABLE: Object.freeze({
    title: "문서 목록을 열 수 없습니다",
    message: "문서 목록과 검색 정보를 불러오지 못했습니다.",
  }),
  DOCUMENT_NAVIGATOR_INVALID_QUERY: Object.freeze({
    title: "검색 조건을 확인해 주세요",
    message: "입력한 조건으로 문서를 검색할 수 없습니다.",
  }),
  CANVAS_RECOVERY_REQUIRED: Object.freeze({
    title: "캔버스 복구가 필요합니다",
    message: "마지막으로 정상 저장된 캔버스 상태를 확인해 복구할 수 있습니다.",
    recoveryAction: "recover",
  }),
  CANVAS_RECOVERY_NO_VALID_REVISION: Object.freeze({
    title: "캔버스를 자동으로 복구할 수 없습니다",
    message: "사용 가능한 이전 저장 상태를 찾지 못했습니다.",
    recoveryAction: "none",
  }),
  COMMAND_BRIDGE_FAILED: Object.freeze({
    title: "작업을 완료할 수 없습니다",
    message: "로컬 앱과 연결하는 중 문제가 발생했습니다.",
  }),
  DOCUMENT_AUTHORING_STORAGE_UNAVAILABLE: Object.freeze({
    title: "문서를 저장하지 못했습니다",
    message: "로컬 저장소에 변경 내용을 기록하지 못했습니다.",
  }),
  DOCUMENT_AUTHORING_POINTER_UPDATE_FAILED: Object.freeze({
    title: "읽기 전용 복구가 필요합니다",
    message: "저장된 문서는 보존됐지만 현재 버전을 확정하지 못했습니다.",
    recoveryAction: "none",
  }),
});

const CONTEXT_COPY: Readonly<Record<UserFacingErrorContext, ErrorCopy>> = Object.freeze({
  workspace_home: Object.freeze({ title: "작업 공간을 열 수 없습니다", message: "로컬 작업 공간을 불러오는 중 문제가 발생했습니다." }),
  navigator: Object.freeze({ title: "문서 목록을 열 수 없습니다", message: "문서 목록을 불러오는 중 문제가 발생했습니다." }),
  canvas: Object.freeze({ title: "캔버스를 열 수 없습니다", message: "캔버스를 불러오는 중 문제가 발생했습니다." }),
  authoring: Object.freeze({ title: "문서 작업을 완료할 수 없습니다", message: "문서를 처리하는 중 문제가 발생했습니다." }),
  backup: Object.freeze({ title: "백업 작업을 완료할 수 없습니다", message: "백업 데이터를 처리하는 중 문제가 발생했습니다." }),
});

export function mapUserFacingError(input: MapUserFacingErrorInput): UserFacingError {
  const known = ERROR_COPY[input.stableCode];
  const copy = known ?? CONTEXT_COPY[input.operationContext];
  const recoveryAction = copy.recoveryAction ?? (input.retryable ? "retry" : "none");
  const recoveryLabel = recoveryAction === "retry" ? "다시 시도"
    : recoveryAction === "recover" ? "복구" : undefined;
  return Object.freeze({
    title: copy.title,
    message: copy.message,
    recoveryAction,
    ...(recoveryLabel ? { recoveryLabel } : {}),
    diagnosticReference: diagnosticReference(input.correlationReference),
    mapping: known ? "known" : "unknown",
  });
}

function diagnosticReference(value: string | undefined): string {
  if (value && /^[a-zA-Z0-9][a-zA-Z0-9._:-]{0,63}$/.test(value)) return value;
  const source = value ?? "unavailable";
  let hash = 0x811c9dc5;
  for (let index = 0; index < source.length; index += 1) {
    hash ^= source.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193);
  }
  return `ref-${(hash >>> 0).toString(16).padStart(8, "0")}`;
}
