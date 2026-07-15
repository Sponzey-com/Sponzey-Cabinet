import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { promisify } from "node:util";

import { validateWorkspaceHomePerformanceReport } from "./phase011_workspace_home_performance.mjs";

const execFileAsync = promisify(execFile);

export async function runWorkspaceHomePerformanceEvidence(root) {
  const { stdout } = await execFileAsync("cargo", [
    "run",
    "--release",
    "--quiet",
    "-p",
    "cabinet-platform",
    "--bin",
    "workspace_home_benchmark",
  ], { cwd: root, maxBuffer: 1024 * 1024 });
  const values = Object.fromEntries(
    stdout.trim().split("\n").map((line) => {
      const index = line.indexOf("=");
      return [line.slice(0, index), line.slice(index + 1)];
    }),
  );
  if (values.workspace_home_benchmark !== "passed") throw new Error("workspace home benchmark failed");
  const inventory = await readFile(join(root, ".tasks", "phase011-current-implementation-inventory.md"), "utf8");
  const sourceFingerprint = inventory.match(/source_fingerprint=([a-f0-9]{64})/)?.[1];
  if (!sourceFingerprint) throw new Error("Phase011 source fingerprint missing");
  const fixtureContract = "current=10000;versions=100000;home_projection=100;query=bounded";
  const report = {
    marker: "phase011_workspace_home_performance=passed",
    sourceFingerprint,
    fixtureHash: createHash("sha256").update(fixtureContract).digest("hex"),
    currentDocumentCount: Number(values.current_document_count),
    totalVersionCount: Number(values.total_version_count),
    warmupCount: Number(values.warmup_count),
    sampleCount: Number(values.sample_count),
    p50Ms: Number(values.p50_ms),
    p95Ms: Number(values.p95_ms),
    maxMs: Number(values.max_ms),
    buildProfile: "release",
    queryPath: values.query_path,
    diagnostics: "sanitized",
  };
  const validation = validateWorkspaceHomePerformanceReport(report, sourceFingerprint);
  if (!validation.passed) throw new Error(`workspace home performance failed: ${validation.findingIds.join(",")}`);
  await writeFile(join(root, ".tasks", "release", "workspace-home-performance-phase011.json"), `${JSON.stringify(report, null, 2)}\n`);
  return report;
}

if (process.argv[1]?.endsWith("run_phase011_workspace_home_performance.mjs")) {
  const report = await runWorkspaceHomePerformanceEvidence(process.cwd());
  console.log("phase011_workspace_home_performance=passed");
  console.log(`p95_ms=${report.p95Ms}`);
}
