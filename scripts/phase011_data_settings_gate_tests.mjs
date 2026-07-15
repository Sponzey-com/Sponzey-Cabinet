import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import test from "node:test";

import {
  DataSettingsGateErrorCode,
  analyzeDataSettingsGateEvidence,
  renderDataSettingsGateMarkdown,
  runDataSettingsGate,
} from "./phase011_data_settings_gate.mjs";

test("phase011 data settings gate rejects missing evidence", () => {
  const result = analyzeDataSettingsGateEvidence({
    sourceFingerprint: "a".repeat(64),
    sources: {
      ".tasks/phase011-discovery-gate-result.md": "phase011_discovery_gate=passed",
    },
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, DataSettingsGateErrorCode.RequiredEvidenceMissing);
});

test("phase011 data settings gate passes complete sanitized evidence", () => {
  const result = analyzeDataSettingsGateEvidence({
    sourceFingerprint: "b".repeat(64),
    sources: completeSources(),
  });
  const artifact = renderDataSettingsGateMarkdown(result);

  assert.equal(result.passed, true);
  assert.match(artifact, /phase011_data_settings_gate=passed/);
  assert.match(artifact, /field_debug_scope_expiry_reason_masking_required=true/);
  assert.match(artifact, /future_server_admin_settings_excluded=true/);
  assert.doesNotMatch(artifact, /provider_api_key_fixture/);
  assert.doesNotMatch(artifact, /\/Users\/example\/private/);
});

test("phase011 data settings gate writes marker under explicit root", async () => {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase011-data-settings-"));
  await mkdir(join(root, ".tasks"), { recursive: true });
  for (const [filePath, text] of Object.entries(completeSources())) {
    await mkdir(join(root, filePath.split("/").slice(0, -1).join("/")), { recursive: true });
    await writeFile(join(root, filePath), text);
  }
  await writeFile(
    join(root, ".tasks/phase011-current-implementation-inventory.md"),
    `source_fingerprint=${"c".repeat(64)}\n`,
  );

  const result = await runDataSettingsGate({ root });

  assert.equal(result.passed, true);
  assert.equal(result.sourceFingerprint, "c".repeat(64));
});

function completeSources() {
  return {
    ".tasks/phase011-discovery-gate-result.md": "phase011_discovery_gate=passed",
    "packages/ui/src/index.ts": [
      "createDataOwnershipSettingsModel",
      "createFieldDebugSettingsModel",
    ].join("\n"),
    "packages/ui/tests/data_ownership_settings_model_tests.ts": [
      "data ownership settings exposes local personal sections",
      "field debug settings require scope expiry reason and masking before activation",
    ].join("\n"),
    "packages/ui/tests/backup_restore_staging_model_tests.ts": "backup settings uses platform default path",
    "packages/ui/tests/import_preview_model_tests.ts": "import preview summary supports markdown folder without raw local data",
    "packages/ui/tests/ai_citation_tool_scope_model_tests.ts": "AI provider settings model is optional and excludes credentials",
    "apps/desktop/src/index.ts": [
      "createDesktopBackupSettings",
      "createDesktopRestoreStagingValidation",
      "createDesktopImportPreview",
    ].join("\n"),
    "apps/desktop/tests/desktop_backup_restore_staging_smoke_tests.ts": "desktop backup settings smoke uses install-once defaults",
    "apps/desktop/tests/desktop_import_preview_smoke_tests.ts": "desktop import preview smoke supports markdown and obsidian without raw paths",
    "crates/cabinet-domain/tests/field_debug_tests.rs": [
      "field_debug_session_rejects_approval_without_scope_or_ttl",
      "field_debug_scope_and_ttl_reject_missing_or_sensitive_values",
    ].join("\n"),
    "crates/cabinet-usecases/tests/field_debug_usecase_tests.rs": [
      "request_and_approve_activate_field_debug_session_with_product_logs",
      "approve_field_debug_session_rejects_missing_scope_or_ttl_before_activation",
    ].join("\n"),
  };
}
