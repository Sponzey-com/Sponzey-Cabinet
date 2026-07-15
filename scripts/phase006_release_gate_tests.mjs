import assert from "node:assert/strict";
import test from "node:test";

import {
  ReleaseGateErrorCode,
  ReleaseGateEvent,
  ReleaseGateState,
  analyzeReleaseEvidence,
  renderReleaseGateMarkdown,
  transitionReleaseGateState,
} from "./phase006_release_gate.mjs";

test("final release gate reports complete evidence as passed", () => {
  const result = analyzeReleaseEvidence({ sources: completeSources() });
  const markdown = renderReleaseGateMarkdown(result);

  assert.equal(result.passed, true);
  assert.equal(result.marker, "phase006_release_gate=passed");
  assert.match(markdown, /phase006_release_gate=passed/);
  assert.doesNotMatch(markdown, /raw markdown body should not leak/);
  assert.doesNotMatch(markdown, /asset binary content should not leak/);
  assert.doesNotMatch(markdown, /phase005-provider-api-key-should-not-log/);
  assert.doesNotMatch(markdown, /\/Users\/example\/private/);
});

test("final release gate fails when product smoke is missing", () => {
  const sources = completeSources();
  sources[".tasks/phase006-product-smoke-gate-result.md"] =
    "phase006_product_smoke_gate=failed";

  const result = analyzeReleaseEvidence({ sources });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, ReleaseGateErrorCode.RequiredEvidenceMissing);
  assert.equal(result.missingEvidence[0].targetId, "product_smoke_prerequisite");
});

test("final release gate fails when security manifest target is not required", () => {
  const sources = completeSources();
  sources[".tasks/release/security-log-policy-manifest.json"] =
    '"id": "phase006_final_release_gate_result", "path": ".tasks/phase006-release-gate-result.md", "required": false';

  const result = analyzeReleaseEvidence({ sources });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, ReleaseGateErrorCode.RequiredEvidenceMissing);
  assert.equal(result.missingEvidence[0].targetId, "security_manifest_final_targets");
});

test("final release gate fails when ownership or performance evidence is missing", () => {
  const sources = completeSources();
  sources[".tasks/release/data-ownership-verification.md"] =
    "phase006_data_ownership_verification=failed";

  const result = analyzeReleaseEvidence({ sources });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, ReleaseGateErrorCode.RequiredEvidenceMissing);
  assert.equal(result.missingEvidence[0].targetId, "ownership_and_performance_evidence");
});

test("final release gate state machine exposes explicit transitions", () => {
  const reading = transitionReleaseGateState(ReleaseGateState.Pending, ReleaseGateEvent.Start);
  const validating = transitionReleaseGateState(reading.state, ReleaseGateEvent.SourcesLoaded);
  const writing = transitionReleaseGateState(validating.state, ReleaseGateEvent.EvidenceValidated);
  const passed = transitionReleaseGateState(writing.state, ReleaseGateEvent.ReportWritten);
  const invalid = transitionReleaseGateState(ReleaseGateState.Pending, ReleaseGateEvent.ReportWritten);

  assert.equal(reading.state, ReleaseGateState.ReadingSources);
  assert.equal(validating.state, ReleaseGateState.ValidatingEvidence);
  assert.equal(writing.state, ReleaseGateState.WritingReport);
  assert.equal(passed.state, ReleaseGateState.Passed);
  assert.equal(invalid.errorCode, ReleaseGateErrorCode.InvalidTransition);
});

function completeSources() {
  return {
    ".tasks/phase006-product-smoke-gate-result.md": "phase006_product_smoke_gate=passed",
    ".tasks/phase006-backup-package-gate-result.md": "phase006_backup_package_gate=passed",
    ".tasks/phase006-ai-ux-gate-result.md": "phase006_ai_ux_gate=passed",
    ".tasks/release/data-ownership-verification.md": "phase006_data_ownership_verification=passed",
    ".tasks/release/performance-budget-phase006.md": [
      "phase006_document_query_budget=passed",
      "phase006_search_graph_asset_budget=passed",
      "phase006_ai_status_result_budget=passed",
    ].join("\n"),
    ".tasks/release/local-desktop-runbook.md": [
      "Clean Install",
      "Startup Repair",
      "Index Rebuild",
      "Read-Only Recovery",
      "Sensitive Data Exclusion",
    ].join("\n"),
    ".tasks/release/security-log-policy-manifest.json": [
      '"id": "phase006_final_release_gate_result"',
      '"path": ".tasks/phase006-release-gate-result.md"',
      '"required": true',
      '"id": "phase006_product_smoke_gate_result"',
      '"id": "phase006_data_ownership_verification"',
    ].join("\n"),
    "package.json": [
      "run:phase006-release-gate-tests",
      "run:phase006-release-gate",
      "run:security-log-scanner",
      "run:runbook-validator",
    ].join("\n"),
  };
}
