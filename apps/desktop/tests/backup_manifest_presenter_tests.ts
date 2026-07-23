import assert from "node:assert/strict";
import test from "node:test";

import { presentBackupCreatedAt, presentBackupManifest, presentBackupRestorePreflight, presentBackupSafetySummary } from "../src/backup_manifest_presenter.ts";

test("restore preflight separates replaced data from rebuildable projections", () => {
  const entries = [
    { dataClass: "current_documents", recordCount: 2, byteCount: 20 },
    { dataClass: "version_history", recordCount: 3, byteCount: 30 },
    { dataClass: "graph_rebuild_metadata", recordCount: 1, byteCount: 10 },
    { dataClass: "search_rebuild_metadata", recordCount: 1, byteCount: 10 },
  ];
  assert.deepEqual(presentBackupRestorePreflight(1, entries), {
    compatible: true,
    compatibilityLabel: "현재 버전과 호환됨",
    authoritativeRecordCount: 5,
    authoritativeByteCount: 50,
    rebuildableRecordCount: 2,
  });
  assert.equal(presentBackupRestorePreflight(2, entries).compatible, false);
});

test("backup manifest presenter labels every durable class and totals records and bytes", () => {
  const dataClasses = [
    "current_documents", "version_history", "canvas_records", "asset_metadata",
    "asset_objects", "asset_associations", "graph_rebuild_metadata", "search_rebuild_metadata",
  ] as const;
  const result = presentBackupManifest(dataClasses.map((dataClass, index) => ({
    dataClass,
    recordCount: index + 1,
    byteCount: (index + 1) * 10,
  })));
  assert.equal(result.totalRecordCount, 36);
  assert.equal(result.totalByteCount, 360);
  assert.equal(result.entries.length, 8);
  assert.equal(result.entries.some((entry) => entry.label === "기타 데이터"), false);
  assert.deepEqual(result.entries.map((entry) => entry.label), [
    "현재 문서", "문서 이력", "캔버스", "첨부 정보",
    "첨부 원본", "첨부 연결", "관계 재구성 정보", "검색 재구성 정보",
  ]);
});

test("backup manifest presenter hides unknown internal class behind a safe fallback", () => {
  const result = presentBackupManifest([{ dataClass: "future_private_class", recordCount: 1, byteCount: 2 }]);
  assert.equal(result.entries[0]?.label, "기타 데이터");
  assert.doesNotMatch(result.entries[0]?.label ?? "", /future_private_class/);
});

test("backup creation time presenter formats known time and labels legacy metadata", () => {
  const formatter = { format: (date: Date) => `formatted:${date.getTime()}` };
  assert.equal(presentBackupCreatedAt(1_784_064_000_000, formatter), "formatted:1784064000000");
  assert.equal(presentBackupCreatedAt(undefined, formatter), "시각 정보 없음");
});

test("backup safety summary presents latest local backup without internal identity", () => {
  const formatter = { format: () => "2026. 7. 16. 오전 6:20" };
  const summary = presentBackupSafetySummary([
    {
      packageId: "internal-old-package",
      schemaVersion: 1,
      createdAtEpochMs: 1_784_064_000_000,
      entries: [{ dataClass: "current_documents", recordCount: 1, byteCount: 10 }],
    },
    {
      packageId: "internal-new-package",
      schemaVersion: 1,
      createdAtEpochMs: 1_784_150_400_000,
      entries: [
        { dataClass: "current_documents", recordCount: 4, byteCount: 40 },
        { dataClass: "asset_metadata", recordCount: 3, byteCount: 30 },
        { dataClass: "canvas_records", recordCount: 2, byteCount: 20 },
        { dataClass: "search_rebuild_metadata", recordCount: 99, byteCount: 1 },
      ],
    },
  ], formatter);

  assert.deepEqual(summary, {
    state: "Safe",
    statusLabel: "내 지식 공간이 안전합니다",
    detailLabel: "마지막 백업 2026. 7. 16. 오전 6:20",
    locationLabel: "이 Mac에 저장",
    contentLabel: "문서 4개 · 첨부 3개 · 캔버스 2개",
  });
  assert.doesNotMatch(JSON.stringify(summary), /internal-new-package|1784150400000|search_rebuild_metadata|\/Users/);
});

test("backup safety summary guides creation when no backup exists", () => {
  const summary = presentBackupSafetySummary([], { format: () => "must-not-render" });
  assert.deepEqual(summary, {
    state: "NeedsBackup",
    statusLabel: "아직 백업이 없습니다",
    detailLabel: "새 백업을 만들어 지식 공간을 보호하세요",
    locationLabel: "이 Mac에 저장",
    contentLabel: "백업할 문서와 첨부 파일을 준비합니다",
  });
});
