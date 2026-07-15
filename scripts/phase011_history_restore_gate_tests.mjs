import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";

import {
  renderPhase011HistoryRestoreGateArtifact,
  runPhase011HistoryRestoreGate,
  validatePhase011HistoryRestoreGateInputs,
} from "./phase011_history_restore_gate.mjs";

test("history restore gate rejects missing authoring prerequisite", () => {
  const result = validatePhase011HistoryRestoreGateInputs(inputs({ authoringGateText: "" }));

  assert.equal(result.passed, false);
  assert.equal(result.findingId, "authoring_gate");
});

test("history restore gate rejects missing expected current version guard", () => {
  const result = validatePhase011HistoryRestoreGateInputs(inputs({ uiText: "restore without guard" }));

  assert.equal(result.passed, false);
  assert.equal(result.findingId, "ui_expected_guard");
});

test("history restore gate passes complete command boundary evidence", () => {
  const result = validatePhase011HistoryRestoreGateInputs(inputs());
  const artifact = renderPhase011HistoryRestoreGateArtifact(result);

  assert.equal(result.passed, true);
  assert.match(artifact, /phase011_history_restore_gate=passed/);
  assert.match(artifact, /expected_current_version_guard=true/);
  assert.doesNotMatch(artifact, /\/Users\/|provider_api_key|raw markdown body/);
});

test("history restore gate writes marker artifact under explicit root", async () => {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase011-history-gate-"));
  await write(join(root, ".tasks/phase011-current-implementation-inventory.md"), inventory());
  await write(join(root, ".tasks/phase011-document-authoring-gate-result.md"), authoringGate());
  await write(join(root, "packages/client-core/src/index.ts"), sourceText());
  await write(join(root, "packages/ui/src/index.ts"), "expectedCurrentVersionId");
  await write(join(root, "apps/desktop/src/tauri_authoring_transport.ts"), "get_document_history preview_document_restore restore_document_version");
  await write(join(root, "apps/desktop/src-tauri/src/lib.rs"), "DOCUMENT_RESTORE_VERSION_CONFLICT load_current_version compare_and_set_current_version");

  const result = await runPhase011HistoryRestoreGate({ root });
  const artifact = await readFile(join(root, ".tasks/phase011-history-restore-gate-result.md"), "utf8");

  assert.equal(result.passed, true);
  assert.match(artifact, /phase011_history_restore_gate=passed/);
});

function inputs(overrides = {}) {
  return {
    inventoryText: inventory(),
    authoringGateText: authoringGate(),
    clientCoreText: sourceText(),
    uiText: "expectedCurrentVersionId",
    tauriTransportText: "get_document_history preview_document_restore restore_document_version",
    desktopRuntimeText: "DOCUMENT_RESTORE_VERSION_CONFLICT load_current_version compare_and_set_current_version",
    ...overrides,
  };
}

function inventory() {
  return [
    "phase011_current_inventory=passed",
    `source_fingerprint=${"a".repeat(64)}`,
  ].join("\n");
}

function authoringGate() {
  return "phase011_document_authoring_gate=passed\n";
}

function sourceText() {
  return [
    "getDocumentVersion(query)",
    "previewDocumentRestore(query)",
    "restoreDocumentVersion(command)",
  ].join("\n");
}

async function write(path, text) {
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, text);
}
