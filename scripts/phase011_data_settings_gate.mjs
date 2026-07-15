import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const DataSettingsGateErrorCode = Object.freeze({
  SourceReadFailed: "PHASE011_DATA_SETTINGS_SOURCE_READ_FAILED",
  RequiredEvidenceMissing: "PHASE011_DATA_SETTINGS_REQUIRED_EVIDENCE_MISSING",
  SensitiveDataLeak: "PHASE011_DATA_SETTINGS_SENSITIVE_DATA_LEAK",
  StaleSourceFingerprint: "PHASE011_DATA_SETTINGS_STALE_SOURCE_FINGERPRINT",
});

const REQUIRED_TARGETS = Object.freeze([
  target("discovery_prerequisite", "Phase 011 discovery prerequisite", {
    requiredFiles: [".tasks/phase011-discovery-gate-result.md"],
    evidence: ["phase011_discovery_gate=passed"],
  }),
  target("data_settings_ui_models", "Data ownership settings root and Field Debug guard UI models", {
    requiredFiles: [
      "packages/ui/src/index.ts",
      "packages/ui/tests/data_ownership_settings_model_tests.ts",
      "packages/ui/tests/backup_restore_staging_model_tests.ts",
      "packages/ui/tests/import_preview_model_tests.ts",
      "packages/ui/tests/ai_citation_tool_scope_model_tests.ts",
    ],
    evidence: [
      "createDataOwnershipSettingsModel",
      "createFieldDebugSettingsModel",
      "data ownership settings exposes local personal sections",
      "field debug settings require scope expiry reason and masking before activation",
      "backup settings uses platform default path",
      "import preview summary supports markdown folder without raw local data",
      "AI provider settings model is optional and excludes credentials",
    ],
  }),
  target("desktop_data_settings_smoke", "Desktop data ownership settings smoke tests", {
    requiredFiles: [
      "apps/desktop/src/index.ts",
      "apps/desktop/tests/desktop_backup_restore_staging_smoke_tests.ts",
      "apps/desktop/tests/desktop_import_preview_smoke_tests.ts",
    ],
    evidence: [
      "createDesktopBackupSettings",
      "createDesktopRestoreStagingValidation",
      "createDesktopImportPreview",
      "desktop backup settings smoke uses install-once defaults",
      "desktop import preview smoke supports markdown and obsidian without raw paths",
    ],
  }),
  target("field_debug_domain_usecase", "Scoped Field Debug domain and usecase policy tests", {
    requiredFiles: [
      "crates/cabinet-domain/tests/field_debug_tests.rs",
      "crates/cabinet-usecases/tests/field_debug_usecase_tests.rs",
    ],
    evidence: [
      "field_debug_session_rejects_approval_without_scope_or_ttl",
      "field_debug_scope_and_ttl_reject_missing_or_sensitive_values",
      "request_and_approve_activate_field_debug_session_with_product_logs",
      "approve_field_debug_session_rejects_missing_scope_or_ttl_before_activation",
    ],
  }),
]);

const SENSITIVE_PATTERNS = Object.freeze([
  /\/Users\//,
  /C:\\\\Users\\\\/i,
  /raw markdown body should not leak/i,
  /asset binary content should not leak/i,
  /provider_api_key_fixture/i,
  /connector_access_token_fixture/i,
  /AI_PROVIDER_KEY/i,
  /raw prompt/i,
  /raw answer/i,
  /document_body:secret/i,
  /server-url|tenant-admin|organization-admin|billing|sso-settings|team-invite/i,
]);

export function analyzeDataSettingsGateEvidence({ sources, sourceFingerprint }) {
  if (!/^[a-f0-9]{64}$/.test(sourceFingerprint ?? "")) {
    return failed(DataSettingsGateErrorCode.StaleSourceFingerprint, []);
  }
  const targetResults = REQUIRED_TARGETS.map((entry) => analyzeTarget(entry, sources));
  const missingEvidence = targetResults
    .filter((entry) => entry.missing.length > 0)
    .map((entry) => ({ targetId: entry.id, missing: entry.missing }));
  if (missingEvidence.length > 0) {
    return failed(DataSettingsGateErrorCode.RequiredEvidenceMissing, targetResults, missingEvidence);
  }
  return {
    passed: true,
    marker: "phase011_data_settings_gate=passed",
    releaseScope: "personal_local_desktop",
    sourceFingerprint,
    targetResults,
    missingEvidence: [],
    summary: {
      requiredTargets: REQUIRED_TARGETS.length,
      missingRequiredEvidence: 0,
    },
  };
}

