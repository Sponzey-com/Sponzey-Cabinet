import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const css = await readFile(new URL("../public/styles.css", import.meta.url), "utf8");

test("shared shell interaction colors are defined once as semantic root tokens", () => {
  for (const token of [
    "--control-hover-border",
    "--control-hover-surface",
    "--focus-ring",
    "--on-primary",
    "--sidebar-surface",
    "--search-surface",
  ]) {
    assert.match(css, new RegExp(`${token}:\\s*#[0-9a-f]{3,8}`, "i"), token);
  }

  for (const selector of [
    "button:hover:not(:disabled)",
    "button:focus-visible, input:focus-visible",
    "button.primary",
    ".desktop-sidebar",
    ".sidebar-new-document",
    ".topbar-search",
  ]) {
    assert.doesNotMatch(rule(selector), /#[0-9a-f]{3,8}/i, selector);
  }
});

test("shared shell compact visible text remains at least eleven pixels", () => {
  assertFontSizeAtLeast(".local-badge", 11);
  assertFontSizeAtLeast(".cabinet-summary small", 11);
  assertFontSizeAtLeast(".sidebar-section-label", 11);
  assertFontSizeAtLeast(".sidebar-document-tree button", 11);
  assertFontSizeAtLeast(".sidebar-footer", 11);
  assertFontSizeAtLeast(".topbar-search", 11);
});

test("shared shell keeps focus disabled and minimum geometry contracts", () => {
  assert.match(css, /button:focus-visible, input:focus-visible\s*\{[^}]*outline:\s*3px solid var\(--focus-ring\)/s);
  assert.match(css, /button:disabled\s*\{[^}]*opacity:\s*\.48/s);
  assert.match(css, /body\s*\{[^}]*min-width:\s*760px/s);
  for (const declaration of [
    "--shell-sidebar-width: 244px",
    "--shell-topbar-height: 50px",
    "--shell-inspector-width: 315px",
    "--shell-content-gap: 24px",
  ]) assert.ok(css.includes(declaration), declaration);
});

test("canvas toolbar keeps every action visible at the minimum supported viewport", () => {
  assert.match(css, /@media \(max-width: 1120px\)[\s\S]*?\.canvas-toolbar\s*\{[^}]*flex-wrap:\s*wrap/s);
  assert.match(css, /@media \(max-width: 1120px\)[\s\S]*?\.canvas-toolbar select\s*\{[^}]*max-width:\s*180px/s);
  assert.doesNotMatch(rule(".canvas-toolbar"), /overflow-x:\s*auto/);
});

test("document attachment panel keeps readable text and bounded actions", () => {
  for (const selector of [
    ".document-attachment-heading span",
    ".document-attachment-status",
    ".document-attachment-empty",
    ".document-attachment-list li strong",
    ".document-attachment-list li small",
    ".document-attachment-label",
  ]) assertFontSizeAtLeast(selector, 11);

  assert.match(rule(".document-attachment-list"), /max-height:\s*218px/);
  assert.match(rule(".document-attachment-list"), /overflow:\s*auto/);
  assert.match(rule(".document-attachment-actions"), /flex-wrap:\s*wrap/);
  assert.match(rule(".document-attachment-label"), /text-overflow:\s*ellipsis/);
  assert.match(rule(".document-attachment-label"), /white-space:\s*nowrap/);
});

test("document inspector keeps one bounded panel and stable tab geometry", () => {
  assert.match(rule(".document-inspector-tabs"), /grid-template-columns:\s*repeat\(3,\s*minmax\(0,\s*1fr\)\)/);
  assert.match(rule(".document-inspector-tabs button"), /min-height:\s*38px/);
  assertFontSizeAtLeast(".document-inspector-tabs button", 11);
  assert.match(rule(".document-inspector-panel"), /max-height:\s*410px/);
  assert.match(rule(".document-inspector-panel"), /overflow:\s*auto/);
  assertFontSizeAtLeast(".connected-documents strong, .connected-documents span", 11);
});

function assertFontSizeAtLeast(selector: string, minimum: number): void {
  const match = rule(selector).match(/font-size:\s*([0-9.]+)px/);
  assert.ok(match, `${selector} font-size`);
  assert.ok(Number(match[1]) >= minimum, `${selector} expected >= ${minimum}px, received ${match[1]}px`);
}

function rule(selector: string): string {
  const start = css.indexOf(`${selector} {`);
  assert.notEqual(start, -1, selector);
  const end = css.indexOf("}", start);
  assert.notEqual(end, -1, `${selector} closing brace`);
  return css.slice(start, end + 1);
}
