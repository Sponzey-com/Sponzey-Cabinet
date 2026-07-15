import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";

import {
  validatePhase011DocumentAuthoringGateInputs,
  renderPhase011DocumentAuthoringGateArtifact,
  runPhase011DocumentAuthoringGate,
} from "./phase011_document_authoring_gate.mjs";

test("document authoring gate rejects missing authoring browser evidence", () => {
  const result = validatePhase011DocumentAuthoringGateInputs({
    inventoryText: inventory(),
    authoringBrowserText: "",
  });

  assert.equal(result.passed, false);
  assert.equal(result.findingId, "authoring_browser_marker");
});

test("document authoring gate rejects stale fingerprint and raw sensitive data", () => {
  const stale = validatePhase011DocumentAuthoringGateInputs({
    inventoryText: inventory("b".repeat(64)),
    authoringBrowserText: authoringReport(),
  });
  const sensitive = validatePhase011DocumentAuthoringGateInputs({
    inventoryText: inventory(),
    authoringBrowserText: authoringReport({ diagnostics: "/Users/private raw markdown body" }),
  });

  assert.equal(stale.passed, false);
  assert.equal(stale.findingId, "source_fingerprint");
  assert.equal(sensitive.passed, false);
  assert.equal(sensitive.findingId, "sensitive_data");
});

test("document authoring gate passes complete evidence and renders safe marker", () => {
  const result = validatePhase011DocumentAuthoringGateInputs({
    inventoryText: inventory(),
    authoringBrowserText: authoringReport(),
  });
  const artifact = renderPhase011DocumentAuthoringGateArtifact(result);

  assert.equal(result.passed, true);
  assert.match(artifact, /phase011_document_authoring_gate=passed/);
  assert.match(artifact, /create_document_count=1/);
  assert.match(artifact, /autosave_count=1/);
  assert.doesNotMatch(artifact, /raw markdown body|\/Users\/private|notes\/architecture\.md/);
});

test("document authoring gate writes marker artifact under explicit root", async () => {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase011-authoring-gate-"));
  await write(join(root, ".tasks/phase011-current-implementation-inventory.md"), inventory());
  await write(join(root, ".tasks/release/authoring-browser-phase011.json"), authoringReport());

  const result = await runPhase011DocumentAuthoringGate({ root });
  const artifact = await readFile(join(root, ".tasks/phase011-document-authoring-gate-result.md"), "utf8");

  assert.equal(result.passed, true);
  assert.match(artifact, /phase011_document_authoring_gate=passed/);
});

function inventory(fingerprint = "a".repeat(64)) {
  return [
    "phase011_current_inventory=passed",
    `source_fingerprint=${fingerprint}`,
    "scripts/run_phase011_authoring_browser.mjs",
    "apps/desktop/src/desktop_entry.ts",
  ].join("\n");
}

function authoringReport(overrides = {}) {
  return JSON.stringify({
    marker: "phase011_authoring_browser=passed",
    sourceFingerprint: "a".repeat(64),
    browserSurface: "local_chrome_cdp",
    diagnostics: "sanitized",
    interactions: {
      documentOpened: true,
      codeMirrorMounted: true,
      createDocumentCount: 1,
      createdDocumentOpened: true,
      sourceMode: true,
      splitMode: true,
      previewMode: true,
      previewTableRendered: true,
      keyboardSave: true,
      manualSaveCount: 1,
      autosaveCount: 1,
      closeBlocked: true,
      closeCancel: true,
      closeRetrySave: true,
      closeDiscard: true,
      historyLoaded: true,
      restorePreviewReady: true,
      restoreApplyCount: 1,
      rawBodyExcluded: true,
      rawPathExcluded: true,
    },
    runs: [
      {
        width: 1024,
        height: 700,
        readyState: true,
        codeMirrorMounted: true,
        previewTableRendered: true,
        nonBlankPixelCount: 10000,
        overlapCount: 0,
        horizontalOverflow: false,
        focusVisible: true,
        screenshot: "authoring-1024x700.png",
      },
      {
        width: 1280,
        height: 800,
        readyState: true,
        codeMirrorMounted: true,
        previewTableRendered: true,
        nonBlankPixelCount: 10000,
        overlapCount: 0,
        horizontalOverflow: false,
        focusVisible: true,
        screenshot: "authoring-1280x800.png",
      },
    ],
    state: "Passed",
    ...overrides,
  });
}

async function write(path, text) {
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, text);
}
