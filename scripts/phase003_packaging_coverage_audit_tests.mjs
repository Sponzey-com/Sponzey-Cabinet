import assert from "node:assert/strict";
import test from "node:test";

import {
  PackagingCoverageAuditErrorCode,
  PackagingCoverageAuditEvent,
  PackagingCoverageAuditState,
  analyzePackagingCoverageSources,
  renderPackagingCoverageAuditMarkdown,
  transitionPackagingCoverageAuditState,
} from "./phase003_packaging_coverage_audit.mjs";

const COMPLETE_SOURCES = Object.freeze({
  "scripts/run_self_host_server_package_smoke.sh":
    "node scripts/run_self_host_server_package_smoke.mjs",
  "scripts/run_self_host_server_package_smoke.mjs":
    "cargo build -p cabinet-server cabinet-server --self-host-package-smoke server_package_smoke=passed assertSensitiveOutputClean",
  "scripts/run_browser_smoke.sh": "node scripts/build_web_app.mjs node scripts/run_browser_smoke.mjs",
  "scripts/run_browser_smoke.mjs":
    "scripts/run_web_app.mjs waitForHttp browser_smoke=passed CodeMirror editor mounted Markdown preview table rendered",
  "scripts/run_web_app.mjs": "createServer web app static serving index.html app.bundle.js",
  "scripts/build_web_app.mjs": "esbuild apps/web/public/app.js apps/web/public/app.bundle.js",
  "scripts/build_desktop_assets.mjs":
    "node ./build_web_app.mjs apps/desktop/dist index.html styles.css app.bundle.js",
  "scripts/run_desktop_package_smoke.sh":
    "node scripts/build_desktop_assets.mjs cargo build -p cabinet-desktop-shell --packaged-smoke",
  "scripts/run_desktop_tauri_build.sh":
    "node scripts/build_desktop_assets.mjs tauri build --debug --bundles app --no-sign --ci",
  "scripts/run_desktop_packaged_app_smoke.sh":
    "scripts/run_desktop_tauri_build.sh packaged_app_binary_found=true --packaged-smoke",
  "scripts/run_desktop_dist_browser_smoke.sh":
    "node scripts/build_desktop_assets.mjs SPONZEY_CABINET_WEB_PUBLIC_DIR=apps/desktop/dist scripts/run_browser_smoke.sh",
  "apps/desktop/src-tauri/src/main.rs": "--packaged-smoke run_packaged_smoke",
  "apps/desktop/src-tauri/src/lib.rs":
    "create_desktop_package_smoke_report packaged_runtime_smoke_does_not_require_node",
  "scripts/run_local_app.sh": "cargo run --quiet -p cabinet-platform --bin cabinet-local",
  "crates/cabinet-platform/src/bin/cabinet_local.rs":
    "run_clean_install_smoke first_run_completed setup_healthy already_present_directories",
  "crates/cabinet-platform/src/release_smoke.rs":
    "run_clean_install_smoke run_data_preservation_smoke run_phase002_migration_fixture_smoke",
  "crates/cabinet-platform/tests/clean_install_smoke.rs":
    "clean_install_smoke_initializes_local_profile_once_without_external_services without_external_services created_directories already_present_directories",
  "crates/cabinet-platform/tests/data_preservation_smoke.rs":
    "local_data_preservation_smoke_keeps_documents_versions_and_assets_after_reinit migration_idempotent",
  "crates/cabinet-platform/tests/phase002_migration_fixture_smoke.rs":
    "phase002_migration_fixture_smoke_preserves_self_host_runtime_records migration_failure_preserved_current_fixture",
  "crates/cabinet-core/src/migration.rs": "MigrationState MigrationEvent MigrationRunner",
  "crates/cabinet-core/tests/migration_tests.rs":
    "migration_transitions_to_completed_through_explicit_events migration_runner_is_idempotent_when_initial_version_is_already_recorded",
  "scripts/run_self_host_upgrade_smoke.sh": "node scripts/run_self_host_upgrade_smoke.mjs",
  "scripts/run_self_host_upgrade_smoke.mjs":
    "run_self_host_upgrade_smoke migration_state_machine upgrade_migration_smoke=passed assertSensitiveOutputClean",
});

test("packaging coverage audit marks complete fixture as covered", () => {
  const audit = analyzePackagingCoverageSources({ sources: COMPLETE_SOURCES });

  assert.equal(audit.phase, "Phase 003.4");
  assert.equal(audit.summary.totalTargets, 5);
  assert.equal(audit.summary.covered, 5);
  assert.equal(audit.summary.partial, 0);
  assert.equal(audit.summary.missing, 0);
  assert.equal(audit.summary.targetsNeedingWork, 0);
  assert.equal(audit.nextImplementationTarget, null);
});

test("packaging coverage audit classifies upgrade command gap as partial next target", () => {
  const {
    "scripts/run_self_host_upgrade_smoke.sh": _upgradeRunner,
    "scripts/run_self_host_upgrade_smoke.mjs": _upgradeScript,
    ...sources
  } = COMPLETE_SOURCES;

  const audit = analyzePackagingCoverageSources({ sources });
  const upgrade = audit.targets.find((target) => target.id === "upgrade_migration_command_flow");

  assert.equal(upgrade.status, "partial");
  assert.equal(audit.summary.partial, 1);
  assert.equal(audit.summary.targetsNeedingWork, 1);
  assert.equal(audit.nextImplementationTarget.id, "upgrade_migration_command_flow");
});

test("packaging coverage audit reports missing server package smoke when package runner is absent", () => {
  const {
    "scripts/run_self_host_server_package_smoke.sh": _runner,
    "scripts/run_self_host_server_package_smoke.mjs": _script,
    ...sources
  } = COMPLETE_SOURCES;

  const audit = analyzePackagingCoverageSources({ sources });
  const serverPackage = audit.targets.find((target) => target.id === "server_package_smoke");

  assert.equal(serverPackage.status, "missing");
  assert.equal(
    serverPackage.missingFiles.includes("scripts/run_self_host_server_package_smoke.mjs"),
    true,
  );
  assert.equal(audit.nextImplementationTarget.id, "server_package_smoke");
});

test("packaging coverage audit fails with stable code when source set is empty", () => {
  assert.throws(
    () => analyzePackagingCoverageSources({ sources: {} }),
    (error) => error.code === PackagingCoverageAuditErrorCode.SourceSetEmpty,
  );
});

test("packaging coverage audit state machine rejects invalid transitions", () => {
  assert.throws(
    () =>
      transitionPackagingCoverageAuditState(
        PackagingCoverageAuditState.Pending,
        PackagingCoverageAuditEvent.ReportWritten,
      ),
    (error) => error.code === PackagingCoverageAuditErrorCode.InvalidTransition,
  );
});

test("packaging coverage audit markdown records summary, target status, and next target", () => {
  const {
    "scripts/run_self_host_upgrade_smoke.sh": _upgradeRunner,
    "scripts/run_self_host_upgrade_smoke.mjs": _upgradeScript,
    ...sources
  } = COMPLETE_SOURCES;
  const audit = analyzePackagingCoverageSources({ sources });
  const markdown = renderPackagingCoverageAuditMarkdown(audit);

  assert.match(markdown, /Phase 003 Packaging Coverage Audit/);
  assert.match(markdown, /upgrade_migration_command_flow/);
  assert.match(markdown, /partial/);
  assert.match(markdown, /targets needing work/);
});
