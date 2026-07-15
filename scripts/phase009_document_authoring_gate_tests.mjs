import assert from "node:assert/strict";
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  DocumentAuthoringGateErrorCode,
  renderPhase009DocumentAuthoringGateArtifact,
  validatePhase009DocumentAuthoringEvidence,
} from "./phase009_document_authoring_gate.mjs";

test("Phase009 document authoring gate accepts explicit editor state and browser smoke evidence", () => {
  const result = validatePhase009DocumentAuthoringEvidence(completeEvidence());

  assert.equal(result.ok, true);
  assert.equal(result.marker, "phase009_document_authoring_gate=passed");
  assert.equal(result.changedLayers.includes("ui-model"), true);
  assert.equal(result.changedLayers.includes("browser-smoke"), true);
});

test("Phase009 document authoring gate rejects missing command runtime prerequisite", () => {
  const result = validatePhase009DocumentAuthoringEvidence({
    ...completeEvidence(),
    commandRuntimeText: "phase009_command_runtime_gate=failed",
  });

  assert.equal(result.ok, false);
  assert.equal(result.errorCode, DocumentAuthoringGateErrorCode.CommandRuntimeMissing);
});

test("Phase009 document authoring gate rejects missing dirty or saved DOM markers", () => {
  const result = validatePhase009DocumentAuthoringEvidence({
    ...completeEvidence(),
    webAppText: completeEvidence().webAppText.replace("data-cabinet-editor-state", "missing-editor-state"),
  });

  assert.equal(result.ok, false);
  assert.equal(result.errorCode, DocumentAuthoringGateErrorCode.VisibleStateMarkerMissing);
});

test("Phase009 document authoring artifact excludes sensitive source text", () => {
  const result = validatePhase009DocumentAuthoringEvidence(completeEvidence());
  const artifact = renderPhase009DocumentAuthoringGateArtifact(result);

  assert.equal(artifact.includes("phase009_document_authoring_gate=passed"), true);
  assert.equal(artifact.includes("raw document body fixture"), false);
  assert.equal(artifact.includes("/Users/example/workspace/private.md"), false);
  assert.equal(artifact.includes("provider_api_key_fixture"), false);
});

test("Phase009 document authoring gate CLI writes marker artifact", async () => {
  const root = await mkdtemp(join(tmpdir(), "phase009-authoring-gate-"));
  await mkdir(join(root, ".tasks"), { recursive: true });
  await mkdir(join(root, "packages/ui/src"), { recursive: true });
  await mkdir(join(root, "packages/ui/tests"), { recursive: true });
  await mkdir(join(root, "apps/web/public"), { recursive: true });
  await mkdir(join(root, "apps/desktop/tests"), { recursive: true });
  await mkdir(join(root, "scripts"), { recursive: true });

  await writeFile(
    join(root, ".tasks/phase009-command-runtime-gate-result.md"),
    "phase009_command_runtime_gate=passed",
  );
  await writeFile(join(root, "packages/ui/src/index.ts"), completeEvidence().uiModelText);
  await writeFile(
    join(root, "packages/ui/tests/document_authoring_preview_model_tests.ts"),
    completeEvidence().uiTestText,
  );
  await writeFile(join(root, "apps/web/public/app.js"), completeEvidence().webAppText);
  await writeFile(
    join(root, "apps/desktop/tests/desktop_document_authoring_smoke_tests.ts"),
    completeEvidence().desktopAuthoringTestText,
  );
  await writeFile(join(root, "scripts/run_browser_smoke.mjs"), completeEvidence().browserSmokeText);

  const { runPhase009DocumentAuthoringGate } = await import("./phase009_document_authoring_gate.mjs");
  const result = await runPhase009DocumentAuthoringGate({ rootDir: root });

  assert.equal(result.ok, true);
});

function completeEvidence() {
  return {
    commandRuntimeText: "phase009_command_runtime_gate=passed",
    uiModelText:
      "DocumentEditorState DocumentEditorEvent transitionDocumentEditorState DOCUMENT_EDITOR_INVALID_TRANSITION DOCUMENT_SAVE_FAILED ReadyClean ReadyDirty Saving Saved SaveFailed",
    uiTestText:
      "document editor state machine marks dirty content and save success explicitly document editor state machine returns stable error code for invalid transitions and save failure",
    webAppText:
      "data-cabinet-editor-state data-cabinet-save-state data-cabinet-saved-version data-cabinet-current-history-split",
    browserSmokeText:
      "dirtyMarkerObserved savedMarkerObserved savedVersionMarkerObserved currentHistorySplitReady previewTableRendered",
    desktopAuthoringTestText:
      "desktop document authoring smoke exposes split source and preview mode document-authoring-workspace get-current-document get-document-history",
  };
}
