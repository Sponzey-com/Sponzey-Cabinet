import { mkdir, readFile, readdir, rename, stat, writeFile } from "node:fs/promises";
import { join, relative } from "node:path";

import { auditCurrentScopeSources } from "./phase014_current_scope_audit.mjs";
import { fingerprintPhase014CurrentSource } from "./phase014_source_fingerprint.mjs";

const root = process.cwd();
const domainSources = await readSources("crates/cabinet-domain/src", ".rs");
const usecaseSources = await readSources("crates/cabinet-usecases/src", ".rs");
const runtimeSources = [
  ...(await readSources("apps/desktop/src", ".ts")),
  ...(await readSources("apps/desktop/src-tauri/src", ".rs")),
];
const releaseTexts = await Promise.all([
  "final-release-gate-phase013.md",
  "packaged-product-journey-phase013.md",
  "query-render-performance-phase013.md",
  "route-geometry-phase013.json",
  "responsive-stress-phase013.json",
].map(async (name) => ({
  path: `.tasks/release/${name}`,
  text: await readFile(join(root, ".tasks", "release", name), "utf8"),
})));
const result = auditCurrentScopeSources({ domainSources, usecaseSources, runtimeSources, releaseTexts });
if (!result.passed) throw new Error(`PHASE014_CURRENT_SCOPE_AUDIT_FAILED:${result.findingIds.join(",")}`);
const report = Object.freeze({
  marker: "phase014_current_scope_audit=passed",
  state: "Passed",
  sourceFingerprint: await fingerprintPhase014CurrentSource(root),
  scannedFileCount: result.scannedFileCount,
  findingCount: 0,
  domainExternalIoFindingCount: 0,
  usecaseExternalIoFindingCount: 0,
  runtimeEnvironmentFindingCount: 0,
  desktopFieldDebugActivationCount: 0,
  releaseSensitiveFindingCount: 0,
  diagnostics: "sanitized",
});
await mkdir(join(root, ".tasks", "release"), { recursive: true });
await writeAtomic(
  join(root, ".tasks", "release", "current-scope-audit-phase014.json"),
  `${JSON.stringify(report, null, 2)}\n`,
);
console.log(report.marker);
console.log(`source_fingerprint=${report.sourceFingerprint}`);
console.log(`scanned_file_count=${report.scannedFileCount}`);
console.log("finding_count=0");

async function readSources(directory, extension) {
  const files = [];
  await collect(join(root, directory), files);
  return Promise.all(files.filter((path) => path.endsWith(extension)).sort().map(async (path) => ({
    path: relative(root, path),
    text: await readFile(path, "utf8"),
  })));
}

async function collect(path, output) {
  for (const name of await readdir(path)) {
    const child = join(path, name);
    const metadata = await stat(child);
    if (metadata.isDirectory()) await collect(child, output);
    else output.push(child);
  }
}

async function writeAtomic(path, content) {
  const temporary = `${path}.tmp`;
  await writeFile(temporary, content, "utf8");
  await rename(temporary, path);
}
