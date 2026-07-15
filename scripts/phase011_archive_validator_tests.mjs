import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";

import {
  Phase011ArchiveErrorCode,
  Phase011ArchiveEvent,
  Phase011ArchiveState,
  renderPhase011ArchiveValidationArtifact,
  renderPhase011CurrentInventoryArtifact,
  renderPhase011RequirementEvidenceMatrix,
  runPhase011ArchiveValidation,
  transitionPhase011ArchiveState,
  validatePhase011InventoryArtifactFreshness,
} from "./phase011_archive_validator.mjs";

const requirementIds = [
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
];

test("phase011 archive validator rejects missing phase010 predecessor file", async () => {
  const root = await createFixture();
  await rm(join(root, ".tasks/phase010/task004.md"));

  const result = await runPhase011ArchiveValidation({ root, writeArtifacts: false });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase011ArchiveErrorCode.ArchiveTaskGap);
  assert.equal(result.findingId, ".tasks/phase010/task004.md");
});

test("phase011 archive validator rejects invalid phase010 release marker", async () => {
  const root = await createFixture({ releaseMarker: "phase010_release_gate=failed\n" });

  const result = await runPhase011ArchiveValidation({ root, writeArtifacts: false });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase011ArchiveErrorCode.Phase010ReleaseMarkerMissing);
  assert.equal(result.findingId, ".tasks/phase010/phase010-release-gate-result.md");
});

test("phase011 inventory rejects future-scope default activation", async () => {
  const root = await createFixture({ remoteEnabledByDefault: true });

  const result = await runPhase011ArchiveValidation({ root, writeArtifacts: false });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase011ArchiveErrorCode.FutureScopeActivated);
  assert.equal(result.findingId, "packages/client-core/src/index.ts");
});

test("phase011 inventory rejects a missing explicit active path", async () => {
  const root = await createFixture();
  await rm(join(root, "packages/editor/tests/source_editing_command_tests.ts"));

  const result = await runPhase011ArchiveValidation({ root, writeArtifacts: false });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase011ArchiveErrorCode.RequiredPathMissing);
  assert.equal(result.findingId, "packages/editor/tests/source_editing_command_tests.ts");
});

test("phase011 inventory freshness rejects stale source fingerprint", () => {
  const findings = validatePhase011InventoryArtifactFreshness(
    "source_fingerprint=old-fingerprint\n",
    "current-fingerprint",
  );

  assert.deepEqual(findings, [
    {
      errorCode: Phase011ArchiveErrorCode.SourceFingerprintMismatch,
      findingId: "source_fingerprint",
    },
  ]);
});

test("phase011 archive validator rejects duplicate requirement ids", async () => {
  const root = await createFixture({ duplicateRequirement: true });

  const result = await runPhase011ArchiveValidation({ root, writeArtifacts: false });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase011ArchiveErrorCode.RequirementRegisterInvalid);
  assert.equal(result.findingId, "SCOPE-01");
});

