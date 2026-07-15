import assert from "node:assert/strict";
import test from "node:test";

import {
  defineUiActionContract,
  validateUiActionContracts,
  type RenderedUiAction,
  type UiActionContract,
} from "../src/ui_action_contract.ts";

test("action contract validator accepts complete connected and hidden contracts", () => {
  const contracts = [connected("navigate-home"), hidden("open-settings")];
  const rendered: RenderedUiAction[] = [
    { actionId: "navigate-home", enabled: true, callbackConnected: true },
  ];
  assert.deepEqual(validateUiActionContracts(contracts, rendered), []);
  assert.ok(Object.isFrozen(contracts[0]));
});

test("action contract validator detects both coverage directions and duplicate contracts", () => {
  const issues = validateUiActionContracts(
    [connected("navigate-home"), connected("save-document"), connected("save-document")],
    [{ actionId: "unexpected-control", enabled: true, callbackConnected: true }],
  );
  assert.deepEqual(issues.map((issue) => issue.code), [
    "ACTION_CONTRACT_DUPLICATE",
    "ACTION_CONTRACT_NOT_RENDERED",
    "ACTION_CONTRACT_NOT_RENDERED",
    "RENDERED_ACTION_UNCLASSIFIED",
  ]);
});

test("action contract validator rejects missing callback and unexplained disabled controls", () => {
  const issues = validateUiActionContracts(
    [connected("create-backup"), connected("save-document")],
    [
      { actionId: "create-backup", enabled: true, callbackConnected: false },
      { actionId: "save-document", enabled: false, callbackConnected: true },
    ],
  );
  assert.deepEqual(issues.map((issue) => issue.code), [
    "ENABLED_ACTION_CALLBACK_MISSING",
    "DISABLED_ACTION_REASON_MISSING",
  ]);
});

test("action contract validator rejects rendering hidden out-of-scope controls", () => {
  const issues = validateUiActionContracts(
    [hidden("toggle-theme")],
    [{ actionId: "toggle-theme", enabled: false, callbackConnected: false }],
  );
  assert.deepEqual(issues.map((issue) => issue.code), ["HIDDEN_ACTION_RENDERED"]);
});

function connected(actionId: string): UiActionContract {
  return defineUiActionContract({
    actionId,
    surface: "shell",
    availability: "connected",
    visibleCondition: "always",
    enabledCondition: "callback_available",
    input: "none",
    boundary: "route",
    target: "DesktopRoute.Home",
    progressState: "none",
    successReadback: "active route and heading",
    failureMapping: "none",
    recoveryAction: "none",
    durability: "none",
    interactionTest: "ui_action_contract_validator_tests.ts",
    packagedTest: "pending:phase013-packaged-action-gate",
  });
}

function hidden(actionId: string): UiActionContract {
  return defineUiActionContract({
    ...connected(actionId),
    availability: "hidden_out_of_scope",
    visibleCondition: "never_in_phase013",
    enabledCondition: "never",
    target: "out_of_scope",
    hiddenReasonKey: "PHASE013_FEATURE_OUT_OF_SCOPE",
  });
}
