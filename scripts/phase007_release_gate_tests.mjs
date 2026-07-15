import assert from "node:assert/strict";
import test from "node:test";

import {
  ReleaseGateErrorCode,
  evaluateReleaseGate,
  renderReleaseGateResult,
} from "./phase007_release_gate.mjs";

test("Phase 007 release gate rejects missing product smoke marker", () => {
  const sources = completeReleaseSources();
  sources[".tasks/phase007-product-smoke-gate-result.md"] = "phase007_product_smoke_gate=failed";

  const result = evaluateReleaseGate({ sources });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, ReleaseGateErrorCode.RequiredEvidenceMissing);
  assert.equal(result.missingEvidence[0]?.targetId, "product_smoke_prerequisite");
});

test("Phase 007 release gate rejects missing security manifest", () => {
  const sources = completeReleaseSources();
  delete sources[".tasks/release/security-log-policy-manifest.json"];

  const result = evaluateReleaseGate({ sources });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, ReleaseGateErrorCode.RequiredEvidenceMissing);
});

test("Phase 007 release gate passes complete evidence and renders safe marker", () => {
  const result = evaluateReleaseGate({ sources: completeReleaseSources() });
  const rendered = renderReleaseGateResult(result);

  assert.equal(result.passed, true);
  assert.match(rendered, /phase007_release_gate=passed/);
  assert.match(rendered, /personal local desktop/);
  assert.doesNotMatch(rendered, /raw markdown body should not leak/);
  assert.doesNotMatch(rendered, /asset binary content should not leak/);
  assert.doesNotMatch(rendered, /provider_api_key_fixture/);
});

function completeReleaseSources() {
  return {
    ".tasks/phase007-product-smoke-gate-result.md": "phase007_product_smoke_gate=passed",
    ".tasks/phase007-data-ownership-gate-result.md": "phase007_data_ownership_gate=passed",
    ".tasks/release/performance-budget-phase007.md": "phase007_performance_budget=passed",
    ".tasks/release/ai-status-result-budget-phase007.md": "phase007_ai_status_result_budget=passed",
    ".tasks/release/security-log-policy-manifest.json": [
      '"id": "phase007_release_gate_result"',
      '"path": ".tasks/phase007-release-gate-result.md"',
      '"required": true',
      '"deniedFixtures"',
      "ai_prompt_fixture",
      "provider_api_key_fixture",
    ].join("\n"),
    ".tasks/release/product-log-event-matrix.md": [
      "Product Log",
      "Field Debug Log",
      "Development Log",
      "workspace.home.ready",
      "document.saved",
      "search.failed",
      "ai.answer.failed",
      "backup.created",
      "Do not record document body, asset content, prompt, answer, token, credential, secret, or raw path.",
    ].join("\n"),
    ".tasks/release/local-desktop-runbook.md": [
      "Phase 007 Local Desktop Runbook",
      "Clean Install",
      "Home",
      "Document Authoring",
      "Search Discovery",
      "AI Provider Disabled",
      "Backup Export Import Restore",
      "Sensitive Data Exclusion",
    ].join("\n"),
    "package.json": "run:phase007-product-smoke run:phase007-release-gate run:phase007-data-ownership-gate",
  };
}
