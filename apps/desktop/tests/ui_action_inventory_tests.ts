import assert from "node:assert/strict";
import { readdir, readFile } from "node:fs/promises";
import test from "node:test";
import React from "react";

import { CORE_UI_ACTION_MANIFEST } from "../src/core_ui_action_manifest.ts";
import { EXPLORATION_UI_ACTION_CONTRACTS } from "../src/exploration_ui_action_manifest.ts";
import {
  auditConnectedUiActionCoverage,
  auditDynamicUiActionFamilies,
  auditLiteralUiActions,
  createUnifiedUiActionCatalog,
  extractTemplateUiActionExpressions,
  extractLiteralUiActions,
} from "../src/ui_action_inventory.ts";
import { CURRENT_DYNAMIC_UI_ACTION_FAMILIES } from "../src/current_dynamic_ui_action_families.ts";
import { CURRENT_CONDITIONAL_UI_ACTION_FAMILIES } from "../src/current_conditional_ui_action_families.ts";
import {
  collectReactUiActions,
  defineUiActionContract,
  validateUiActionContracts,
  type UiActionContract,
} from "../src/ui_action_contract.ts";
import { KO_KR_MESSAGES } from "../src/ko_kr_catalog.ts";
import { createWorkspaceShellElement } from "../src/react_workspace_shell.ts";
import { createWorkspaceShellModel, type WorkspaceShellRouteKind } from "../src/workspace_shell_contract.ts";

const routes = ["Home", "Search", "Document", "Graph", "Canvas", "Assets", "Backup"] as const;

test("unified action catalog merges compatible contracts and rejects conflicting execution meanings", () => {
  const first = contract("save-document", "native_command", "save_document", "reopen");
  const compatible = Object.freeze({ ...first, surface: "document-toolbar" });
  const conflict = contract("save-document", "route", "DesktopRoute.Document", "none");

  const compatibleResult = createUnifiedUiActionCatalog([
    { source: "first", contracts: [first] },
    { source: "compatible", contracts: [compatible] },
  ]);
  assert.deepEqual(compatibleResult.issues, []);
  assert.deepEqual(compatibleResult.contracts.map((entry) => entry.actionId), ["save-document"]);

  const conflictResult = createUnifiedUiActionCatalog([
    { source: "first", contracts: [first] },
    { source: "conflict", contracts: [conflict] },
  ]);
  assert.deepEqual(conflictResult.issues, [{
    actionId: "save-document",
    code: "ACTION_CONTRACT_CONFLICT",
    sources: ["first", "conflict"],
  }]);
});

test("literal action extraction reports source locations and audits unclassified actions", () => {
  const inventory = extractLiteralUiActions("fixture.ts", [
    'e("button", { "data-action": "save-document" })',
    'e("button", { "data-action": "unknown-action" })',
    'e("button", { "data-action": `dynamic-${value}` })',
  ].join("\n"));

  assert.deepEqual(inventory, [
    { actionId: "save-document", line: 1, source: "fixture.ts" },
    { actionId: "unknown-action", line: 2, source: "fixture.ts" },
  ]);
  assert.deepEqual(auditLiteralUiActions([contract("save-document", "native_command", "save_document", "reopen")], inventory), [{
    actionId: "unknown-action",
    code: "LITERAL_ACTION_UNCLASSIFIED",
    line: 2,
    source: "fixture.ts",
  }]);
});

test("reverse coverage reports a connected contract absent from current source", () => {
  assert.deepEqual(auditConnectedUiActionCoverage(
    [
      contract("save-document", "native_command", "save_document", "reopen"),
      contract("stale-action", "view_state", "stale", "none"),
      Object.freeze({
        ...contract("hidden-action", "view_state", "hidden", "none"),
        availability: "hidden_out_of_scope" as const,
      }),
    ],
    [{ actionId: "save-document", line: 1, source: "fixture.ts" }],
    [],
    [],
  ), [{ actionId: "stale-action", code: "CONNECTED_ACTION_NOT_IN_SOURCE" }]);
});

test("every current React literal data action belongs to the unified current-source catalog", async () => {
  const sourceDirectory = new URL("../src/", import.meta.url);
  const sourceNames = (await readdir(sourceDirectory))
    .filter((name) => (name.startsWith("react_") || name === "codemirror_document_editor.ts")
      && name.endsWith(".ts"))
    .sort();
  const inventory = (await Promise.all(sourceNames.map(async (name) =>
    extractLiteralUiActions(name, await readFile(new URL(name, sourceDirectory), "utf8"))
  ))).flat();
  const catalog = createUnifiedUiActionCatalog([
    { source: "core", contracts: CORE_UI_ACTION_MANIFEST },
    { source: "exploration", contracts: EXPLORATION_UI_ACTION_CONTRACTS },
  ]);

  assert.deepEqual(catalog.issues, []);
  assert.deepEqual(auditLiteralUiActions(catalog.contracts, inventory), []);
  assert.deepEqual(auditConnectedUiActionCoverage(
    catalog.contracts,
    inventory,
    CURRENT_DYNAMIC_UI_ACTION_FAMILIES,
    CURRENT_CONDITIONAL_UI_ACTION_FAMILIES,
  ), []);
  assert.ok(inventory.length >= 80, `expected broad current-source inventory, received ${inventory.length}`);
});

