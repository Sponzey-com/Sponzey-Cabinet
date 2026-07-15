import assert from "node:assert/strict";
import test from "node:test";

import type {
  ImportConflictItemView,
  ImportPreviewSummaryView,
} from "../../../packages/ui/src/index.ts";
import { createDesktopImportPreview } from "../src/index.ts";

test("desktop import preview smoke supports markdown and obsidian without raw paths", () => {
  const markdown = createDesktopImportPreview({
    state: "PreviewReady",
    summary: previewSummary("markdown-folder"),
    conflicts: [],
  });
  const obsidian = createDesktopImportPreview({
    state: "PreviewReady",
    summary: previewSummary("obsidian-vault"),
    conflicts: [],
  });
  const serialized = JSON.stringify([markdown, obsidian]);

  assert.equal(markdown.sourceKind, "markdown-folder");
  assert.equal(obsidian.sourceKind, "obsidian-vault");
  assert.equal(markdown.canApply, true);
  assert.equal(markdown.unsupportedItemCount, 1);
  assert.equal(obsidian.sourceHash.startsWith("uihash:"), true);
  assert.equal(serialized.includes("/Users/example/private/vault"), false);
  assert.equal(serialized.includes("raw markdown body should not leak"), false);
  assert.equal(serialized.includes("asset binary content should not leak"), false);
  assert.equal(serialized.includes("phase005-provider-api-key-should-not-log"), false);
});

test("desktop import preview smoke blocks apply for blocking conflicts", () => {
  const model = createDesktopImportPreview({
    state: "PreviewReady",
    summary: previewSummary("markdown-folder"),
    conflicts: [conflict("document.path.conflict", "blocking")],
  });

  assert.equal(model.canApply, false);
  assert.deepEqual(model.actions.map((action) => action.id), ["rescan-import-source"]);
  assert.deepEqual(model.conflicts.map((item) => item.code), ["document.path.conflict"]);
  assert.deepEqual(model.conflictResolutionPolicies[0]?.options, ["rename", "skip"]);
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
