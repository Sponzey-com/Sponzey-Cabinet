export const KO_KR_CATALOG = Object.freeze({
  "route.home": "홈",
  "route.search": "검색",
  "route.document": "문서",
  "route.graph": "지식 지도",
  "route.canvas": "캔버스",
  "route.assets": "첨부 파일",
  "route.backup": "백업 및 복원",
  "routeContext.home": "개인 지식 공간",
  "routeContext.search": "검색 및 발견",
  "routeContext.document": "작성 및 검토",
  "routeContext.graph": "문서 관계 탐색",
  "routeContext.canvas": "시각적 지식 구성",
  "routeContext.assets": "파일 및 연결 관리",
  "routeContext.backup": "로컬 데이터 보호",
  "shell.brand": "Cabinet",
  "shell.local": "로컬",
  "shell.newDocument": "새 문서",
  "shell.cabinet": "내 캐비닛",
  "shell.cabinetStorage": "이 PC에 안전하게 저장됨",
  "shell.navigationLabel": "주요 메뉴",
  "shell.documentTreeLabel": "문서",
  "shell.gettingStarted": "시작하기",
  "shell.welcomeDocument": "Cabinet에 오신 것을 환영해요",
  "shell.projects": "프로젝트",
  "shell.reading": "읽을거리",
  "shell.searchPrompt": "검색",
  "shell.searchPlaceholder": "검색어를 입력하세요",
  "shell.searchUnavailable": "아래 검색 입력을 사용하세요",
  "shell.saved": "모든 변경 저장됨",
  "document.emptyTitle": "문서가 없습니다",
  "document.emptyDescription": "최근 문서를 열거나 새 문서를 만드세요.",
  "action.save": "저장",
  "action.createDocument": "새 문서 만들기",
  "action.retry": "다시 시도",
  "action.cancel": "취소",
  "action.discard": "변경 취소",
  "status.saved": "모든 변경 저장됨",
  "status.loading": "불러오는 중",
  "status.failed": "작업을 완료할 수 없습니다",
  "history.empty": "저장된 이력이 없습니다.",
  "history.loadMore": "이전 이력 더 보기",
  "history.loadingMore": "불러오는 중",
  "history.loadMoreFailed": "이전 이력을 불러오지 못했습니다.",
  "history.compareSelected": "선택한 두 버전 비교",
  "history.previousWindow": "더 최근 이력",
  "history.nextWindow": "더 오래된 이력",
  "diff.attachmentsHeading": "첨부 파일 변경",
  "diff.attachmentsAdded": "추가됨",
  "diff.attachmentsRemoved": "제거됨",
  "diff.attachmentsRelabeled": "이름 변경",
  "diff.attachmentsUnchanged": "변경 없음",
  "diff.attachmentsNone": "첨부 파일 변경 없음",
  "diff.attachmentsLegacyUnknown": "과거 형식으로 저장된 버전이라 첨부 파일 변경을 확인할 수 없습니다.",
  "diff.attachmentMissing": "파일을 찾을 수 없음",
  "diff.backgroundAccepted": "큰 문서 비교를 준비하고 있습니다.",
  "diff.backgroundRunning": "큰 문서 변경 내용을 비교하고 있습니다.",
  "diff.backgroundCancel": "비교 취소",
  "diff.backgroundCancelled": "문서 비교를 취소했습니다.",
  "diff.backgroundExpired": "앱이 다시 시작되어 문서를 다시 비교해야 합니다.",
  "diff.backgroundRetry": "다시 비교",
  "restore.preview": "복원 미리보기",
  "restore.previewing": "복원할 변경 내용을 확인하고 있습니다.",
  "restore.review": "복원 내용 검토",
  "restore.confirmHeading": "복원 전 변경 내용 확인",
  "restore.confirm": "복원 확인",
  "restore.cancelConfirmation": "검토 닫기",
  "restore.apply": "이 버전으로 복원",
  "restore.applying": "복원하는 중",
  "restore.completed": "복원이 완료되었습니다.",
  "restore.conflict": "문서가 변경되어 기존 미리보기로 복원할 수 없습니다.",
  "restore.refreshPreview": "미리보기 새로고침",
  "restore.recoveryRequired": "문서는 복원되었지만 일부 화면 정보를 다시 정리해야 합니다.",
  "restore.continueRecovery": "복구 계속",
  "restore.missingAsset": "이 버전에 연결된 파일을 찾을 수 없어 복원할 수 없습니다.",
  "restore.largeDiffBlocked": "전체 변경 내용을 확인할 수 없어 복원할 수 없습니다.",
  "restore.failed": "복원을 완료할 수 없습니다. 문서 내용은 변경되지 않았습니다.",
} as const);

export type KoKrMessageKey = keyof typeof KO_KR_CATALOG;

export interface MessageCatalog {
  readonly message: (key: KoKrMessageKey) => string;
}

export class MessageCatalogError extends Error {
  readonly code = "MESSAGE_KEY_UNKNOWN";
  constructor() { super("MESSAGE_KEY_UNKNOWN"); this.name = "MessageCatalogError"; }
}

export function messageKoKr(key: KoKrMessageKey): string {
  const message = KO_KR_CATALOG[key];
  if (typeof message !== "string") throw new MessageCatalogError();
  return message;
}

export const KO_KR_MESSAGES: MessageCatalog = Object.freeze({ message: messageKoKr });

export function formatCountKoKr(value: number, noun: string): string {
  validateNonNegative(value);
  if (!noun.trim()) throw new Error("FORMAT_NOUN_INVALID");
  return `${noun} ${Math.trunc(value).toLocaleString("ko-KR")}개`;
}

export function formatHistoryRangeKoKr(start: number, endExclusive: number, total: number): string {
  validateNonNegative(start);
  validateNonNegative(endExclusive);
  validateNonNegative(total);
  if (start > endExclusive || endExclusive > total) throw new Error("FORMAT_RANGE_INVALID");
  const first = total === 0 ? 0 : Math.trunc(start) + 1;
  return `이력 ${first}-${Math.trunc(endExclusive)} / 전체 ${Math.trunc(total)}개`;
}

export function formatBytesKoKr(value: number): string {
  validateNonNegative(value);
  if (value < 1024) return `${Math.trunc(value)} B`;
  if (value < 1024 * 1024) return `${trimDecimal(value / 1024)} KB`;
  if (value < 1024 * 1024 * 1024) return `${trimDecimal(value / (1024 * 1024))} MB`;
  return `${trimDecimal(value / (1024 * 1024 * 1024))} GB`;
}

export function formatDateKoKr(timestampMs: number, timeZone: string): string {
  if (!Number.isFinite(timestampMs)) throw new Error("FORMAT_VALUE_INVALID");
  let parts: Intl.DateTimeFormatPart[];
  try {
    parts = new Intl.DateTimeFormat("ko-KR", { timeZone, year: "numeric", month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit", hourCycle: "h23" }).formatToParts(new Date(timestampMs));
  } catch {
    throw new Error("FORMAT_TIMEZONE_INVALID");
  }
  const value = (type: Intl.DateTimeFormatPartTypes) => parts.find((part) => part.type === type)?.value ?? "";
  return `${value("year")}. ${Number(value("month"))}. ${Number(value("day"))}. ${value("hour")}:${value("minute")}`;
}

function validateNonNegative(value: number): void {
  if (!Number.isFinite(value) || value < 0) throw new Error("FORMAT_VALUE_INVALID");
}

function trimDecimal(value: number): string {
  return (Math.round(value * 10) / 10).toFixed(1).replace(/\.0$/, "");
}
