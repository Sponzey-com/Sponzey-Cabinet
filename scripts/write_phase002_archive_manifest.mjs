import { createHash } from "node:crypto";
import { readdir, readFile, stat, writeFile } from "node:fs/promises";
import { join, relative } from "node:path";
import { pathToFileURL } from "node:url";

const root = process.cwd();
const archiveRoot = join(root, ".tasks", "phase002");
const manifestPath = join(archiveRoot, "archive-manifest.json");

async function main() {
  const files = await listFiles(archiveRoot);
  const entries = [];
  for (const absolutePath of files) {
    if (absolutePath === manifestPath) {
      continue;
    }
    const archivePath = normalizePath(relative(root, absolutePath));
    const sourcePath = archivePath.replace(/^\.tasks\/phase002\//, ".tasks/");
    const body = await readFile(absolutePath);
    const metadata = await stat(absolutePath);
    entries.push({
      sourcePath,
      archivePath,
      sizeBytes: metadata.size,
      sha256: createHash("sha256").update(body).digest("hex"),
    });
  }

  entries.sort((left, right) => left.archivePath.localeCompare(right.archivePath));
  await writeFile(
    manifestPath,
    `${JSON.stringify(
      {
        schemaVersion: 1,
        phase: "Phase 002",
        status: "archived",
        archiveRoot: ".tasks/phase002",
        generatedBy: "scripts/write_phase002_archive_manifest.mjs",
        entries,
      },
      null,
      2,
    )}\n`,
  );
  console.log(`phase002_archive_manifest_written=${normalizePath(relative(root, manifestPath))}`);
  console.log(`phase002_archive_manifest_entry_count=${entries.length}`);
}

async function listFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const absolutePath = join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await listFiles(absolutePath)));
      continue;
    }
    if (entry.isFile()) {
      files.push(absolutePath);
    }
  }
  return files;
}

function normalizePath(path) {
  return path.split("\\").join("/");
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
