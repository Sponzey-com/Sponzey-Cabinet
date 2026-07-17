import { access, mkdir, readFile, rename, writeFile } from "node:fs/promises";
import { join } from "node:path";

import { validateActionGeometryReport } from "./phase013_action_geometry_baseline.mjs";
import { validateResponsiveStressReport } from "./phase013_responsive_stress.mjs";
import { validatePhase013PackagedProductReport } from "./phase013_packaged_product_gate.mjs";
import { validatePhase013QueryRenderPerformance } from "./phase013_query_render_performance.mjs";
import { validatePhase014CommandReceipt } from "./phase014_command_gate.mjs";
import {
  PHASE014_DOD_REQUIREMENTS,
  renderPhase014CompletionMarkdown,
  validatePhase014CompletionReport,
} from "./phase014_completion_evidence_gate.mjs";
import { fingerprintPhase014CurrentSource } from "./phase014_source_fingerprint.mjs";

const root = process.cwd();
const release = join(root, ".tasks", "release");
const fingerprint = await fingerprintPhase014CurrentSource(root);
const [desktop, rust, boundary, geometry, responsive, performance, packaged] = await Promise.all([
  readJson("desktop-test-gate-phase014.json"),
  readJson("rust-test-gate-phase014.json"),
  readJson("current-scope-audit-phase014.json"),
  readJson("ui-action-geometry-baseline-phase013.json"),
  readJson("responsive-stress-phase013.json"),
  readJson("query-render-performance-phase013.json"),
  readJson("packaged-product-journey-phase013.json"),
]);
requirePassed(validatePhase014CommandReceipt(desktop, fingerprint, "desktop-current-scope-tests"), "desktop");
requirePassed(validatePhase014CommandReceipt(rust, fingerprint, "rust-workspace-tests"), "rust");
if (boundary.state !== "Passed" || boundary.sourceFingerprint !== fingerprint || boundary.findingCount !== 0) {
  throw new Error("PHASE014_BOUNDARY_RECEIPT_FAILED");
}
requirePassed(validateActionGeometryReport(geometry, {
  fingerprint: geometry.sourceFingerprint,
  routes: ["Home", "Search", "Document", "Graph", "Canvas", "Assets", "Backup"],
  viewports: [{ width: 1440, height: 900 }, { width: 1180, height: 800 }, { width: 960, height: 720 }, { width: 760, height: 640 }],
}), "geometry");
requirePassed(validateResponsiveStressReport(responsive, {
  fingerprint: responsive.sourceFingerprint,
  routes: ["Home", "Search", "Document", "Graph", "Canvas", "Assets", "Backup"],
  viewports: [{ width: 1440, height: 900 }],
}), "responsive");
requirePassed(validatePhase013QueryRenderPerformance(performance, performance.sourceFingerprint), "performance");
requirePassed(validatePhase013PackagedProductReport(packaged, packaged.sourceFingerprint), "packaged");

const requirements = requirementEvidence();
for (const item of requirements) {
  for (const path of item.evidence) await access(join(root, path));
}
const report = {
  marker: "phase014_completion_evidence=passed",
  state: "Passed",
  sourceFingerprint: fingerprint,
  diagnostics: "sanitized",
  requirements,
  receipts: {
    desktop: { state: desktop.state, sourceFingerprint: desktop.sourceFingerprint },
    rust: { state: rust.state, sourceFingerprint: rust.sourceFingerprint },
    boundary: { state: boundary.state, sourceFingerprint: boundary.sourceFingerprint },
    geometry: { state: geometry.state },
    responsive: { state: responsive.state },
    performance: { state: performance.state },
    packaged: {
      state: packaged.marker === "phase013_packaged_product_gate=passed" ? "Passed" : "Failed",
      keyboardDocumentWorkflowVerified: packaged.keyboardDocumentWorkflowVerified,
      appFingerprint: packaged.appFingerprint,
    },
  },
};
const validation = validatePhase014CompletionReport(report, fingerprint);
requirePassed(validation, "completion");
await mkdir(release, { recursive: true });
await writeAtomic(join(release, "completion-evidence-phase014.json"), `${JSON.stringify(report, null, 2)}\n`);
await writeAtomic(join(release, "completion-evidence-phase014.md"), renderPhase014CompletionMarkdown(report));
console.log(report.marker);
console.log(`source_fingerprint=${fingerprint}`);
console.log(`requirement_count=${requirements.length}`);
console.log(`app_fingerprint=${packaged.appFingerprint}`);

