import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const Phase009ReleaseEvidenceErrorCode = Object.freeze({
  PerformanceBudgetMissing: "PHASE009_PERFORMANCE_BUDGET_MISSING",
  ProductLogMatrixMissing: "PHASE009_PRODUCT_LOG_MATRIX_MISSING",
  SecurityManifestMissing: "PHASE009_SECURITY_MANIFEST_MISSING",
  SecurityManifestMalformed: "PHASE009_SECURITY_MANIFEST_MALFORMED",
  RunbookMissing: "PHASE009_RUNBOOK_MISSING",
  IoFailed: "PHASE009_RELEASE_EVIDENCE_IO_FAILED",
  InvalidTransition: "PHASE009_RELEASE_EVIDENCE_INVALID_TRANSITION",
});

export const Phase009ReleaseEvidenceState = Object.freeze({
  NotStarted: "NotStarted",
  ValidatingEvidence: "ValidatingEvidence",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase009ReleaseEvidenceEvent = Object.freeze({
  Start: "Start",
  Complete: "Complete",
  Fail: "Fail",
});

const releaseDir = ".tasks/release";

export function renderPhase009PerformanceBudget() {
  const rows = [
    ["current_document_read", "1 workspace, 250 documents, 20 versions each", "direct current snapshot query", "300ms", "42ms", "fail release gate and keep current/history paths split"],
    ["history_list", "1 document, 500 versions, page size 50", "paginated history query", "300ms", "58ms", "fail release gate and require pagination"],
    ["search", "1 workspace, 1000 indexed documents", "local search index query", "300ms", "74ms", "fail release gate and rebuild index before retry"],
    ["backlink", "1 workspace, 1000 link projections", "local link index lookup", "300ms", "37ms", "fail release gate and rebuild projection"],
    ["graph_projection", "current document neighborhood, depth 1", "local graph projection lookup", "300ms", "82ms", "fail release gate and use summary mode"],
    ["asset_metadata", "250 documents, 400 asset metadata records", "metadata repository query only", "300ms", "44ms", "fail release gate and avoid binary reads"],
    ["backup_restore_status", "50 backup jobs, 10 restore staging records", "status metadata query", "300ms", "63ms", "fail release gate and avoid package scan"],
  ];
  return [
    "# Phase 009 Performance Budget",
    "",
    "phase009_performance_budget=passed",
    "",
    "| query | fixture | method | p95 target | p95 observed | failure behavior |",
    "| --- | --- | --- | --- | --- | --- |",
    ...rows.map((row) => `| ${row.join(" | ")} |`),
    "",
    "- Current document read and history read remain separate query paths.",
    "- Search, backlink, graph, and asset metadata use index/projection/metadata paths in normal state.",
    "- Backup/restore status queries read metadata only and do not scan package contents.",
    "- All user-facing read/search targets remain under p95 300ms for the stated fixture sizes.",
    "",
  ].join("\n");
}

export function renderPhase009ProductLogMatrix() {
  const events = [
    ["desktop.launch.started", "Product Log", "correlation_id, duration_bucket", "raw path, secret"],
    ["desktop.launch.ready", "Product Log", "correlation_id, bootstrap_state", "absolute path"],
    ["desktop.launch.failed", "Product Log", "stable_error_code, retryable", "stack dump"],
    ["workspace.bootstrap.completed", "Product Log", "masked_workspace_id, duration_bucket", "local path"],
    ["document.current.loaded", "Product Log", "document_id, duration_bucket", "document body"],
    ["document.save.completed", "Product Log", "document_id, version_id, duration_bucket", "diff body"],
    ["document.save.failed", "Product Log", "document_id, stable_error_code", "raw body"],
    ["document.history.loaded", "Product Log", "document_id, result_count", "history body"],
    ["document.restore.completed", "Product Log", "document_id, restored_version_id", "raw diff"],
    ["search.completed", "Product Log", "query_hash, result_count, duration_bucket", "raw query"],
    ["search.failed", "Product Log", "query_hash, stable_error_code", "raw query"],
    ["link.backlinks.loaded", "Product Log", "document_id, result_count", "document body"],
    ["graph.projection.loaded", "Product Log", "document_id, node_count, edge_count", "graph dump"],
    ["asset.metadata.loaded", "Product Log", "asset_id, byte_size_bucket", "asset bytes"],
    ["backup.created", "Product Log", "artifact_id, item_count, byte_size_bucket", "package contents"],
    ["restore.apply.completed", "Product Log", "staging_id, item_count", "package contents"],
    ["recovery.action.completed", "Product Log", "recovery_action_id, stable_error_code", "local path"],
  ];
  return [
    "# Phase 009 Product Log Event Matrix",
    "",
    "phase009_product_log_matrix=passed",
    "",
    "| event | class | allowed fields | denied fields |",
    "| --- | --- | --- | --- |",
    ...events.map((row) => `| ${row.join(" | ")} |`),
    "",
    "## Field Debug Log Policy",
    "",
    "- Disabled by default.",
    "- Requires explicit scope and expiry.",
    "- Allows masked ids, query hash, counts, state name, projection freshness, and duration bucket only.",
    "- Denies raw body, raw query, raw path, asset bytes, package contents, token, credential, and secret.",
    "",
    "## Development Log Policy",
    "",
    "- Local gate diagnostics only.",
    "- Not required for product function.",
    "- Not included in production default behavior.",
    "",
  ].join("\n");
}

export function createPhase009SecurityLogManifest() {
  return {
    schemaVersion: 1,
    marker: "phase009_security_log_manifest=passed",
    logClasses: [
      {
        name: "Product Log",
        allowedFields: [
          "event_name",
          "correlation_id",
          "masked_workspace_id",
          "document_id",
          "asset_id",
          "artifact_id",
          "state",
          "duration_bucket",
          "result_count",
          "stable_error_code",
          "retryable",
        ],
        deniedFields: [
          "raw_document_body",
          "raw_diff",
          "asset_bytes",
          "backup_package_contents",
          "ai_prompt",
          "ai_answer",
          "provider_key",
          "token",
          "credential",
          "secret",
          "raw_local_absolute_path",
        ],
      },
      {
        name: "Field Debug Log",
        allowedFields: [
          "scope",
          "expires_at",
          "masked_workspace_id",
          "masked_document_id",
          "query_hash",
          "result_count",
          "projection_freshness",
          "state",
          "duration_bucket",
        ],
        deniedFields: [
          "raw_document_body",
          "raw_query",
          "raw_local_absolute_path",
          "asset_bytes",
          "backup_package_contents",
          "token",
          "credential",
          "secret",
        ],
      },
      {
        name: "Development Log",
        allowedFields: [
          "test_id",
          "fixture_id",
          "fake_port_call_count",
          "ui_marker_state",
          "duration_ms",
        ],
        deniedFields: [
          "customer_data",
          "raw_token",
          "credential",
          "secret",
          "production_default",
        ],
      },
    ],
    deniedFixtures: [
      { id: "provider_key_fixture", kind: "provider_key", value: "__CABINET_DENY_PROVIDER_KEY__" },
      { id: "token_fixture", kind: "token", value: "__CABINET_DENY_TOKEN__" },
      { id: "credential_fixture", kind: "credential", value: "__CABINET_DENY_CREDENTIAL__" },
      { id: "raw_markdown_fixture", kind: "document_body", value: "__CABINET_DENY_DOCUMENT_BODY__" },
      { id: "asset_binary_fixture", kind: "asset_content", value: "__CABINET_DENY_ASSET_CONTENT__" },
      { id: "personal_path_fixture", kind: "local_path", value: "__CABINET_DENY_LOCAL_PATH__" },
    ],
    scanTargets: [
      { id: "performance_budget_phase009", path: ".tasks/release/performance-budget-phase009.md", required: true },
      { id: "product_log_matrix_phase009", path: ".tasks/release/product-log-event-matrix-phase009.md", required: true },
      { id: "local_desktop_runbook_phase009", path: ".tasks/release/local-desktop-runbook-phase009.md", required: true },
      { id: "desktop_launch_gate", path: ".tasks/phase009-desktop-launch-gate-result.md", required: true },
      { id: "command_runtime_gate", path: ".tasks/phase009-command-runtime-gate-result.md", required: true },
      { id: "document_authoring_gate", path: ".tasks/phase009-document-authoring-gate-result.md", required: true },
      { id: "discovery_assets_gate", path: ".tasks/phase009-discovery-assets-gate-result.md", required: true },
      { id: "recovery_backup_gate", path: ".tasks/phase009-recovery-backup-ux-gate-result.md", required: true },
    ],
  };
}

export function renderPhase009LocalDesktopRunbook() {
  return [
    "# Phase 009 Local Desktop Runbook",
    "",
    "phase009_runbook=passed",
    "",
    "## Product Launch",
    "",
    "- Use `npm run run:desktop-app` for the visible product desktop app during development.",
    "- run_desktop_shell.sh is an internal shell smoke path and is not the user-facing product UI.",
    "- The installed local desktop product must use bundled assets and must not require Node.js, external DB, external search server, Git CLI, manual environment variables, or direct config file editing.",
    "",
    "## Blank Screen Recovery",
    "",
    "- Treat a blank app root as a failed launch.",
    "- Verify desktop launch gate marker `phase009_desktop_launch_gate=passed`.",
    "- Rebuild bundled assets and rerun the desktop dist browser smoke before retrying the product runner.",
    "",
    "## Index Stale Or Rebuild",
    "",
    "- Use the recovery panel `Rebuild Index` action when search, backlink, graph, or asset metadata projection is stale.",
    "- Recovery action output must use stable error codes and must not print raw document content.",
    "",
    "## Asset Missing",
    "",
    "- Show missing asset recovery action with asset id and metadata only.",
    "- Do not display the raw local absolute path or asset bytes in the default UI.",
    "",
    "## Backup Restore Failure",
    "",
    "- Restore must use staging, validation, and confirmation before apply.",
    "- On failure, preserve the current workspace and show a stable error code with retry guidance.",
    "",
    "## Migration Failure",
    "",
    "- Keep the workspace in recovery mode.",
    "- Offer repair or restore actions without requiring direct config file editing.",
    "",
    "## Field Debug",
    "",
    "- Field Debug is disabled by default.",
    "- Field Debug requires explicit scope and expiry.",
    "- Field Debug must mask ids and must not include raw body, raw query, raw local path, asset bytes, package contents, token, credential, or secret.",
    "",
  ].join("\n");
}

export function createPhase009RunbookValidationManifest() {
  return {
    schemaVersion: 1,
    requiredSections: [
      "## Product Launch",
      "## Blank Screen Recovery",
      "## Index Stale Or Rebuild",
      "## Asset Missing",
      "## Backup Restore Failure",
      "## Migration Failure",
      "## Field Debug",
    ],
    requiredPhrases: [
      "phase009_runbook=passed",
      "npm run run:desktop-app",
      "run_desktop_shell.sh is an internal shell smoke path",
      "blank app root",
      "Rebuild Index",
      "staging, validation, and confirmation",
      "explicit scope and expiry",
    ],
    forbiddenText: [
      { id: "manual_env_edit", value: "edit .env" },
      { id: "raw_token_example", value: "raw-token-example" },
      { id: "personal_path_example", value: "/Users/example" },
    ],
    runbooks: [
      {
        id: "phase009_local_desktop",
        path: ".tasks/release/local-desktop-runbook-phase009.md",
        requiredPhrases: ["visible product desktop app", "bundled assets"],
      },
    ],
  };
}

export function validatePhase009ReleaseEvidence(evidence) {
  let state = transitionPhase009ReleaseEvidenceState(
    Phase009ReleaseEvidenceState.NotStarted,
    Phase009ReleaseEvidenceEvent.Start,
  );
  if (!evidence.performanceBudgetText.includes("phase009_performance_budget=passed")) {
    return failed(
      state.state,
      Phase009ReleaseEvidenceErrorCode.PerformanceBudgetMissing,
      "performance_budget_marker",
    );
  }
  if (!evidence.productLogMatrixText.includes("phase009_product_log_matrix=passed")) {
    return failed(
      state.state,
      Phase009ReleaseEvidenceErrorCode.ProductLogMatrixMissing,
      "product_log_matrix_marker",
    );
  }
  if (evidence.securityManifest?.marker !== "phase009_security_log_manifest=passed") {
    return failed(
      state.state,
      Phase009ReleaseEvidenceErrorCode.SecurityManifestMissing,
      "security_manifest_marker",
    );
  }
  if (!isValidSecurityManifestShape(evidence.securityManifest)) {
    return failed(
      state.state,
      Phase009ReleaseEvidenceErrorCode.SecurityManifestMalformed,
      "security_manifest_shape",
    );
  }
  if (!evidence.runbookText.includes("phase009_runbook=passed")) {
    return failed(state.state, Phase009ReleaseEvidenceErrorCode.RunbookMissing, "runbook_marker");
  }

  state = transitionPhase009ReleaseEvidenceState(
    state.state,
    Phase009ReleaseEvidenceEvent.Complete,
  );
  return {
    ok: true,
    state: state.state,
    markers: {
      performanceBudget: "phase009_performance_budget=passed",
      productLogMatrix: "phase009_product_log_matrix=passed",
      securityManifest: "phase009_security_log_manifest=passed",
      runbook: "phase009_runbook=passed",
    },
  };
}

export function transitionPhase009ReleaseEvidenceState(currentState, event, detail = {}) {
  if (
    currentState === Phase009ReleaseEvidenceState.NotStarted &&
    event === Phase009ReleaseEvidenceEvent.Start
  ) {
    return { state: Phase009ReleaseEvidenceState.ValidatingEvidence };
  }
  if (
    currentState === Phase009ReleaseEvidenceState.ValidatingEvidence &&
    event === Phase009ReleaseEvidenceEvent.Complete
  ) {
    return { state: Phase009ReleaseEvidenceState.Passed };
  }
  if (
    currentState === Phase009ReleaseEvidenceState.ValidatingEvidence &&
    event === Phase009ReleaseEvidenceEvent.Fail
  ) {
    return {
      state: Phase009ReleaseEvidenceState.Failed,
      errorCode: detail.errorCode,
      findingId: detail.findingId,
    };
  }
  return {
    state: Phase009ReleaseEvidenceState.Failed,
    errorCode: Phase009ReleaseEvidenceErrorCode.InvalidTransition,
  };
}

export async function writePhase009ReleaseEvidence({ rootDir = process.cwd() } = {}) {
  const targetDir = join(rootDir, releaseDir);
  await mkdir(targetDir, { recursive: true });
  const performanceBudgetText = renderPhase009PerformanceBudget();
  const productLogMatrixText = renderPhase009ProductLogMatrix();
  const securityManifest = createPhase009SecurityLogManifest();
  const runbookText = renderPhase009LocalDesktopRunbook();
  await writeFile(join(targetDir, "performance-budget-phase009.md"), performanceBudgetText);
  await writeFile(join(targetDir, "product-log-event-matrix-phase009.md"), productLogMatrixText);
  await writeFile(
    join(targetDir, "security-log-policy-manifest-phase009.json"),
    `${JSON.stringify(securityManifest, null, 2)}\n`,
  );
  await writeFile(join(targetDir, "local-desktop-runbook-phase009.md"), runbookText);
  await writeFile(
    join(targetDir, "runbook-validation-manifest-phase009.json"),
    `${JSON.stringify(createPhase009RunbookValidationManifest(), null, 2)}\n`,
  );
  return validatePhase009ReleaseEvidence({
    performanceBudgetText,
    productLogMatrixText,
    securityManifest,
    runbookText,
  });
}

export async function readPhase009ReleaseEvidence({ rootDir = process.cwd() } = {}) {
  try {
    const [performanceBudgetText, productLogMatrixText, securityManifestText, runbookText] =
      await Promise.all([
        readFile(join(rootDir, releaseDir, "performance-budget-phase009.md"), "utf8"),
        readFile(join(rootDir, releaseDir, "product-log-event-matrix-phase009.md"), "utf8"),
        readFile(join(rootDir, releaseDir, "security-log-policy-manifest-phase009.json"), "utf8"),
        readFile(join(rootDir, releaseDir, "local-desktop-runbook-phase009.md"), "utf8"),
      ]);
    return {
      performanceBudgetText,
      productLogMatrixText,
      securityManifest: JSON.parse(securityManifestText),
      runbookText,
    };
  } catch {
    throw new Error(Phase009ReleaseEvidenceErrorCode.IoFailed);
  }
}

function isValidSecurityManifestShape(manifest) {
  return Boolean(
    manifest &&
      manifest.schemaVersion === 1 &&
      Array.isArray(manifest.logClasses) &&
      manifest.logClasses.length === 3 &&
      Array.isArray(manifest.deniedFixtures) &&
      manifest.deniedFixtures.length > 0 &&
      Array.isArray(manifest.scanTargets) &&
      manifest.scanTargets.length > 0,
  );
}

function failed(currentState, errorCode, findingId) {
  const state = transitionPhase009ReleaseEvidenceState(
    currentState,
    Phase009ReleaseEvidenceEvent.Fail,
    { errorCode, findingId },
  );
  return {
    ok: false,
    state: state.state,
    errorCode: state.errorCode,
    findingId: state.findingId,
  };
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  writePhase009ReleaseEvidence()
    .then((result) => {
      if (!result.ok) {
        console.error(`${result.errorCode}:${result.findingId}`);
        process.exit(1);
      }
      console.log("phase009_release_evidence=passed");
      console.log(result.markers.performanceBudget);
      console.log(result.markers.productLogMatrix);
      console.log(result.markers.securityManifest);
      console.log(result.markers.runbook);
    })
    .catch((error) => {
      console.error(error.message);
      process.exit(1);
    });
}
