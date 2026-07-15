import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const Phase011ArchiveState = Object.freeze({
  Pending: "Pending",
  ReadingArchive: "ReadingArchive",
  ValidatingInventory: "ValidatingInventory",
  RenderingEvidence: "RenderingEvidence",
  WritingResult: "WritingResult",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase011ArchiveEvent = Object.freeze({
  Start: "Start",
  ArchiveRead: "ArchiveRead",
  InventoryValidated: "InventoryValidated",
  EvidenceRendered: "EvidenceRendered",
  ResultWritten: "ResultWritten",
  Fail: "Fail",
});

export const Phase011ArchiveErrorCode = Object.freeze({
  ArchivePlanMissing: "PHASE011_ARCHIVE_PLAN_MISSING",
  ArchiveReadmeMissing: "PHASE011_ARCHIVE_README_MISSING",
  ArchiveTaskGap: "PHASE011_ARCHIVE_TASK_GAP",
  ArchiveEvidenceMissing: "PHASE011_ARCHIVE_EVIDENCE_MISSING",
  ArchiveEvidenceMarkerMissing: "PHASE011_ARCHIVE_EVIDENCE_MARKER_MISSING",
  Phase010ReleaseMarkerMissing: "PHASE011_PHASE010_RELEASE_MARKER_MISSING",
  Phase010ScopeMismatch: "PHASE011_PHASE010_SCOPE_MISMATCH",
  RequiredPathMissing: "PHASE011_REQUIRED_PATH_MISSING",
  CommandContractInvalid: "PHASE011_COMMAND_CONTRACT_INVALID",
  FutureScopeActivated: "PHASE011_FUTURE_SCOPE_ACTIVATED",
  SourceFingerprintMismatch: "PHASE011_SOURCE_FINGERPRINT_MISMATCH",
  RequirementRegisterInvalid: "PHASE011_REQUIREMENT_REGISTER_INVALID",
  UnsafeArtifact: "PHASE011_UNSAFE_ARTIFACT",
  IoFailed: "PHASE011_ARCHIVE_IO_FAILED",
  InvalidTransition: "PHASE011_ARCHIVE_INVALID_TRANSITION",
});

const requirementIds = Object.freeze([
  "SCOPE-01",
  "BOOT-01",
  "HOME-01",
  "NAV-01",
  "DOC-01",
  "DOC-02",
  "DOC-03",
  "HIST-01",
  "HIST-02",
  "DISC-01",
  "DATA-01",
  "CFG-01",
  "CFG-02",
  "LOG-01",
  "STATE-01",
  "PERF-01",
  "SEC-01",
  "UX-01",
  "PLAT-01",
  "COMPAT-01",
]);

const structureVerifiedRequirementIds = new Set(["SCOPE-01", "CFG-01", "LOG-01", "STATE-01"]);

const archiveGateEvidence = Object.freeze([
  evidence(".tasks/phase010/phase010-archive-validation-result.md", "phase010_archive_validation=passed"),
  evidence(".tasks/phase010/phase010-plan-validation-result.md", "phase010_plan_validation=passed"),
  evidence(".tasks/phase010/phase010-packaged-launch-gate-result.md", "phase010_packaged_launch_gate=passed"),
  evidence(".tasks/phase010/phase010-first-run-workspace-gate-result.md", "phase010_first_run_workspace_gate=passed"),
  evidence(".tasks/phase010/phase010-durable-authoring-gate-result.md", "phase010_durable_authoring_gate=passed"),
  evidence(".tasks/phase010/phase010-data-portability-gate-result.md", "phase010_data_portability_gate=passed"),
  evidence(".tasks/phase010/phase010-index-health-repair-gate-result.md", "phase010_index_health_repair_gate=passed"),
  evidence(".tasks/phase010/phase010-settings-observability-gate-result.md", "phase010_settings_observability_gate=passed"),
]);

const archiveReleaseEvidence = Object.freeze([
  evidence(".tasks/phase010/release/performance-budget-phase010.md", "phase010_performance_budget=passed"),
  evidence(".tasks/phase010/release/packaged-runtime-manifest-phase010.json", "phase010_packaged_runtime_manifest=passed"),
  evidence(".tasks/phase010/release/data-portability-manifest-phase010.json", "phase010_data_portability_manifest=passed"),
  evidence(".tasks/phase010/release/product-log-event-matrix-phase010.md", "phase010_product_log_matrix=passed"),
  evidence(".tasks/phase010/release/security-log-policy-manifest-phase010.json", "phase010_security_log_manifest=passed"),
  evidence(".tasks/phase010/release/local-desktop-runbook-phase010.md", "phase010_runbook=passed"),
]);

const productCommands = Object.freeze([
  command("run:desktop-app", "sh scripts/run_desktop_app.sh", "Development product launcher"),
  command("run:desktop-package-smoke", "sh scripts/run_desktop_package_smoke.sh", "Packaged runtime smoke"),
  command("run:desktop-packaged-app-smoke", "sh scripts/run_desktop_packaged_app_smoke.sh", "Native packaged app smoke"),
  command("run:phase011-workspace-home-gate-tests", "sh scripts/run_phase011_workspace_home_gate_tests.sh", "Workspace home gate tests"),
  command("run:phase011-workspace-home-visual", "sh scripts/run_phase011_workspace_home_visual.sh", "Workspace home visual evidence"),
  command("run:phase011-workspace-home-performance", "sh scripts/run_phase011_workspace_home_performance.sh", "Workspace home performance evidence"),
  command("run:phase011-authoring-browser", "sh scripts/run_phase011_authoring_browser.sh", "Authoring browser interaction evidence"),
  command("run:phase011-document-authoring-gate-tests", "sh scripts/run_phase011_document_authoring_gate_tests.sh", "Document authoring gate tests"),
  command("run:phase011-document-authoring-gate", "sh scripts/run_phase011_document_authoring_gate.sh", "Document authoring product gate"),
  command("run:phase011-history-restore-gate-tests", "sh scripts/run_phase011_history_restore_gate_tests.sh", "History restore gate tests"),
  command("run:phase011-history-restore-gate", "sh scripts/run_phase011_history_restore_gate.sh", "History restore product gate"),
  command("run:phase011-discovery-gate-tests", "sh scripts/run_phase011_discovery_gate_tests.sh", "Discovery gate tests"),
  command("run:phase011-discovery-gate", "sh scripts/run_phase011_discovery_gate.sh", "Discovery product gate"),
  command("run:phase011-data-settings-gate-tests", "sh scripts/run_phase011_data_settings_gate_tests.sh", "Data settings gate tests"),
  command("run:phase011-data-settings-gate", "sh scripts/run_phase011_data_settings_gate.sh", "Data settings product gate"),
  command("run:phase011-recovery-observability-gate-tests", "sh scripts/run_phase011_recovery_observability_gate_tests.sh", "Recovery observability gate tests"),
  command("run:phase011-recovery-observability-gate", "sh scripts/run_phase011_recovery_observability_gate.sh", "Recovery observability product gate"),
  command("run:phase011-native-platform-evidence-tests", "sh scripts/run_phase011_native_platform_evidence_tests.sh", "Native platform evidence tests"),
  command("run:phase011-native-platform-evidence", "sh scripts/run_phase011_native_platform_evidence.sh", "Native platform evidence writer"),
  command("run:phase011-product-smoke-gate-tests", "sh scripts/run_phase011_product_smoke_gate_tests.sh", "Product smoke gate tests"),
  command("run:phase011-product-smoke-gate", "sh scripts/run_phase011_product_smoke_gate.sh", "Daily product smoke gate"),
  command("run:phase011-release-gate-tests", "sh scripts/run_phase011_release_gate_tests.sh", "Final release gate tests"),
  command("run:phase011-release-gate", "sh scripts/run_phase011_release_gate.sh", "Final release gate"),
  command("run:phase011-workspace-home-gate", "sh scripts/run_phase011_workspace_home_gate.sh", "Workspace home product gate"),
]);

const currentRuntimePaths = Object.freeze([
  pathItem("scripts/run_desktop_app.sh", "Development desktop launcher boundary"),
  pathItem("scripts/run_desktop_package_smoke.sh", "Packaged runtime smoke boundary"),
  pathItem("scripts/run_desktop_packaged_app_smoke.sh", "Native packaged app smoke boundary"),
  pathItem("scripts/build_desktop_assets.mjs", "Bundled asset build boundary"),
  pathItem("apps/desktop/src/index.ts", "Desktop current product composition"),
  pathItem("apps/desktop/src/desktop_entry.ts", "React desktop bootstrap entry"),
  pathItem("apps/desktop/package.json", "Desktop CodeMirror and React runtime dependencies"),
  pathItem("apps/desktop/src/react_workspace_home.ts", "Rendered personal workspace home"),
  pathItem("apps/desktop/src/react_document_navigator.ts", "Rendered document navigator"),
  pathItem("apps/desktop/src/desktop_navigator_controller.ts", "Document navigator UI controller"),
  pathItem(
    "apps/desktop/src/desktop_document_authoring_controller.ts",
    "Revision-safe document authoring UI controller",
  ),
  pathItem("apps/desktop/src/codemirror_document_editor.ts", "CodeMirror Markdown runtime adapter"),
  pathItem(
    "apps/desktop/src/react_document_authoring_workbench.ts",
    "React source split preview authoring workbench",
  ),
  pathItem(
    "apps/desktop/src/desktop_revision_metadata_generator.ts",
    "Injected desktop revision version and snapshot metadata adapter",
  ),
  pathItem("apps/desktop/src/tauri_desktop_transport.ts", "Composite local desktop transport"),
  pathItem("apps/desktop/src/tauri_authoring_transport.ts", "Tauri document authoring transport adapter"),
  pathItem("apps/desktop/src/tauri_home_transport.ts", "Tauri workspace home transport adapter"),
  pathItem("apps/desktop/src/tauri_navigator_transport.ts", "Tauri document navigator transport adapter"),
  pathItem("apps/desktop/public/index.html", "Desktop-only packaged HTML"),
  pathItem("apps/desktop/public/styles.css", "Desktop workspace styles"),
  pathItem("scripts/phase011_desktop_asset_builder.mjs", "Desktop-only asset builder"),
  pathItem("scripts/phase011_workspace_home_visual.mjs", "Workspace home visual validator"),
  pathItem("scripts/phase011_authoring_browser.mjs", "Authoring browser evidence validator"),
  pathItem("scripts/phase011_workspace_home_performance.mjs", "Workspace home performance validator"),
  pathItem("scripts/phase011_document_authoring_gate.mjs", "Document authoring gate validator"),
  pathItem("scripts/phase011_history_restore_gate.mjs", "History restore gate validator"),
  pathItem("scripts/phase011_discovery_gate.mjs", "Discovery gate validator"),
  pathItem("scripts/phase011_data_settings_gate.mjs", "Data settings gate validator"),
  pathItem("scripts/phase011_recovery_observability_gate.mjs", "Recovery observability gate validator"),
  pathItem("scripts/phase011_workspace_home_gate.mjs", "Workspace home gate validator"),
  pathItem("scripts/run_phase011_workspace_home_visual.mjs", "Chrome workspace home evidence runner"),
  pathItem("scripts/run_phase011_workspace_home_visual.sh", "Visual evidence command boundary"),
  pathItem("scripts/run_phase011_authoring_browser.mjs", "Local browser authoring interaction runner"),
  pathItem("scripts/run_phase011_authoring_browser.sh", "Authoring browser evidence command boundary"),
  pathItem("scripts/run_phase011_document_authoring_gate_tests.sh", "Document authoring gate test command boundary"),
  pathItem("scripts/run_phase011_document_authoring_gate.sh", "Document authoring gate command boundary"),
  pathItem("scripts/run_phase011_history_restore_gate_tests.sh", "History restore gate test command boundary"),
  pathItem("scripts/run_phase011_history_restore_gate.sh", "History restore gate command boundary"),
  pathItem("scripts/run_phase011_discovery_gate_tests.sh", "Discovery gate test command boundary"),
  pathItem("scripts/run_phase011_discovery_gate.sh", "Discovery gate command boundary"),
  pathItem("scripts/run_phase011_data_settings_gate_tests.sh", "Data settings gate test command boundary"),
  pathItem("scripts/run_phase011_data_settings_gate.sh", "Data settings gate command boundary"),
  pathItem("scripts/run_phase011_recovery_observability_gate_tests.sh", "Recovery observability gate test command boundary"),
  pathItem("scripts/run_phase011_recovery_observability_gate.sh", "Recovery observability gate command boundary"),
  pathItem("scripts/run_phase011_workspace_home_performance.mjs", "Release performance evidence runner"),
  pathItem("scripts/run_phase011_workspace_home_performance.sh", "Performance evidence command boundary"),
  pathItem("scripts/run_phase011_workspace_home_gate_tests.sh", "Workspace home test command boundary"),
  pathItem("scripts/run_phase011_workspace_home_gate.sh", "Workspace home gate command boundary"),
  pathItem("crates/cabinet-platform/src/bin/workspace_home_benchmark.rs", "Release home query benchmark"),
  pathItem("apps/desktop/src-tauri/src/lib.rs", "Tauri local command adapter"),
  pathItem("apps/desktop/src-tauri/src/main.rs", "Tauri startup composition root"),
  pathItem("apps/desktop/src-tauri/tauri.conf.json", "Tauri package configuration"),
  pathItem("packages/client-core/src/index.ts", "Current product capability and command DTOs"),
  pathItem("packages/ui/src/index.ts", "Personal workspace UI models"),
  pathItem("packages/editor/src/index.ts", "CodeMirror editor boundary"),
  pathItem("crates/cabinet-platform/src/local_desktop_runtime.rs", "Local desktop composition runtime"),
  pathItem("crates/cabinet-platform/src/release_smoke.rs", "Local product smoke runtime"),
  pathItem(
    "crates/cabinet-platform/src/workspace_home_command.rs",
    "Workspace home usecase command executor and safe DTO mapping",
  ),
  pathItem(
    "crates/cabinet-platform/src/document_navigator_command.rs",
    "Document navigator usecase command executor and safe DTO mapping",
  ),
  pathItem(
    "crates/cabinet-platform/src/document_authoring_command.rs",
    "Guarded document authoring command executor and safe DTO mapping",
  ),
  pathItem("crates/cabinet-usecases/src/document.rs", "Document usecase boundary"),
  pathItem(
    "crates/cabinet-usecases/src/guarded_authoring.rs",
    "Expected-version guarded create, update, and current query usecase",
  ),
  pathItem("crates/cabinet-usecases/src/search.rs", "Search usecase boundary"),
  pathItem("crates/cabinet-usecases/src/graph.rs", "Graph usecase boundary"),
  pathItem("crates/cabinet-usecases/src/backup.rs", "Backup usecase boundary"),
  pathItem("crates/cabinet-usecases/src/import.rs", "Import usecase boundary"),
  pathItem("crates/cabinet-usecases/src/workspace_home.rs", "Workspace home usecase boundary"),
  pathItem("crates/cabinet-usecases/src/document_navigator.rs", "Document navigator query usecase"),
  pathItem(
    "crates/cabinet-usecases/src/workspace_home_update.rs",
    "Incremental workspace home document event projector usecase",
  ),
  pathItem("crates/cabinet-ports/src/lib.rs", "External dependency ports"),
  pathItem(
    "crates/cabinet-ports/src/current_document_version.rs",
    "Current document version pointer port",
  ),
  pathItem("crates/cabinet-ports/src/workspace_home.rs", "Workspace home projection port"),
  pathItem("crates/cabinet-ports/src/document_navigator.rs", "Document navigator projection port"),
  pathItem("crates/cabinet-adapters/src/local_document_repository.rs", "Local current document adapter"),
  pathItem("crates/cabinet-adapters/src/local_version_store.rs", "Local version adapter"),
  pathItem(
    "crates/cabinet-adapters/src/local_current_document_version_pointer.rs",
    "Atomic local current document version pointer adapter",
  ),
  pathItem("crates/cabinet-adapters/src/local_search_index.rs", "Local search projection adapter"),
  pathItem("crates/cabinet-adapters/src/local_link_index.rs", "Local link projection adapter"),
  pathItem("crates/cabinet-adapters/src/local_graph_projection.rs", "Local graph projection adapter"),
  pathItem("crates/cabinet-adapters/src/local_asset_store.rs", "Local asset adapter"),
  pathItem("crates/cabinet-adapters/src/local_backup_store.rs", "Local backup adapter"),
  pathItem(
    "crates/cabinet-adapters/src/local_workspace_home_projection.rs",
    "Durable bounded workspace home projection adapter",
  ),
  pathItem(
    "crates/cabinet-adapters/src/local_document_navigator_projection.rs",
    "Durable bounded document navigator projection adapter",
  ),
]);

const activeTypeScriptTests = Object.freeze([
  "packages/client-core/tests/local_desktop_command_client_tests.ts",
  "packages/client-core/tests/personal_local_desktop_capability_tests.ts",
  "packages/editor/tests/source_editing_command_tests.ts",
  "packages/editor/tests/revision_safe_editor_session_tests.ts",
  "packages/ui/tests/autosave_state_model_tests.ts",
  "packages/ui/tests/revision_safe_save_coordinator_tests.ts",
  "packages/ui/tests/backup_restore_staging_model_tests.ts",
  "packages/ui/tests/document_authoring_preview_model_tests.ts",
  "packages/ui/tests/graph_canvas_panel_model_tests.ts",
  "packages/ui/tests/import_preview_model_tests.ts",
  "packages/ui/tests/local_discovery_panel_model_tests.ts",
  "packages/ui/tests/markdown_preview_model_tests.ts",
  "packages/ui/tests/personal_workspace_home_model_tests.ts",
  "packages/ui/tests/document_navigator_model_tests.ts",
  "packages/ui/tests/personal_workspace_shell_model_tests.ts",
  "packages/ui/tests/restore_flow_model_tests.ts",
  "apps/desktop/tests/desktop_local_command_facade_tests.ts",
  "apps/desktop/tests/desktop_personal_workspace_shell_tests.ts",
  "apps/desktop/tests/desktop_personal_workspace_home_tests.ts",
  "apps/desktop/tests/desktop_document_authoring_smoke_tests.ts",
  "apps/desktop/tests/desktop_discovery_smoke_tests.ts",
  "apps/desktop/tests/desktop_backup_restore_staging_smoke_tests.ts",
  "apps/desktop/tests/desktop_import_preview_smoke_tests.ts",
  "apps/desktop/tests/desktop_react_home_render_tests.ts",
  "apps/desktop/tests/desktop_tauri_home_transport_tests.ts",
  "apps/desktop/tests/desktop_tauri_navigator_transport_tests.ts",
  "apps/desktop/tests/desktop_tauri_authoring_transport_tests.ts",
  "apps/desktop/tests/desktop_navigator_controller_tests.ts",
  "apps/desktop/tests/desktop_document_authoring_controller_tests.ts",
  "apps/desktop/tests/desktop_codemirror_adapter_contract_tests.ts",
  "apps/desktop/tests/desktop_react_authoring_workbench_tests.ts",
  "apps/desktop/tests/desktop_revision_metadata_generator_tests.ts",
  "apps/desktop/tests/desktop_entry_authoring_contract_tests.ts",
  "apps/desktop/tests/desktop_react_navigator_render_tests.ts",
  "packages/client-core/tests/document_navigator_command_client_tests.ts",
  "packages/client-core/tests/document_authoring_command_client_tests.ts",
  "scripts/phase011_desktop_asset_builder_tests.mjs",
  "scripts/phase011_workspace_home_visual_tests.mjs",
  "scripts/phase011_authoring_browser_tests.mjs",
  "scripts/phase011_document_authoring_gate_tests.mjs",
  "scripts/phase011_history_restore_gate_tests.mjs",
  "scripts/phase011_discovery_gate_tests.mjs",
  "scripts/phase011_data_settings_gate_tests.mjs",
  "scripts/phase011_recovery_observability_gate_tests.mjs",
  "scripts/phase011_workspace_home_performance_tests.mjs",
  "scripts/phase011_workspace_home_gate_tests.mjs",
]);

const activeRustTests = Object.freeze([
  "crates/cabinet-core/tests/local_desktop_config_tests.rs",
  "crates/cabinet-core/tests/logging_tests.rs",
  "crates/cabinet-core/tests/migration_tests.rs",
  "crates/cabinet-core/tests/performance_tests.rs",
  "crates/cabinet-adapters/tests/local_asset_store_tests.rs",
  "crates/cabinet-adapters/tests/local_backup_store_tests.rs",
  "crates/cabinet-adapters/tests/local_document_asset_repository_tests.rs",
  "crates/cabinet-adapters/tests/local_document_repository_tests.rs",
  "crates/cabinet-adapters/tests/local_graph_projection_store_tests.rs",
  "crates/cabinet-adapters/tests/local_migration_store_tests.rs",
  "crates/cabinet-adapters/tests/local_search_index_tests.rs",
  "crates/cabinet-adapters/tests/local_workspace_home_projection_tests.rs",
  "crates/cabinet-adapters/tests/local_workspace_home_mutation_tests.rs",
  "crates/cabinet-adapters/tests/local_document_navigator_projection_tests.rs",
  "crates/cabinet-adapters/tests/local_current_document_version_pointer_tests.rs",
  "crates/cabinet-platform/tests/local_desktop_bootstrap_state_tests.rs",
  "crates/cabinet-platform/tests/local_desktop_command_runtime_tests.rs",
  "crates/cabinet-platform/tests/local_durable_authoring_flow_tests.rs",
  "crates/cabinet-platform/tests/query_performance_benchmarks.rs",
  "crates/cabinet-platform/tests/startup_repair_smoke.rs",
  "crates/cabinet-platform/tests/workspace_home_command_executor_tests.rs",
  "crates/cabinet-platform/tests/document_navigator_command_executor_tests.rs",
  "crates/cabinet-platform/tests/document_authoring_command_executor_tests.rs",
  "apps/desktop/src-tauri/tests/workspace_home_runtime_tests.rs",
  "apps/desktop/src-tauri/tests/document_navigator_runtime_tests.rs",
  "apps/desktop/src-tauri/tests/document_authoring_runtime_tests.rs",
  "crates/cabinet-usecases/tests/backup_usecase_tests.rs",
  "crates/cabinet-usecases/tests/compare_document_versions_tests.rs",
  "crates/cabinet-usecases/tests/create_document_tests.rs",
  "crates/cabinet-usecases/tests/get_current_document_tests.rs",
  "crates/cabinet-usecases/tests/get_document_history_tests.rs",
  "crates/cabinet-usecases/tests/get_document_version_tests.rs",
  "crates/cabinet-usecases/tests/get_workspace_home_tests.rs",
  "crates/cabinet-usecases/tests/guarded_authoring_tests.rs",
  "crates/cabinet-usecases/tests/document_navigator_tests.rs",
  "crates/cabinet-usecases/tests/graph_lite_projection_tests.rs",
  "crates/cabinet-usecases/tests/import_markdown_folder_tests.rs",
  "crates/cabinet-usecases/tests/list_document_assets_tests.rs",
  "crates/cabinet-usecases/tests/preview_document_restore_tests.rs",
  "crates/cabinet-usecases/tests/restore_document_version_tests.rs",
  "crates/cabinet-usecases/tests/search_documents_tests.rs",
  "crates/cabinet-usecases/tests/update_document_tests.rs",
  "crates/cabinet-usecases/tests/update_workspace_home_projection_tests.rs",
]);

const futureScopePaths = Object.freeze([
  pathItem("crates/cabinet-server/src/lib.rs", "Future server/self-host/SaaS path; not current product evidence"),
  pathItem("apps/mobile/src/index.ts", "Future iOS/Android path; not current product evidence"),
]);

const sourceContractPaths = Object.freeze([
  ".tasks/plan.md",
  "PROJECT.md",
  "AGENTS.md",
  "package.json",
  ...currentRuntimePaths.map((item) => item.path),
  ...activeRustTests,
  ...activeTypeScriptTests,
  ...futureScopePaths.map((item) => item.path),
]);

const unsafeArtifactTerms = Object.freeze([
  "raw_document_body_fixture",
  "provider_api_key_fixture",
  "personal_absolute_path_fixture",
  "raw_prompt_fixture",
  "raw_answer_fixture",
  "/Users/example/private",
  "C:\\Users\\example\\private",
]);

export function transitionPhase011ArchiveState(currentState, event, detail = {}) {
  if (currentState === Phase011ArchiveState.Pending && event === Phase011ArchiveEvent.Start) {
    return { state: Phase011ArchiveState.ReadingArchive };
  }
  if (
    currentState === Phase011ArchiveState.ReadingArchive &&
    event === Phase011ArchiveEvent.ArchiveRead
  ) {
    return { state: Phase011ArchiveState.ValidatingInventory };
  }
  if (
    currentState === Phase011ArchiveState.ValidatingInventory &&
    event === Phase011ArchiveEvent.InventoryValidated
  ) {
    return { state: Phase011ArchiveState.RenderingEvidence };
  }
  if (
    currentState === Phase011ArchiveState.RenderingEvidence &&
    event === Phase011ArchiveEvent.EvidenceRendered
  ) {
    return { state: Phase011ArchiveState.WritingResult };
  }
  if (
    currentState === Phase011ArchiveState.WritingResult &&
    event === Phase011ArchiveEvent.ResultWritten
  ) {
    return { state: Phase011ArchiveState.Passed };
  }
  if (
    [
      Phase011ArchiveState.ReadingArchive,
      Phase011ArchiveState.ValidatingInventory,
      Phase011ArchiveState.RenderingEvidence,
      Phase011ArchiveState.WritingResult,
    ].includes(currentState) &&
    event === Phase011ArchiveEvent.Fail
  ) {
    return {
      state: Phase011ArchiveState.Failed,
      errorCode: detail.errorCode ?? Phase011ArchiveErrorCode.IoFailed,
      findingId: detail.findingId,
    };
  }
  return {
    state: Phase011ArchiveState.Failed,
    errorCode: Phase011ArchiveErrorCode.InvalidTransition,
  };
}

export function validatePhase011InventoryArtifactFreshness(artifactText, expectedFingerprint) {
  if (!artifactText.includes(`source_fingerprint=${expectedFingerprint}`)) {
    return [
      {
        errorCode: Phase011ArchiveErrorCode.SourceFingerprintMismatch,
        findingId: "source_fingerprint",
      },
    ];
  }
  return [];
}

export async function runPhase011ArchiveValidation({
  root = process.cwd(),
  writeArtifacts = true,
  expectedSourceFingerprint,
} = {}) {
  let state = transitionPhase011ArchiveState(
    Phase011ArchiveState.Pending,
    Phase011ArchiveEvent.Start,
  ).state;

  try {
    const archivePaths = [];
    const phase010PlanPath = ".tasks/phase010/plan.md";
    const phase010Plan = await readOrFail(root, phase010PlanPath, Phase011ArchiveErrorCode.ArchivePlanMissing);
    if (!phase010Plan.startsWith("# Phase 010 Development Plan")) {
      return failed(state, Phase011ArchiveErrorCode.ArchivePlanMissing, phase010PlanPath);
    }
    archivePaths.push(phase010PlanPath);

    const phase010ReadmePath = ".tasks/phase010/readme.md";
    const phase010Readme = await readOrFail(root, phase010ReadmePath, Phase011ArchiveErrorCode.ArchiveReadmeMissing);
    if (
      !phase010Readme.includes("Active phase: Phase 010") ||
      !phase010Readme.includes("Current product scope: `personal_local_desktop`")
    ) {
      return failed(state, Phase011ArchiveErrorCode.ArchiveReadmeMissing, phase010ReadmePath);
    }
    archivePaths.push(phase010ReadmePath);

    for (let index = 1; index <= 8; index += 1) {
      const taskPath = `.tasks/phase010/task${String(index).padStart(3, "0")}.md`;
      try {
        await readFile(join(root, taskPath), "utf8");
      } catch {
        return failed(state, Phase011ArchiveErrorCode.ArchiveTaskGap, taskPath);
      }
      archivePaths.push(taskPath);
    }

    for (const item of [...archiveGateEvidence, ...archiveReleaseEvidence]) {
      let text;
      try {
        text = await readFile(join(root, item.path), "utf8");
      } catch {
        return failed(state, Phase011ArchiveErrorCode.ArchiveEvidenceMissing, item.path);
      }
      if (!text.includes(item.marker)) {
        return failed(state, Phase011ArchiveErrorCode.ArchiveEvidenceMarkerMissing, item.path);
      }
      archivePaths.push(item.path);
    }

    const releasePath = ".tasks/phase010/phase010-release-gate-result.md";
    let phase010Release;
    try {
      phase010Release = await readFile(join(root, releasePath), "utf8");
    } catch {
      return failed(state, Phase011ArchiveErrorCode.Phase010ReleaseMarkerMissing, releasePath);
    }
    if (!phase010Release.includes("phase010_release_gate=passed")) {
      return failed(state, Phase011ArchiveErrorCode.Phase010ReleaseMarkerMissing, releasePath);
    }
    if (!phase010Release.includes("release_scope=personal_local_desktop")) {
      return failed(state, Phase011ArchiveErrorCode.Phase010ScopeMismatch, releasePath);
    }
    archivePaths.push(releasePath);

    const archiveFingerprint = await fingerprintFiles(root, archivePaths);
    state = transitionPhase011ArchiveState(state, Phase011ArchiveEvent.ArchiveRead).state;

    for (const path of sourceContractPaths) {
      try {
        await readFile(join(root, path), "utf8");
      } catch {
        return failed(state, Phase011ArchiveErrorCode.RequiredPathMissing, path);
      }
    }
    if (activeTypeScriptTests.some((path) => path.includes("*") || path.includes("?"))) {
      return failed(state, Phase011ArchiveErrorCode.CommandContractInvalid, "activeTypeScriptTests");
    }

    const packageText = await readFile(join(root, "package.json"), "utf8");
    let packageJson;
    try {
      packageJson = JSON.parse(packageText);
    } catch {
      return failed(state, Phase011ArchiveErrorCode.CommandContractInvalid, "package.json");
    }
    for (const item of productCommands) {
      if (packageJson.scripts?.[item.id] !== item.command) {
        return failed(state, Phase011ArchiveErrorCode.CommandContractInvalid, item.id);
      }
    }

    const clientCoreText = await readFile(join(root, "packages/client-core/src/index.ts"), "utf8");
    if (
      !clientCoreText.includes('productScope: "personal_local_desktop"') ||
      !clientCoreText.includes("supportsRemoteWorkspace: false") ||
      !clientCoreText.includes('platforms: ["windows", "macos", "linux"]')
    ) {
      return failed(
        state,
        Phase011ArchiveErrorCode.FutureScopeActivated,
        "packages/client-core/src/index.ts",
      );
    }

    const desktopText = await readFile(join(root, "apps/desktop/src/index.ts"), "utf8");
    if (!desktopText.includes("desktopShell = createDesktopCurrentProductShellDescriptor()")) {
      return failed(
        state,
        Phase011ArchiveErrorCode.FutureScopeActivated,
        "apps/desktop/src/index.ts",
      );
    }

    const tauriText = await readFile(join(root, "apps/desktop/src-tauri/src/lib.rs"), "utf8");
    const commandAllowlist = tauriText.match(/const LOCAL_DESKTOP_COMMAND_NAMES:[\s\S]*?\];/)?.[0];
    if (
      !commandAllowlist ||
      /(server|tenant|organization|billing|sso|admin|remote_workspace)/i.test(commandAllowlist)
    ) {
      return failed(
        state,
        Phase011ArchiveErrorCode.FutureScopeActivated,
        "apps/desktop/src-tauri/src/lib.rs",
      );
    }

    const planText = await readFile(join(root, ".tasks/plan.md"), "utf8");
    const parsedRequirementIds = [...planText.matchAll(/^\| `([A-Z]+-\d+)` \|/gm)].map(
      (match) => match[1],
    );
    const duplicateRequirement = parsedRequirementIds.find(
      (id, index) => parsedRequirementIds.indexOf(id) !== index,
    );
    const missingRequirement = requirementIds.find((id) => !parsedRequirementIds.includes(id));
    const unknownRequirement = parsedRequirementIds.find((id) => !requirementIds.includes(id));
    if (
      duplicateRequirement ||
      missingRequirement ||
      unknownRequirement ||
      parsedRequirementIds.length !== requirementIds.length
    ) {
      return failed(
        state,
        Phase011ArchiveErrorCode.RequirementRegisterInvalid,
        duplicateRequirement ?? missingRequirement ?? unknownRequirement ?? ".tasks/plan.md",
      );
    }

    const sourceFingerprint = await fingerprintFiles(root, sourceContractPaths);
    if (expectedSourceFingerprint && sourceFingerprint !== expectedSourceFingerprint) {
      return failed(
        state,
        Phase011ArchiveErrorCode.SourceFingerprintMismatch,
        "source_fingerprint",
      );
    }

    state = transitionPhase011ArchiveState(
      state,
      Phase011ArchiveEvent.InventoryValidated,
    ).state;

    const result = {
      passed: true,
      state: Phase011ArchiveState.Passed,
      productScope: "personal_local_desktop",
      archivedTaskCount: 8,
      archivedGateCount: archiveGateEvidence.length + 1,
      archivedReleaseEvidenceCount: archiveReleaseEvidence.length,
      archiveFingerprint,
      sourceFingerprint,
      requirementIds: [...requirementIds],
      productCommands: [...productCommands],
      currentRuntimePaths: [...currentRuntimePaths],
      activeRustTests: [...activeRustTests],
      activeTypeScriptTests: [...activeTypeScriptTests],
      futureScopePaths: [...futureScopePaths],
    };

    const archiveArtifact = renderPhase011ArchiveValidationArtifact(result);
    const inventoryArtifact = renderPhase011CurrentInventoryArtifact(result);
    const evidenceMatrix = renderPhase011RequirementEvidenceMatrix(result);
    for (const artifact of [archiveArtifact, inventoryArtifact, evidenceMatrix]) {
      const unsafeTerm = unsafeArtifactTerms.find((term) => artifact.includes(term));
      if (unsafeTerm) {
        return failed(state, Phase011ArchiveErrorCode.UnsafeArtifact, unsafeTerm);
      }
    }

    state = transitionPhase011ArchiveState(state, Phase011ArchiveEvent.EvidenceRendered).state;
    if (writeArtifacts) {
      await mkdir(join(root, ".tasks", "release"), { recursive: true });
      await writeFile(join(root, ".tasks", "phase011-archive-validation-result.md"), archiveArtifact);
      await writeFile(
        join(root, ".tasks", "phase011-current-implementation-inventory.md"),
        inventoryArtifact,
      );
      await writeFile(
        join(root, ".tasks", "release", "requirement-evidence-matrix-phase011.md"),
        evidenceMatrix,
      );
    }

    state = transitionPhase011ArchiveState(state, Phase011ArchiveEvent.ResultWritten).state;
    return { ...result, state };
  } catch (error) {
    if (error?.phase011ValidationError) return error.result;
    return failed(
      state,
      Phase011ArchiveErrorCode.IoFailed,
      typeof error?.message === "string" ? "validator_io" : "unknown",
    );
  }
}

export function renderPhase011ArchiveValidationArtifact(result) {
  const lines = [
    "# Phase 011 Archive Validation Result",
    "",
    `phase011_archive_validation=${result.passed ? "passed" : "failed"}`,
    `validation_state=${result.state}`,
    `release_scope=${result.productScope ?? "personal_local_desktop"}`,
    `archive_fingerprint=${result.archiveFingerprint ?? "unavailable"}`,
    `source_fingerprint=${result.sourceFingerprint ?? "unavailable"}`,
    "",
    "- phase: `Phase 011.0`",
    "- gate: `Phase 010 Archive And Active Desktop Inventory Validation`",
    `- archived task count: ${result.archivedTaskCount ?? 0}`,
    `- archived gate count: ${result.archivedGateCount ?? 0}`,
    `- archived release evidence count: ${result.archivedReleaseEvidenceCount ?? 0}`,
    "- prerequisite: `.tasks/phase010/phase010-release-gate-result.md` with `phase010_release_gate=passed` and `release_scope=personal_local_desktop`.",
    "- validation commands: `npm run run:phase011-archive-validator-tests`, `npm run run:phase011-archive-validator`.",
    "- changed layers: `task-tooling`, `release-tooling`.",
    "- configuration: explicit repository root only; no environment lookup or mutation.",
    "- logging: Development diagnostics only; no Product Log or Field Debug Log.",
    "- p95 300ms path impact: none; product query paths are not executed.",
    "- sensitive data exclusion: marker names, counts, relative paths, fingerprints, and stable error codes only; no document body, asset content, prompt, answer, credential, secret, or absolute path.",
  ];
  if (!result.passed) {
    lines.push(`- error_code: \`${result.errorCode}\``);
    lines.push(`- finding_id: \`${result.findingId ?? "unknown"}\``);
  }
  lines.push("");
  return lines.join("\n");
}

export function renderPhase011CurrentInventoryArtifact(result) {
  const lines = [
    "# Phase 011 Current Implementation Inventory",
    "",
    "phase011_current_inventory=passed",
    `product_scope=${result.productScope}`,
    `source_fingerprint=${result.sourceFingerprint}`,
    `archive_fingerprint=${result.archiveFingerprint}`,
    "",
    "## Product Commands",
    "",
    "| id | command | responsibility |",
    "| --- | --- | --- |",
    ...result.productCommands.map(
      (item) => `| \`${item.id}\` | \`${item.command}\` | ${item.responsibility} |`,
    ),
    "",
    "## Current Runtime Paths",
    "",
    "| path | responsibility |",
    "| --- | --- |",
    ...result.currentRuntimePaths.map(
      (item) => `| \`${item.path}\` | ${item.responsibility} |`,
    ),
    "",
    "## Active Rust Tests",
    "",
    ...result.activeRustTests.map((path) => `- \`${path}\``),
    "",
    "## Active TypeScript Tests",
    "",
    ...result.activeTypeScriptTests.map((path) => `- \`${path}\``),
    "",
    "## Phase 010 Compatibility Artifacts",
    "",
    ...archiveReleaseEvidence.map((item) => `- \`${item.path}\` with \`${item.marker}\``),
    "",
    "## Future-Scope Exclusions",
    "",
    ...result.futureScopePaths.map((item) => `- \`${item.path}\`: ${item.responsibility}`),
    "",
    "- scope rule: future paths may remain in the repository but are not current product entry points, current settings, or Phase 011 active product evidence.",
    "- test selection rule: active TypeScript paths are explicit and contain no glob patterns.",
    "- sensitive data exclusion: this inventory records relative paths, command ids, responsibilities, marker names, and fingerprints only.",
    "",
  ];
  return lines.join("\n");
}

export function renderPhase011RequirementEvidenceMatrix(result) {
  const lines = [
    "# Phase 011 Requirement Evidence Matrix",
    "",
    "phase011_requirement_evidence=pending",
    `requirement_count=${result.requirementIds.length}`,
    `source_fingerprint=${result.sourceFingerprint}`,
    "",
    "| requirement | status | current evidence | next owner |",
    "| --- | --- | --- | --- |",
    ...result.requirementIds.map((id) => {
      const verified = structureVerifiedRequirementIds.has(id);
      return `| \`${id}\` | \`${verified ? "structure_verified" : "pending"}\` | ${
        verified ? "Task001 archive/inventory structure" : "none"
      } | ${requirementOwner(id)} |`;
    }),
    "",
    "- completion rule: `structure_verified` is not product completion and does not satisfy the final `passed` marker.",
    "- freshness rule: each later gate must replace its owned rows with current test, command, marker, source fingerprint, fixture hash, and platform evidence.",
    "- sensitive data exclusion: no document body, title, query, asset bytes, prompt, answer, credential, secret, or absolute path is recorded.",
    "",
  ];
  return lines.join("\n");
}

async function fingerprintFiles(root, paths) {
  const hash = createHash("sha256");
  for (const path of [...paths].sort()) {
    const body = await readFile(join(root, path));
    hash.update(path);
    hash.update("\0");
    hash.update(body);
    hash.update("\0");
  }
  return hash.digest("hex");
}

async function readOrFail(root, path, errorCode) {
  try {
    return await readFile(join(root, path), "utf8");
  } catch {
    const result = failed(Phase011ArchiveState.ReadingArchive, errorCode, path);
    const error = new Error(errorCode);
    error.phase011ValidationError = true;
    error.result = result;
    throw error;
  }
}

function failed(state, errorCode, findingId) {
  const transition = transitionPhase011ArchiveState(state, Phase011ArchiveEvent.Fail, {
    errorCode,
    findingId,
  });
  return {
    passed: false,
    state: Phase011ArchiveState.Failed,
    errorCode: transition.errorCode,
    findingId: transition.findingId,
    productScope: "personal_local_desktop",
    archivedTaskCount: 0,
    archivedGateCount: archiveGateEvidence.length + 1,
    archivedReleaseEvidenceCount: archiveReleaseEvidence.length,
    requirementIds: [...requirementIds],
  };
}

function evidence(path, marker) {
  return Object.freeze({ path, marker });
}

function pathItem(path, responsibility) {
  return Object.freeze({ path, responsibility });
}

function command(id, commandText, responsibility) {
  return Object.freeze({ id, command: commandText, responsibility });
}

function requirementOwner(id) {
  const owners = {
    "SCOPE-01": "Phase 011.0/011.7",
    "BOOT-01": "Phase 011.1/011.7",
    "HOME-01": "Phase 011.1",
    "NAV-01": "Phase 011.1",
    "DOC-01": "Phase 011.2",
    "DOC-02": "Phase 011.2",
    "DOC-03": "Phase 011.2",
    "HIST-01": "Phase 011.3",
    "HIST-02": "Phase 011.3",
    "DISC-01": "Phase 011.4",
    "DATA-01": "Phase 011.5/011.6",
    "CFG-01": "Phase 011.0/011.2",
    "CFG-02": "Phase 011.5",
    "LOG-01": "Phase 011.0/011.6",
    "STATE-01": "Phase 011.0-011.8",
    "PERF-01": "Phase 011.1-011.4",
    "SEC-01": "Phase 011.2/011.5/011.6",
    "UX-01": "Phase 011.1-011.7",
    "PLAT-01": "Phase 011.7",
    "COMPAT-01": "Phase 011.2/011.3/011.6",
  };
  return owners[id] ?? "Phase 011.8";
}

async function main() {
  const result = await runPhase011ArchiveValidation({
    root: process.cwd(),
    writeArtifacts: true,
  });
  if (!result.passed) {
    console.error(`${result.errorCode} ${result.findingId ?? ""}`.trim());
    process.exit(1);
  }
  console.log("phase011_archive_validation=passed");
  console.log(`source_fingerprint=${result.sourceFingerprint}`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    console.error(error);
    process.exit(1);
  });
}
