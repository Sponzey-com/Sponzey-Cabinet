import assert from "node:assert/strict";
import test from "node:test";

import {
  ProductSmokeGateErrorCode,
  evaluateProductSmokeGate,
  renderProductSmokeGateResult,
} from "./phase007_product_smoke_gate.mjs";

test("Phase 007 product smoke gate rejects missing lower marker", () => {
  const sources = completeProductSmokeSources();
  sources[".tasks/phase007-discovery-gate-result.md"] = "phase007_discovery_gate=failed";

  const result = evaluateProductSmokeGate({ sources });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, ProductSmokeGateErrorCode.RequiredEvidenceMissing);
  assert.equal(result.missingEvidence[0]?.targetId, "phase007_lower_gates");
});

test("Phase 007 product smoke gate rejects missing document preview evidence", () => {
  const sources = completeProductSmokeSources();
  sources["packages/ui/tests/document_authoring_preview_model_tests.ts"] = "missing table evidence";

  const result = evaluateProductSmokeGate({ sources });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, ProductSmokeGateErrorCode.RequiredEvidenceMissing);
});

test("Phase 007 product smoke gate passes complete evidence and renders safe marker", () => {
  const result = evaluateProductSmokeGate({ sources: completeProductSmokeSources() });
  const rendered = renderProductSmokeGateResult(result);

  assert.equal(result.passed, true);
  assert.match(rendered, /phase007_product_smoke_gate=passed/);
  assert.match(rendered, /desktop_daily_workspace_flow/);
  assert.doesNotMatch(rendered, /raw markdown body should not leak/);
  assert.doesNotMatch(rendered, /asset binary content should not leak/);
  assert.doesNotMatch(rendered, /provider_api_key_fixture/);
});

function completeProductSmokeSources() {
  return {
    ".tasks/phase007-plan-validation-result.md": "phase007_plan_validation=passed",
    ".tasks/phase007-workspace-home-gate-result.md": "phase007_workspace_home_gate=passed",
    ".tasks/phase007-document-authoring-gate-result.md": "phase007_document_authoring_gate=passed",
    ".tasks/phase007-local-persistence-gate-result.md": "phase007_local_persistence_gate=passed",
    ".tasks/phase007-discovery-gate-result.md": "phase007_discovery_gate=passed",
    ".tasks/phase007-ai-assistant-gate-result.md": "phase007_ai_assistant_gate=passed",
    ".tasks/phase007-data-ownership-gate-result.md": "phase007_data_ownership_gate=passed",
    ".tasks/release/performance-budget-phase007.md": "phase007_performance_budget=passed",
    ".tasks/release/ai-status-result-budget-phase007.md": "phase007_ai_status_result_budget=passed",
    "packages/ui/tests/personal_workspace_home_model_tests.ts": "recent-documents quick-search ai-entry backup-status workspace-health",
    "packages/ui/tests/document_authoring_preview_model_tests.ts": "| 항목 | 내용 | heading anchor callout missing asset source/preview/split",
    "apps/desktop/tests/desktop_local_persistence_flow_tests.ts": "save current document version append current read history list",
    "apps/desktop/tests/desktop_discovery_smoke_tests.ts": "desktop local discovery smoke graph smoke canvas smoke",
    "apps/desktop/tests/desktop_ai_local_ux_smoke_tests.ts": "desktop AI local UX smoke provider setup optional",
    "apps/desktop/tests/desktop_backup_restore_staging_smoke_tests.ts": "desktop backup settings smoke restore staging smoke confirmation",
    "apps/desktop/tests/desktop_import_preview_smoke_tests.ts": "desktop import preview smoke markdown obsidian conflict",
    "package.json": "run:phase007-product-smoke run:phase007-release-gate",
  };
}
