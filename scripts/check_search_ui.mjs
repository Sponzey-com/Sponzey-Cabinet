import { readFileSync } from "node:fs";
import { join } from "node:path";

const root = process.cwd();
const clientCoreSource = readFileSync(join(root, "packages/client-core/src/index.ts"), "utf8");
const uiSource = readFileSync(join(root, "packages/ui/src/index.ts"), "utf8");

function fail(message) {
  console.error(`search ui check failed: ${message}`);
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
    "SearchDocumentsQuery",
    "SearchResultView",
    "SearchResultsPage",
    "searchDocuments",
    "createSearchDocumentsQuery",
    "search-documents",
  ],
  "client-core",
);

requireIncludes(
  uiSource,
  [
    "SearchPanelViewModel",
    "SearchResultItemViewModel",
    "createSearchPanelViewModel",
    "createOpenSearchResultCommand",
    "queryName: \"search-documents\"",
    "queryName: \"get-current-document\"",
    "snippet",
    "score",
  ],
  "ui",
);

const openResultFactory = extractFunction(uiSource, "createOpenSearchResultCommand");
if (!openResultFactory.includes("createCurrentDocumentQuery")) {
  fail("search result open command must create current document query");
}
if (/getDocumentHistory|get-document-history|DocumentHistory/.test(openResultFactory)) {
  fail("search result open command must not use history query");
}

const searchPanelFactory = extractFunction(uiSource, "createSearchPanelViewModel");
if (/body\s*:|currentBody|loadedBody/.test(searchPanelFactory)) {
  fail("search panel view model must not include full document body");
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

console.log("search ui check ok");
