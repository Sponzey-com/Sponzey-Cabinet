import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import { auditCurrentScopeSources } from "./phase014_current_scope_audit.mjs";
import {
  PHASE014_DOD_REQUIREMENTS,
  validatePhase014CompletionReport,
} from "./phase014_completion_evidence_gate.mjs";
import { validatePhase014CommandReceipt } from "./phase014_command_gate.mjs";

const fingerprint = "a".repeat(64);

test("final shell executes current Node Rust boundary and completion gates", async () => {
  const source = await readFile("scripts/run_phase013_final_release_gate.sh", "utf8");
  for (const command of [
    "run_phase014_desktop_test_gate.mjs",
    "run_phase014_rust_test_gate.mjs",
    "run_phase014_current_scope_audit.mjs",
    "run_phase014_completion_evidence_gate.mjs",
  ]) assert.match(source, new RegExp(command.replaceAll(".", "\\.")), command);
  assert.doesNotMatch(source, /cargo check --workspace --all-targets/);
});

test("current scope audit rejects domain IO runtime environment mutation and sensitive release text", () => {
  const result = auditCurrentScopeSources({
    domainSources: [{ path: "domain.rs", text: "use std::fs;" }],
    usecaseSources: [{ path: "usecase.rs", text: "let _ = reqwest::get(url);" }],
    runtimeSources: [{ path: "runtime.rs", text: "std::env::set_var(\"MODE\", \"test\");" }],
    releaseTexts: [{ path: "release.md", text: "body=# Private\npath=/Users/private/note.md" }],
  });
  assert.equal(result.passed, false);
  for (const id of [
    "domain_external_io",
    "usecase_external_io",
    "runtime_environment_mutation",
    "release_sensitive_content",
  ]) assert.ok(result.findingIds.includes(id), id);
});

test("current scope audit accepts clean inward dependencies and sanitized evidence", () => {
  assert.deepEqual(auditCurrentScopeSources({
    domainSources: [{ path: "domain.rs", text: "pub struct DocumentId(String);" }],
    usecaseSources: [{ path: "usecase.rs", text: "pub fn execute<P: DocumentPort>(port: &P) {}" }],
    runtimeSources: [{ path: "main.rs", text: "let args = std::env::args();" }],
    releaseTexts: [{ path: "release.md", text: "state=Passed\ndiagnostics=sanitized" }],
  }), {
    passed: true,
    findingIds: [],
    scannedFileCount: 4,
  });
});

test("command receipt requires a current successful sanitized command", () => {
  assert.deepEqual(validatePhase014CommandReceipt({
    marker: "phase014_command_gate=passed",
    state: "Passed",
    commandId: "rust-workspace-tests",
    sourceFingerprint: fingerprint,
    diagnostics: "sanitized",
  }, fingerprint, "rust-workspace-tests"), { passed: true, findingIds: [] });
  const stale = validatePhase014CommandReceipt({
    marker: "phase014_command_gate=passed",
    state: "Passed",
    commandId: "wrong",
    sourceFingerprint: "b".repeat(64),
    diagnostics: "/Users/private",
  }, fingerprint, "rust-workspace-tests");
  for (const id of ["command_id", "stale_source_fingerprint", "diagnostics"]) {
    assert.ok(stale.findingIds.includes(id), id);
  }
});

test("completion report requires every DOD requirement exactly once", () => {
  assert.deepEqual(validatePhase014CompletionReport(validReport(), fingerprint), { passed: true, findingIds: [] });
  const incomplete = validReport();
  incomplete.requirements.pop();
  incomplete.requirements.push({ ...incomplete.requirements[0] });
  incomplete.sourceFingerprint = "b".repeat(64);
  incomplete.receipts.rust.state = "Failed";
  const result = validatePhase014CompletionReport(incomplete, fingerprint);
  for (const id of [
    "stale_source_fingerprint",
    "requirement_missing",
    "requirement_duplicate",
    "receipt_failed",
  ]) assert.ok(result.findingIds.includes(id), id);
});

function validReport() {
  return {
    marker: "phase014_completion_evidence=passed",
    state: "Passed",
    sourceFingerprint: fingerprint,
    diagnostics: "sanitized",
    requirements: PHASE014_DOD_REQUIREMENTS.map((id) => ({
      id,
      state: "Passed",
      evidence: [".tasks/release/current-evidence.json"],
    })),
    receipts: {
      desktop: { state: "Passed", sourceFingerprint: fingerprint },
      rust: { state: "Passed", sourceFingerprint: fingerprint },
      boundary: { state: "Passed", sourceFingerprint: fingerprint },
      geometry: { state: "Passed" },
      responsive: { state: "Passed" },
      performance: { state: "Passed" },
      packaged: { state: "Passed", keyboardDocumentWorkflowVerified: true },
    },
  };
}