test("the six primary routes have one active menu while Search remains a global tool route", () => {
  const primaryRoutes = ["Home", "Document", "Graph", "Canvas", "Assets", "Backup"] as const;
  for (const route of routes) {
    const model = createWorkspaceShellModel({ route, availableActions: routes, messages: KO_KR_MESSAGES });
    assert.deepEqual(model.navigation.map((item) => item.route), primaryRoutes);
    assert.equal(model.navigation.filter((item) => item.active).length, route === "Search" ? 0 : 1);
    assert.equal(model.navigation.find((item) => item.active)?.route, route === "Search" ? undefined : route);
  }
});

test("every current template action expression has one classified bounded family", async () => {
  const sourceDirectory = new URL("../src/", import.meta.url);
  const sourceNames = (await readdir(sourceDirectory))
    .filter((name) => name.startsWith("react_") && name.endsWith(".ts"))
    .sort();
  const occurrences = (await Promise.all(sourceNames.map(async (name) =>
    extractTemplateUiActionExpressions(name, await readFile(new URL(name, sourceDirectory), "utf8"))
  ))).flat();
  const catalog = createUnifiedUiActionCatalog([
    { source: "core", contracts: CORE_UI_ACTION_MANIFEST },
    { source: "exploration", contracts: EXPLORATION_UI_ACTION_CONTRACTS },
  ]);

  assert.deepEqual(auditDynamicUiActionFamilies(catalog.contracts, occurrences, CURRENT_DYNAMIC_UI_ACTION_FAMILIES), []);
  assert.equal(occurrences.length, 7);
});

test("dynamic action audit fails closed for a missing family and an unclassified expansion", () => {
  const occurrence = extractTemplateUiActionExpressions(
    "fixture.ts",
    'e("button", { "data-action": `mode-${value}` })',
  );
  assert.deepEqual(auditDynamicUiActionFamilies([], occurrence, []), [{
    code: "DYNAMIC_ACTION_FAMILY_MISSING",
    expression: "mode-${value}",
    line: 1,
    source: "fixture.ts",
  }]);
  assert.deepEqual(auditDynamicUiActionFamilies(
    [contract("mode-a", "view_state", "mode", "none")],
    occurrence,
    [{ source: "fixture.ts", expression: "mode-${value}", actionIds: ["mode-a", "mode-b"] }],
  ), [{
    actionId: "mode-b",
    code: "DYNAMIC_ACTION_UNCLASSIFIED",
    expression: "mode-${value}",
    source: "fixture.ts",
  }]);
});

test("shared shell form-bound search and route controls have connected callbacks", () => {
  const catalog = createUnifiedUiActionCatalog([
    { source: "core", contracts: CORE_UI_ACTION_MANIFEST },
    { source: "exploration", contracts: EXPLORATION_UI_ACTION_CONTRACTS },
  ]);
  for (const route of routes) {
    const routeActions = Object.fromEntries(routes.map((candidate) => [candidate, () => undefined]));
    const tree = createWorkspaceShellElement({
      model: createWorkspaceShellModel({ route, availableActions: routes, messages: KO_KR_MESSAGES }),
      messages: KO_KR_MESSAGES,
      routeActions,
      onCreateDocument() {},
      onSearch() {},
      content: "route content",
    });
    const observations = collectReactUiActions(tree);
    assert.deepEqual(
      validateUiActionContracts(catalog.contracts, observations.actions, observations.unidentifiedControlCount),
      [],
      route,
    );
  }
});

test("a named input is not connected when its ancestor form has no submit callback", () => {
  const tree = React.createElement(
    "form",
    null,
    React.createElement("input", { name: "query", "data-action": "workspace-search-input" }),
  );
  assert.deepEqual(collectReactUiActions(tree).actions, [{
    actionId: "workspace-search-input",
    callbackConnected: false,
    enabled: true,
  }]);
});

function contract(
  actionId: string,
  boundary: UiActionContract["boundary"],
  target: string,
  durability: UiActionContract["durability"],
): UiActionContract {
  return defineUiActionContract({
    actionId,
    surface: "fixture",
    availability: "connected",
    visibleCondition: "route_and_state_specific",
    enabledCondition: "callback",
    disabledReasonKey: "ACTION_STATE_UNAVAILABLE",
    input: "fixture",
    boundary,
    target,
    progressState: "none",
    successReadback: "fixture",
    failureMapping: "none",
    recoveryAction: "none",
    durability,
    interactionTest: "tests/fixture.ts",
    packagedTest: "tests/fixture.ts",
  });
}
