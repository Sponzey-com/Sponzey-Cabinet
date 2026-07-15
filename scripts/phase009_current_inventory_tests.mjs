import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  Phase009InventoryErrorCode,
  Phase009InventoryEvent,
  Phase009InventoryState,
  renderPhase009CurrentInventoryArtifact,
  runPhase009CurrentInventory,
  transitionPhase009InventoryState,
  validatePhase009CurrentInventoryText,
} from "./phase009_current_inventory.mjs";

test("phase009 current inventory rejects missing product UI runner", async () => {
  const root = await createInventoryFixtureRoot();
  await rm(join(root, "scripts", "run_desktop_app.sh"));

  const result = await runPhase009CurrentInventory({ root, writeArtifact: false });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase009InventoryErrorCode.RequiredPathMissing);
  assert.equal(result.findingId, "scripts/run_desktop_app.sh");
});

test("phase009 current inventory rejects shell smoke described as product UI launcher", () => {
  const findings = validatePhase009CurrentInventoryText(
    [
      "# Phase 009 Current Implementation Inventory",
      "",
      "phase009_current_inventory=passed",
      "",
      "- `run_desktop_shell.sh` is the product UI launcher.",
    ].join("\n"),
  );

  assert.equal(findings[0].errorCode, Phase009InventoryErrorCode.ShellSmokeMisclassified);
  assert.equal(findings[0].findingId, "run_desktop_shell.sh");
});

test("phase009 current inventory passes fixture and renders sanitized artifact", async () => {
  const root = await createInventoryFixtureRoot();

  const result = await runPhase009CurrentInventory({ root, writeArtifact: false });
  const artifact = renderPhase009CurrentInventoryArtifact(result);

  assert.equal(result.passed, true);
  assert.equal(result.state, Phase009InventoryState.Passed);
  assert.match(artifact, /phase009_current_inventory=passed/);
  assert.match(artifact, /Product UI Runner/);
  assert.match(artifact, /Future Out Of Scope Paths/);
  assert.match(artifact, /crates\/cabinet-server/);
  assert.match(artifact, /apps\/mobile/);
  assert.doesNotMatch(artifact, /raw_document_body_fixture/);
  assert.doesNotMatch(artifact, /asset_content_fixture/);
  assert.doesNotMatch(artifact, /provider_api_key_fixture/);
  assert.doesNotMatch(artifact, /personal_absolute_path_fixture/);
});

test("phase009 current inventory writes marker artifact to explicit root", async () => {
  const root = await createInventoryFixtureRoot();

  const result = await runPhase009CurrentInventory({ root, writeArtifact: true });
  const written = await readFile(
    join(root, ".tasks", "phase009-current-implementation-inventory.md"),
    "utf8",
  );

  assert.equal(result.passed, true);
  assert.match(written, /phase009_current_inventory=passed/);
  assert.match(written, /product_scope: `personal_local_desktop`/);
});

test("phase009 current inventory state machine exposes success, failure, and invalid transition", () => {
  const inspecting = transitionPhase009InventoryState(
    Phase009InventoryState.NotStarted,
    Phase009InventoryEvent.Start,
  );
  const rendering = transitionPhase009InventoryState(
    inspecting.state,
    Phase009InventoryEvent.PathsInspected,
  );
  const writing = transitionPhase009InventoryState(
    rendering.state,
    Phase009InventoryEvent.ArtifactRendered,
  );
  const passed = transitionPhase009InventoryState(
    writing.state,
    Phase009InventoryEvent.ArtifactWritten,
  );
  const failed = transitionPhase009InventoryState(
    inspecting.state,
    Phase009InventoryEvent.Fail,
    {
      errorCode: Phase009InventoryErrorCode.RequiredPathMissing,
      findingId: "scripts/run_desktop_app.sh",
    },
  );
  const invalid = transitionPhase009InventoryState(
    Phase009InventoryState.NotStarted,
    Phase009InventoryEvent.ArtifactRendered,
  );

  assert.equal(inspecting.state, Phase009InventoryState.InspectingPaths);
  assert.equal(rendering.state, Phase009InventoryState.RenderingArtifact);
  assert.equal(writing.state, Phase009InventoryState.WritingArtifact);
  assert.equal(passed.state, Phase009InventoryState.Passed);
  assert.equal(failed.state, Phase009InventoryState.Failed);
  assert.equal(failed.findingId, "scripts/run_desktop_app.sh");
  assert.equal(invalid.errorCode, Phase009InventoryErrorCode.InvalidTransition);
});

async function createInventoryFixtureRoot() {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase009-inventory-"));

  for (const directory of [
    ".tasks/phase008/release",
    "scripts",
    "apps/desktop/src-tauri",
    "apps/desktop/src",
    "packages/ui/src",
    "packages/editor/src",
    "packages/client-core/src",
    "crates/cabinet-platform/src",
    "crates/cabinet-usecases/src",
    "crates/cabinet-ports/src",
    "crates/cabinet-adapters/src",
    "crates/cabinet-server/src",
    "apps/mobile/src",
  ]) {
    await mkdir(join(root, directory), { recursive: true });
  }

  const fixtureFiles = [
    ".tasks/plan.md",
    ".tasks/phase008/plan.md",
    ".tasks/phase008/task001.md",
    ".tasks/phase008/task011.md",
    ".tasks/phase008/phase008-release-gate-result.md",
    ".tasks/phase008/release/performance-budget-phase008.md",
    "scripts/run_desktop_app.sh",
    "scripts/run_desktop_shell.sh",
    "scripts/run_web_app.mjs",
    "scripts/build_desktop_assets.mjs",
    "apps/desktop/src/index.ts",
    "apps/desktop/src-tauri/tauri.conf.json",
    "packages/ui/src/index.ts",
    "packages/editor/src/index.ts",
    "packages/client-core/src/index.ts",
    "crates/cabinet-platform/src/local_desktop_runtime.rs",
    "crates/cabinet-platform/src/release_smoke.rs",
    "crates/cabinet-usecases/src/document.rs",
    "crates/cabinet-usecases/src/search.rs",
    "crates/cabinet-usecases/src/graph.rs",
    "crates/cabinet-usecases/src/backup.rs",
    "crates/cabinet-usecases/src/import.rs",
    "crates/cabinet-ports/src/lib.rs",
    "crates/cabinet-adapters/src/local_document_repository.rs",
    "crates/cabinet-adapters/src/local_version_store.rs",
    "crates/cabinet-adapters/src/local_search_index.rs",
    "crates/cabinet-adapters/src/local_link_index.rs",
    "crates/cabinet-adapters/src/local_graph_projection.rs",
    "crates/cabinet-adapters/src/local_asset_store.rs",
    "crates/cabinet-adapters/src/local_backup_store.rs",
    "crates/cabinet-server/src/lib.rs",
    "apps/mobile/src/index.ts",
  ];

  for (const filePath of fixtureFiles) {
    const text = filePath === ".tasks/plan.md"
      ? [
          "# Phase 009 Development Plan",
          "",
          "Current product scope marker: `personal_local_desktop`",
          "",
          "| `.tasks/phase009-current-implementation-inventory.md` | `phase009_current_inventory=passed` | Phase 009.0 | plan validation, all later gates |",
          "",
        ].join("\n")
      : filePath.endsWith("phase008-release-gate-result.md")
        ? "phase008_release_gate=passed\n"
        : `${filePath}\n`;
    await writeFile(join(root, filePath), text);
  }

  return root;
}
