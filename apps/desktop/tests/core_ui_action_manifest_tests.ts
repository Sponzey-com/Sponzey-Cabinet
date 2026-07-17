import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import React from "react";

import { CORE_UI_ACTION_MANIFEST } from "../src/core_ui_action_manifest.ts";
import { collectReactUiActions, validateUiActionContracts } from "../src/ui_action_contract.ts";

test("core action manifest has unique complete contracts and explicit hidden future controls", () => {
  const ids = CORE_UI_ACTION_MANIFEST.map((entry) => entry.actionId);
  assert.equal(new Set(ids).size, ids.length);
  assert.ok(ids.length >= 35);
  for (const entry of CORE_UI_ACTION_MANIFEST) {
    assert.ok(entry.surface.length > 0);
    assert.ok(entry.successReadback.length > 0);
    assert.match(entry.interactionTest, /tests\//);
    assert.notEqual((entry.availability as string), "disabled_pending");
  }
  for (const id of ["open-settings", "toggle-theme", "open-ai"]) {
    const entry = CORE_UI_ACTION_MANIFEST.find((candidate) => candidate.actionId === id);
    assert.equal(entry?.availability, "hidden_out_of_scope");
    assert.match(entry?.hiddenReasonKey ?? "", /^[A-Z0-9_]+$/);
  }
  assert.equal(
    CORE_UI_ACTION_MANIFEST.find((entry) => entry.actionId === "load-more-history")?.durability,
    "readback",
  );
  assert.equal(CORE_UI_ACTION_MANIFEST.find((entry) => entry.actionId === "select-history-version")?.boundary, "view_state");
  assert.equal(CORE_UI_ACTION_MANIFEST.find((entry) => entry.actionId === "previous-history-window")?.boundary, "view_state");
  assert.equal(CORE_UI_ACTION_MANIFEST.find((entry) => entry.actionId === "next-history-window")?.boundary, "view_state");
  assert.equal(CORE_UI_ACTION_MANIFEST.find((entry) => entry.actionId === "compare-selected-versions")?.durability, "readback");
  assert.equal(CORE_UI_ACTION_MANIFEST.find((entry) => entry.actionId === "cancel-background-document-diff")?.boundary, "native_command");
  assert.equal(CORE_UI_ACTION_MANIFEST.find((entry) => entry.actionId === "retry-background-document-diff")?.durability, "readback");
});

test("React action collector records callback and disabled state and flags missing action identity", () => {
  const tree = React.createElement("div", null,
    React.createElement("button", { "data-action": "navigate-home", onClick() {} }, "홈"),
    React.createElement("button", { "data-action": "save-document", disabled: true, onClick() {} }, "저장"),
    React.createElement("input", { onChange() {}, "aria-label": "검색" }),
  );
  const observations = collectReactUiActions(tree);
  assert.deepEqual(observations.actions, [
    { actionId: "navigate-home", enabled: true, callbackConnected: true },
    { actionId: "save-document", enabled: false, callbackConnected: true },
  ]);
  assert.equal(observations.unidentifiedControlCount, 1);
  assert.deepEqual(
    validateUiActionContracts(CORE_UI_ACTION_MANIFEST, observations.actions, observations.unidentifiedControlCount)
      .filter((issue) => issue.code === "RENDERED_CONTROL_ACTION_ID_MISSING")
      .map((issue) => issue.code),
    ["RENDERED_CONTROL_ACTION_ID_MISSING"],
  );
});

test("core manifest classifies every document attachment action", async () => {
  const source = await readFile(new URL("../src/react_document_attachment_panel.ts", import.meta.url), "utf8");
  const rendered = [...source.matchAll(/"data-action": "([a-z0-9-]+)"/g)].map((match) => match[1]);
  const byId = new Map(CORE_UI_ACTION_MANIFEST.map((entry) => [entry.actionId, entry]));

  assert.deepEqual([...new Set(rendered)].filter((actionId) => !byId.has(actionId)), []);
  assert.equal(byId.get("open-document-asset-library")?.boundary, "route");
  assert.equal(byId.get("close-document-asset-preview")?.boundary, "view_state");
  assert.equal(byId.get("import-document-asset")?.durability, "reopen");
  assert.equal(byId.get("unlink-document-asset")?.boundary, "view_state");
  assert.equal(byId.get("confirm-document-asset-unlink")?.durability, "reopen");
  assert.equal(byId.get("preview-document-asset")?.durability, "readback");
  for (const actionId of [
    "select-document-inspector-links",
    "select-document-inspector-attachments",
    "select-document-inspector-history",
    "cancel-document-asset-unlink",
    "confirm-document-asset-unlink",
  ]) assert.ok(byId.has(actionId), actionId);
});