test("phase011 archive validator passes complete fixture and renders sanitized evidence", async () => {
  const root = await createFixture();

  const result = await runPhase011ArchiveValidation({ root, writeArtifacts: false });
  const archiveArtifact = renderPhase011ArchiveValidationArtifact(result);
  const inventoryArtifact = renderPhase011CurrentInventoryArtifact(result);
  const evidenceMatrix = renderPhase011RequirementEvidenceMatrix(result);

  assert.equal(result.passed, true);
  assert.equal(result.state, Phase011ArchiveState.Passed);
  assert.equal(result.archivedTaskCount, 8);
  assert.equal(result.archivedGateCount, 9);
  assert.equal(result.archivedReleaseEvidenceCount, 6);
  assert.equal(result.requirementIds.length, 20);
  assert.match(result.sourceFingerprint, /^[a-f0-9]{64}$/);
  assert.match(result.archiveFingerprint, /^[a-f0-9]{64}$/);
  assert.match(archiveArtifact, /phase011_archive_validation=passed/);
  assert.match(archiveArtifact, /release_scope=personal_local_desktop/);
  assert.match(inventoryArtifact, /phase011_current_inventory=passed/);
  assert.match(inventoryArtifact, /Active Rust Tests/);
  assert.match(inventoryArtifact, /crates\/cabinet-platform\/tests\/local_desktop_command_runtime_tests\.rs/);
  assert.match(inventoryArtifact, /crates\/cabinet-ports\/src\/workspace_home\.rs/);
  assert.match(inventoryArtifact, /crates\/cabinet-usecases\/src\/workspace_home\.rs/);
  assert.match(inventoryArtifact, /crates\/cabinet-usecases\/tests\/get_workspace_home_tests\.rs/);
  assert.match(inventoryArtifact, /crates\/cabinet-adapters\/src\/local_workspace_home_projection\.rs/);
  assert.match(inventoryArtifact, /crates\/cabinet-adapters\/tests\/local_workspace_home_projection_tests\.rs/);
  assert.match(inventoryArtifact, /crates\/cabinet-platform\/src\/workspace_home_command\.rs/);
  assert.match(inventoryArtifact, /crates\/cabinet-platform\/tests\/workspace_home_command_executor_tests\.rs/);
  assert.match(inventoryArtifact, /apps\/desktop\/src-tauri\/tests\/workspace_home_runtime_tests\.rs/);
  assert.match(inventoryArtifact, /crates\/cabinet-usecases\/src\/workspace_home_update\.rs/);
  assert.match(inventoryArtifact, /crates\/cabinet-usecases\/tests\/update_workspace_home_projection_tests\.rs/);
  assert.match(inventoryArtifact, /crates\/cabinet-adapters\/tests\/local_workspace_home_mutation_tests\.rs/);
  assert.match(inventoryArtifact, /crates\/cabinet-ports\/src\/document_navigator\.rs/);
  assert.match(inventoryArtifact, /crates\/cabinet-usecases\/src\/document_navigator\.rs/);
  assert.match(inventoryArtifact, /crates\/cabinet-usecases\/tests\/document_navigator_tests\.rs/);
  assert.match(inventoryArtifact, /crates\/cabinet-adapters\/src\/local_document_navigator_projection\.rs/);
  assert.match(inventoryArtifact, /crates\/cabinet-adapters\/tests\/local_document_navigator_projection_tests\.rs/);
  assert.match(inventoryArtifact, /crates\/cabinet-platform\/src\/document_navigator_command\.rs/);
  assert.match(inventoryArtifact, /crates\/cabinet-platform\/tests\/document_navigator_command_executor_tests\.rs/);
  assert.match(inventoryArtifact, /apps\/desktop\/src-tauri\/src\/main\.rs/);
  assert.match(inventoryArtifact, /apps\/desktop\/src-tauri\/tests\/document_navigator_runtime_tests\.rs/);
  assert.match(inventoryArtifact, /apps\/desktop\/src\/tauri_navigator_transport\.ts/);
  assert.match(inventoryArtifact, /apps\/desktop\/tests\/desktop_tauri_navigator_transport_tests\.ts/);
  assert.match(inventoryArtifact, /packages\/client-core\/tests\/document_navigator_command_client_tests\.ts/);
  assert.match(inventoryArtifact, /apps\/desktop\/tests\/desktop_tauri_authoring_transport_tests\.ts/);
  assert.match(inventoryArtifact, /packages\/client-core\/tests\/document_authoring_command_client_tests\.ts/);
  assert.match(inventoryArtifact, /apps\/desktop\/src\/desktop_navigator_controller\.ts/);
  assert.match(inventoryArtifact, /apps\/desktop\/src\/desktop_document_authoring_controller\.ts/);
  assert.match(inventoryArtifact, /apps\/desktop\/src\/codemirror_document_editor\.ts/);
  assert.match(inventoryArtifact, /apps\/desktop\/src\/react_document_authoring_workbench\.ts/);
  assert.match(inventoryArtifact, /apps\/desktop\/src\/desktop_revision_metadata_generator\.ts/);
  assert.match(inventoryArtifact, /apps\/desktop\/src\/react_document_navigator\.ts/);
  assert.match(inventoryArtifact, /apps\/desktop\/src\/tauri_desktop_transport\.ts/);
  assert.match(inventoryArtifact, /apps\/desktop\/src\/tauri_authoring_transport\.ts/);
  assert.match(inventoryArtifact, /packages\/ui\/tests\/document_navigator_model_tests\.ts/);
  assert.match(inventoryArtifact, /apps\/desktop\/tests\/desktop_navigator_controller_tests\.ts/);
  assert.match(inventoryArtifact, /apps\/desktop\/tests\/desktop_document_authoring_controller_tests\.ts/);
  assert.match(inventoryArtifact, /apps\/desktop\/tests\/desktop_codemirror_adapter_contract_tests\.ts/);
  assert.match(inventoryArtifact, /apps\/desktop\/tests\/desktop_react_authoring_workbench_tests\.ts/);
  assert.match(inventoryArtifact, /apps\/desktop\/tests\/desktop_revision_metadata_generator_tests\.ts/);
  assert.match(inventoryArtifact, /apps\/desktop\/tests\/desktop_entry_authoring_contract_tests\.ts/);
  assert.match(inventoryArtifact, /apps\/desktop\/tests\/desktop_react_navigator_render_tests\.ts/);
  assert.match(inventoryArtifact, /packages\/editor\/tests\/revision_safe_editor_session_tests\.ts/);
  assert.match(inventoryArtifact, /packages\/ui\/tests\/revision_safe_save_coordinator_tests\.ts/);
  assert.match(inventoryArtifact, /crates\/cabinet-ports\/src\/current_document_version\.rs/);
  assert.match(inventoryArtifact, /crates\/cabinet-usecases\/src\/guarded_authoring\.rs/);
  assert.match(inventoryArtifact, /crates\/cabinet-usecases\/tests\/guarded_authoring_tests\.rs/);
  assert.match(inventoryArtifact, /crates\/cabinet-adapters\/src\/local_current_document_version_pointer\.rs/);
  assert.match(inventoryArtifact, /crates\/cabinet-adapters\/tests\/local_current_document_version_pointer_tests\.rs/);
  assert.match(inventoryArtifact, /crates\/cabinet-platform\/src\/document_authoring_command\.rs/);
  assert.match(inventoryArtifact, /crates\/cabinet-platform\/tests\/document_authoring_command_executor_tests\.rs/);
  assert.match(inventoryArtifact, /apps\/desktop\/src-tauri\/tests\/document_authoring_runtime_tests\.rs/);
  assert.match(inventoryArtifact, /apps\/desktop\/src\/desktop_entry\.ts/);
  assert.match(inventoryArtifact, /apps\/desktop\/src\/react_workspace_home\.ts/);
  assert.match(inventoryArtifact, /apps\/desktop\/src\/tauri_home_transport\.ts/);
  assert.match(inventoryArtifact, /apps\/desktop\/tests\/desktop_react_home_render_tests\.ts/);
  assert.match(inventoryArtifact, /apps\/desktop\/tests\/desktop_tauri_home_transport_tests\.ts/);
  assert.match(inventoryArtifact, /scripts\/phase011_workspace_home_gate\.mjs/);
  assert.match(inventoryArtifact, /scripts\/phase011_authoring_browser\.mjs/);
  assert.match(inventoryArtifact, /scripts\/run_phase011_authoring_browser\.mjs/);
  assert.match(inventoryArtifact, /scripts\/phase011_document_authoring_gate\.mjs/);
  assert.match(inventoryArtifact, /scripts\/phase011_history_restore_gate\.mjs/);
  assert.match(inventoryArtifact, /scripts\/phase011_discovery_gate\.mjs/);
  assert.match(inventoryArtifact, /scripts\/phase011_data_settings_gate\.mjs/);
  assert.match(inventoryArtifact, /scripts\/phase011_recovery_observability_gate\.mjs/);
  assert.match(inventoryArtifact, /crates\/cabinet-platform\/src\/bin\/workspace_home_benchmark\.rs/);
  assert.match(inventoryArtifact, /Active TypeScript Tests/);
  assert.match(inventoryArtifact, /Future-Scope Exclusions/);
  assert.match(evidenceMatrix, /phase011_requirement_evidence=pending/);
  assert.match(evidenceMatrix, /\| `SCOPE-01` \| `structure_verified` \|/);
  assert.match(evidenceMatrix, /\| `HOME-01` \| `pending` \|/);
  for (const artifact of [archiveArtifact, inventoryArtifact, evidenceMatrix]) {
    assert.doesNotMatch(artifact, /raw_document_body_fixture/);
    assert.doesNotMatch(artifact, /provider_api_key_fixture/);
    assert.doesNotMatch(artifact, /personal_absolute_path_fixture/);
    assert.equal(artifact.includes(root), false);
  }
});