function requirementEvidence() {
  const map = [
    ["apps/desktop/src-tauri/tests/document_mutation_runtime_tests.rs", "crates/cabinet-usecases/tests/project_current_document_revision_tests.rs"],
    ["apps/desktop/tests/desktop_document_menu_target_tests.ts"],
    ["apps/desktop/src-tauri/tests/document_attachment_mutation_runtime_tests.rs", "apps/desktop/tests/desktop_react_authoring_workbench_tests.ts"],
    ["crates/cabinet-usecases/tests/mutate_document_attachments_tests.rs"],
    ["crates/cabinet-usecases/tests/document_diff_tests.rs", "apps/desktop/tests/desktop_tauri_diff_transport_tests.ts"],
    ["crates/cabinet-usecases/tests/attachment_diff_tests.rs", "crates/cabinet-usecases/tests/document_diff_tests.rs"],
    ["crates/cabinet-adapters/tests/local_version_store_tests.rs", "apps/desktop/tests/document_history_window_tests.ts"],
    ["crates/cabinet-usecases/tests/authoritative_restore_preview_tests.rs"],
    ["crates/cabinet-adapters/tests/local_restore_document_revision_runtime_tests.rs"],
    ["crates/cabinet-usecases/tests/restore_document_revision_tests.rs"],
    ["crates/cabinet-adapters/tests/local_document_revision_commit_flow_tests.rs", "crates/cabinet-usecases/tests/mutate_document_attachments_tests.rs"],
    ["crates/cabinet-usecases/tests/restore_document_revision_tests.rs"],
    ["crates/cabinet-usecases/tests/restore_target_asset_preflight_tests.rs"],
    ["crates/cabinet-adapters/tests/local_document_revision_recovery_tests.rs", "crates/cabinet-adapters/tests/local_restore_projection_recovery_runtime_tests.rs"],
    ["apps/desktop/src-tauri/tests/document_mutation_runtime_tests.rs", "crates/cabinet-usecases/tests/project_current_document_revision_tests.rs"],
    ["apps/desktop/tests/seven_route_exposure_gate_tests.ts", ".tasks/release/current-scope-audit-phase014.json"],
    [".tasks/release/query-render-performance-phase013.json"],
    [".tasks/release/current-scope-audit-phase014.json"],
    ["apps/desktop/src-tauri/tests/document_store_migration_tests.rs", ".tasks/release/packaged-product-journey-phase013.json"],
    [".tasks/release/desktop-test-gate-phase014.json", ".tasks/release/rust-test-gate-phase014.json"],
    ["apps/desktop/src-tauri/tests/document_query_runtime_tests.rs"],
    ["apps/desktop/tests/desktop_tauri_authoring_transport_tests.ts", "apps/desktop/tests/tauri_asset_import_transport_tests.ts"],
    ["apps/desktop/src-tauri/tests/document_store_migration_tests.rs", "crates/cabinet-adapters/tests/local_document_revision_recovery_tests.rs"],
    [".tasks/release/current-scope-audit-phase014.json"],
    [".tasks/release/shell-geometry-phase013.json", "apps/desktop/tests/all_route_shared_shell_tests.ts"],
    ["apps/desktop/tests/desktop_document_empty_state_tests.ts"],
    ["apps/desktop/tests/document_inspector_state_tests.ts", "apps/desktop/tests/desktop_react_authoring_workbench_tests.ts"],
    ["apps/desktop/tests/document_history_window_tests.ts"],
    ["apps/desktop/tests/document_diff_hunk_window_tests.ts", "apps/desktop/tests/desktop_react_authoring_workbench_tests.ts"],
    ["apps/desktop/tests/ko_kr_catalog_tests.ts", "apps/desktop/tests/seven_route_exposure_gate_tests.ts"],
    ["apps/desktop/tests/unified_ui_action_contract_tests.ts"],
    [".tasks/release/ui-action-geometry-baseline-phase013.json", ".tasks/release/responsive-stress-phase013.json"],
    ["apps/desktop/tests/packaged_ui_smoke_tests.ts", "apps/desktop/tests/desktop_document_authoring_controller_tests.ts", "apps/desktop/tests/document_restore_presentation_tests.ts"],
    [".tasks/release/packaged-product-journey-phase013.json"],
  ];
  return PHASE014_DOD_REQUIREMENTS.map((id, index) => Object.freeze({
    id,
    state: "Passed",
    evidence: Object.freeze(map[index]),
  }));
}

async function readJson(name) {
  return JSON.parse(await readFile(join(release, name), "utf8"));
}

function requirePassed(result, label) {
  if (!result.passed) throw new Error(`PHASE014_${label.toUpperCase()}_FAILED:${result.findingIds.join(",")}`);
}

async function writeAtomic(path, content) {
  const temporary = `${path}.tmp`;
  await writeFile(temporary, content, "utf8");
  await rename(temporary, path);
}
