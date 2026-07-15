import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import os from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

import {
  Phase012QueryPerformanceState,
  advancePhase012QueryPerformanceState,
  parseNativeQueryBenchmarkOutput,
  validatePhase012QueryPerformanceReport,
} from "./phase012_query_performance.mjs";

const execFileAsync = promisify(execFile);
const SOURCE_FILES = Object.freeze([
  "crates/cabinet-platform/Cargo.toml",
  "crates/cabinet-platform/src/bin/phase012_query_benchmark.rs",
  "crates/cabinet-usecases/src/graph.rs",
  "crates/cabinet-ports/src/link_index.rs",
  "crates/cabinet-adapters/src/local_link_index.rs",
  "crates/cabinet-adapters/src/durable_local_link_index.rs",
  "crates/cabinet-adapters/src/durable_local_graph_projection.rs",
  "crates/cabinet-adapters/src/durable_canvas_repository.rs",
  "crates/cabinet-adapters/src/durable_asset_metadata_catalog.rs",
  "scripts/phase012_query_performance.mjs",
  "scripts/run_phase012_query_performance.mjs",
]);

const FIXTURE = Object.freeze({
  seed: 12012,
  documentCount: 10_000,
  historyVersionCount: 1_000,
  linkCount: 50_000,
  graphNodeCount: 10_000,
  graphEdgeCount: 50_000,
  canvasNodeCount: 2_000,
  canvasEdgeCount: 4_000,
  assetCount: 10_000,
  pageSize: 50,
});

export async function runPhase012QueryPerformance(root = process.cwd()) {
  let state = Phase012QueryPerformanceState.Pending;
  state = advancePhase012QueryPerformanceState(state, "BUILD");
  const sourceFingerprint = await fingerprint(root, SOURCE_FILES);
  const fixtureHash = createHash("sha256").update(JSON.stringify(FIXTURE)).digest("hex");
  state = advancePhase012QueryPerformanceState(state, "MEASURE");
  let stdout;
  try {
    ({ stdout } = await execFileAsync("cargo", [
      "run", "--release", "--quiet", "-p", "cabinet-platform", "--bin", "phase012-query-benchmark",
    ], { cwd: root, maxBuffer: 4 * 1024 * 1024 }));
  } catch (error) {
    state = advancePhase012QueryPerformanceState(state, "FAIL");
    const detail = typeof error?.stdout === "string" ? error.stdout.trim() : "benchmark process failed";
    throw new Error(`Phase 012 native query benchmark failed in ${state}: ${detail}`);
  }
  const parsed = parseNativeQueryBenchmarkOutput(stdout);
  state = advancePhase012QueryPerformanceState(state, "VALIDATE");
  const report = {
    marker: "phase012_native_query_performance=passed",
    sourceFingerprint,
    fixtureHash,
    buildProfile: "release",
    timingBoundary: "native_backend_only",
    percentileMethod: parsed.percentileMethod,
    warmupCount: parsed.warmupCount,
    sampleCount: parsed.sampleCount,
    budgetMs: 300,
    fixture: FIXTURE,
    environment: {
      os: os.platform(),
      arch: os.arch(),
      cpu: sanitize(os.cpus()[0]?.model ?? "unknown"),
      logicalCpuCount: os.cpus().length,
      totalMemoryBytes: os.totalmem(),
      queryState: "warm_indexed",
    },
    queries: parsed.queries,
    diagnostics: "sanitized",
  };
  const validation = validatePhase012QueryPerformanceReport(report, sourceFingerprint);
  if (!validation.passed) {
    state = advancePhase012QueryPerformanceState(state, "FAIL");
    throw new Error(`Phase 012 query report failed in ${state}: ${validation.findingIds.join(",")}`);
  }
  state = advancePhase012QueryPerformanceState(state, "WRITE");
  const releaseDir = join(root, ".tasks", "release");
  await mkdir(releaseDir, { recursive: true });
  await writeFile(join(releaseDir, "query-performance-phase012.json"), `${JSON.stringify(report, null, 2)}\n`);
  await writeFile(join(releaseDir, "query-performance-phase012.md"), renderMarkdown(report));
  state = advancePhase012QueryPerformanceState(state, "PASS");
  return { ...report, runnerState: state };
}

export function renderMarkdown(report) {
  const lines = [
    "# Phase 012 Native Query Performance",
    "",
    report.marker,
    `source_fingerprint=${report.sourceFingerprint}`,
    `fixture_hash=${report.fixtureHash}`,
    `timing_boundary=${report.timingBoundary}`,
    `build_profile=${report.buildProfile}`,
    `warmup_count=${report.warmupCount}`,
    `sample_count=${report.sampleCount}`,
    `percentile_method=${report.percentileMethod}`,
    "",
    "| Query | Path | Result count | p50 ms | p95 ms | max ms | Budget |",
    "| --- | --- | ---: | ---: | ---: | ---: | ---: |",
  ];
  for (const query of report.queries) {
    lines.push(`| ${query.id} | ${query.queryPath} | ${query.resultCount} | ${query.p50Ms.toFixed(3)} | ${query.p95Ms.toFixed(3)} | ${query.maxMs.toFixed(3)} | 300 |`);
  }
  lines.push(
    "",
    "This evidence measures the native local adapter and usecase boundary only. Controller dispatch through rendered marker is validated by the separate Phase 012 end-to-end performance gate.",
    "",
    "The artifact contains stable query identifiers, counts, timing metrics, hashes, and sanitized machine metadata only. It excludes document content, search text, filenames, absolute paths, asset bytes, and credentials.",
    "",
  );
  return lines.join("\n");
}

async function fingerprint(root, files) {
  const hash = createHash("sha256");
  for (const relativePath of [...files].sort()) {
    hash.update(relativePath);
    hash.update("\0");
    hash.update(await readFile(join(root, relativePath)));
    hash.update("\0");
  }
  return hash.digest("hex");
}

function sanitize(value) {
  return String(value).replace(/[\r\n\t]/g, " ").slice(0, 120);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const report = await runPhase012QueryPerformance();
  console.log(report.marker);
  console.log(`source_fingerprint=${report.sourceFingerprint}`);
  for (const query of report.queries) console.log(`${query.id}_p95_ms=${query.p95Ms.toFixed(3)}`);
}
