import { readFileSync } from "node:fs";
import { join } from "node:path";

const root = process.cwd();
const clientCoreSource = readFileSync(join(root, "packages/client-core/src/index.ts"), "utf8");
const uiSource = readFileSync(join(root, "packages/ui/src/index.ts"), "utf8");

function fail(message) {
  console.error(`current/history ui check failed: ${message}`);
  process.exit(1);
}

function requireIncludes(source, values, label) {
  for (const value of values) {
    if (!source.includes(value)) {
      fail(`${label} missing ${value}`);
    }
  }
}

function extractFunction(source, name) {
  const start = source.indexOf(`export function ${name}`);
  if (start === -1) {
    fail(`missing function ${name}`);
  }
  const next = source.indexOf("\nexport ", start + 1);
  return source.slice(start, next === -1 ? source.length : next);
}

requireIncludes(
  clientCoreSource,
  [
    "CurrentDocumentQuery",
    "DocumentHistoryQuery",
    "CurrentDocumentView",
    "DocumentHistoryEntry",
    "DocumentHistoryPage",
    "CabinetDocumentClient",
    "getCurrentDocument",
    "getDocumentHistory",
    "get-current-document",
    "get-document-history",
  ],
  "client-core",
);

requireIncludes(
  uiSource,
  [
    "CurrentDocumentViewModel",
    "HistoryPanelViewModel",
    "createCurrentDocumentViewModel",
    "createHistoryPanelViewModel",
    "mode: \"current\"",
    "mode: \"history\"",
    "queryName: \"get-current-document\"",
    "queryName: \"get-document-history\"",
  ],
  "ui",
);

const currentFactory = extractFunction(uiSource, "createCurrentDocumentViewModel");
if (/history|History|getDocumentHistory|get-document-history/.test(currentFactory)) {
  fail("current document view model must not reference history query or history entries");
}

const historyFactory = extractFunction(uiSource, "createHistoryPanelViewModel");
if (/body\s*:|currentBody|loadedBody/.test(historyFactory)) {
  fail("history panel view model must not include document body");
}

const forbiddenPatterns = [
  /process\.env/,
  /from\s+["']fs["']/,
  /from\s+["']node:fs["']/,
  /@tauri-apps\//,
  /cabinet-domain/,
  /cabinet-usecases/,
  /cabinet-adapters/,
  /\bGit\b/,
  /\bcommit\b/i,
  /\bPR\b/,
];

for (const source of [clientCoreSource, uiSource]) {
  for (const pattern of forbiddenPatterns) {
    if (pattern.test(source)) {
      fail(`forbidden pattern ${pattern}`);
    }
  }
}

console.log("current/history ui check ok");
