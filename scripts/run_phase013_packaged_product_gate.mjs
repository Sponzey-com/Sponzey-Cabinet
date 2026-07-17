import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";

import { runPackagedUiSmokeProcess } from "./desktop_packaged_ui_smoke.mjs";
import { PHASE013_PACKAGED_JOURNEYS, validatePhase013PackagedProductReport } from "./phase013_packaged_product_gate.mjs";

const sourceFiles = Object.freeze([
  "apps/desktop/src/desktop_entry.ts",
  "apps/desktop/src/packaged_ui_smoke.ts",
  "apps/desktop/src-tauri/src/main.rs",
  "apps/desktop/src-tauri/src/lib.rs",
  "apps/desktop/public/styles.css",
  "scripts/desktop_packaged_ui_smoke.mjs",
  "scripts/phase013_packaged_product_gate.mjs",
  "scripts/run_phase013_packaged_product_gate.mjs",
]);

async function main() {
  const root = process.cwd();
  const binary = process.argv[2];
  if (!binary) throw new Error("packaged binary argument missing");
  const sourceFingerprint = await fingerprintFiles(root, sourceFiles);
  const appFingerprint = await fingerprintFiles(root, [
    binary,
    "apps/desktop/dist/index.html",
    "apps/desktop/dist/app.bundle.js",
    "apps/desktop/dist/styles.css",
  ]);
  const smoke = await runPackagedUiSmokeProcess(binary, { timeoutMs: 180_000 });
  if (!smoke.passed) throw new Error(`packaged UI smoke failed: ${smoke.errorCode}`);
  const report = {
    marker: "phase013_packaged_product_gate=passed",
    sourceFingerprint,
    appFingerprint,
    platform: "macos",
    cleanProfile: true,
    externalRuntimeRequired: false,
    journeys: PHASE013_PACKAGED_JOURNEYS,
    sampleCount: smoke.sampleCount,
    p95Ms: smoke.p95Ms,
    errorCount: smoke.errorCount,
    actionCount: smoke.actionCount,
    durableReadbackCount: smoke.durableReadbackCount,
    keyboardDocumentWorkflowVerified: smoke.keyboardDocumentWorkflowVerified,
    diagnostics: "sanitized",
  };
  const validation = validatePhase013PackagedProductReport(report, sourceFingerprint);
  if (!validation.passed) throw new Error(`Phase 013 packaged report failed: ${validation.findingIds.join(",")}`);
  const releaseDir = join(root, ".tasks", "release");
  await mkdir(releaseDir, { recursive: true });
  await writeFile(join(releaseDir, "packaged-product-journey-phase013.json"), `${JSON.stringify(report, null, 2)}\n`);
  await writeFile(join(releaseDir, "packaged-product-journey-phase013.md"), renderMarkdown(report));
  console.log(report.marker);
  console.log(`source_fingerprint=${report.sourceFingerprint}`);
  console.log(`app_fingerprint=${report.appFingerprint}`);
  console.log(`action_count=${report.actionCount}`);
  console.log(`durable_readback_count=${report.durableReadbackCount}`);
  console.log(`keyboard_document_workflow_verified=${report.keyboardDocumentWorkflowVerified}`);
  console.log(`packaged_route_p95_ms=${report.p95Ms}`);
}

async function fingerprintFiles(root, files) {
  const hash = createHash("sha256");
  for (const path of [...files].sort()) {
    hash.update(path).update("\0");
    hash.update(await readFile(path.startsWith("/") ? path : join(root, path))).update("\0");
  }
  return hash.digest("hex");
}

function renderMarkdown(report) {
  return [
    "# Phase 013 Packaged macOS Product Journey",
    "",
    report.marker,
    `source_fingerprint=${report.sourceFingerprint}`,
    `app_fingerprint=${report.appFingerprint}`,
    `clean_profile=${report.cleanProfile}`,
    `external_runtime_required=${report.externalRuntimeRequired}`,
    `journeys=${report.journeys.join(",")}`,
    `sample_count=${report.sampleCount}`,
    `p95_ms=${report.p95Ms}`,
    `action_count=${report.actionCount}`,
    `durable_readback_count=${report.durableReadbackCount}`,
    `keyboard_document_workflow_verified=${report.keyboardDocumentWorkflowVerified}`,
    "",
    "The report contains only stable journey names, counts, timings, and fingerprints. User content, filenames, absolute paths, asset bytes, and credentials are excluded.",
    "",
  ].join("\n");
}

await main();
