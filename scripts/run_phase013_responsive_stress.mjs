import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";

import { validateResponsiveStressReport } from "./phase013_responsive_stress.mjs";
import {
  resolveChromePath,
  runPhase013ActionGeometryBaseline,
} from "./run_phase013_action_geometry_baseline.mjs";

const routes = Object.freeze(["Home", "Search", "Document", "Graph", "Canvas", "Assets", "Backup"]);
const viewports = Object.freeze([
  { width: 1440, height: 900 },
]);
const captureProfile = Object.freeze({
  marker: "phase013_responsive_stress=recorded",
  fixtureVersion: "phase013-responsive-stress-v1",
  textZoomPercent: 200,
  fixtureItemCount: 100,
  longContentFixture: true,
});

async function main() {
  const root = process.cwd();
  const report = await runPhase013ActionGeometryBaseline({
    root,
    chromePath: resolveChromePath(),
    captureProfile,
    captureViewports: viewports,
  });
  const validation = validateResponsiveStressReport(report, {
    fingerprint: report.sourceFingerprint,
    routes,
    viewports,
  });
  if (!validation.passed) {
    const clipped = report.actions.filter((action) => action.horizontallyClipped);
    const failedRuns = report.runs.filter((run) => run.horizontalOverflow || run.clippedActionCount > 0);
    throw new Error(`responsive stress validation failed: ${validation.findingIds.join(",")}; runs=${JSON.stringify(failedRuns)}; controls=${JSON.stringify(clipped)}`);
  }
  const releaseDir = join(root, ".tasks", "release");
  await mkdir(releaseDir, { recursive: true });
  await writeFile(join(releaseDir, "responsive-stress-phase013.json"), `${JSON.stringify(report, null, 2)}\n`);
  console.log("phase013_responsive_stress=recorded");
  console.log(`source_fingerprint=${report.sourceFingerprint}`);
  console.log(`route_run_count=${report.runs.length}`);
  console.log(`fixture_item_count=${report.fixtureItemCount}`);
  console.log(`text_zoom_percent=${report.textZoomPercent}`);
}

await main();
