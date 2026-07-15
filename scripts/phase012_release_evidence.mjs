import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { lstat, mkdir, readdir, readFile, rename, writeFile } from "node:fs/promises";
import { dirname, extname, join, relative, resolve } from "node:path";
import { pathToFileURL } from "node:url";

import {
  analyzePhase012ReleaseEvidence,
  phase012RequirementIds,
  renderPhase012PlatformMatrix,
  renderPhase012ReleaseResult,
  renderPhase012RequirementMatrix,
} from "./phase012_release_gate.mjs";

export const Phase012EvidenceState = Object.freeze({
  NotStarted: "NotStarted",
  Fingerprinted: "Fingerprinted",
  CommandsRunning: "CommandsRunning",
  CommandsPassed: "CommandsPassed",
  GateValidated: "GateValidated",
  ArtifactsWritten: "ArtifactsWritten",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase012EvidenceErrorCode = Object.freeze({
  FingerprintFailed: "PHASE012_EVIDENCE_FINGERPRINT_FAILED",
  CommandFailed: "PHASE012_EVIDENCE_COMMAND_FAILED",
  RequirementCoverageMissing: "PHASE012_EVIDENCE_REQUIREMENT_COVERAGE_MISSING",
  SourceChanged: "PHASE012_EVIDENCE_SOURCE_CHANGED",
  GateFailed: "PHASE012_EVIDENCE_GATE_FAILED",
  ArtifactWriteFailed: "PHASE012_EVIDENCE_ARTIFACT_WRITE_FAILED",
});

const sourceRoots = Object.freeze([
  "AGENTS.md", "PROJECT.md", "Cargo.toml", "Cargo.lock", "package.json",
  "apps/desktop/package.json", "apps/desktop/src", "apps/desktop/src-tauri/Cargo.toml",
  "apps/desktop/src-tauri/src", "apps/desktop/src-tauri/tests", "crates", "packages", "scripts",
]);
const sourceExtensions = new Set([".rs", ".ts", ".tsx", ".js", ".mjs", ".cjs", ".sh", ".json", ".toml", ".md", ".css"]);
const ignoredDirectoryNames = new Set([".git", ".tasks", "node_modules", "target", "dist", "coverage"]);

export const phase012ReleaseCommandPlan = Object.freeze([
  command("archive-contract", "sh", ["scripts/run_phase012_archive_validator_tests.sh"]),
  command("archive-current", "sh", ["scripts/run_phase012_archive_validator.sh"]),
  command("plan-contract", "sh", ["scripts/run_phase012_plan_validator_tests.sh"]),
  command("plan-current", "sh", ["scripts/run_phase012_plan_validator.sh"]),
  command("release-contract", "sh", ["scripts/run_phase012_release_gate_tests.sh"]),
  command("rust-workspace", "cargo", ["test", "--workspace"]),
  command("desktop-local-typescript", "sh", ["scripts/run_phase012_local_desktop_tests.sh"]),
  command("query-performance-contract", "sh", ["scripts/run_phase012_query_performance_tests.sh"]),
  command("query-performance-current", "sh", ["scripts/run_phase012_query_performance.sh"]),
  command("query-render-contract", "sh", ["scripts/run_phase012_query_render_performance_tests.sh"]),
  command("query-render-current", "sh", ["scripts/run_phase012_query_render_performance.sh"]),
  command("visual-contract", "sh", ["scripts/run_phase012_exploration_visual_tests.sh"]),
  command("visual-current", "sh", ["scripts/run_phase012_exploration_visual.sh"]),
  command("macos-packaged-smoke", "sh", ["scripts/run_desktop_packaged_app_smoke.sh"]),
  command("packaged-ui-contract", "sh", ["scripts/run_desktop_packaged_ui_smoke_tests.sh"]),
  command("macos-packaged-ui-smoke", "sh", ["scripts/run_desktop_packaged_ui_smoke.sh", "--reuse-existing"]),
  command("security-contract", "sh", ["scripts/run_phase012_security_manifest_tests.sh"]),
  command("security-manifest", "node", ["scripts/phase012_security_manifest.mjs"]),
  command("security-current", "node", ["scripts/security_log_scanner.mjs", ".tasks/release/security-log-policy-manifest-phase012.json"]),
]);

const requirementCommand = Object.freeze({
  "SCOPE-012-01": "archive-current", "BASE-012-01": "archive-current",
  "SAVE-012-01": "desktop-local-typescript", "GRAPH-012-01": "rust-workspace",
  "GRAPH-012-02": "desktop-local-typescript", "CANVAS-012-01": "rust-workspace",
  "CANVAS-012-02": "rust-workspace", "CANVAS-012-03": "rust-workspace",
  "ASSET-012-01": "rust-workspace", "ASSET-012-02": "rust-workspace",
  "ASSET-012-03": "desktop-local-typescript", "ASSET-012-04": "rust-workspace",
  "PROJ-012-01": "rust-workspace", "PROJ-012-02": "rust-workspace",
  "UX-012-01": "visual-current", "UX-012-02": "visual-current",
  "UI-CONN-012-01": "desktop-local-typescript", "UI-CONN-012-02": "desktop-local-typescript",
  "UI-CONN-012-03": "desktop-local-typescript", "UI-CONN-012-04": "desktop-local-typescript",
  "PERF-012-01": "macos-packaged-ui-smoke", "CFG-012-01": "rust-workspace",
  "LOG-012-01": "security-current", "RECOVERY-012-01": "rust-workspace",
  "BACKUP-012-01": "rust-workspace", "PLAT-012-01": "macos-packaged-ui-smoke",
  "DATA-012-01": "rust-workspace", "DATA-012-02": "rust-workspace",
  "ROUTE-012-01": "desktop-local-typescript", "EVID-012-01": "release-contract",
  "OPS-012-01": "rust-workspace", "ERR-012-01": "rust-workspace", "SEC-012-01": "rust-workspace",
});

export async function collectPhase012SourceFingerprint(root) {
  const base = resolve(root);
  const files = [];
  for (const sourceRoot of sourceRoots) {
    const path = join(base, sourceRoot);
    try {
      await collectFiles(base, path, files);
    } catch (error) {
      if (error?.code !== "ENOENT") throw error;
    }
  }
  const uniqueFiles = [...new Set(files)].sort();
  const hash = createHash("sha256");
  for (const path of uniqueFiles) {
    const relativePath = relative(base, path);
    hash.update(relativePath).update("\0").update(await readFile(path)).update("\0");
  }
  return { sourceFingerprint: hash.digest("hex"), sourceFileCount: uniqueFiles.length };
}

export async function runPhase012ReleaseEvidence({
  root = process.cwd(),
  commandPlan = phase012ReleaseCommandPlan,
  executeCommand = executeReleaseCommand,
} = {}) {
  let fingerprint;
  try {
    fingerprint = await collectPhase012SourceFingerprint(root);
  } catch {
    return failed(Phase012EvidenceErrorCode.FingerprintFailed, "source-fingerprint");
  }

  const planIds = new Set(commandPlan.map((item) => item.id));
  const uncovered = phase012RequirementIds.find((id) => !planIds.has(requirementCommand[id]));
  if (uncovered) return failed(Phase012EvidenceErrorCode.RequirementCoverageMissing, uncovered);

  const commandResults = [];
  for (const entry of commandPlan) {
    const result = await executeCommand(entry, root);
    commandResults.push({ id: entry.id, passed: result.passed === true, durationMs: result.durationMs ?? 0 });
    if (result.passed !== true) {
      return failed(Phase012EvidenceErrorCode.CommandFailed, entry.id, { commandResults });
    }
  }

  const finalFingerprint = await collectPhase012SourceFingerprint(root);
  if (finalFingerprint.sourceFingerprint !== fingerprint.sourceFingerprint) {
    return failed(Phase012EvidenceErrorCode.SourceChanged, "source-fingerprint", { commandResults });
  }
  const requirementEvidence = phase012RequirementIds.map((requirementId) => ({
    requirementId,
    status: "passed",
    sourceFingerprint: fingerprint.sourceFingerprint,
    artifactId: `phase012-${requirementCommand[requirementId]}`,
    commandId: requirementCommand[requirementId],
  }));
  const gate = analyzePhase012ReleaseEvidence({
    expectedSourceFingerprint: fingerprint.sourceFingerprint,
    requirementEvidence,
    platformEvidence: {
      macos: {
        status: "passed",
        sourceFingerprint: fingerprint.sourceFingerprint,
        evidenceId: "phase012-macos-packaged-ui-smoke",
      },
      windows: { status: "deferred_future" },
      linux: { status: "deferred_future" },
    },
    artifactTexts: commandResults.map((result) =>
      `command_id=${result.id} status=passed duration_bucket=${durationBucket(result.durationMs)}`
    ),
  });
  if (!gate.passed) return failed(Phase012EvidenceErrorCode.GateFailed, gate.findingId, { commandResults });

  const artifacts = new Map([
    [".tasks/release/requirement-evidence-matrix-phase012.md", renderPhase012RequirementMatrix(gate)],
    [".tasks/release/native-platform-matrix-phase012.md", renderPhase012PlatformMatrix(gate)],
    [".tasks/release/command-summary-phase012.md", renderCommandSummary(gate, commandResults, fingerprint.sourceFileCount)],
    [".tasks/phase012-release-gate-result.md", renderPhase012ReleaseResult(gate)],
  ]);
  try {
    for (const [path, text] of artifacts) await writeAtomically(join(root, path), text);
  } catch {
    return failed(Phase012EvidenceErrorCode.ArtifactWriteFailed, "release-artifacts", { commandResults });
  }
  return {
    ...gate,
    commandCount: commandResults.length,
    commandResults,
    sourceFileCount: fingerprint.sourceFileCount,
  };
}

export async function executeReleaseCommand(entry, root) {
  const started = Date.now();
  const passed = await new Promise((resolve) => {
    const child = spawn(entry.executable, entry.args, { cwd: root, stdio: "inherit" });
    child.once("error", () => resolve(false));
    child.once("exit", (code) => resolve(code === 0));
  });
  return { passed, durationMs: Date.now() - started };
}

function command(id, executable, args) {
  return Object.freeze({ id, executable, args: Object.freeze(args) });
}

async function collectFiles(base, path, files) {
  const stat = await lstat(path);
  if (stat.isSymbolicLink()) return;
  if (stat.isFile()) {
    if (sourceExtensions.has(extname(path)) || sourceRoots.includes(relative(base, path))) files.push(path);
    return;
  }
  if (!stat.isDirectory() || ignoredDirectoryNames.has(path.split("/").at(-1))) return;
  const entries = await readdir(path);
  for (const entry of entries.sort()) await collectFiles(base, join(path, entry), files);
}

function renderCommandSummary(gate, commandResults, sourceFileCount) {
  return [
    "# Phase 012 Command Summary",
    "",
    "phase012_command_summary=passed",
    `source_fingerprint=${gate.sourceFingerprint}`,
    `source_file_count=${sourceFileCount}`,
    `command_count=${commandResults.length}`,
    "",
    "| Command | Status | Duration bucket |",
    "| --- | --- | --- |",
    ...commandResults.map((result) => `| \`${result.id}\` | \`passed\` | \`${durationBucket(result.durationMs)}\` |`),
    "",
    "Command output is intentionally excluded from this artifact.",
    "",
  ].join("\n");
}

function durationBucket(durationMs) {
  if (durationMs < 100) return "lt_100ms";
  if (durationMs < 1_000) return "lt_1s";
  if (durationMs < 10_000) return "lt_10s";
  if (durationMs < 60_000) return "lt_60s";
  return "gte_60s";
}

async function writeAtomically(path, text) {
  await mkdir(dirname(path), { recursive: true });
  const temporary = `${path}.tmp`;
  await writeFile(temporary, text);
  await rename(temporary, path);
}

function failed(errorCode, findingId, detail = {}) {
  return {
    passed: false,
    state: Phase012EvidenceState.Failed,
    errorCode,
    findingId,
    commandResults: detail.commandResults ?? [],
  };
}

async function main() {
  const result = await runPhase012ReleaseEvidence({ root: process.argv[2] ?? process.cwd() });
  if (!result.passed) {
    process.stderr.write(`phase012_release_evidence=failed error_code=${result.errorCode} finding=${result.findingId}\n`);
    process.exitCode = 1;
    return;
  }
  process.stdout.write(`phase012_release_evidence=passed requirements=${result.requirementCount} commands=${result.commandCount}\n`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) await main();
