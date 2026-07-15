import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";

import { runPhase012QueryPerformance } from "./run_phase012_query_performance.mjs";
import { runPhase012QueryRenderPerformance } from "./run_phase012_query_render_performance.mjs";
import { PHASE013_PERFORMANCE_FIXTURE, transitionPhase013Performance, validatePhase013QueryRenderPerformance } from "./phase013_query_render_performance.mjs";

export async function runPhase013QueryRenderPerformance(root = process.cwd()) {
  let state = transitionPhase013Performance("Pending", "Build");
  state = transitionPhase013Performance(state, "Native");
  const native = await runPhase012QueryPerformance(root);
  state = transitionPhase013Performance(state, "Render");
  const render = await runPhase012QueryRenderPerformance(root);
  const sourceFingerprint = await aggregateFingerprint(root, native.sourceFingerprint, render.sourceFingerprint);
  const queries = render.queries.map((query) => Object.freeze({
    ...query,
    queryPath: native.queries.find((candidate) => candidate.id === query.queryId)?.queryPath,
  }));
  state = transitionPhase013Performance(state, "Validate");
  const report = {
    marker: "phase013_query_render_performance=passed",
    state: "Passed",
    sourceFingerprint,
    nativeSourceFingerprint: native.sourceFingerprint,
    renderSourceFingerprint: render.sourceFingerprint,
    fixtureHash: native.fixtureHash,
    policy: Object.freeze({ budgetMs: 300, warmupCount: native.warmupCount, sampleCount: native.sampleCount, percentile: "nearest_rank", ipcAllowanceMs: render.policy.ipcAllowanceMs }),
    fixture: PHASE013_PERFORMANCE_FIXTURE,
    queries,
    boundaries: Object.freeze(["native_indexed_query", "desktop_controller_to_rendered_marker", "explicit_ipc_allowance"]),
    diagnostics: "sanitized",
  };
  const validation = validatePhase013QueryRenderPerformance(report, sourceFingerprint);
  if (!validation.passed) throw new Error(`Phase 013 performance validation failed: ${validation.findingIds.join(",")}`);
  state = transitionPhase013Performance(state, "Write");
  const releaseDir = join(root, ".tasks", "release");
  await mkdir(releaseDir, { recursive: true });
  await writeFile(join(releaseDir, "query-render-performance-phase013.json"), `${JSON.stringify(report, null, 2)}\n`);
  await writeFile(join(releaseDir, "query-render-performance-phase013.md"), renderMarkdown(report));
  state = transitionPhase013Performance(state, "Pass");
  return Object.freeze({ ...report, state });
}

function renderMarkdown(report) {
  const lines = [
    "# Phase 013 Query and Render Performance",
    "",
    report.marker,
    `source_fingerprint=${report.sourceFingerprint}`,
    `fixture_hash=${report.fixtureHash}`,
    "",
    "| Query | Path | Bounded | Native p95 ms | Render p95 ms | Combined p95 ms | Budget ms |",
    "| --- | --- | ---: | ---: | ---: | ---: | ---: |",
  ];
  for (const query of report.queries) lines.push(`| ${query.queryId} | ${query.queryPath} | ${query.boundedResultCount} | ${query.nativeP95Ms.toFixed(3)} | ${query.desktopRenderP95Ms.toFixed(3)} | ${query.combinedP95Ms.toFixed(3)} | 300 |`);
  lines.push("", "The report contains only stable query identifiers, bounded counts, timings, and hashes. It excludes user content, filenames, paths, asset bytes, and credentials.", "");
  return lines.join("\n");
}

async function aggregateFingerprint(root, nativeFingerprint, renderFingerprint) {
  const hash = createHash("sha256").update(nativeFingerprint).update(renderFingerprint);
  for (const file of ["scripts/phase013_query_render_performance.mjs", "scripts/run_phase013_query_render_performance.mjs"]) hash.update(file).update(await readFile(join(root, file)));
  return hash.digest("hex");
}

const report = await runPhase013QueryRenderPerformance();
console.log(report.marker);
console.log(`source_fingerprint=${report.sourceFingerprint}`);
for (const query of report.queries) console.log(`${query.queryId}_combined_p95_ms=${query.combinedP95Ms.toFixed(3)}`);
