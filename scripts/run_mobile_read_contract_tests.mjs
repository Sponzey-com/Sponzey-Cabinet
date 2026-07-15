import { mkdir, readFile, rm } from "node:fs/promises";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { spawn } from "node:child_process";

let build;

try {
  ({ build } = await import("esbuild"));
} catch {
  console.error("Mobile read contract tests require npm dependencies. Run `npm install` once, then retry.");
  process.exit(1);
}

const root = process.cwd();
const outdir = join(root, ".tmp", "mobile-read-contract-tests");
const sourceScanTargets = [
  "apps/mobile/src/index.ts",
  "apps/mobile/tests/mobile_read_skeleton_tests.ts",
  "apps/mobile/tests/mobile_push_notification_tests.ts",
  "packages/client-core/tests/mobile_read_contract_tests.ts",
];
const forbiddenProductLogTokens = [
  "product_log_event",
  "ProductLogEvent",
  "ProductLogger",
  "write_product",
  "writeProduct",
];
const forbiddenMobileSdkTokens = [
  "UIKit",
  "SwiftUI",
  "androidx.",
  "android.content",
  "android.app",
  "ReactNative",
  "Capacitor",
  "Flutter",
];

await scanMobileReadBoundaries();

await rm(outdir, { recursive: true, force: true });
await mkdir(outdir, { recursive: true });

const entries = [
  {
    entryPoint: join(root, "packages/client-core/tests/mobile_read_contract_tests.ts"),
    outfile: join(outdir, "mobile-read-contract-tests.mjs"),
  },
  {
    entryPoint: join(root, "apps/mobile/tests/mobile_read_skeleton_tests.ts"),
    outfile: join(outdir, "mobile-read-skeleton-tests.mjs"),
  },
  {
    entryPoint: join(root, "apps/mobile/tests/mobile_push_notification_tests.ts"),
    outfile: join(outdir, "mobile-push-notification-tests.mjs"),
  },
];

for (const entry of entries) {
  await build({
    entryPoints: [entry.entryPoint],
    outfile: entry.outfile,
    bundle: true,
    platform: "node",
    format: "esm",
    target: "node20",
    logLevel: "silent",
  });
}

const child = spawn(process.execPath, ["--test", ...entries.map((entry) => entry.outfile)], {
  cwd: root,
  stdio: "inherit",
});

child.on("exit", (code, signal) => {
  if (signal) {
    console.error(`mobile_read_contract_tests_signal=${signal}`);
    process.exit(1);
  }
  process.exit(code ?? 1);
});

async function scanMobileReadBoundaries() {
  for (const target of sourceScanTargets) {
    const absolutePath = join(root, target);
    if (!existsSync(absolutePath)) {
      console.error(`mobile_read_boundary_scan=failed`);
      console.error(`missing_source=${target}`);
      process.exit(1);
    }
    const source = await readFile(absolutePath, "utf8");
    for (const token of forbiddenProductLogTokens) {
      if (source.includes(token)) {
        console.error("mobile_read_boundary_scan=failed");
        console.error("failure_category=mobile_client_product_log_direct_write");
        console.error(`file_path=${target}`);
        process.exit(1);
      }
    }
    for (const token of forbiddenMobileSdkTokens) {
      if (source.includes(token)) {
        console.error("mobile_read_boundary_scan=failed");
        console.error("failure_category=mobile_sdk_type_leakage");
        console.error(`file_path=${target}`);
        process.exit(1);
      }
    }
  }

  console.log("mobile_read_boundary_scan=passed");
}
