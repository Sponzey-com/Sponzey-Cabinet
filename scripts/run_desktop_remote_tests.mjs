import { mkdir, rm } from "node:fs/promises";
import { join } from "node:path";
import { spawn } from "node:child_process";

let build;

try {
  ({ build } = await import("esbuild"));
} catch {
  console.error("Desktop remote tests require npm dependencies. Run `npm install` once, then retry.");
  process.exit(1);
}

const root = process.cwd();
const outdir = join(root, ".tmp", "desktop-remote-tests");

await rm(outdir, { recursive: true, force: true });
await mkdir(outdir, { recursive: true });

const outfile = join(outdir, "desktop-remote-tests.mjs");

await build({
  entryPoints: [join(root, "apps/desktop/tests/desktop_remote_workspace_tests.ts")],
  outfile,
  bundle: true,
  platform: "node",
  format: "esm",
  target: "node20",
  logLevel: "silent",
});

const child = spawn(process.execPath, ["--test", outfile], {
  cwd: root,
  stdio: "inherit",
});

child.on("exit", (code, signal) => {
  if (signal) {
    console.error(`desktop_remote_tests_signal=${signal}`);
    process.exit(1);
  }
  process.exit(code ?? 1);
});
