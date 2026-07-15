import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import test from "node:test";

import {
  RecoveryObservabilityGateErrorCode,
  analyzeRecoveryObservabilityEvidence,
  renderRecoveryObservabilityGateMarkdown,
  renderRunbook,
  renderSecurityManifest,
  runRecoveryObservabilityGate,
} from "./phase011_recovery_observability_gate.mjs";

test("phase011 recovery observability gate rejects missing evidence", () => {
  const result = analyzeRecoveryObservabilityEvidence({
    sourceFingerprint: "a".repeat(64),
    sources: {
      ".tasks/phase011-data-settings-gate-result.md": "phase011_data_settings_gate=passed",
    },
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, RecoveryObservabilityGateErrorCode.RequiredEvidenceMissing);
});

test("phase011 recovery observability gate renders sanitized marker manifest and runbook", () => {
  const result = analyzeRecoveryObservabilityEvidence({
    sourceFingerprint: "b".repeat(64),
    sources: completeSources(),
  });
  const artifact = renderRecoveryObservabilityGateMarkdown(result);
  const manifest = renderSecurityManifest("b".repeat(64));
  const runbook = renderRunbook("b".repeat(64));

  assert.equal(result.passed, true);
  assert.match(artifact, /phase011_recovery_observability_gate=passed/);
  assert.match(manifest, /phase011_security_log_manifest=passed/);
  assert.match(runbook, /phase011_runbook=passed/);
  assert.match(runbook, /Startup Repair/);
  assert.match(runbook, /Field Debug Activation/);
  assert.doesNotMatch(artifact, /raw markdown body should not leak/i);
  assert.doesNotMatch(manifest, /provider_api_key_fixture/i);
  assert.doesNotMatch(runbook, /\/Users\/example\/private/i);
});

test("phase011 recovery observability gate writes release artifacts", async () => {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase011-recovery-"));
  await mkdir(join(root, ".tasks/release"), { recursive: true });
  for (const [filePath, text] of Object.entries(completeSources())) {
    await mkdir(join(root, filePath.split("/").slice(0, -1).join("/")), { recursive: true });
    await writeFile(join(root, filePath), text);
  }
  await writeFile(
    join(root, ".tasks/phase011-current-implementation-inventory.md"),
    `source_fingerprint=${"c".repeat(64)}\n`,
  );

  const result = await runRecoveryObservabilityGate({ root });
  const runbook = await readFile(join(root, ".tasks/release/local-desktop-runbook-phase011.md"), "utf8");

  assert.equal(result.passed, true);
  assert.match(runbook, /phase011_runbook=passed/);
});

function completeSources() {
  return {
    ".tasks/phase011-data-settings-gate-result.md": "phase011_data_settings_gate=passed",
    "packages/ui/src/index.ts": "createRecoveryActionPanelModel ReadOnlyRecovery",
    "packages/ui/tests/recovery_observability_model_tests.ts": "recovery action panel maps local failures to safe user actions",
    "packages/ui/tests/revision_safe_save_coordinator_tests.ts": "ReadOnlyRecovery",
    "apps/desktop/tests/desktop_document_authoring_controller_tests.ts": "authoring controller exposes retry close discard and repair-required read-only recovery",
    "crates/cabinet-platform/tests/startup_repair_smoke.rs": "startup_repair_smoke_rebuilds_corrupted_indexes_without_losing_current_workspace_data",
    "crates/cabinet-usecases/tests/backup_usecase_tests.rs": "restore_failure_preserves_workspace_current_data_and_logs_safe_failure",
    "crates/cabinet-usecases/tests/import_markdown_folder_tests.rs": "import_markdown_folder_continues_after_duplicate_entry_as_partial_failure",
    "crates/cabinet-usecases/tests/field_debug_usecase_tests.rs": "field_debug_diagnostic_writes_only_for_active_session_and_sanitized_fields",
    "scripts/security_log_scanner.mjs": "Product Log Field Debug Log Development Log",
    "scripts/security_log_scanner_tests.mjs": "Product Log Field Debug Log Development Log",
    "scripts/runbook_validator.mjs": "runbook validator rejects forbidden manual env edit",
    "scripts/runbook_validator_tests.mjs": "runbook validator rejects forbidden manual env edit",
  };
}
