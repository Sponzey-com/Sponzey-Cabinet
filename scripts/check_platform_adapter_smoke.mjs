import { readFileSync } from "node:fs";
import { join } from "node:path";

const root = process.cwd();
const webSource = readFileSync(join(root, "apps/web/src/index.ts"), "utf8");
const desktopSource = readFileSync(join(root, "apps/desktop/src/index.ts"), "utf8");
const desktopRustSource = readFileSync(join(root, "apps/desktop/src-tauri/src/lib.rs"), "utf8");

function fail(message) {
  console.error(`platform adapter smoke failed: ${message}`);
  process.exit(1);
}

function requireIncludes(source, values, label) {
  for (const value of values) {
    if (!source.includes(value)) {
      fail(`${label} missing ${value}`);
    }
  }
}

requireIncludes(
  webSource,
  [
    "createClientCapabilities",
    "createWorkspaceShellModel",
    "createEditorBoundaryDescriptor",
    "createAttachAssetClientCommand",
    "WebSelectedAsset",
    "mapWebAssetSelection",
    "createWebAttachAssetCommand",
    "web-local",
  ],
  "web shell",
);

requireIncludes(
  desktopSource,
  [
    "createClientCapabilities",
    "createWorkspaceShellModel",
    "createEditorBoundaryDescriptor",
    "createAttachAssetClientCommand",
    "DesktopSelectedAsset",
    "mapDesktopAssetSelection",
    "createDesktopAttachAssetCommand",
    "desktop-local",
  ],
  "desktop shell",
);

requireIncludes(desktopRustSource, ["cabinet_platform::layer_name", "route_desktop_command"], "desktop rust shell");

const forbiddenPatterns = [
  /process\.env/,
  /from\s+["']fs["']/,
  /from\s+["']node:fs["']/,
  /@tauri-apps\//,
  /cabinet-domain/,
  /cabinet-usecases/,
  /cabinet-adapters/,
  /localPath/,
  /\bFile\b/,
  /\bGit\b/,
  /\bcommit\b/i,
  /\bPR\b/,
];

for (const [label, source] of [
  ["web shell", webSource],
  ["desktop shell", desktopSource],
]) {
  for (const pattern of forbiddenPatterns) {
    if (pattern.test(source)) {
      fail(`${label} contains forbidden pattern ${pattern}`);
    }
  }
}

console.log("platform adapter smoke ok");
