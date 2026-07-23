import { copyFile, mkdir, rm } from "node:fs/promises";
import { join } from "node:path";

import { build } from "esbuild";

export async function buildDesktopAssets(root) {
  const publicDir = join(root, "apps", "desktop", "public");
  const distDir = join(root, "apps", "desktop", "dist");
  const indexHtml = join(distDir, "index.html");
  const stylesCss = join(distDir, "styles.css");
  const appBundle = join(distDir, "app.bundle.js");

  await rm(distDir, { recursive: true, force: true });
  await mkdir(distDir, { recursive: true });
  await Promise.all([
    copyFile(join(publicDir, "index.html"), indexHtml),
    copyFile(join(publicDir, "styles.css"), stylesCss),
  ]);
  const buildResult = await build({
    absWorkingDir: root,
    entryPoints: ["apps/desktop/src/desktop_entry.ts"],
    outfile: appBundle,
    bundle: true,
    format: "iife",
    target: "es2022",
    logLevel: "silent",
    metafile: true,
  });

  const sourcePaths = Object.freeze([...new Set([
    "apps/desktop/public/index.html",
    "apps/desktop/public/styles.css",
    ...Object.keys(buildResult.metafile.inputs)
      .map((path) => path.replaceAll("\\", "/"))
      .filter((path) => !path.startsWith("/") && !path.startsWith("../") && !path.includes("node_modules/")),
  ])].sort());

  return { distDir, indexHtml, stylesCss, appBundle, sourcePaths };
}
