import assert from "node:assert/strict";
import test from "node:test";

import { KO_KR_CATALOG, MessageCatalogError, formatBytesKoKr, formatCountKoKr, formatDateKoKr, messageKoKr } from "../src/ko_kr_catalog.ts";

test("bundled catalog exposes immutable canonical workspace terminology", () => {
  assert.equal(Object.isFrozen(KO_KR_CATALOG), true);
  assert.deepEqual([messageKoKr("route.home"), messageKoKr("route.search"), messageKoKr("route.document"), messageKoKr("route.graph"), messageKoKr("route.canvas"), messageKoKr("route.assets"), messageKoKr("route.backup")], ["홈", "검색", "문서", "지식 지도", "캔버스", "첨부 파일", "백업 및 복원"]);
  assert.equal(messageKoKr("action.save"), "저장");
  assert.equal(messageKoKr("status.saved"), "모든 변경 저장됨");
});

test("unknown catalog key fails with a stable presentation error", () => {
  assert.throws(() => messageKoKr("unknown" as never), (error: unknown) => error instanceof MessageCatalogError && error.code === "MESSAGE_KEY_UNKNOWN");
});

test("formatters are deterministic and do not read ambient locale", () => {
  assert.equal(formatCountKoKr(12, "문서"), "문서 12개");
  assert.equal(formatBytesKoKr(0), "0 B");
  assert.equal(formatBytesKoKr(1536), "1.5 KB");
  assert.equal(formatDateKoKr(Date.UTC(2026, 6, 15, 3, 4), "Asia/Seoul"), "2026. 7. 15. 12:04");
});

test("formatters reject invalid numeric and timezone inputs", () => {
  assert.throws(() => formatCountKoKr(-1, "문서"), /FORMAT_VALUE_INVALID/);
  assert.throws(() => formatBytesKoKr(Number.NaN), /FORMAT_VALUE_INVALID/);
  assert.throws(() => formatDateKoKr(0, "Invalid/Zone"), /FORMAT_TIMEZONE_INVALID/);
});
