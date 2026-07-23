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

export interface BackupManifestSummaryInput {
  readonly packageId: string;
  readonly schemaVersion: number;
  readonly createdAtEpochMs?: number;
  readonly entries: readonly BackupManifestEntryInput[];
}

export interface BackupSafetySummaryPresentation {
  readonly state: "Safe" | "NeedsBackup";
  readonly statusLabel: string;
  readonly detailLabel: string;
  readonly locationLabel: string;
  readonly contentLabel: string;
}

const REBUILDABLE_CLASSES = new Set(["graph_rebuild_metadata", "search_rebuild_metadata"]);

export function presentBackupRestorePreflight(
  schemaVersion: number,
  entries: readonly BackupManifestEntryInput[],
): Readonly<{
  compatible: boolean;
  compatibilityLabel: string;
  authoritativeRecordCount: number;
  authoritativeByteCount: number;
  rebuildableRecordCount: number;
}> {
  const authoritative = entries.filter((entry) => !REBUILDABLE_CLASSES.has(entry.dataClass));
  const rebuildable = entries.filter((entry) => REBUILDABLE_CLASSES.has(entry.dataClass));
  return Object.freeze({
    compatible: schemaVersion === 1,
    compatibilityLabel: schemaVersion === 1 ? "현재 버전과 호환됨" : "현재 버전에서 복원할 수 없음",
    authoritativeRecordCount: authoritative.reduce((total, entry) => total + Math.max(0, entry.recordCount), 0),
    authoritativeByteCount: authoritative.reduce((total, entry) => total + Math.max(0, entry.byteCount), 0),
    rebuildableRecordCount: rebuildable.reduce((total, entry) => total + Math.max(0, entry.recordCount), 0),
  });
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

export function presentBackupSafetySummary(
  manifests: readonly BackupManifestSummaryInput[],
  formatter: BackupDateFormatter,
): BackupSafetySummaryPresentation {
  const latest = [...manifests].sort((left, right) => (right.createdAtEpochMs ?? 0) - (left.createdAtEpochMs ?? 0))[0];
  if (!latest) {
    return Object.freeze({
      state: "NeedsBackup",
      statusLabel: "아직 백업이 없습니다",
      detailLabel: "새 백업을 만들어 지식 공간을 보호하세요",
      locationLabel: "이 Mac에 저장",
      contentLabel: "백업할 문서와 첨부 파일을 준비합니다",
    });
  }
  const documentCount = sumClass(latest.entries, "current_documents");
  const assetCount = sumClass(latest.entries, "asset_metadata");
  const canvasCount = sumClass(latest.entries, "canvas_records");
  return Object.freeze({
    state: "Safe",
    statusLabel: "내 지식 공간이 안전합니다",
    detailLabel: `마지막 백업 ${presentBackupCreatedAt(latest.createdAtEpochMs, formatter)}`,
    locationLabel: "이 Mac에 저장",
    contentLabel: `문서 ${documentCount}개 · 첨부 ${assetCount}개 · 캔버스 ${canvasCount}개`,
  });
}

function sumClass(entries: readonly BackupManifestEntryInput[], dataClass: string): number {
  return entries
    .filter((entry) => entry.dataClass === dataClass)
    .reduce((total, entry) => total + Math.max(0, entry.recordCount), 0);
}
