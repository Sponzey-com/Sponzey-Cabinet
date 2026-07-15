import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";

import { buildPhase012SecurityManifest, writePhase012SecurityManifest } from "./phase012_security_manifest.mjs";
import { runSecurityLogScan } from "./security_log_scanner.mjs";

test("manifest contains three log classes and current Phase 012 targets only", () => {
  const manifest = buildPhase012SecurityManifest("a".repeat(64));
  assert.deepEqual(manifest.logClasses.map((entry) => entry.name), [
    "Product Log", "Field Debug Log", "Development Log",
  ]);
  assert.ok(manifest.scanTargets.every((target) => target.path.includes("phase012")));
  assert.ok(manifest.scanTargets.every((target) => !target.path.includes("phase011")));
  assert.ok(manifest.scanTargets.every((target) => target.required === true));
  assert.ok(manifest.scanTargets.some((target) => target.id === "packaged_ui_smoke"));
});

test("current evidence fixture passes and a denied fixture fails closed", async () => {
  const root = await mkdtemp(join(tmpdir(), "cabinet-phase012-security-"));
  const manifest = buildPhase012SecurityManifest("a".repeat(64));
  for (const target of manifest.scanTargets) {
    const path = join(root, target.path);
    await mkdir(dirname(path), { recursive: true });
    await writeFile(path, "event=phase012.current status=passed count=1\n");
  }
  const outputPath = await writePhase012SecurityManifest({ root, sourceFingerprint: "a".repeat(64) });
  const passed = await runSecurityLogScan({ manifestPath: outputPath, root });
  assert.equal(passed.passed, true);

  const firstTarget = join(root, manifest.scanTargets[0].path);
  await writeFile(firstTarget, "PHASE012_RAW_DOCUMENT_BODY_FIXTURE\n");
  const failed = await runSecurityLogScan({ manifestPath: outputPath, root });
  assert.equal(failed.passed, false);
  assert.equal(failed.findings.length, 1);
});

test("written manifest contains no absolute root or raw evidence", async () => {
  const root = await mkdtemp(join(tmpdir(), "cabinet-phase012-security-write-"));
  const outputPath = await writePhase012SecurityManifest({ root, sourceFingerprint: "b".repeat(64) });
  const text = await readFile(join(root, outputPath), "utf8");
  assert.equal(text.includes(root), false);
  assert.doesNotMatch(text, /private document|asset byte payload/);
});
