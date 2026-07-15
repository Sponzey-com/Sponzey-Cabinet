import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

export const Phase012ArchiveState = Object.freeze({
  NotStarted: "NotStarted",
  ArchiveValidated: "ArchiveValidated",
  InventoryValidated: "InventoryValidated",
  ContractValidated: "ContractValidated",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase012ArchiveEvent = Object.freeze({
  ArchiveAccepted: "ArchiveAccepted",
  InventoryAccepted: "InventoryAccepted",
  ContractAccepted: "ContractAccepted",
  Complete: "Complete",
  Fail: "Fail",
});

export const Phase012ArchiveErrorCode = Object.freeze({
  ArchivePlanMissing: "PHASE012_ARCHIVE_PLAN_MISSING",
  ArchiveReadmeMissing: "PHASE012_ARCHIVE_README_MISSING",
  ArchiveTaskGap: "PHASE012_ARCHIVE_TASK_GAP",
  ArchiveReleaseMarkerMissing: "PHASE012_ARCHIVE_RELEASE_MARKER_MISSING",
  RequiredPathMissing: "PHASE012_REQUIRED_PATH_MISSING",
  FutureScopeActivated: "PHASE012_FUTURE_SCOPE_ACTIVATED",
  RequirementRegisterInvalid: "PHASE012_REQUIREMENT_REGISTER_INVALID",
  SourceFingerprintMismatch: "PHASE012_SOURCE_FINGERPRINT_MISMATCH",
  InvalidTransition: "PHASE012_ARCHIVE_INVALID_TRANSITION",
  IoFailed: "PHASE012_ARCHIVE_IO_FAILED",
});

const archiveFiles = Object.freeze([
  ".tasks/phase011/plan.md",
  ".tasks/phase011/README.md",
  ".tasks/phase011/phase011-release-gate-result.md",
]);

const sourceFiles = Object.freeze([
  ".tasks/plan.md",
  "AGENTS.md",
  "PROJECT.md",
  "apps/desktop/src/desktop_entry.ts",
  "apps/desktop/src/react_exploration_surfaces.ts",
  "apps/desktop/src/tauri_desktop_transport.ts",
  "apps/desktop/src-tauri/src/main.rs",
  "apps/desktop/src-tauri/src/lib.rs",
  "crates/cabinet-adapters/src/local_graph_projection.rs",
  "crates/cabinet-adapters/src/local_canvas_repository.rs",
  "crates/cabinet-adapters/src/local_asset_store.rs",
]);

const inventoryDefinitions = Object.freeze([
  ["Graph UI", "apps/desktop/src/react_exploration_surfaces.ts", "recentDocuments", "UIOnly", "Implemented"],
  ["Graph repository", "crates/cabinet-adapters/src/local_graph_projection.rs", "HashMap", "MemoryOnly", "Implemented"],
  ["Canvas UI", "apps/desktop/src/react_exploration_surfaces.ts", "useState", "UIOnly", "Implemented"],
  ["Canvas repository", "crates/cabinet-adapters/src/local_canvas_repository.rs", "HashMap", "MemoryOnly", "Implemented"],
  ["Asset UI", "apps/desktop/src/react_exploration_surfaces.ts", "FileList", "UIOnly", "Implemented"],
  ["Asset store primitive", "crates/cabinet-adapters/src/local_asset_store.rs", "fs::", "RuntimeOnly", "ContractOnly"],
  ["Projection event wiring", "apps/desktop/src-tauri/src/lib.rs", "DesktopDocumentChangeSink", "NotWired", "Implemented"],
]);

export function transitionPhase012ArchiveState(state, event, failure = {}) {
  const next = new Map([
    [`${Phase012ArchiveState.NotStarted}:${Phase012ArchiveEvent.ArchiveAccepted}`, Phase012ArchiveState.ArchiveValidated],
    [`${Phase012ArchiveState.ArchiveValidated}:${Phase012ArchiveEvent.InventoryAccepted}`, Phase012ArchiveState.InventoryValidated],
    [`${Phase012ArchiveState.InventoryValidated}:${Phase012ArchiveEvent.ContractAccepted}`, Phase012ArchiveState.ContractValidated],
    [`${Phase012ArchiveState.ContractValidated}:${Phase012ArchiveEvent.Complete}`, Phase012ArchiveState.Passed],
  ]).get(`${state}:${event}`);
  if (event === Phase012ArchiveEvent.Fail) {
    return { state: Phase012ArchiveState.Failed, ...failure };
  }
  if (!next) {
    return { state: Phase012ArchiveState.Failed, errorCode: Phase012ArchiveErrorCode.InvalidTransition };
  }
  return { state: next };
}

export function validateInventoryFingerprint(artifact, currentFingerprint) {
  return artifact.includes(`source_fingerprint=${currentFingerprint}`) ? [] : [{
    errorCode: Phase012ArchiveErrorCode.SourceFingerprintMismatch,
    findingId: "source_fingerprint",
  }];
}

export async function runPhase012ArchiveValidation({ root, writeArtifacts = true }) {
  try {
    const normalizedRoot = resolve(root);
    const archiveContents = new Map();
    for (const path of archiveFiles) {
      const contents = await readRequired(normalizedRoot, path, errorForArchivePath(path));
      if (contents.failure) return contents.failure;
      archiveContents.set(path, contents.text);
    }
    for (let index = 1; index <= 33; index += 1) {
      const path = `.tasks/phase011/task${String(index).padStart(3, "0")}.md`;
      const contents = await readRequired(normalizedRoot, path, Phase012ArchiveErrorCode.ArchiveTaskGap);
      if (contents.failure) return contents.failure;
      archiveContents.set(path, contents.text);
    }
    const release = archiveContents.get(".tasks/phase011/phase011-release-gate-result.md");
    if (!release.includes("phase011_release_gate=passed")) {
      return failed(Phase012ArchiveErrorCode.ArchiveReleaseMarkerMissing, ".tasks/phase011/phase011-release-gate-result.md");
    }
    let state = transitionPhase012ArchiveState(Phase012ArchiveState.NotStarted, Phase012ArchiveEvent.ArchiveAccepted);

    const sourceContents = new Map();
    for (const path of sourceFiles) {
      const contents = await readRequired(normalizedRoot, path, Phase012ArchiveErrorCode.RequiredPathMissing);
      if (contents.failure) return contents.failure;
      sourceContents.set(path, contents.text);
    }
    if (/openServerAdmin|openCollaboration|createRemoteWorkspace|openMobileRoute/.test(sourceContents.get("apps/desktop/src/desktop_entry.ts"))) {
      return failed(Phase012ArchiveErrorCode.FutureScopeActivated, "apps/desktop/src/desktop_entry.ts");
    }
    const inventory = inventoryDefinitions.map(([name, path, marker, whenPresent, whenAbsent]) => ({
      name,
      path,
      status: sourceContents.get(path).includes(marker) ? whenPresent : whenAbsent,
    }));
    state = transitionPhase012ArchiveState(state.state, Phase012ArchiveEvent.InventoryAccepted);

    const plan = sourceContents.get(".tasks/plan.md");
    const requirementIds = [...plan.matchAll(/^\| `([A-Z-]+012-[0-9]+)`/gm)].map((match) => match[1]);
    const duplicate = requirementIds.find((id, index) => requirementIds.indexOf(id) !== index);
    if (requirementIds.length === 0 || duplicate) {
      return failed(Phase012ArchiveErrorCode.RequirementRegisterInvalid, duplicate ?? "requirement_register");
    }
    state = transitionPhase012ArchiveState(state.state, Phase012ArchiveEvent.ContractAccepted);

    const sourceFingerprint = fingerprint(sourceContents);
    const archiveFingerprint = fingerprint(archiveContents);
    state = transitionPhase012ArchiveState(state.state, Phase012ArchiveEvent.Complete);
    const result = {
      passed: true,
      state: state.state,
      archivedTaskCount: 33,
      requirementIds,
      inventory,
      sourceFingerprint,
      archiveFingerprint,
    };
    if (writeArtifacts) await writeArtifactsForResult(normalizedRoot, result);
    return result;
  } catch {
    return failed(Phase012ArchiveErrorCode.IoFailed, "validator_io");
  }
}

function errorForArchivePath(path) {
  if (path.endsWith("plan.md")) return Phase012ArchiveErrorCode.ArchivePlanMissing;
  if (path.endsWith("README.md")) return Phase012ArchiveErrorCode.ArchiveReadmeMissing;
  return Phase012ArchiveErrorCode.ArchiveReleaseMarkerMissing;
}

async function readRequired(root, path, errorCode) {
  try {
    return { text: await readFile(join(root, path), "utf8"), failure: undefined };
  } catch {
    return { text: "", failure: failed(errorCode, path) };
  }
}

function failed(errorCode, findingId) {
  return { passed: false, state: Phase012ArchiveState.Failed, errorCode, findingId };
}

function fingerprint(contents) {
  const hash = createHash("sha256");
  for (const [path, text] of [...contents.entries()].sort(([left], [right]) => left.localeCompare(right))) {
    hash.update(path).update("\0").update(text).update("\0");
  }
  return hash.digest("hex");
}

async function writeArtifactsForResult(root, result) {
  const files = new Map([
    [".tasks/phase012-archive-validation-result.md", renderArchive(result)],
    [".tasks/phase012-current-implementation-inventory.md", renderInventory(result)],
    [".tasks/release/requirement-evidence-matrix-phase012.md", renderMatrix(result)],
  ]);
  for (const [path, text] of files) {
    const output = join(root, path);
    await mkdir(dirname(output), { recursive: true });
    await writeFile(output, text);
  }
}

function renderArchive(result) {
  return [
    "phase012_archive_validation=passed",
    `validation_state=${result.state}`,
    "release_scope=personal_local_desktop",
    `archived_task_count=${result.archivedTaskCount}`,
    `archive_fingerprint=${result.archiveFingerprint}`,
    `source_fingerprint=${result.sourceFingerprint}`,
    "",
  ].join("\n");
}

function renderInventory(result) {
  return [
    "# Phase 012 Current Implementation Inventory",
    "",
    "phase012_current_inventory=passed",
    `source_fingerprint=${result.sourceFingerprint}`,
    "",
    "| Component | Status | Source |",
    "| --- | --- | --- |",
    ...result.inventory.map((item) => `| ${item.name} | ${item.status} | \`${item.path}\` |`),
    "",
    "Dormant server/mobile/Web source is not classified as active unless wired into the desktop entry or composition root.",
    "",
  ].join("\n");
}

function renderMatrix(result) {
  return [
    "# Phase 012 Requirement Evidence Matrix",
    "",
    "phase012_requirement_evidence=pending",
    `source_fingerprint=${result.sourceFingerprint}`,
    `requirement_count=${result.requirementIds.length}`,
    "",
    "| Requirement | Status | Evidence |",
    "| --- | --- | --- |",
    ...result.requirementIds.map((id) => `| \`${id}\` | pending | not_verified |`),
    "",
  ].join("\n");
}

async function main() {
  const root = process.argv[2] ? resolve(process.argv[2]) : process.cwd();
  const result = await runPhase012ArchiveValidation({ root, writeArtifacts: true });
  if (!result.passed) {
    process.stderr.write(`${result.errorCode} finding=${result.findingId}\n`);
    process.exitCode = 1;
    return;
  }
  process.stdout.write(`phase012_archive_validation=passed tasks=${result.archivedTaskCount} requirements=${result.requirementIds.length}\n`);
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) await main();
