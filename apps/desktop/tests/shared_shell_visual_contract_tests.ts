import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  PENPOT_20260721_PALETTE,
  PENPOT_20260721_TYPOGRAPHY,
} from "../src/penpot_20260721_design_contract.ts";

const css = await readFile(new URL("../public/styles.css", import.meta.url), "utf8");

test("shared shell root tokens match the Penpot 20260721 design contract", () => {
  const root = rule(":root");
  assert.match(root, new RegExp(`font-family:\\s*"${PENPOT_20260721_TYPOGRAPHY.fontFamily}"`));
  assert.match(root, new RegExp(`font-size:\\s*${PENPOT_20260721_TYPOGRAPHY.bodyFontSizePx}px`));
  assert.match(root, new RegExp(`line-height:\\s*${PENPOT_20260721_TYPOGRAPHY.bodyLineHeight}`));
  assert.match(root, new RegExp(`background:\\s*${PENPOT_20260721_PALETTE.canvas}`, "i"));
  assert.match(root, new RegExp(`color:\\s*${PENPOT_20260721_PALETTE.ink}`, "i"));
  assert.match(root, new RegExp(`--canvas:\\s*${PENPOT_20260721_PALETTE.canvas}`, "i"));
  assert.match(root, new RegExp(`--ink:\\s*${PENPOT_20260721_PALETTE.ink}`, "i"));
  assert.match(root, new RegExp(`--teal:\\s*${PENPOT_20260721_PALETTE.cabinetTeal}`, "i"));
  assert.match(root, new RegExp(`--blue:\\s*${PENPOT_20260721_PALETTE.knowledgeBlue}`, "i"));
  assert.match(root, new RegExp(`--amber:\\s*${PENPOT_20260721_PALETTE.decisionAmber}`, "i"));
  assert.match(root, new RegExp(`--rose:\\s*${PENPOT_20260721_PALETTE.referenceRose}`, "i"));
});

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

test("shared shell remains bounded to the desktop viewport while route content owns scrolling", () => {
  assert.match(rule(".desktop-shell"), /height:\s*100vh/);
  assert.match(rule(".desktop-shell"), /overflow:\s*hidden/);
  assert.match(rule(".desktop-main"), /overflow:\s*auto/);
});

test("graph renderer consumes the bounded grid row instead of forcing viewport overflow", () => {
  assert.match(rule(".graph-main"), /grid-template-rows:\s*auto auto minmax\(0,\s*1fr\)/);
  assert.match(rule(".graph-stage"), /min-height:\s*0/);
});

test("canvas renderer consumes the bounded grid row instead of forcing viewport overflow", () => {
  assert.match(rule(".canvas-main"), /grid-template-rows:\s*auto auto minmax\(0,\s*1fr\)/);
  assert.match(css, /\.canvas-stage\s*\{\s*position:\s*relative;\s*min-height:\s*0/);
});

test("canvas toolbar keeps every action visible at the minimum supported viewport", () => {
  assert.match(rule(".canvas-toolbar"), /overflow-x:\s*auto/);
  assert.match(rule(".canvas-toolbar"), /overflow-y:\s*hidden/);
  assert.match(rule(".canvas-toolbar"), /scrollbar-gutter:\s*stable/);
  assert.match(css, /@media \(max-width: 1280px\)[\s\S]*?\.canvas-toolbar\s*\{[^}]*flex-wrap:\s*wrap/s);
  assert.match(css, /@media \(max-width: 1280px\)[\s\S]*?\.canvas-toolbar select\s*\{[^}]*max-width:\s*180px/s);
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
  assert.match(rule(".document-attachment-operation-list"), /max-height:\s*184px/);
  assert.match(rule(".document-attachment-operation-list"), /overflow:\s*auto/);
  assert.match(rule(".document-attachment-operation"), /grid-template-columns:/);
  assert.match(rule(".document-attachment-operation-copy strong,\n.document-attachment-operation-copy small"), /text-overflow:\s*ellipsis/);
  assert.match(rule(".document-attachment-operation-copy strong,\n.document-attachment-operation-copy small"), /white-space:\s*nowrap/);
});

test("document inspector keeps one bounded panel and stable tab geometry", () => {
  assert.match(rule(".document-inspector-tabs"), /grid-template-columns:\s*repeat\(3,\s*minmax\(0,\s*1fr\)\)/);
  assert.match(rule(".document-inspector-tabs button"), /min-height:\s*38px/);
  assertFontSizeAtLeast(".document-inspector-tabs button", 11);
  assert.match(rule(".document-inspector-panel"), /max-height:\s*410px/);
  assert.match(rule(".document-inspector-panel"), /overflow:\s*auto/);
  assertFontSizeAtLeast(".connected-documents strong, .connected-documents span", 11);
});

test("document formatting toolbar keeps icon commands at the Penpot touch target", () => {
  assert.match(rule(".formatting-toolbar"), /display:\s*flex/);
  assert.match(rule(".formatting-command"), /min-width:\s*44px/);
  assert.match(rule(".formatting-command"), /min-height:\s*44px/);
  assert.match(rule(".formatting-command"), /justify-content:\s*center/);
  assert.match(rule(".formatting-command"), /padding:\s*0/);
});

test("global search overlay is a bounded dialog panel inside the shell viewport", () => {
  assert.match(rule(".search-main"), /display:\s*grid/);
  assert.match(rule(".search-main"), /place-items:\s*start center/);
  assert.match(rule(".global-search-overlay"), /width:\s*min\(100%,\s*1128px\)/);
  assert.match(rule(".global-search-overlay"), /max-height:\s*calc\(100vh - 120px\)/);
  assert.match(rule(".global-search-overlay"), /overflow:\s*auto/);
  assert.match(rule(".global-search-heading"), /display:\s*flex/);
  assert.match(rule(".global-search-close"), /min-width:\s*44px/);
  assert.match(rule(".global-search-close"), /min-height:\s*44px/);
  assert.match(rule(".global-search-footer"), /border-top:\s*1px solid var\(--border\)/);
  assert.match(rule(".global-search-empty"), /min-height:\s*160px/);
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