test("phase011 archive validator writes all task001 artifacts under explicit root", async () => {
  const root = await createFixture();

  const result = await runPhase011ArchiveValidation({ root, writeArtifacts: true });
  const archiveArtifact = await readFile(
    join(root, ".tasks/phase011-archive-validation-result.md"),
    "utf8",
  );
  const inventoryArtifact = await readFile(
    join(root, ".tasks/phase011-current-implementation-inventory.md"),
    "utf8",
  );
  const evidenceMatrix = await readFile(
    join(root, ".tasks/release/requirement-evidence-matrix-phase011.md"),
    "utf8",
  );

  assert.equal(result.passed, true);
  assert.match(archiveArtifact, /validation_state=Passed/);
  assert.match(inventoryArtifact, new RegExp(`source_fingerprint=${result.sourceFingerprint}`));
  assert.match(evidenceMatrix, /requirement_count=20/);
});

test("phase011 archive state machine exposes success, failure, and invalid transition", () => {
  const reading = transitionPhase011ArchiveState(
    Phase011ArchiveState.Pending,
    Phase011ArchiveEvent.Start,
  );
  const validating = transitionPhase011ArchiveState(
    reading.state,
    Phase011ArchiveEvent.ArchiveRead,
  );
  const rendering = transitionPhase011ArchiveState(
    validating.state,
    Phase011ArchiveEvent.InventoryValidated,
  );
  const writing = transitionPhase011ArchiveState(
    rendering.state,
    Phase011ArchiveEvent.EvidenceRendered,
  );
  const passed = transitionPhase011ArchiveState(
    writing.state,
    Phase011ArchiveEvent.ResultWritten,
  );
  const failed = transitionPhase011ArchiveState(reading.state, Phase011ArchiveEvent.Fail, {
    errorCode: Phase011ArchiveErrorCode.ArchiveTaskGap,
    findingId: ".tasks/phase010/task004.md",
  });
  const invalid = transitionPhase011ArchiveState(
    Phase011ArchiveState.Pending,
    Phase011ArchiveEvent.InventoryValidated,
  );

  assert.equal(reading.state, Phase011ArchiveState.ReadingArchive);
  assert.equal(validating.state, Phase011ArchiveState.ValidatingInventory);
  assert.equal(rendering.state, Phase011ArchiveState.RenderingEvidence);
  assert.equal(writing.state, Phase011ArchiveState.WritingResult);
  assert.equal(passed.state, Phase011ArchiveState.Passed);
  assert.equal(failed.state, Phase011ArchiveState.Failed);
  assert.equal(failed.findingId, ".tasks/phase010/task004.md");
  assert.equal(invalid.errorCode, Phase011ArchiveErrorCode.InvalidTransition);
});

