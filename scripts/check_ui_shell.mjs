import { readFileSync } from "node:fs";
import { join } from "node:path";

const root = process.cwd();
const source = readFileSync(join(root, "packages/ui/src/index.ts"), "utf8");

function fail(message) {
  console.error(`ui shell check failed: ${message}`);
  process.exit(1);
}

const requiredExports = [
  "WorkspaceShellZone",
  "WorkspaceShellModel",
  "createWorkspaceShellModel",
  "document-list",
  "editor",
  "metadata-panel",
  "history-panel",
  "status-bar",
  "command-palette",
];

for (const required of requiredExports) {
  if (!source.includes(required)) {
    fail(`missing ${required}`);
  }
}

const forbiddenPatterns = [
  /process\.env/,
  /node:fs/,
  /@tauri-apps\//,
  /cabinet-domain/,
  /cabinet-usecases/,
  /cabinet-adapters/,
  /\bGit\b/,
  /\bcommit\b/i,
  /\bPR\b/,
];

for (const pattern of forbiddenPatterns) {
  if (pattern.test(source)) {
    fail(`forbidden pattern ${pattern}`);
  }
}

console.log("ui shell check ok");
