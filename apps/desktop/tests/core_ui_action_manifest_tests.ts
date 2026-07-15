import assert from "node:assert/strict";
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
