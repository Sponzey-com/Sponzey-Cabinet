import { mkdir, rm } from "node:fs/promises";
import { join } from "node:path";
import { spawn } from "node:child_process";

let build;

try {
  ({ build } = await import("esbuild"));
} catch {
  console.error("Web collaboration UI tests require npm dependencies. Run `npm install` once, then retry.");
  process.exit(1);
}

const root = process.cwd();
const outdir = join(root, ".tmp", "web-collaboration-ui-tests");

await rm(outdir, { recursive: true, force: true });
await mkdir(outdir, { recursive: true });

const outfile = join(outdir, "web-collaboration-ui-tests.mjs");

await build({
  stdin: {
    contents: [
      `import ${JSON.stringify(join(root, "packages/client-core/tests/collaboration_api_client_tests.ts"))};`,
      `import ${JSON.stringify(join(root, "packages/ui/tests/collaboration_ui_model_tests.ts"))};`,
    ].join("\n"),
    resolveDir: root,
    loader: "ts",
  },
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
    console.error(`web_collaboration_ui_tests_signal=${signal}`);
    process.exit(1);
  }
  process.exit(code ?? 1);
});
