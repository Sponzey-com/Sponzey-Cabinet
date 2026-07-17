import assert from "node:assert/strict";
import test from "node:test";

import {
  createDocumentHistoryCompareSelection,
  transitionDocumentHistoryCompareSelection,
} from "../src/document_history_compare_selection.ts";

test("history compare selection transitions through idle one and two selected states", () => {
  const idle = createDocumentHistoryCompareSelection();
  const one = transitionDocumentHistoryCompareSelection(idle, {
    type: "Toggle",
    versionId: "opaque-v3",
    versionLabel: "버전 3",
  });
  const two = transitionDocumentHistoryCompareSelection(one, {
    type: "Toggle",
    versionId: "opaque-v2",
    versionLabel: "버전 2",
  });

  assert.deepEqual(idle, { status: "Idle", selections: [] });
  assert.deepEqual(one, {
    status: "OneSelected",
    selections: [{ versionId: "opaque-v3", versionLabel: "버전 3" }],
  });
  assert.deepEqual(two, {
    status: "TwoSelected",
    selections: [
      { versionId: "opaque-v3", versionLabel: "버전 3" },
      { versionId: "opaque-v2", versionLabel: "버전 2" },
    ],
  });
});

test("history compare selection toggles off and keeps the two most recent unique revisions", () => {
  const selected = [
    ["v3", "버전 3"],
    ["v2", "버전 2"],
    ["v1", "버전 1"],
  ].reduce((state, [versionId, versionLabel]) => transitionDocumentHistoryCompareSelection(state, {
    type: "Toggle",
    versionId,
    versionLabel,
  }), createDocumentHistoryCompareSelection());

  assert.deepEqual(selected, {
    status: "TwoSelected",
    selections: [
      { versionId: "v2", versionLabel: "버전 2" },
      { versionId: "v1", versionLabel: "버전 1" },
    ],
  });
  assert.deepEqual(transitionDocumentHistoryCompareSelection(selected, {
    type: "Toggle",
    versionId: "v2",
    versionLabel: "버전 2",
  }), {
    status: "OneSelected",
    selections: [{ versionId: "v1", versionLabel: "버전 1" }],
  });
});

test("history compare selection rejects empty opaque identity and supports explicit clear", () => {
  const idle = createDocumentHistoryCompareSelection();
  assert.throws(() => transitionDocumentHistoryCompareSelection(idle, {
    type: "Toggle",
    versionId: " ",
    versionLabel: "버전 1",
  }), /HISTORY_COMPARE_SELECTION_INVALID/);
  assert.deepEqual(transitionDocumentHistoryCompareSelection(idle, { type: "Clear" }), idle);
});