async function createFixture({
  releaseMarker = "phase010_release_gate=passed\nrelease_scope=personal_local_desktop\n",
  remoteEnabledByDefault = false,
  duplicateRequirement = false,
} = {}) {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase011-archive-"));
  const files = new Map();

  files.set(
    ".tasks/plan.md",
    [
      "# Phase 011 Development Plan",
      "",
      "Current product scope marker: `personal_local_desktop`.",
      "",
      ...requirementIds.map((id) => `| \`${id}\` | requirement | evidence |`),
      ...(duplicateRequirement ? ["| `SCOPE-01` | duplicate | evidence |"] : []),
      "",
    ].join("\n"),
  );
  files.set("PROJECT.md", "현재 최종 목표: 개인 사용자의 개인 PC 설치형 앱\n현재 공식 대상 플랫폼: Windows, macOS, Linux\n");
  files.set("AGENTS.md", "Layered Architecture\nClean Architecture\nTidy First\nTDD\nProduct Log\nField Debug Log\nDevelopment Log\nState Machine\n");
  files.set(
    "package.json",
    JSON.stringify({
      scripts: {
        "run:desktop-app": "sh scripts/run_desktop_app.sh",
        "run:desktop-package-smoke": "sh scripts/run_desktop_package_smoke.sh",
        "run:desktop-packaged-app-smoke": "sh scripts/run_desktop_packaged_app_smoke.sh",
        "run:phase011-workspace-home-gate-tests": "sh scripts/run_phase011_workspace_home_gate_tests.sh",
        "run:phase011-workspace-home-visual": "sh scripts/run_phase011_workspace_home_visual.sh",
        "run:phase011-workspace-home-performance": "sh scripts/run_phase011_workspace_home_performance.sh",
        "run:phase011-authoring-browser": "sh scripts/run_phase011_authoring_browser.sh",
        "run:phase011-document-authoring-gate-tests": "sh scripts/run_phase011_document_authoring_gate_tests.sh",
        "run:phase011-document-authoring-gate": "sh scripts/run_phase011_document_authoring_gate.sh",
        "run:phase011-history-restore-gate-tests": "sh scripts/run_phase011_history_restore_gate_tests.sh",
        "run:phase011-history-restore-gate": "sh scripts/run_phase011_history_restore_gate.sh",
        "run:phase011-discovery-gate-tests": "sh scripts/run_phase011_discovery_gate_tests.sh",
        "run:phase011-discovery-gate": "sh scripts/run_phase011_discovery_gate.sh",
        "run:phase011-data-settings-gate-tests": "sh scripts/run_phase011_data_settings_gate_tests.sh",
        "run:phase011-data-settings-gate": "sh scripts/run_phase011_data_settings_gate.sh",
        "run:phase011-recovery-observability-gate-tests": "sh scripts/run_phase011_recovery_observability_gate_tests.sh",
        "run:phase011-recovery-observability-gate": "sh scripts/run_phase011_recovery_observability_gate.sh",
        "run:phase011-native-platform-evidence-tests": "sh scripts/run_phase011_native_platform_evidence_tests.sh",
        "run:phase011-native-platform-evidence": "sh scripts/run_phase011_native_platform_evidence.sh",
        "run:phase011-product-smoke-gate-tests": "sh scripts/run_phase011_product_smoke_gate_tests.sh",
        "run:phase011-product-smoke-gate": "sh scripts/run_phase011_product_smoke_gate.sh",
        "run:phase011-release-gate-tests": "sh scripts/run_phase011_release_gate_tests.sh",
        "run:phase011-release-gate": "sh scripts/run_phase011_release_gate.sh",
        "run:phase011-workspace-home-gate": "sh scripts/run_phase011_workspace_home_gate.sh",
      },
    }),
  );
  files.set(
    "packages/client-core/src/index.ts",
    [
      'productScope: "personal_local_desktop"',
      `supportsRemoteWorkspace: ${remoteEnabledByDefault ? "true" : "false"}`,
      'platforms: ["windows", "macos", "linux"]',
      '"server-url", "tenant-admin", "team-invite", "billing", "admin-console"',
    ].join("\n"),
  );
  files.set(
    "apps/desktop/src/index.ts",
    "export const desktopShell = createDesktopCurrentProductShellDescriptor();\n",
  );
  files.set(
    "apps/desktop/src-tauri/src/lib.rs",
    'const LOCAL_DESKTOP_COMMAND_NAMES: &[&str] = &["local_workspace_home", "get_current_document"];\n',
  );

  for (const path of requiredCurrentPaths()) {
    if (!files.has(path)) files.set(path, `${path}\n`);
  }
  for (const path of explicitActiveTestPaths()) {
    files.set(path, `${path}\n`);
  }
  for (const path of explicitActiveRustTestPaths()) {
    files.set(path, `${path}\n`);
  }
  files.set("crates/cabinet-server/src/lib.rs", "future server scope\n");
  files.set("apps/mobile/src/index.ts", "future mobile scope\n");

  files.set(".tasks/phase010/plan.md", "# Phase 010 Development Plan\n");
  files.set(".tasks/phase010/readme.md", "Active phase: Phase 010\nCurrent product scope: `personal_local_desktop`\n");
  for (let index = 1; index <= 8; index += 1) {
    files.set(`.tasks/phase010/task${String(index).padStart(3, "0")}.md`, `# Task ${index}\n`);
  }
  for (const [path, marker] of archiveEvidence()) files.set(path, `${marker}\n`);
  files.set(".tasks/phase010/phase010-release-gate-result.md", releaseMarker);

  for (const [path, contents] of files) {
    await mkdir(join(root, dirname(path)), { recursive: true });
    await writeFile(join(root, path), contents);
  }
  return root;
}

