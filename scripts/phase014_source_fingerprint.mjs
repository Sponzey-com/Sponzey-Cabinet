import { createHash } from "node:crypto";
import { readFile, readdir, stat } from "node:fs/promises";
import { join, relative } from "node:path";

const SOURCE_ROOTS = Object.freeze([
  "apps/desktop/src",
  "apps/desktop/tests",
  "apps/desktop/src-tauri/src",
  "apps/desktop/src-tauri/tests",
  "crates/cabinet-domain/src",
  "crates/cabinet-domain/tests",
  "crates/cabinet-ports/src",
  "crates/cabinet-ports/tests",
  "crates/cabinet-usecases/src",
  "crates/cabinet-usecases/tests",
  "crates/cabinet-core/src",
  "crates/cabinet-core/tests",
  "crates/cabinet-adapters/src",
  "crates/cabinet-adapters/tests",
  "crates/cabinet-platform/src",
  "crates/cabinet-platform/tests",
  "packages/client-core/src",
  "packages/client-core/tests",
  "packages/ui/src",
  "packages/ui/tests",
  "scripts",
]);

export async function fingerprintPhase014CurrentSource(root = process.cwd()) {
  const files = [];
  for (const directory of SOURCE_ROOTS) await collect(join(root, directory), files);
  const selected = files
    .filter((path) => /\.(?:rs|ts|mjs|sh|json)$/.test(path) && !path.includes("/dist/"))
    .sort();
  const hash = createHash("sha256");
  for (const path of selected) {
    hash.update(relative(root, path)).update("\0").update(await readFile(path)).update("\0");
  }
  return hash.digest("hex");
}

async function collect(path, output) {
  for (const name of await readdir(path)) {
    const child = join(path, name);
    const metadata = await stat(child);
    if (metadata.isDirectory()) await collect(child, output);
    else output.push(child);
  }
}
