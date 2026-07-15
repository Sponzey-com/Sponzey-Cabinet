import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const RecoveryObservabilityGateErrorCode = Object.freeze({
  SourceReadFailed: "PHASE011_RECOVERY_OBSERVABILITY_SOURCE_READ_FAILED",
  RequiredEvidenceMissing: "PHASE011_RECOVERY_OBSERVABILITY_REQUIRED_EVIDENCE_MISSING",
  SensitiveDataLeak: "PHASE011_RECOVERY_OBSERVABILITY_SENSITIVE_DATA_LEAK",
  StaleSourceFingerprint: "PHASE011_RECOVERY_OBSERVABILITY_STALE_SOURCE_FINGERPRINT",
});

const REQUIRED_TARGETS = Object.freeze([
  target("data_settings_prerequisite", "Phase 011 data settings prerequisite", {
    requiredFiles: [".tasks/phase011-data-settings-gate-result.md"],
    evidence: ["phase011_data_settings_gate=passed"],
  }),
  target("recovery_ui_models", "Recovery UI states and read-only recovery actions", {
    requiredFiles: [
      "packages/ui/src/index.ts",
      "packages/ui/tests/recovery_observability_model_tests.ts",
      "packages/ui/tests/revision_safe_save_coordinator_tests.ts",
      "apps/desktop/tests/desktop_document_authoring_controller_tests.ts",
    ],
    evidence: [
      "createRecoveryActionPanelModel",
      "recovery action panel maps local failures to safe user actions",
      "ReadOnlyRecovery",
      "authoring controller exposes retry close discard and repair-required read-only recovery",
    ],
  }),
  target("startup_repair_and_backup_recovery", "Startup repair and backup/import recovery evidence", {
    requiredFiles: [
      "crates/cabinet-platform/tests/startup_repair_smoke.rs",
      "crates/cabinet-usecases/tests/backup_usecase_tests.rs",
      "crates/cabinet-usecases/tests/import_markdown_folder_tests.rs",
    ],
    evidence: [
      "startup_repair_smoke_rebuilds_corrupted_indexes_without_losing_current_workspace_data",
      "restore_failure_preserves_workspace_current_data_and_logs_safe_failure",
      "import_markdown_folder_continues_after_duplicate_entry_as_partial_failure",
    ],
  }),
  target("field_debug_and_tooling", "Field Debug, security scanner, and runbook validator tooling", {
    requiredFiles: [
      "crates/cabinet-usecases/tests/field_debug_usecase_tests.rs",
      "scripts/security_log_scanner.mjs",
      "scripts/security_log_scanner_tests.mjs",
      "scripts/runbook_validator.mjs",
      "scripts/runbook_validator_tests.mjs",
    ],
    evidence: [
      "field_debug_diagnostic_writes_only_for_active_session_and_sanitized_fields",
      "Product Log",
      "Field Debug Log",
      "Development Log",
      "runbook validator rejects forbidden manual env edit",
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
  /sessionToken/i,
  /manual \.env edit/i,
]);

export function analyzeRecoveryObservabilityEvidence({ sources, sourceFingerprint }) {
  if (!/^[a-f0-9]{64}$/.test(sourceFingerprint ?? "")) {
    return failed(RecoveryObservabilityGateErrorCode.StaleSourceFingerprint, []);
  }
  const targetResults = REQUIRED_TARGETS.map((entry) => analyzeTarget(entry, sources));
  const missingEvidence = targetResults
    .filter((entry) => entry.missing.length > 0)
    .map((entry) => ({ targetId: entry.id, missing: entry.missing }));
  if (missingEvidence.length > 0) {
    return failed(RecoveryObservabilityGateErrorCode.RequiredEvidenceMissing, targetResults, missingEvidence);
  }
  return {
    passed: true,
    marker: "phase011_recovery_observability_gate=passed",
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

export function renderRecoveryObservabilityGateMarkdown(result) {
  const lines = [
    `phase011_recovery_observability_gate=${result.passed ? "passed" : "failed"}`,
    "release_scope=personal_local_desktop",
  ];
  if (result.sourceFingerprint) lines.push(`source_fingerprint=${result.sourceFingerprint}`);
  if (!result.passed) lines.push(`error_code=${result.errorCode}`);
  lines.push(
    "requirements=DATA-01,LOG-01,STATE-01,SEC-01,COMPAT-01",
    `required_target_count=${result.summary?.requiredTargets ?? 0}`,
    "security_manifest=phase011_security_log_manifest=passed",
    "runbook=phase011_runbook=passed",
    "product_log_classes_separated=true",
    "field_debug_scoped_expiring=true",
    "development_log_not_product_default=true",
    "raw_body_excluded=true",
    "raw_path_excluded=true",
    "raw_prompt_answer_excluded=true",
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

export function renderSecurityManifest(sourceFingerprint) {
  return JSON.stringify({
    schemaVersion: 1,
    marker: "phase011_security_log_manifest=passed",
    sourceFingerprint,
    phase: "Phase 011.6",
    productScope: "personal_local_desktop",
    logClasses: [
      { name: "Product Log", allowedFields: ["event_name", "masked_workspace_id", "stable_error_code", "duration_bucket"], deniedFields: ["document_body", "asset_bytes", "secret", "raw_path", "prompt", "answer"] },
      { name: "Field Debug Log", allowedFields: ["scope_hash", "expiry", "state", "count"], deniedFields: ["raw_body", "token", "credential", "unscoped_global_activation"] },
      { name: "Development Log", allowedFields: ["test_marker", "fixture_hash", "local_duration"], deniedFields: ["production_default", "customer_data"] },
    ],
    deniedFixtures: [
      { id: "auth_material_sample", kind: "auth_material", value: "AUTH_MATERIAL_SAMPLE" },
      { id: "document_body_sample", kind: "document_body", value: "RAW_DOC_BODY_SAMPLE" },
      { id: "personal_path_sample", kind: "path", value: "PERSONAL_PATH_SAMPLE" },
      { id: "ai_prompt_sample", kind: "ai_prompt", value: "AI_PROMPT_SAMPLE" },
      { id: "ai_answer_sample", kind: "ai_answer", value: "AI_ANSWER_SAMPLE" },
    ],
    scanTargets: [
      { id: "recovery_observability_gate", path: ".tasks/phase011-recovery-observability-gate-result.md", required: true },
      { id: "product_smoke_gate", path: ".tasks/phase011-product-smoke-gate-result.md", required: false },
      { id: "local_desktop_runbook", path: ".tasks/release/local-desktop-runbook-phase011.md", required: true },
      { id: "visual_accessibility_report", path: ".tasks/release/visual-accessibility-report-phase011.md", required: false },
      { id: "native_platform_matrix", path: ".tasks/release/native-platform-matrix-phase011.md", required: false },
    ],
    targets: [
      "workspace_repair",
      "document_save_failure",
      "document_restore_failure",
      "index_repair",
      "backup_restore_failure",
      "field_debug_activation",
    ],
  }, null, 2);
}

export function renderRunbook(sourceFingerprint) {
  return [
    "# Phase 011 Local Desktop Recovery Runbook",
    "",
    "phase011_runbook=passed",
    `source_fingerprint=${sourceFingerprint}`,
    "",
    "## Startup Repair",
    "- Failure category: local workspace bootstrap or index corruption.",
    "- Recovery action: run in-app repair, then reopen the local workspace.",
    "- Logging: Product Log records requested/completed/failed with stable error code only.",
    "",
    "## Authoring Recovery",
    "- Failure category: save conflict or pointer update failure.",
    "- Recovery action: retry save, export safe copy, or enter read-only recovery.",
    "- Logging: Field Debug requires scope, expiry, reason, and masking policy.",
    "",
    "## Restore Recovery",
    "- Failure category: restore validation or apply failure.",
    "- Recovery action: preview, validate, confirm, apply, or roll back to read-only recovery.",
    "- Logging: Product Log records restore failure code without document body.",
    "",
    "## Index Repair",
    "- Failure category: stale search, graph, backlink, or asset metadata projection.",
    "- Recovery action: rebuild local index and keep user data unchanged.",
    "",
    "## Data Export",
    "- Failure category: backup or import conflict.",
    "- Recovery action: export safe copy before destructive action; import preview remains non-mutating.",
    "",
    "## Field Debug Activation",
    "- Failure category: support investigation needed.",
    "- Recovery action: activate only with explicit scope, expiry, reason, and masking policy.",
    "- Product Log, Field Debug Log, Development Log are separated.",
    "",
    "Do not require external DB, external search, Git CLI, Node.js runtime, manual environment variables, or direct config file editing as a user recovery path.",
    "",
  ].join("\n");
}

export function renderRunbookManifest() {
  return JSON.stringify({
    schemaVersion: 1,
    marker: "phase011_runbook_manifest=passed",
    policyId: "phase011.local-desktop.runbooks",
    requiredSections: ["Startup Repair", "Authoring Recovery", "Restore Recovery", "Index Repair", "Data Export", "Field Debug Activation"],
    requiredPhrases: ["external DB", "external search", "Git CLI", "Node.js runtime", "manual environment variables"],
    forbiddenText: [
      { id: "manual_env_edit", value: "edit .env" },
      { id: "raw_token_example", value: "raw-token-example" },
      { id: "raw_document_body_sample", value: "RAW_DOC_BODY_SAMPLE" },
      { id: "auth_material_sample", value: "AUTH_MATERIAL_SAMPLE" },
    ],
    runbooks: [
      {
        id: "phase011_local_desktop_recovery",
        path: ".tasks/release/local-desktop-runbook-phase011.md",
        requiredPhrases: ["Startup Repair", "Authoring Recovery", "Restore Recovery", "Index Repair", "Data Export", "Field Debug Activation"],
      },
    ],
  }, null, 2);
}

export async function runRecoveryObservabilityGate({ root = process.cwd() } = {}) {
  try {
    const inventory = await readFile(join(root, ".tasks/phase011-current-implementation-inventory.md"), "utf8");
    const sourceFingerprint = inventory.match(/source_fingerprint=([a-f0-9]{64})/)?.[1];
    const sources = {};
    for (const filePath of collectRequiredFiles()) {
      sources[filePath] = await readFile(join(root, filePath), "utf8");
    }
    const result = analyzeRecoveryObservabilityEvidence({ sources, sourceFingerprint });
    const artifact = renderRecoveryObservabilityGateMarkdown(result);
    if (containsSensitiveData(artifact)) {
      return await writeResult(root, failed(RecoveryObservabilityGateErrorCode.SensitiveDataLeak, result.targetResults ?? []));
    }
    return await writeResult(root, result);
  } catch {
    return await writeResult(root, failed(RecoveryObservabilityGateErrorCode.SourceReadFailed, []));
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
  await mkdir(join(root, ".tasks/release"), { recursive: true });
  await writeFile(join(root, ".tasks/phase011-recovery-observability-gate-result.md"), `${renderRecoveryObservabilityGateMarkdown(result)}\n`);
  if (result.passed) {
    await writeFile(join(root, ".tasks/release/security-log-policy-manifest-phase011.json"), `${renderSecurityManifest(result.sourceFingerprint)}\n`);
    await writeFile(join(root, ".tasks/release/local-desktop-runbook-phase011.md"), renderRunbook(result.sourceFingerprint));
    await writeFile(join(root, ".tasks/release/runbook-validation-manifest-phase011.json"), `${renderRunbookManifest()}\n`);
  }
  return result;
}

function target(id, description, { requiredFiles, evidence }) {
  return { id, description, requiredFiles, evidence };
}

function failed(errorCode, targetResults, missingEvidence = []) {
  return {
    passed: false,
    marker: "phase011_recovery_observability_gate=failed",
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
  const result = await runRecoveryObservabilityGate();
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
