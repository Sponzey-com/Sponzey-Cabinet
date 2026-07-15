import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  DEFAULT_UI_AUDIT_POLICY,
  collectBaseline,
  renderBaselineArtifact,
  validateBaselineReport,
} from "./phase013_ui_baseline.mjs";

export const PHASE013_ROUTE_DESCRIPTORS = Object.freeze([
  descriptor("Home", "apps/desktop/src/react_workspace_home.ts", "cabinet-home-shell"),
  descriptor("Search", "apps/desktop/src/react_document_navigator.ts", "navigator-shell"),
  descriptor("Document", "apps/desktop/src/react_document_authoring_workbench.ts", "authoring-shell"),
  descriptor("Graph", "apps/desktop/src/react_exploration_surfaces.ts", "surfaceShell(", "function DesktopKnowledgeGraph(", "function DesktopCanvas("),
  descriptor("Canvas", "apps/desktop/src/react_exploration_surfaces.ts", "surfaceShell(", "function DesktopCanvas(", "function DesktopAttachments("),
  descriptor("Assets", "apps/desktop/src/react_exploration_surfaces.ts", "surfaceShell(", "function DesktopAttachments("),
  descriptor("Backup", "apps/desktop/src/react_backup_recovery.ts", "backup-recovery-surface"),
]);

export async function runPhase013UiBaseline({ rootDir, outputPath } = {}) {
  const root = rootDir ?? resolve(dirname(fileURLToPath(import.meta.url)), "..");
  const target = outputPath ?? join(root, ".tasks/release/ui-baseline-phase013.md");
  const report = await collectBaseline({
    rootDir: root,
    descriptors: PHASE013_ROUTE_DESCRIPTORS,
    readText: (path) => readFile(path, "utf8"),
    policy: DEFAULT_UI_AUDIT_POLICY,
  });
  const validation = validateBaselineReport(report, {
    sourceFingerprint: report.sourceFingerprint,
    fixtureHash: report.fixtureHash,
  });
  if (validation.length > 0) throw new Error(`PHASE013_UI_BASELINE_INVALID:${validation.join(",")}`);

  await mkdir(dirname(target), { recursive: true });
  await writeFile(target, renderBaselineArtifact(report), "utf8");
  return Object.freeze({ report, outputPath: target });
}

function descriptor(route, sourceFile, shellMarker, auditStartMarker, auditEndMarker) {
  return Object.freeze({ route, sourceFile, shellMarker, sidebarMarker: "desktop-sidebar", topbarMarker: "desktop-topbar", auditStartMarker, auditEndMarker });
}

async function main() {
  const { report } = await runPhase013UiBaseline();
  console.log("phase013_ui_baseline=recorded");
  console.log(`source_fingerprint=${report.sourceFingerprint}`);
  console.log(`fixture_hash=${report.fixtureHash}`);
  console.log(`route_count=${report.routeCount}`);
  console.log(`shell_owner_count=${report.shellOwnerCount}`);
  console.log(`finding_count=${report.findingCount}`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main();
}
