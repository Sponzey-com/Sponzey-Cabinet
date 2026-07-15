import assert from "node:assert/strict";
import test from "node:test";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";

import { createWorkspaceShellModel } from "../src/workspace_shell_contract.ts";
import { createWorkspaceShellElement } from "../src/react_workspace_shell.ts";
import { KO_KR_MESSAGES, type MessageCatalog } from "../src/ko_kr_catalog.ts";

const routes = ["Home", "Search", "Document", "Graph", "Canvas", "Assets", "Backup"] as const;

test("shared shell renders one frame and the contract navigation", () => {
  const markup = renderToStaticMarkup(createWorkspaceShellElement({
    model: createWorkspaceShellModel({ route: "Search", availableActions: routes, messages: KO_KR_MESSAGES }),
    messages: KO_KR_MESSAGES,
    routeActions: { Home() {}, Graph() {} },
    rootAttributes: { "data-test-state": "Ready" },
    content: React.createElement("section", null, "route content"),
  }));
  assert.equal((markup.match(/class="[^"]*workspace-shell-frame/g) ?? []).length, 1);
  assert.equal((markup.match(/class="desktop-sidebar"/g) ?? []).length, 1);
  assert.equal((markup.match(/class="desktop-topbar"/g) ?? []).length, 1);
  assert.equal((markup.match(/<main/g) ?? []).length, 1);
  for (const label of ["홈", "검색", "문서", "지식 지도", "캔버스", "첨부 파일", "백업 및 복원"]) assert.match(markup, new RegExp(`>${label}<`));
  assert.match(markup, /data-action="navigate-search"[^>]*aria-current="page"[^>]*disabled/);
  assert.match(markup, /data-action="navigate-assets"[^>]*disabled/);
  assert.match(markup, /data-test-state="Ready"/);
  assert.match(markup, /<kbd>⌘K<\/kbd>/);
  assert.doesNotMatch(markup, /Ctrl K/);
});

test("shared shell renders a custom presentation topbar without domain knowledge", () => {
  const markup = renderToStaticMarkup(createWorkspaceShellElement({
    model: createWorkspaceShellModel({ route: "Document", availableActions: routes, messages: KO_KR_MESSAGES }),
    messages: KO_KR_MESSAGES,
    routeActions: {},
    topbarContent: React.createElement("button", { "data-action": "save-document" }, "저장"),
    content: React.createElement("section", null, "editor"),
  }));
  assert.equal((markup.match(/<header/g) ?? []).length, 1);
  assert.match(markup, /data-action="save-document"/);
  assert.doesNotMatch(markup, /data-action="toggle-theme"/);
});

test("shared shell owns one persistent global status host", () => {
  const markup = renderToStaticMarkup(createWorkspaceShellElement({
    model: createWorkspaceShellModel({ route: "Document", availableActions: routes, messages: KO_KR_MESSAGES }),
    messages: KO_KR_MESSAGES,
    routeActions: {},
    globalLayer: React.createElement("div", { role: "alert" }, "저장 필요"),
    content: React.createElement("section", null, "editor"),
  }));
  assert.equal((markup.match(/data-workspace-global-host/g) ?? []).length, 1);
  assert.match(markup, /data-workspace-global-host[^>]*><div role="alert">저장 필요/);
});

test("shared shell renders visible and accessible chrome through the injected catalog", () => {
  const messages: MessageCatalog = Object.freeze({ message: (key) => `fixture:${key}` });
  const markup = renderToStaticMarkup(createWorkspaceShellElement({
    model: createWorkspaceShellModel({ route: "Home", availableActions: routes, messages }),
    messages,
    routeActions: {},
    content: React.createElement("section", null, "content"),
  }));
  for (const key of ["shell.local", "shell.newDocument", "shell.cabinet", "shell.saved", "shell.navigationLabel", "shell.documentTreeLabel"]) {
    assert.match(markup, new RegExp(`fixture:${key}`));
  }
  assert.doesNotMatch(markup, /data-action="(?:open-settings|toggle-theme|open-ai)"/);
  assert.doesNotMatch(markup, />Light</);
});
