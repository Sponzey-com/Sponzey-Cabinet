import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  WORKSPACE_SHELL_LAYOUT,
  WorkspaceShellContractError,
  createWorkspaceShellModel,
  type WorkspaceShellRouteKind,
} from "../src/workspace_shell_contract.ts";
import { KO_KR_MESSAGES, type MessageCatalog } from "../src/ko_kr_catalog.ts";

const routes: readonly WorkspaceShellRouteKind[] = ["Home", "Search", "Document", "Graph", "Canvas", "Assets", "Backup"];

test("every route uses one Korean navigation order and exactly one active item", () => {
  for (const route of routes) {
    const model = createWorkspaceShellModel({ route, availableActions: routes, messages: KO_KR_MESSAGES });
    assert.deepEqual(model.navigation.map((item) => item.label), ["홈", "검색", "문서", "지식 지도", "캔버스", "첨부 파일", "백업 및 복원"]);
    assert.deepEqual(model.navigation.map((item) => item.route), routes);
    assert.equal(model.navigation.filter((item) => item.active).length, 1);
    assert.equal(model.navigation.find((item) => item.active)?.route, route);
    assert.equal(Object.isFrozen(model), true);
    assert.equal(Object.isFrozen(model.navigation), true);
  }
});

test("shell model exposes public context without durable identity", () => {
  const model = createWorkspaceShellModel({ route: "Document", availableActions: routes, variant: "focused", messages: KO_KR_MESSAGES });
  assert.deepEqual({ title: model.pageTitle, context: model.pageContext, variant: model.variant }, { title: "문서", context: "작성 및 검토", variant: "focused" });
  assert.equal(JSON.stringify(model).includes("documentId"), false);
  assert.equal(JSON.stringify(model).includes("workspaceId"), false);
  assert.equal(JSON.stringify(model).includes("assetId"), false);
});

test("shell contract rejects an unknown route and missing route action", () => {
  assert.throws(() => createWorkspaceShellModel({ route: "Unknown" as WorkspaceShellRouteKind, availableActions: routes, messages: KO_KR_MESSAGES }), (error: unknown) => error instanceof WorkspaceShellContractError && error.code === "SHELL_ROUTE_UNKNOWN");
  assert.throws(() => createWorkspaceShellModel({ route: "Home", availableActions: routes.filter((route) => route !== "Backup"), messages: KO_KR_MESSAGES }), (error: unknown) => error instanceof WorkspaceShellContractError && error.code === "SHELL_ACTION_MISSING");
});

test("shell model resolves route labels and contexts through the injected catalog", () => {
  const messages: MessageCatalog = Object.freeze({
    message: (key) => `fixture:${key}`,
  });
  const model = createWorkspaceShellModel({ route: "Graph", availableActions: routes, messages });
  assert.equal(model.pageTitle, "fixture:route.graph");
  assert.equal(model.pageContext, "fixture:routeContext.graph");
  assert.deepEqual(model.navigation.map((item) => item.label), routes.map((route) => `fixture:route.${route.toLowerCase()}`));
});

test("layout tokens preserve the measured desktop baseline", async () => {
  assert.deepEqual(WORKSPACE_SHELL_LAYOUT, { sidebarWidthPx: 244, topbarHeightPx: 50, inspectorWidthPx: 315, contentGapPx: 24 });
  assert.equal(Object.isFrozen(WORKSPACE_SHELL_LAYOUT), true);
  const css = await readFile(new URL("../public/styles.css", import.meta.url), "utf8");
  for (const declaration of ["--shell-sidebar-width: 244px", "--shell-topbar-height: 50px", "--shell-inspector-width: 315px", "--shell-content-gap: 24px"]) assert.ok(css.includes(declaration), declaration);
  assert.match(css, /grid-template-columns:\s*var\(--shell-sidebar-width\)/);
  assert.match(css, /grid-template-rows:\s*var\(--shell-topbar-height\)/);
  assert.match(css, /grid-template-columns:\s*minmax\(0, 1fr\) var\(--shell-inspector-width\)/);
});
