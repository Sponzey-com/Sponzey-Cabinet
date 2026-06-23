import { readFileSync } from "node:fs";
import { join } from "node:path";

const root = process.cwd();
const source = readFileSync(join(root, "packages/editor/src/index.ts"), "utf8");

function fail(message) {
  console.error(`editor integration check failed: ${message}`);
  process.exit(1);
}

const requiredExports = [
  "EditorDirtyState",
  "EditorDocumentSnapshot",
  "EditorSessionModel",
  "EditorSaveCommand",
  "createEditorSession",
  "applyEditorContentChange",
  "createEditorLoadOperation",
  "createEditorSaveCommand",
  "clean",
  "dirty",
  "load-document",
  "save-document",
];

for (const required of requiredExports) {
  if (!source.includes(required)) {
    fail(`missing ${required}`);
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
  /\bCodeMirrorState\b/,
  /\bEditorState\b/,
  /\bGit\b/,
  /\bcommit\b/i,
  /\bPR\b/,
];

for (const pattern of forbiddenPatterns) {
  if (pattern.test(source)) {
    fail(`forbidden pattern ${pattern}`);
  }
}

console.log("editor integration check ok");
