import assert from "node:assert/strict";
import test from "node:test";

import {
  createImportPreviewViewModel,
  transitionImportPreviewState,
} from "../src/index.ts";
import type {
  ImportConflictItemView,
  ImportPreviewSummaryView,
} from "../src/index.ts";

test("import preview summary supports markdown folder without raw local data", () => {
  const model = createImportPreviewViewModel({
    state: "PreviewReady",
    summary: previewSummary("markdown-folder"),
    conflicts: [],
  });
  const serialized = JSON.stringify(model);

  assert.equal(model.mode, "import-preview");
  assert.equal(model.sourceKind, "markdown-folder");
  assert.equal(model.scannedDocumentCount, 10);
  assert.equal(model.assetReferenceCount, 3);
  assert.equal(model.linkCount, 8);
  assert.equal(model.unsupportedItemCount, 1);
  assert.equal(model.canApply, true);
  assert.deepEqual(model.actions.map((action) => action.id), ["apply-import-preview"]);
  assert.equal(serialized.includes("raw markdown body should not leak"), false);
  assert.equal(serialized.includes("/Users/example/private/vault"), false);
  assert.equal(serialized.includes("asset binary content should not leak"), false);
  assert.equal(serialized.includes("phase005-provider-api-key-should-not-log"), false);
});

test("import preview summary supports obsidian vault source kind", () => {
  const model = createImportPreviewViewModel({
    state: "PreviewReady",
    summary: previewSummary("obsidian-vault"),
    conflicts: [],
  });

  assert.equal(model.sourceKind, "obsidian-vault");
  assert.equal(model.sourceHash.startsWith("uihash:"), true);
});

test("import preview blocks apply while scanning failed or blocked by conflicts", () => {
  const scanning = createImportPreviewViewModel({
    state: "Scanning",
    summary: previewSummary("markdown-folder"),
    conflicts: [],
  });
  const failed = createImportPreviewViewModel({
    state: "Failed",
    summary: previewSummary("markdown-folder"),
    conflicts: [conflict("scan.failed", "blocking")],
  });
  const blocked = createImportPreviewViewModel({
    state: "PreviewReady",
    summary: previewSummary("markdown-folder"),
    conflicts: [conflict("document.path.conflict", "blocking")],
  });

  assert.equal(scanning.canApply, false);
  assert.deepEqual(scanning.actions, []);
  assert.equal(failed.canApply, false);
  assert.deepEqual(failed.actions.map((action) => action.id), ["rescan-import-source"]);
  assert.equal(blocked.canApply, false);
  assert.deepEqual(blocked.actions.map((action) => action.id), ["rescan-import-source"]);
  assert.deepEqual(blocked.conflictResolutionPolicies, [
    {
      code: "document.path.conflict",
      options: ["rename", "skip"],
    },
  ]);
});

test("import preview state machine exposes explicit transitions and rejects invalid apply", () => {
  const scanning = transitionImportPreviewState("Selected", "ScanRequested");
  const ready = transitionImportPreviewState(scanning.state, "ScanCompleted");
  const applying = transitionImportPreviewState(ready.state, "ApplyRequested");
  const completed = transitionImportPreviewState(applying.state, "ApplySucceeded");
  const invalid = transitionImportPreviewState("Selected", "ApplyRequested");

  assert.deepEqual(scanning, { state: "Scanning" });
  assert.deepEqual(ready, { state: "PreviewReady" });
  assert.deepEqual(applying, { state: "Applying" });
  assert.deepEqual(completed, { state: "Completed" });
  assert.equal(invalid.state, "Failed");
  assert.equal(invalid.errorCode, "IMPORT_PREVIEW_INVALID_TRANSITION");
});

function previewSummary(sourceKind: ImportPreviewSummaryView["sourceKind"]): ImportPreviewSummaryView {
  return {
    sourceKind,
    sourceHash: "raw source path /Users/example/private/vault should be hashed",
    scannedDocumentCount: 10,
    assetReferenceCount: 3,
    linkCount: 8,
    unsupportedItemCount: 1,
    estimatedByteSizeBucket: "1mb-10mb",
  };
}

function conflict(
  code: string,
  severity: ImportConflictItemView["severity"],
): ImportConflictItemView {
  return {
    code,
    severity,
    count: 2,
  };
}
