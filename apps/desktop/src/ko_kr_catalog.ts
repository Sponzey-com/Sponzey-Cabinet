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
  "action.save": "저장",
  "action.retry": "다시 시도",
  "action.cancel": "취소",
  "action.discard": "변경 취소",
  "status.saved": "모든 변경 저장됨",
  "status.loading": "불러오는 중",
  "status.failed": "작업을 완료할 수 없습니다",
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
