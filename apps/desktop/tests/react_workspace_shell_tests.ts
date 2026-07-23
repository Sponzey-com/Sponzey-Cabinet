import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";

import { createWorkspaceShellModel } from "../src/workspace_shell_contract.ts";
import { createWorkspaceShellElement } from "../src/react_workspace_shell.ts";

const shellSource = await readFile(new URL("../src/react_workspace_shell.ts", import.meta.url), "utf8");
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
  for (const label of ["홈", "문서", "지식 지도", "캔버스", "첨부 파일", "백업과 복원"]) assert.match(markup, new RegExp(`>${label}<`));
  assert.doesNotMatch(markup, /data-action="navigate-search"[^>]*aria-current="page"/);
  assert.match(markup, /data-action="navigate-assets"[^>]*disabled/);
  assert.match(markup, /data-test-state="Ready"/);
  for (const icon of ["plus", "search", "chevron-down", "chevron-right"]) {
    assert.match(markup, new RegExp(`lucide-${icon}`));
  }
  assert.doesNotMatch(markup, /<kbd>|⌘K|⌄|›/);
  assert.match(markup, /data-design-reference="penpot-20260721"/);
  assert.match(markup, /placeholder="문서와 첨부 파일 검색"/);
  assert.doesNotMatch(markup, /penpot-20260713|검색어를 입력하세요|백업 및 복원/);
  assert.doesNotMatch(markup, /tone-(?:teal|blue|amber|rose|slate)/);
  assert.match(markup, /class="nav-marker"/);
});

test("shared shell exposes the global search as an input instead of a navigation button", () => {
  const markup = renderToStaticMarkup(createWorkspaceShellElement({
    model: createWorkspaceShellModel({ route: "Home", availableActions: routes, messages: KO_KR_MESSAGES }),
    messages: KO_KR_MESSAGES,
    routeActions: {},
    onSearch() {},
    content: React.createElement("section", null, "content"),
  }));
  assert.match(markup, /<form[^>]*class="topbar-search"[^>]*role="search"/);
  assert.match(markup, /<input[^>]*type="search"[^>]*name="workspace-search"/);
  assert.doesNotMatch(markup, /<button[^>]*class="topbar-search"/);
});

test("shared shell search button is explicitly clickable and does not rely on submit-only behavior", () => {
  assert.match(shellSource, /type:\s*"button"[\s\S]{0,220}"data-action":\s*"submit-workspace-search"[\s\S]{0,260}onClick:/);
  assert.match(shellSource, /event\.currentTarget\.form/);
});

test("shared shell routes Escape through an explicit search callback without submitting", () => {
  assert.match(shellSource, /onSearchEscape/);
  assert.match(shellSource, /event\.key !== "Escape"/);
  assert.match(shellSource, /event\.preventDefault\(\)/);
});

test("shared shell opens the global search overlay on search focus through an explicit callback", () => {
  assert.match(shellSource, /onSearchOpen/);
  assert.match(shellSource, /onFocus: \(\) => options\.onSearchOpen\?\.\(\)/);
  assert.doesNotMatch(shellSource, /onFocus: \(\) => options\.onSearch\?\(/);
});

test("shared shell document tree uses native collapsible folders around root-owned shortcuts", () => {
  const markup = renderToStaticMarkup(createWorkspaceShellElement({
    model: createWorkspaceShellModel({ route: "Document", availableActions: routes, messages: KO_KR_MESSAGES }),
    messages: KO_KR_MESSAGES,
    routeActions: {},
    documentShortcuts: [
      { label: "첫번째 문서", onOpen() {} },
      { label: "두번째 문서" },
    ],
    content: React.createElement("section", null, "editor"),
  }));
  assert.equal((markup.match(/<details/g) ?? []).length, 2);
  assert.match(markup, /<details class="tree-folder" open="">/);
  assert.match(markup, /<summary class="tree-section-heading"/);
  assert.match(markup, /<button[^>]*data-action="open-sidebar-document"[^>]*>첫번째 문서<\/button>/);
  assert.match(markup, /<span[^>]*class="sidebar-current-document"[^>]*aria-current="page"[^>]*>두번째 문서<\/span>/);
  assert.match(markup, /lucide-chevron-down/);
  assert.match(markup, /lucide-chevron-right/);
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
  for (const key of ["shell.brand", "shell.local", "shell.newDocument", "shell.cabinet", "shell.saved", "shell.navigationLabel", "shell.documentTreeLabel"]) {
    assert.match(markup, new RegExp(`fixture:${key}`));
  }
  assert.doesNotMatch(markup, /data-action="(?:open-settings|toggle-theme|open-ai)"/);
  assert.doesNotMatch(markup, />Light</);
});

test("shared shell uses the fixed icon dependency and one route-neutral navigation marker", async () => {
  const [source, manifest] = await Promise.all([
    readFile(new URL("../src/react_workspace_shell.ts", import.meta.url), "utf8"),
    readFile(new URL("../package.json", import.meta.url), "utf8"),
  ]);
  assert.match(source, /from "lucide-react"/);
  for (const icon of ["Plus", "Search", "ChevronDown", "ChevronRight"]) {
    assert.match(source, new RegExp(`\\b${icon}\\b`));
  }
  assert.doesNotMatch(source, /const tones|tone-\$\{/);
  assert.equal(typeof JSON.parse(manifest).dependencies["lucide-react"], "string");
});
