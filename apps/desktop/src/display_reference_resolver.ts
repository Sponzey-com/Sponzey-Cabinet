export type DisplayReferenceCategory = "document" | "graphNode" | "canvas" | "asset" | "version";
export type DisplayReferenceState = "resolved" | "stale" | "missing";

export interface DisplayReferenceRequest {
  readonly category: DisplayReferenceCategory;
  readonly identity: string;
}

export interface DisplayProjectionEntry extends DisplayReferenceRequest {
  readonly title: string;
  readonly breadcrumb?: readonly string[];
  readonly freshness: "ready" | "stale";
}

export interface DisplayProjectionPort {
  resolveBatch(requests: readonly DisplayReferenceRequest[]): Promise<readonly DisplayProjectionEntry[]>;
}

export interface DisplayFallbackPolicy {
  readonly emptyTitle: Readonly<Record<DisplayReferenceCategory, string>>;
  readonly missingTitle: Readonly<Record<DisplayReferenceCategory, string>>;
  readonly staleStatus: string;
  readonly missingStatus: string;
}

export interface DisplayReference extends DisplayReferenceRequest {
  readonly label: string;
  readonly breadcrumbLabel: string;
  readonly statusLabel: string;
  readonly state: DisplayReferenceState;
}

export function createKoKrDisplayFallbackPolicy(): DisplayFallbackPolicy {
  return Object.freeze({
    emptyTitle: Object.freeze({
      document: "제목 없는 문서",
      graphNode: "이름 없는 항목",
      canvas: "제목 없는 캔버스",
      asset: "이름 없는 첨부 파일",
      version: "이름 없는 변경 이력",
    }),
    missingTitle: Object.freeze({
      document: "찾을 수 없는 문서",
      graphNode: "찾을 수 없는 항목",
      canvas: "찾을 수 없는 캔버스",
      asset: "찾을 수 없는 첨부 파일",
      version: "찾을 수 없는 변경 이력",
    }),
    staleStatus: "최신 정보 확인 필요",
    missingStatus: "대상을 찾을 수 없습니다",
  });
}

export async function resolveDisplayReferences(
  port: DisplayProjectionPort,
  requests: readonly DisplayReferenceRequest[],
  policy: DisplayFallbackPolicy,
): Promise<readonly DisplayReference[]> {
  validateRequests(requests);
  if (requests.length === 0) return Object.freeze([]);

  const unique = deduplicate(requests);
  const requestedKeys = new Set(unique.map(requestKey));
  const entries = await port.resolveBatch(unique);
  const entryByKey = new Map<string, DisplayProjectionEntry>();
  for (const entry of entries) {
    const key = requestKey(entry);
    if (requestedKeys.has(key) && !entryByKey.has(key)) entryByKey.set(key, entry);
  }

  return Object.freeze(requests.map((request) => Object.freeze(
    presentReference(request, entryByKey.get(requestKey(request)), policy),
  )));
}

function presentReference(
  request: DisplayReferenceRequest,
  entry: DisplayProjectionEntry | undefined,
  policy: DisplayFallbackPolicy,
): DisplayReference {
  if (!entry) {
    return {
      ...request,
      label: policy.missingTitle[request.category],
      breadcrumbLabel: "",
      statusLabel: policy.missingStatus,
      state: "missing",
    };
  }
  const safeTitle = visibleText(entry.title, request.identity) || policy.emptyTitle[request.category];
  const breadcrumbLabel = (entry.breadcrumb ?? [])
    .map((segment) => visibleText(segment, request.identity))
    .filter(Boolean)
    .join(" / ");
  return {
    ...request,
    label: safeTitle,
    breadcrumbLabel,
    statusLabel: entry.freshness === "stale" ? policy.staleStatus : "",
    state: entry.freshness === "stale" ? "stale" : "resolved",
  };
}

function validateRequests(requests: readonly DisplayReferenceRequest[]): void {
  const categories: readonly DisplayReferenceCategory[] = ["document", "graphNode", "canvas", "asset", "version"];
  for (const request of requests) {
    if (!categories.includes(request.category) || !request.identity.trim()) {
      throw new Error("DISPLAY_IDENTITY_INVALID");
    }
  }
}

function deduplicate(requests: readonly DisplayReferenceRequest[]): readonly DisplayReferenceRequest[] {
  const seen = new Set<string>();
  return Object.freeze(requests.filter((request) => {
    const key = requestKey(request);
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  }).map((request) => Object.freeze({ ...request })));
}

function requestKey(request: DisplayReferenceRequest): string {
  return `${request.category}\u0000${request.identity}`;
}

function visibleText(value: string, identity: string): string {
  const text = value.trim();
  return text === identity ? "" : text;
}
