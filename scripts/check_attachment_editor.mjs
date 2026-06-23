import { readFileSync } from "node:fs";
import { join } from "node:path";

const root = process.cwd();
const source = readFileSync(join(root, "packages/editor/src/index.ts"), "utf8");

function fail(message) {
  console.error(`attachment editor check failed: ${message}`);
  process.exit(1);
}

const requiredExports = [
  "AssetReferenceDecoration",
  "AssetReferenceOpenCommand",
  "findAssetReferenceDecorations",
  "createInsertAssetReferenceOperation",
  "createOpenAssetReferenceCommand",
  "insert-asset-reference",
  "open-asset-reference",
  "assetId",
  "asset:",
  "![[",
  "]]",
];

for (const required of requiredExports) {
  if (!source.includes(required)) {
    fail(`missing ${required}`);
  }
}

const requiredBehaviors = [
  "source.indexOf(\"![[asset:\", cursor)",
  "split(\"|\")",
  "trim()",
];

for (const required of requiredBehaviors) {
  if (!source.includes(required)) {
    fail(`missing behavior marker ${required}`);
  }
}

const forbiddenPatterns = [
  /process\.env/,
  /from\s+["']fs["']/,
  /from\s+["']node:fs["']/,
  /@tauri-apps\//,
  /cabinet-domain/,
  /cabinet-usecases/,
  /cabinet-adapters/,
  /\bAssetId\b/,
  /\bAssetStore\b/,
  /\bFile\b/,
  /\bGit\b/,
  /\bcommit\b/i,
  /\bPR\b/,
];

for (const pattern of forbiddenPatterns) {
  if (pattern.test(source)) {
    fail(`forbidden pattern ${pattern}`);
  }
}

console.log("attachment editor check ok");
