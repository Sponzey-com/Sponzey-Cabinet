import { readFileSync } from "node:fs";
import { join } from "node:path";

const root = process.cwd();
const clientCoreSource = readFileSync(join(root, "packages/client-core/src/index.ts"), "utf8");
const uiSource = readFileSync(join(root, "packages/ui/src/index.ts"), "utf8");

function fail(message) {
  console.error(`link ui check failed: ${message}`);
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
    "LinkOverviewQuery",
    "LinkOverviewView",
    "BacklinkView",
    "UnresolvedLinkView",
    "OrphanDocumentView",
    "getLinkOverview",
    "createLinkOverviewQuery",
    "get-link-overview",
  ],
  "client-core",
);

requireIncludes(
  uiSource,
  [
    "LinkPanelViewModel",
    "BacklinkItemViewModel",
    "UnresolvedLinkItemViewModel",
    "OrphanDocumentItemViewModel",
    "createLinkPanelViewModel",
    "createOpenBacklinkCommand",
    "createOpenOrphanDocumentCommand",
    "queryName: \"get-link-overview\"",
    "targetSlug",
  ],
  "ui",
);

for (const name of ["createOpenBacklinkCommand", "createOpenOrphanDocumentCommand"]) {
  const factory = extractFunction(uiSource, name);
  if (!factory.includes("createCurrentDocumentQuery")) {
    fail(`${name} must create current document query`);
  }
  if (/getDocumentHistory|get-document-history|DocumentHistory/.test(factory)) {
    fail(`${name} must not use history query`);
  }
}

const panelFactory = extractFunction(uiSource, "createLinkPanelViewModel");
if (/body\s*:|currentBody|loadedBody/.test(panelFactory)) {
  fail("link panel view model must not include full document body");
}

const forbiddenPatterns = [
  /process\.env/,
  /from\s+["']fs["']/,
  /from\s+["']node:fs["']/,
  /@tauri-apps\//,
  /cabinet-domain/,
  /cabinet-usecases/,
  /cabinet-adapters/,
  /\bDocumentLink\b/,
  /\bLinkTarget\b/,
  /\bLinkStatus\b/,
  /\bGraphNodeKind\b/,
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

console.log("link ui check ok");
