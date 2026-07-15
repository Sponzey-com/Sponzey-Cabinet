export interface BackupManifestEntryInput {
  readonly dataClass: string;
  readonly recordCount: number;
  readonly byteCount: number;
}

export interface BackupManifestEntryPresentation extends BackupManifestEntryInput {
  readonly label: string;
}

export interface BackupManifestPresentation {
  readonly entries: readonly BackupManifestEntryPresentation[];
  readonly totalRecordCount: number;
  readonly totalByteCount: number;
}

const DATA_CLASS_LABELS: Readonly<Record<string, string>> = Object.freeze({
  current_documents: "현재 문서",
  version_history: "문서 이력",
  canvas_records: "캔버스",
  asset_metadata: "첨부 정보",
  asset_objects: "첨부 원본",
  asset_associations: "첨부 연결",
  graph_rebuild_metadata: "관계 재구성 정보",
  search_rebuild_metadata: "검색 재구성 정보",
});

export interface BackupDateFormatter {
  format(date: Date): string;
}

export function createKoKrBackupDateFormatter(): BackupDateFormatter {
  return new Intl.DateTimeFormat("ko-KR", { dateStyle: "medium", timeStyle: "short" });
}

export function presentBackupCreatedAt(
  createdAtEpochMs: number | undefined,
  formatter: BackupDateFormatter,
): string {
  if (createdAtEpochMs === undefined) return "시각 정보 없음";
  return formatter.format(new Date(createdAtEpochMs));
}

export function presentBackupManifest(entries: readonly BackupManifestEntryInput[]): BackupManifestPresentation {
  const presented = entries.map((entry) => Object.freeze({
    ...entry,
    label: DATA_CLASS_LABELS[entry.dataClass] ?? "기타 데이터",
  }));
  return Object.freeze({
    entries: Object.freeze(presented),
    totalRecordCount: entries.reduce((total, entry) => total + Math.max(0, entry.recordCount), 0),
    totalByteCount: entries.reduce((total, entry) => total + Math.max(0, entry.byteCount), 0),
  });
}
