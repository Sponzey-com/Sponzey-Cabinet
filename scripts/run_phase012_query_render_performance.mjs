import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

import { build } from "esbuild";

import {
  parseDesktopQueryRenderBenchmarkOutput,
  transitionQueryRenderPerformanceState,
  validateQueryRenderPerformanceReport,
} from "./phase012_query_render_performance.mjs";

const execFileAsync = promisify(execFile);
const policy = Object.freeze({
  warmupCount: 30,
  measuredCount: 200,
  percentile: "nearest_rank",
  budgetMs: 300,
  ipcAllowanceMs: 25,
});
const sourceFiles = Object.freeze([
  "apps/desktop/tests/phase012_query_render_benchmark.ts",
  "apps/desktop/src/desktop_document_authoring_controller.ts",
  "apps/desktop/src/desktop_navigator_controller.ts",
  "apps/desktop/src/desktop_link_overview_controller.ts",
  "apps/desktop/src/desktop_graph_controller.ts",
  "apps/desktop/src/desktop_canvas_controller.ts",
  "apps/desktop/src/desktop_asset_controller.ts",
  "apps/desktop/src/document_history_window.ts",
  "apps/desktop/src/ko_kr_catalog.ts",
  "apps/desktop/src/react_document_authoring_workbench.ts",
  "apps/desktop/public/styles.css",
  "apps/desktop/src/react_document_navigator.ts",
  "apps/desktop/src/react_exploration_surfaces.ts",
  "scripts/phase012_query_render_performance.mjs",
  "scripts/run_phase012_query_render_performance.mjs",
]);

export async function runPhase012QueryRenderPerformance(root = process.cwd()) {
  let state = "Pending";
  state = transitionQueryRenderPerformanceState(state, "LoadNative").state;
  const nativeReport = JSON.parse(await readFile(join(root, ".tasks/release/query-performance-phase012.json"), "utf8"));
  if (nativeReport.marker !== "phase012_native_query_performance=passed") throw new Error("native performance evidence is invalid");
  const sourceFingerprint = await fingerprint(root, sourceFiles);
  const temporary = join(root, ".tasks", "tmp", "phase012-query-render-benchmark.cjs");
  await mkdir(join(root, ".tasks", "tmp"), { recursive: true });
  state = transitionQueryRenderPerformanceState(state, "Measure").state;
  try {
    await build({
      absWorkingDir: root,
      entryPoints: ["apps/desktop/tests/phase012_query_render_benchmark.ts"],
      outfile: temporary,
      bundle: true,
      platform: "node",
      format: "cjs",
      minify: true,
      define: { "process.env.NODE_ENV": '"production"' },
      logLevel: "silent",
    });
    const { stdout } = await execFileAsync(process.execPath, [temporary], { cwd: root, maxBuffer: 4 * 1024 * 1024 });
    const desktopQueries = parseDesktopQueryRenderBenchmarkOutput(stdout);
    const queries = desktopQueries.map((desktop) => {
      const native = nativeReport.queries.find((candidate) => candidate.id === desktop.queryId);
      if (!native) throw new Error(`native performance query missing: ${desktop.queryId}`);
      const combinedP95Ms = native.p95Ms + desktop.desktopRenderP95Ms + policy.ipcAllowanceMs;
      return Object.freeze({
        ...desktop,
        nativeP95Ms: native.p95Ms,
        ipcAllowanceMs: policy.ipcAllowanceMs,
        combinedP95Ms: Number(combinedP95Ms.toFixed(6)),
      });
    });
    state = transitionQueryRenderPerformanceState(state, "Validate").state;
    const report = {
      marker: "phase012_query_render_performance=passed",
      state: "Passed",
      sourceFingerprint,
      nativeSourceFingerprint: nativeReport.sourceFingerprint,
      fixtureHash: nativeReport.fixtureHash,
      timingBoundary: "desktop_controller_dispatch_to_rendered_markup_marker",
      combinedMethod: "native_p95_plus_desktop_render_p95_plus_ipc_allowance",
      packagedEndToEndMeasured: false,
      followUp: "phase012_packaged_end_to_end",
      policy,
      environment: {
        platform: os.platform() === "darwin" ? "macos" : "unsupported",
        architecture: os.arch(),
        profile: "release",
        renderer: "react_dom_server_production_bundle",
      },
      queries,
      diagnostics: "sanitized",
    };
    const validation = validateQueryRenderPerformanceReport(report, {
      sourceFingerprint,
      nativeSourceFingerprint: nativeReport.sourceFingerprint,
      fixtureHash: nativeReport.fixtureHash,
    });
    if (!validation.passed) throw new Error(`query render performance validation failed: ${validation.findingIds.join(",")}`);
    state = transitionQueryRenderPerformanceState(state, "Write").state;
    const release = join(root, ".tasks", "release");
    await mkdir(release, { recursive: true });
    await writeFile(join(release, "query-render-performance-phase012.json"), `${JSON.stringify(report, null, 2)}\n`);
    await writeFile(join(release, "query-render-performance-phase012.md"), renderMarkdown(report));
    state = transitionQueryRenderPerformanceState(state, "Pass").state;
    return { ...report, state };
  } finally {
    await rm(temporary, { force: true });
  }
}

function renderMarkdown(report) {
  const lines = [
    "# Phase 012 Desktop Query Render Performance",
    "",
    report.marker,
    `source_fingerprint=${report.sourceFingerprint}`,
    `native_source_fingerprint=${report.nativeSourceFingerprint}`,
    `fixture_hash=${report.fixtureHash}`,
    `timing_boundary=${report.timingBoundary}`,
    `combined_method=${report.combinedMethod}`,
    `packaged_end_to_end_measured=${report.packagedEndToEndMeasured}`,
    "",
    "| Query | Fixture | Bounded | Native p95 ms | Desktop render p95 ms | IPC allowance ms | Combined p95 ms |",
    "| --- | ---: | ---: | ---: | ---: | ---: | ---: |",
  ];
  for (const query of report.queries) {
    lines.push(`| ${query.queryId} | ${query.standardFixtureCount} | ${query.boundedResultCount} | ${query.nativeP95Ms.toFixed(3)} | ${query.desktopRenderP95Ms.toFixed(3)} | ${query.ipcAllowanceMs.toFixed(3)} | ${query.combinedP95Ms.toFixed(3)} |`);
  }
  lines.push(
    "",
    "This gate combines current native p95 with a production-bundled desktop controller-to-rendered-markup p95 and an explicit IPC allowance. It does not claim a packaged WebView end-to-end measurement; that remains a Phase 012 final release requirement.",
    "",
  );
  return lines.join("\n");
}

async function fingerprint(root, files) {
  const hash = createHash("sha256");
  for (const path of [...files].sort()) hash.update(path).update("\0").update(await readFile(join(root, path))).update("\0");
  return hash.digest("hex");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const report = await runPhase012QueryRenderPerformance();
  console.log(report.marker);
  console.log(`source_fingerprint=${report.sourceFingerprint}`);
  for (const query of report.queries) console.log(`${query.queryId}_combined_p95_ms=${query.combinedP95Ms.toFixed(3)}`);
}
