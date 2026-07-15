import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";

import {
  Phase012ArchiveErrorCode,
  Phase012ArchiveEvent,
  Phase012ArchiveState,
  runPhase012ArchiveValidation,
  transitionPhase012ArchiveState,
  validateInventoryFingerprint,
} from "./phase012_archive_validator.mjs";

const requirements = ["SCOPE-012-01", "BASE-012-01", "EVID-012-01"];

test("rejects a gap in the 33 archived phase011 tasks", async () => {
  const root = await fixture();
  await rm(join(root, ".tasks/phase011/task017.md"));
  const result = await runPhase012ArchiveValidation({ root, writeArtifacts: false });
  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase012ArchiveErrorCode.ArchiveTaskGap);
  assert.equal(result.findingId, ".tasks/phase011/task017.md");
});

test("rejects an invalid phase011 release marker", async () => {
  const root = await fixture({ releaseMarker: "phase011_release_gate=failed\n" });
  const result = await runPhase012ArchiveValidation({ root, writeArtifacts: false });
  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase012ArchiveErrorCode.ArchiveReleaseMarkerMissing);
});

test("rejects future scope wired into the active desktop entry", async () => {
  const root = await fixture({ activeFutureScope: true });
  const result = await runPhase012ArchiveValidation({ root, writeArtifacts: false });
  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase012ArchiveErrorCode.FutureScopeActivated);
  assert.equal(result.findingId, "apps/desktop/src/desktop_entry.ts");
});

test("does not reject dormant mobile source that is absent from desktop entry", async () => {
  const root = await fixture({ dormantMobileSource: true });
  const result = await runPhase012ArchiveValidation({ root, writeArtifacts: false });
  assert.equal(result.passed, true);
});

test("rejects duplicate phase012 requirement ids", async () => {
  const root = await fixture({ duplicateRequirement: true });
  const result = await runPhase012ArchiveValidation({ root, writeArtifacts: false });
  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase012ArchiveErrorCode.RequirementRegisterInvalid);
  assert.equal(result.findingId, "SCOPE-012-01");
});

test("rejects a stale inventory fingerprint", () => {
  assert.deepEqual(validateInventoryFingerprint("source_fingerprint=old\n", "new"), [{
    errorCode: Phase012ArchiveErrorCode.SourceFingerprintMismatch,
    findingId: "source_fingerprint",
  }]);
});

test("passes a complete fixture and writes sanitized artifacts", async () => {
  const root = await fixture();
  const result = await runPhase012ArchiveValidation({ root, writeArtifacts: true });
  assert.equal(result.passed, true);
  assert.equal(result.archivedTaskCount, 33);
  assert.equal(result.requirementIds.length, requirements.length);
  assert.match(result.sourceFingerprint, /^[a-f0-9]{64}$/);
  assert.deepEqual(result.inventory.map((item) => item.status), [
    "UIOnly", "MemoryOnly", "UIOnly", "MemoryOnly", "UIOnly", "RuntimeOnly", "NotWired",
  ]);
  const archive = await readFile(join(root, ".tasks/phase012-archive-validation-result.md"), "utf8");
  const inventory = await readFile(join(root, ".tasks/phase012-current-implementation-inventory.md"), "utf8");
  const matrix = await readFile(join(root, ".tasks/release/requirement-evidence-matrix-phase012.md"), "utf8");
  assert.match(archive, /phase012_archive_validation=passed/);
  assert.match(inventory, /phase012_current_inventory=passed/);
  assert.match(inventory, /Graph UI.*UIOnly/);
  assert.match(inventory, /Canvas repository.*MemoryOnly/);
  assert.match(matrix, /requirement_count=3/);
  for (const text of [archive, inventory, matrix]) {
    assert.equal(text.includes(root), false);
    assert.doesNotMatch(text, /secret-fixture|raw-body-fixture/);
  }
});

test("state machine accepts ordered transitions and rejects invalid transitions", () => {
  let result = transitionPhase012ArchiveState(Phase012ArchiveState.NotStarted, Phase012ArchiveEvent.ArchiveAccepted);
  assert.equal(result.state, Phase012ArchiveState.ArchiveValidated);
  result = transitionPhase012ArchiveState(result.state, Phase012ArchiveEvent.InventoryAccepted);
  assert.equal(result.state, Phase012ArchiveState.InventoryValidated);
  result = transitionPhase012ArchiveState(result.state, Phase012ArchiveEvent.ContractAccepted);
  assert.equal(result.state, Phase012ArchiveState.ContractValidated);
  result = transitionPhase012ArchiveState(result.state, Phase012ArchiveEvent.Complete);
  assert.equal(result.state, Phase012ArchiveState.Passed);
  const invalid = transitionPhase012ArchiveState(Phase012ArchiveState.NotStarted, Phase012ArchiveEvent.Complete);
  assert.equal(invalid.state, Phase012ArchiveState.Failed);
  assert.equal(invalid.errorCode, Phase012ArchiveErrorCode.InvalidTransition);
});

async function fixture({
  releaseMarker = "phase011_release_gate=passed\n",
  activeFutureScope = false,
  dormantMobileSource = false,
  duplicateRequirement = false,
} = {}) {
  const root = await mkdtemp(join(tmpdir(), "cabinet-phase012-"));
  const files = new Map();
  files.set(".tasks/phase011/plan.md", "# Phase 011\n");
  files.set(".tasks/phase011/README.md", "archive\n");
  files.set(".tasks/phase011/phase011-release-gate-result.md", releaseMarker);
  for (let index = 1; index <= 33; index += 1) {
    files.set(`.tasks/phase011/task${String(index).padStart(3, "0")}.md`, `task ${index}\n`);
  }
  files.set(".tasks/plan.md", [
    "# Phase 012 Development Plan",
    ...requirements.map((id) => `| \`${id}\` | requirement | evidence |`),
    ...(duplicateRequirement ? ["| `SCOPE-012-01` | duplicate | evidence |"] : []),
  ].join("\n"));
  files.set("AGENTS.md", "Layered Architecture\nClean Architecture\nTidy First\nTDD\n");
  files.set("PROJECT.md", "personal local desktop\n");
  files.set("apps/desktop/src/desktop_entry.ts", activeFutureScope ? "openServerAdmin();\n" : "openGraph();\n");
  files.set("apps/desktop/src/react_exploration_surfaces.ts", "recentDocuments\nuseState notes\nFileList\n");
  files.set("apps/desktop/src/tauri_desktop_transport.ts", "typed transport\n");
  files.set("apps/desktop/src-tauri/src/main.rs", "DesktopDocumentAuthoringRuntime\n");
  files.set("apps/desktop/src-tauri/src/lib.rs", "DesktopDocumentChangeSink\nfn publish() {}\n");
  files.set("crates/cabinet-adapters/src/local_graph_projection.rs", "HashMap\n");
  files.set("crates/cabinet-adapters/src/local_canvas_repository.rs", "HashMap\n");
  files.set("crates/cabinet-adapters/src/local_asset_store.rs", "fs::write\n");
  if (dormantMobileSource) files.set("apps/mobile/src/index.ts", "mobile server client\n");
  for (const [path, contents] of files) {
    const full = join(root, path);
    await mkdir(dirname(full), { recursive: true });
    await writeFile(full, contents);
  }
  return root;
}
