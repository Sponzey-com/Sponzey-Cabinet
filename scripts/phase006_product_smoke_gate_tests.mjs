import assert from "node:assert/strict";
import test from "node:test";

import {
  ProductSmokeGateErrorCode,
  ProductSmokeGateEvent,
  ProductSmokeGateState,
  analyzeProductSmokeEvidence,
  renderProductSmokeGateMarkdown,
  transitionProductSmokeGateState,
} from "./phase006_product_smoke_gate.mjs";

test("product smoke gate reports complete evidence as passed", () => {
  const result = analyzeProductSmokeEvidence({ sources: completeSources() });
  const markdown = renderProductSmokeGateMarkdown(result);

  assert.equal(result.passed, true);
  assert.equal(result.marker, "phase006_product_smoke_gate=passed");
  assert.match(markdown, /phase006_product_smoke_gate=passed/);
  assert.doesNotMatch(markdown, /raw markdown body should not leak/);
  assert.doesNotMatch(markdown, /asset binary content should not leak/);
  assert.doesNotMatch(markdown, /phase005-provider-api-key-should-not-log/);
  assert.doesNotMatch(markdown, /\/Users\/example\/private/);
});

test("product smoke gate fails when a lower gate marker is missing", () => {
  const sources = completeSources();
  sources[".tasks/phase006-ai-ux-gate-result.md"] = "phase006_ai_ux_gate=failed";

  const result = analyzeProductSmokeEvidence({ sources });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, ProductSmokeGateErrorCode.RequiredEvidenceMissing);
  assert.equal(result.missingEvidence[0].targetId, "lower_phase006_gates");
});

test("product smoke gate fails when data ownership report is missing", () => {
  const sources = completeSources();
  sources[".tasks/release/data-ownership-verification.md"] =
    "phase006_data_ownership_verification=failed";

  const result = analyzeProductSmokeEvidence({ sources });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, ProductSmokeGateErrorCode.RequiredEvidenceMissing);
  assert.equal(result.missingEvidence[0].targetId, "data_ownership_report");
});

test("product smoke gate fails when performance budget marker is missing", () => {
  const sources = completeSources();
  sources[".tasks/release/performance-budget-phase006.md"] =
    "phase006_document_query_budget=passed";

  const result = analyzeProductSmokeEvidence({ sources });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, ProductSmokeGateErrorCode.RequiredEvidenceMissing);
  assert.equal(result.missingEvidence[0].targetId, "phase006_performance_budgets");
});

test("product smoke gate state machine exposes explicit transitions", () => {
  const reading = transitionProductSmokeGateState(ProductSmokeGateState.Pending, ProductSmokeGateEvent.Start);
  const validating = transitionProductSmokeGateState(reading.state, ProductSmokeGateEvent.SourcesLoaded);
  const writing = transitionProductSmokeGateState(validating.state, ProductSmokeGateEvent.EvidenceValidated);
  const passed = transitionProductSmokeGateState(writing.state, ProductSmokeGateEvent.ReportWritten);
  const invalid = transitionProductSmokeGateState(ProductSmokeGateState.Pending, ProductSmokeGateEvent.ReportWritten);

  assert.equal(reading.state, ProductSmokeGateState.ReadingSources);
  assert.equal(validating.state, ProductSmokeGateState.ValidatingEvidence);
  assert.equal(writing.state, ProductSmokeGateState.WritingReport);
  assert.equal(passed.state, ProductSmokeGateState.Passed);
  assert.equal(invalid.errorCode, ProductSmokeGateErrorCode.InvalidTransition);
});

function completeSources() {
  return {
    ".tasks/phase006-plan-validation-result.md": "phase006_plan_validation=passed",
    ".tasks/phase006-local-runtime-gate-result.md": "phase006_local_runtime_gate=passed",
    ".tasks/phase006-workspace-shell-gate-result.md": "phase006_workspace_shell_gate=passed",
    ".tasks/phase006-document-ux-gate-result.md": "phase006_document_ux_gate=passed",
    ".tasks/phase006-search-graph-asset-gate-result.md": "phase006_search_graph_asset_gate=passed",
    ".tasks/phase006-ai-ux-gate-result.md": "phase006_ai_ux_gate=passed",
    ".tasks/phase006-backup-package-gate-result.md": "phase006_backup_package_gate=passed",
    ".tasks/release/data-ownership-verification.md": "phase006_data_ownership_verification=passed",
    ".tasks/release/performance-budget-phase006.md": [
      "phase006_document_query_budget=passed",
      "phase006_search_graph_asset_budget=passed",
      "phase006_ai_status_result_budget=passed",
    ].join("\n"),
    "apps/desktop/tests/desktop_personal_workspace_shell_tests.ts": "desktop current product shell",
    "apps/desktop/tests/desktop_document_ux_smoke_tests.ts": "desktop document UX smoke",
    "apps/desktop/tests/desktop_discovery_smoke_tests.ts": "desktop local discovery smoke",
    "apps/desktop/tests/desktop_ai_local_ux_smoke_tests.ts": "desktop AI local UX smoke",
    "apps/desktop/tests/desktop_backup_restore_staging_smoke_tests.ts": "desktop backup restore smoke",
    "apps/desktop/tests/desktop_import_preview_smoke_tests.ts": "desktop import preview smoke",
    ".tasks/release/local-desktop-runbook.md": [
      "clean install",
      "startup repair",
      "Index Rebuild",
      "backup",
      "restore",
    ].join("\n"),
  };
}