function requiredCurrentPaths() {
  return [
    "scripts/run_desktop_app.sh",
    "scripts/run_desktop_package_smoke.sh",
    "scripts/run_desktop_packaged_app_smoke.sh",
    "scripts/build_desktop_assets.mjs",
    "apps/desktop/src/index.ts",
    "apps/desktop/src/desktop_entry.ts",
    "apps/desktop/package.json",
    "apps/desktop/src/react_workspace_home.ts",
    "apps/desktop/src/react_document_navigator.ts",
    "apps/desktop/src/desktop_navigator_controller.ts",
    "apps/desktop/src/desktop_document_authoring_controller.ts",
    "apps/desktop/src/codemirror_document_editor.ts",
    "apps/desktop/src/react_document_authoring_workbench.ts",
    "apps/desktop/src/desktop_revision_metadata_generator.ts",
    "apps/desktop/src/tauri_desktop_transport.ts",
    "apps/desktop/src/tauri_authoring_transport.ts",
    "apps/desktop/src/tauri_home_transport.ts",
    "apps/desktop/src/tauri_navigator_transport.ts",
    "apps/desktop/public/index.html",
    "apps/desktop/public/styles.css",
    "scripts/phase011_desktop_asset_builder.mjs",
    "scripts/phase011_workspace_home_visual.mjs",
    "scripts/phase011_authoring_browser.mjs",
    "scripts/phase011_workspace_home_performance.mjs",
    "scripts/phase011_document_authoring_gate.mjs",
    "scripts/phase011_history_restore_gate.mjs",
    "scripts/phase011_discovery_gate.mjs",
    "scripts/phase011_data_settings_gate.mjs",
    "scripts/phase011_recovery_observability_gate.mjs",
    "scripts/phase011_workspace_home_gate.mjs",
    "scripts/run_phase011_workspace_home_visual.mjs",
    "scripts/run_phase011_workspace_home_visual.sh",
    "scripts/run_phase011_authoring_browser.mjs",
    "scripts/run_phase011_authoring_browser.sh",
    "scripts/run_phase011_document_authoring_gate_tests.sh",
    "scripts/run_phase011_document_authoring_gate.sh",
    "scripts/run_phase011_history_restore_gate_tests.sh",
    "scripts/run_phase011_history_restore_gate.sh",
    "scripts/run_phase011_discovery_gate_tests.sh",
    "scripts/run_phase011_discovery_gate.sh",
    "scripts/run_phase011_data_settings_gate_tests.sh",
    "scripts/run_phase011_data_settings_gate.sh",
    "scripts/run_phase011_recovery_observability_gate_tests.sh",
    "scripts/run_phase011_recovery_observability_gate.sh",
    "scripts/run_phase011_workspace_home_performance.mjs",
    "scripts/run_phase011_workspace_home_performance.sh",
    "scripts/run_phase011_workspace_home_gate_tests.sh",
    "scripts/run_phase011_workspace_home_gate.sh",
    "crates/cabinet-platform/src/bin/workspace_home_benchmark.rs",
    "apps/desktop/src-tauri/src/lib.rs",
    "apps/desktop/src-tauri/src/main.rs",
    "apps/desktop/src-tauri/tauri.conf.json",
    "packages/client-core/src/index.ts",
    "packages/ui/src/index.ts",
    "packages/editor/src/index.ts",
    "crates/cabinet-platform/src/local_desktop_runtime.rs",
    "crates/cabinet-platform/src/release_smoke.rs",
    "crates/cabinet-platform/src/workspace_home_command.rs",
    "crates/cabinet-platform/src/document_navigator_command.rs",
    "crates/cabinet-platform/src/document_authoring_command.rs",
    "crates/cabinet-usecases/src/document.rs",
    "crates/cabinet-usecases/src/guarded_authoring.rs",
    "crates/cabinet-usecases/src/search.rs",
    "crates/cabinet-usecases/src/graph.rs",
    "crates/cabinet-usecases/src/backup.rs",
    "crates/cabinet-usecases/src/import.rs",
    "crates/cabinet-ports/src/lib.rs",
    "crates/cabinet-ports/src/current_document_version.rs",
    "crates/cabinet-ports/src/workspace_home.rs",
    "crates/cabinet-ports/src/document_navigator.rs",
    "crates/cabinet-usecases/src/workspace_home.rs",
    "crates/cabinet-usecases/src/document_navigator.rs",
    "crates/cabinet-usecases/src/workspace_home_update.rs",
    "crates/cabinet-adapters/src/local_document_repository.rs",
    "crates/cabinet-adapters/src/local_version_store.rs",
    "crates/cabinet-adapters/src/local_current_document_version_pointer.rs",
    "crates/cabinet-adapters/src/local_search_index.rs",
    "crates/cabinet-adapters/src/local_link_index.rs",
    "crates/cabinet-adapters/src/local_graph_projection.rs",
    "crates/cabinet-adapters/src/local_asset_store.rs",
    "crates/cabinet-adapters/src/local_backup_store.rs",
    "crates/cabinet-adapters/src/local_workspace_home_projection.rs",
    "crates/cabinet-adapters/src/local_document_navigator_projection.rs",
  ];
}

