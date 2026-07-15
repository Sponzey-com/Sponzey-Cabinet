import assert from "node:assert/strict";
import test from "node:test";

import { presentBackupCreatedAt, presentBackupManifest } from "../src/backup_manifest_presenter.ts";

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
