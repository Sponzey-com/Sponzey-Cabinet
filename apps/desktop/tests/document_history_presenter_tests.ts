import assert from "node:assert/strict";
import test from "node:test";

import { presentDocumentHistory } from "../src/document_history_presenter.ts";

test("history presenter separates internal identity from version date and localized summary", () => {
  const presented = presentDocumentHistory([
    { versionId: "internal-v1", summary: "Created", author: "local-user", createdAt: "1721000000123" },
    { versionId: "internal-v2", summary: "Updated", author: "local-user", createdAt: "" },
    { versionId: "internal-v3", summary: "Restore internal-v1", author: "local-user", createdAt: "invalid" },
  ], (epochMs) => `날짜-${epochMs}`);

  assert.deepEqual(presented, [
    { versionId: "internal-v1", versionLabel: "버전 1", createdAtLabel: "날짜-1721000000123", summaryLabel: "문서 생성" },
    { versionId: "internal-v2", versionLabel: "버전 2", createdAtLabel: "시각 정보 없음", summaryLabel: "문서 저장" },
    { versionId: "internal-v3", versionLabel: "버전 3", createdAtLabel: "시각 정보 없음", summaryLabel: "이전 버전 복원" },
  ]);
});

test("history presenter preserves a user summary and accepts an ISO compatibility timestamp", () => {
  const presented = presentDocumentHistory([
    { versionId: "v1", summary: "회의 노트 정리", author: "local-user", createdAt: "2026-07-15T00:00:00Z" },
  ], (epochMs) => `날짜-${epochMs}`);

  assert.equal(presented[0]?.summaryLabel, "회의 노트 정리");
  assert.match(presented[0]?.createdAtLabel ?? "", /^날짜-\d+$/);
});