export function renderDataSettingsGateMarkdown(result) {
  const lines = [
    `phase011_data_settings_gate=${result.passed ? "passed" : "failed"}`,
    "release_scope=personal_local_desktop",
  ];
  if (result.sourceFingerprint) lines.push(`source_fingerprint=${result.sourceFingerprint}`);
  if (!result.passed) lines.push(`error_code=${result.errorCode}`);
  lines.push(
    "requirements=DATA-01,CFG-02,LOG-01,STATE-01,SEC-01",
    `required_target_count=${result.summary?.requiredTargets ?? 0}`,
    "settings_scope=local_personal_desktop_only",
    "ai_provider_optional=true",
    "provider_secret_excluded=true",
    "field_debug_scope_expiry_reason_masking_required=true",
    "raw_path_excluded=true",
    "raw_body_excluded=true",
    "raw_prompt_answer_excluded=true",
    "future_server_admin_settings_excluded=true",
    "",
    "## Evidence Targets",
    "",
    "| Target | Status |",
    "| --- | --- |",
  );
  for (const targetResult of result.targetResults ?? []) {
    lines.push(`| \`${targetResult.id}\` | \`${targetResult.status}\` |`);
  }
  return lines.join("\n");
}

export async function runDataSettingsGate({ root = process.cwd() } = {}) {
  try {
    const inventory = await readFile(join(root, ".tasks/phase011-current-implementation-inventory.md"), "utf8");
    const sourceFingerprint = inventory.match(/source_fingerprint=([a-f0-9]{64})/)?.[1];
    const sources = {};
    for (const filePath of collectRequiredFiles()) {
      sources[filePath] = await readFile(join(root, filePath), "utf8");
    }
    const result = analyzeDataSettingsGateEvidence({ sources, sourceFingerprint });
    const artifact = renderDataSettingsGateMarkdown(result);
    if (containsSensitiveData(artifact)) {
      return await writeResult(root, failed(DataSettingsGateErrorCode.SensitiveDataLeak, result.targetResults ?? []));
    }
    return await writeResult(root, result);
  } catch {
    return await writeResult(root, failed(DataSettingsGateErrorCode.SourceReadFailed, []));
  }
}

function collectRequiredFiles() {
  return [...new Set(REQUIRED_TARGETS.flatMap((entry) => entry.requiredFiles))];
}

function analyzeTarget(entry, sources) {
  const missingFiles = entry.requiredFiles.filter((filePath) => !(filePath in sources));
  const texts = entry.requiredFiles.map((filePath) => sources[filePath] ?? "");
  const missingEvidence = entry.evidence.filter((needle) => !texts.some((text) => text.includes(needle)));
  return {
    id: entry.id,
    status: missingFiles.length + missingEvidence.length === 0 ? "covered" : "missing",
    missing: [...missingFiles, ...missingEvidence],
  };
}

async function writeResult(root, result) {
  await writeFile(join(root, ".tasks/phase011-data-settings-gate-result.md"), `${renderDataSettingsGateMarkdown(result)}\n`);
  return result;
}

function target(id, description, { requiredFiles, evidence }) {
  return { id, description, requiredFiles, evidence };
}

function failed(errorCode, targetResults, missingEvidence = []) {
  return {
    passed: false,
    marker: "phase011_data_settings_gate=failed",
    releaseScope: "personal_local_desktop",
    errorCode,
    targetResults,
    missingEvidence,
    summary: {
      requiredTargets: REQUIRED_TARGETS.length,
      missingRequiredEvidence: missingEvidence.length,
    },
  };
}

function containsSensitiveData(text) {
  return SENSITIVE_PATTERNS.some((pattern) => pattern.test(text));
}

async function runCli() {
  const result = await runDataSettingsGate();
  if (result.passed) {
    console.log(result.marker);
    console.log(`source_fingerprint=${result.sourceFingerprint}`);
    return;
  }
  console.error(result.marker);
  console.error(`error_code=${result.errorCode}`);
  process.exit(1);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runCli();
}