function explicitActiveTestPaths() {
  return [
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
  ];
}

function explicitActiveRustTestPaths() {
  return [
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
  ];
}

function archiveEvidence() {
  return new Map([
    [".tasks/phase010/phase010-archive-validation-result.md", "phase010_archive_validation=passed"],
    [".tasks/phase010/phase010-plan-validation-result.md", "phase010_plan_validation=passed"],
    [".tasks/phase010/phase010-packaged-launch-gate-result.md", "phase010_packaged_launch_gate=passed"],
    [".tasks/phase010/phase010-first-run-workspace-gate-result.md", "phase010_first_run_workspace_gate=passed"],
    [".tasks/phase010/phase010-durable-authoring-gate-result.md", "phase010_durable_authoring_gate=passed"],
    [".tasks/phase010/phase010-data-portability-gate-result.md", "phase010_data_portability_gate=passed"],
    [".tasks/phase010/phase010-index-health-repair-gate-result.md", "phase010_index_health_repair_gate=passed"],
    [".tasks/phase010/phase010-settings-observability-gate-result.md", "phase010_settings_observability_gate=passed"],
    [".tasks/phase010/release/performance-budget-phase010.md", "phase010_performance_budget=passed"],
    [".tasks/phase010/release/packaged-runtime-manifest-phase010.json", "phase010_packaged_runtime_manifest=passed"],
    [".tasks/phase010/release/data-portability-manifest-phase010.json", "phase010_data_portability_manifest=passed"],
    [".tasks/phase010/release/product-log-event-matrix-phase010.md", "phase010_product_log_matrix=passed"],
    [".tasks/phase010/release/security-log-policy-manifest-phase010.json", "phase010_security_log_manifest=passed"],
    [".tasks/phase010/release/local-desktop-runbook-phase010.md", "phase010_runbook=passed"],
  ]);
}
