import type { DocumentHistoryEntry } from "@sponzey-cabinet/client-core";

export interface DocumentHistoryEntryDisplay {
  readonly versionId: string;
  readonly versionLabel: string;
  readonly createdAtLabel: string;
  readonly summaryLabel: string;
}

export type DocumentHistoryDateFormatter = (epochMs: number) => string;

export function presentDocumentHistory(
  entries: readonly DocumentHistoryEntry[],
  formatDate: DocumentHistoryDateFormatter,
): readonly DocumentHistoryEntryDisplay[] {
  return entries.map((entry, index) => ({
    versionId: entry.versionId,
    versionLabel: `버전 ${presentRevisionNumber(entry.revisionNumber, index)}`,
    createdAtLabel: presentCreatedAt(entry.createdAt, formatDate),
    summaryLabel: presentSummary(entry.summary),
  }));
}

function presentRevisionNumber(revisionNumber: number | undefined, index: number): number {
  return Number.isSafeInteger(revisionNumber) && (revisionNumber ?? 0) > 0
    ? revisionNumber as number
    : index + 1;
}

export function createKoKrHistoryDateFormatter(): DocumentHistoryDateFormatter {
  const formatter = new Intl.DateTimeFormat("ko-KR", {
    dateStyle: "medium",
    timeStyle: "short",
  });
  return (epochMs) => formatter.format(new Date(epochMs));
}

function presentCreatedAt(value: string, formatDate: DocumentHistoryDateFormatter): string {
  const trimmed = value.trim();
  const epochMs = /^\d+$/.test(trimmed) ? Number(trimmed) : Date.parse(trimmed);
  if (!Number.isFinite(epochMs) || epochMs <= 0) return "시각 정보 없음";
  try {
    return formatDate(epochMs);
  } catch {
    return "시각 정보 없음";
  }
}

function presentSummary(value: string): string {
  const summary = value.trim();
  if (/^Created(?: document)?$/i.test(summary)) return "문서 생성";
  if (/^(?:Updated|Saved document)$/i.test(summary)) return "문서 저장";
  if (/^Restore\b/i.test(summary)) return "이전 버전 복원";
  return summary || "변경 내용 없음";
}
