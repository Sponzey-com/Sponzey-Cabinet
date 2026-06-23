import { readFileSync } from "node:fs";
import { join } from "node:path";

const root = process.cwd();
const clientCoreSource = readFileSync(join(root, "packages/client-core/src/index.ts"), "utf8");
const uiSource = readFileSync(join(root, "packages/ui/src/index.ts"), "utf8");

function fail(message) {
  console.error(`asset ui check failed: ${message}`);
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
    "ListDocumentAssetsQuery",
    "AssetView",
    "DocumentAssetsPage",
    "SelectedAssetDraft",
    "AttachAssetCommand",
    "listDocumentAssets",
    "attachAsset",
    "list-document-assets",
    "attach-file-to-document",
  ],
  "client-core",
);

requireIncludes(
  uiSource,
  [
    "AssetPanelViewModel",
    "AssetItemViewModel",
    "createAssetPanelViewModel",
    "createAttachAssetCommand",
    "queryName: \"list-document-assets\"",
    "commandName: \"attach-file-to-document\"",
    "missing",
    "available",
  ],
  "ui",
);

const panelFactory = extractFunction(uiSource, "createAssetPanelViewModel");
if (/path\s*:|localPath|filesystem|picker|bytes/.test(panelFactory)) {
  fail("asset panel view model must not expose platform or storage details");
}

const attachFactory = extractFunction(uiSource, "createAttachAssetCommand");
if (/from\s+["']fs["']|from\s+["']node:fs["']|@tauri-apps\/|localPath|picker/.test(attachFactory)) {
  fail("attach command factory must not access filesystem or platform picker");
}

const forbiddenPatterns = [
  /process\.env/,
  /from\s+["']fs["']/,
  /from\s+["']node:fs["']/,
  /@tauri-apps\//,
  /cabinet-domain/,
  /cabinet-usecases/,
  /cabinet-adapters/,
  /\bAssetStore\b/,
  /\bDocumentAssetRepository\b/,
  /\bFile\b/,
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

console.log("asset ui check ok");
