import assert from "node:assert/strict";
import test from "node:test";

import {
  DataOwnershipReportErrorCode,
  analyzeDataOwnershipEvidence,
  renderDataOwnershipReportMarkdown,
} from "./phase006_data_ownership_report.mjs";

test("data ownership report passes with local desktop ownership evidence", () => {
  const result = analyzeDataOwnershipEvidence({ sources: completeSources() });
  const markdown = renderDataOwnershipReportMarkdown(result);

  assert.equal(result.passed, true);
  assert.equal(result.marker, "phase006_data_ownership_verification=passed");
  assert.match(markdown, /phase006_data_ownership_verification=passed/);
  assert.doesNotMatch(markdown, /raw markdown body should not leak/);
  assert.doesNotMatch(markdown, /asset binary content should not leak/);
  assert.doesNotMatch(markdown, /phase005-provider-api-key-should-not-log/);
  assert.doesNotMatch(markdown, /\/Users\/example\/private/);
});

test("data ownership report fails when backup package gate is missing", () => {
  const sources = completeSources();
  sources[".tasks/phase006-backup-package-gate-result.md"] =
    "phase006_backup_package_gate=failed";

  const result = analyzeDataOwnershipEvidence({ sources });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, DataOwnershipReportErrorCode.RequiredEvidenceMissing);
  assert.equal(result.missingEvidence[0].targetId, "backup_import_export_ownership");
});

function completeSources() {
  return {
    "PROJECT.md": [
      "현재 최종 목표: 개인 사용자의 개인 PC에 설치되는 단일 사용자 지식 관리 앱",
      "개인 구축의 로컬 설정은 설치 1회로 완료되어야 한다",
      "백업/복원, import/export",
      "서버 호스팅, SaaS 형태는 차후 목표다",
    ].join("\n"),
    ".tasks/phase006-local-runtime-gate-result.md": "phase006_local_runtime_gate=passed",
    ".tasks/phase006-backup-package-gate-result.md": "phase006_backup_package_gate=passed",
    ".tasks/release/performance-budget-phase006.md": [
      "phase006_document_query_budget=passed",
      "phase006_search_graph_asset_budget=passed",
      "phase006_ai_status_result_budget=passed",
    ].join("\n"),
  };
}
