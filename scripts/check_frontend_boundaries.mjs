import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

const root = process.cwd();

function fail(message) {
  console.error(`frontend boundary violation: ${message}`);
  process.exit(1);
}

function readJson(path) {
  if (!existsSync(path)) {
    fail(`missing required file: ${path}`);
  }
  return JSON.parse(readFileSync(path, "utf8"));
}

function requireFile(path) {
  if (!existsSync(path)) {
    fail(`missing required file: ${path}`);
  }
}

const rootManifest = readJson(join(root, "package.json"));
const expectedWorkspaces = ["packages/*", "apps/*"];
const workspaces = rootManifest.workspaces ?? [];
for (const workspace of expectedWorkspaces) {
  if (!workspaces.includes(workspace)) {
    fail(`root package.json must include workspace ${workspace}`);
  }
}

const packages = [
  ["packages/client-core", "@sponzey-cabinet/client-core"],
  ["packages/ui", "@sponzey-cabinet/ui"],
  ["packages/editor", "@sponzey-cabinet/editor"],
  ["apps/web", "@sponzey-cabinet/web"],
  ["apps/desktop", "@sponzey-cabinet/desktop"],
];

for (const [dir, name] of packages) {
  const manifest = readJson(join(root, dir, "package.json"));
  if (manifest.name !== name) {
    fail(`${dir}/package.json must use package name ${name}`);
  }
  requireFile(join(root, dir, "src/index.ts"));
}

const forbiddenSourcePatterns = [
  /from\s+["']fs["']/,
  /from\s+["']node:fs["']/,
  /from\s+["']child_process["']/,
  /from\s+["']node:child_process["']/,
  /process\.env/,
  /@tauri-apps\//,
  /cabinet-domain/,
  /cabinet-usecases/,
  /cabinet-adapters/,
];

const sourceFiles = packages.map(([dir]) => join(root, dir, "src/index.ts"));
for (const sourceFile of sourceFiles) {
  const source = readFileSync(sourceFile, "utf8");
  for (const pattern of forbiddenSourcePatterns) {
    if (pattern.test(source)) {
      fail(`${sourceFile} contains forbidden pattern ${pattern}`);
    }
  }
}

const clientCore = readJson(join(root, "packages/client-core/package.json"));
if (clientCore.dependencies && Object.keys(clientCore.dependencies).length > 0) {
  fail("client-core must not depend on UI, editor, platform, or external runtime packages in this scaffold");
}

const ui = readJson(join(root, "packages/ui/package.json"));
if (!ui.dependencies?.["@sponzey-cabinet/client-core"]) {
  fail("ui must depend on client-core boundary");
}

const editor = readJson(join(root, "packages/editor/package.json"));
if (!editor.dependencies?.["@sponzey-cabinet/client-core"]) {
  fail("editor must depend on client-core boundary");
}

console.log("frontend boundaries ok");
