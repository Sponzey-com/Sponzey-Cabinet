import { spawn } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

const root = process.cwd();
const manifestPath = join(root, ".tasks", "release", "phase002-release-manifest.json");
const defaultRequiredCompletedTaskCount = 32;
const resultDirectory = join(root, ".tasks", "release", "results");

export const ReleaseGateState = Object.freeze({
  Pending: "Pending",
  Running: "Running",
  StepPassed: "StepPassed",
  StepFailed: "StepFailed",
  Sanitizing: "Sanitizing",
  ArtifactWritten: "ArtifactWritten",
  Failed: "Failed",
});

export const ReleaseGateEvent = Object.freeze({
  Start: "Start",
  StepStart: "StepStart",
  StepExit: "StepExit",
  Sanitize: "Sanitize",
  WriteArtifact: "WriteArtifact",
  Fail: "Fail",
});

export function transitionReleaseGateState(currentState, event, detail = {}) {
  if (currentState === ReleaseGateState.Pending && event === ReleaseGateEvent.Start) {
    return { state: ReleaseGateState.Running };
  }
  if (
    [ReleaseGateState.Running, ReleaseGateState.StepPassed].includes(currentState) &&
    event === ReleaseGateEvent.StepStart
  ) {
    return { state: ReleaseGateState.Running, currentStepId: detail.stepId };
  }
  if (currentState === ReleaseGateState.Running && event === ReleaseGateEvent.StepExit) {
    if (detail.status === "passed") {
      return { state: ReleaseGateState.StepPassed, currentStepId: detail.stepId };
    }
    return {
      state: ReleaseGateState.StepFailed,
      failedStepId: detail.stepId,
      failureCategory: detail.failureCategory ?? "command_failed",
    };
  }
  if (
    [
      ReleaseGateState.StepPassed,
      ReleaseGateState.StepFailed,
      ReleaseGateState.Running,
      ReleaseGateState.Failed,
    ].includes(currentState) &&
    event === ReleaseGateEvent.Sanitize
  ) {
    return { state: ReleaseGateState.Sanitizing };
  }
  if (currentState === ReleaseGateState.Sanitizing && event === ReleaseGateEvent.WriteArtifact) {
    return { state: ReleaseGateState.ArtifactWritten };
  }
  if (event === ReleaseGateEvent.Fail) {
    return {
      state: ReleaseGateState.Failed,
      failedStepId: detail.stepId,
      failureCategory: detail.failureCategory ?? "release_gate_failed",
    };
  }
  return {
    state: ReleaseGateState.Failed,
    failureCategory: "invalid_release_gate_state_transition",
  };
}

async function main() {
  const startedAt = new Date();
  let result;
  let artifactPath = "";
  try {
    result = await runReleaseGate({
      root,
      manifestPath,
      startedAt,
      commandRunner: spawnCommand,
      gitStatusProvider: readGitStatusSummary,
    });
    artifactPath = await writeReleaseGateResultArtifact({
      root,
      result,
      securityManifestPath: join(root, ".tasks", "release", "security-log-policy-manifest.json"),
    });
  } catch (error) {
    result = createUnexpectedFailureResult({
      root,
      manifestPath,
      startedAt,
      error,
    });
    artifactPath = await writeReleaseGateResultArtifact({
      root,
      result,
      securityManifestPath: join(root, ".tasks", "release", "security-log-policy-manifest.json"),
    });
  }

  if (result.status === "passed") {
    console.log("phase002_release_gate=passed");
    console.log(`release_manifest=${relative(manifestPath, root)}`);
    console.log(`release_result_artifact=${relative(artifactPath, root)}`);
    return;
  }

  console.error("phase002_release_gate=failed");
  console.error(`failure_category=${result.failureCategory}`);
  if (result.failedStepId) {
    console.error(`failed_step_id=${result.failedStepId}`);
  }
  console.error(`release_result_artifact=${relative(artifactPath, root)}`);
  process.exit(1);
}

export async function runReleaseGate({
  root,
  manifestPath,
  startedAt = new Date(),
  commandRunner,
  gitStatusProvider = readGitStatusSummary,
}) {
  let state = transitionReleaseGateState(ReleaseGateState.Pending, ReleaseGateEvent.Start);
  const manifest = await readManifest(manifestPath);
  const commandResults = [];
  let failureCategory = "none";
  let failedStepId;

  try {
    validatePrerequisites(manifest, root);
    validateReportDocuments(manifest, root);
  } catch (error) {
    state = transitionReleaseGateState(state.state, ReleaseGateEvent.Fail, {
      stepId: "prerequisites",
      failureCategory: "prerequisite_validation_failed",
    });
    return createReleaseGateResult({
      root,
      manifest,
      manifestPath,
      startedAt,
      completedAt: new Date(),
      commandResults,
      status: "failed",
      state,
      failureCategory: "prerequisite_validation_failed",
      failedStepId: "prerequisites",
      failureMessage: stableErrorMessage(error),
      gitStatusSummary: await gitStatusProvider(root),
    });
  }

  for (const step of manifest.commands) {
    validateReleaseStep(step);
    state = transitionReleaseGateState(state.state, ReleaseGateEvent.StepStart, { stepId: step.id });
    console.log(`phase002_release_step_start=${step.id}`);
    const started = Date.now();
    const execution = await commandRunner(step.command[0], step.command.slice(1), root);
    const durationMs = Date.now() - started;
    const status = execution.exitCode === 0 && !execution.signal ? "passed" : "failed";
    const commandResult = {
      id: step.id,
      command: step.command,
      status,
      exitCode: execution.exitCode,
      signal: execution.signal,
      durationMs,
    };
    commandResults.push(commandResult);
    state = transitionReleaseGateState(state.state, ReleaseGateEvent.StepExit, {
      stepId: step.id,
      status,
      failureCategory: execution.signal ? "command_signal" : "command_exit_nonzero",
    });

    if (status === "passed") {
      console.log(`phase002_release_step_passed=${step.id}`);
      continue;
    }

    failureCategory = execution.signal ? "command_signal" : "command_exit_nonzero";
    failedStepId = step.id;
    console.error(`phase002_release_step_failed=${step.id}`);
    break;
  }

  const status = failedStepId ? "failed" : "passed";
  return createReleaseGateResult({
    root,
    manifest,
    manifestPath,
    startedAt,
    completedAt: new Date(),
    commandResults,
    status,
    state,
    failureCategory,
    failedStepId,
    failureMessage: failedStepId ? "A release gate command failed." : "none",
    gitStatusSummary: await gitStatusProvider(root),
  });
}

function createReleaseGateResult({
  root,
  manifest,
  manifestPath,
  startedAt,
  completedAt,
  commandResults,
  status,
  state,
  failureCategory,
  failedStepId,
  failureMessage,
  gitStatusSummary,
}) {
  const performanceReportPath = join(root, ".tasks", "release", "phase002-performance-report.md");
  const performanceReport = existsSync(performanceReportPath)
    ? readFileSync(performanceReportPath, "utf8")
    : "";
  const securityStep = commandResults.find((step) => step.id === "security_release_artifact_scan");

  return {
    phase: manifest.phase ?? "Phase 002",
    gate: manifest.gate ?? "Release Boundary",
    status,
    state: state.state,
    startedAt: startedAt.toISOString(),
    completedAt: completedAt.toISOString(),
    manifestPath: relative(manifestPath, root),
    failureCategory,
    failedStepId,
    failureMessage,
    commandResults,
    performanceSummary: summarizePerformanceReport(performanceReport),
    sensitiveScanSummary: summarizeSensitiveScan(securityStep),
    gitStatusSummary,
  };
}

function createUnexpectedFailureResult({ root, manifestPath, startedAt, error }) {
  return {
    phase: "Phase 002",
    gate: "Release Boundary",
    status: "failed",
    state: ReleaseGateState.Failed,
    startedAt: startedAt.toISOString(),
    completedAt: new Date().toISOString(),
    manifestPath: relative(manifestPath, root),
    failureCategory: "unexpected_release_gate_failure",
    failedStepId: "runner",
    failureMessage: stableErrorMessage(error),
    commandResults: [],
    performanceSummary: "unavailable",
    sensitiveScanSummary: "not_run",
    gitStatusSummary: { modified: 0, added: 0, deleted: 0, renamed: 0, untracked: 0, other: 0 },
  };
}

export function renderReleaseGateResult(result) {
  const lines = [
    `# ${result.phase} Release Gate Result`,
    "",
    "## Summary",
    "",
    `- Status: ${result.status}`,
    `- Runner State: ${result.state}`,
    `- Started At: ${result.startedAt}`,
    `- Completed At: ${result.completedAt}`,
    `- Manifest: ${result.manifestPath}`,
    `- Failure Category: ${result.failureCategory}`,
    `- Failed Step Id: ${result.failedStepId ?? "none"}`,
    `- Failure Message: ${result.failureMessage}`,
    "",
    "## Evidence Summary",
    "",
    `- p95 Summary: ${result.performanceSummary}`,
    `- Sensitive Scan: ${result.sensitiveScanSummary}`,
    `- Git Status: ${formatGitStatusSummary(result.gitStatusSummary)}`,
    "",
    "## Command Results",
    "",
    "| Command Id | Command | Status | Exit Code | Duration Ms |",
    "| --- | --- | --- | --- | --- |",
    ...result.commandResults.map((step) => {
      const exitCode = step.signal ? `signal:${step.signal}` : String(step.exitCode);
      return `| ${step.id} | \`${step.command.join(" ")}\` | ${step.status} | ${exitCode} | ${step.durationMs} |`;
    }),
    "",
    "## Artifact Policy",
    "",
    "- This result artifact is local/test release evidence, not Product Log.",
    "- Command stdout and stderr are not embedded in this artifact.",
    "- Sensitive fixture scanning runs before and after artifact write.",
  ];

  return `${lines.join("\n")}\n`;
}

export async function writeReleaseGateResultArtifact({ root, result, securityManifestPath }) {
  const deniedFixtures = loadDeniedFixtures(securityManifestPath);
  const sanitizing = transitionReleaseGateState(result.state, ReleaseGateEvent.Sanitize);
  const artifactWritten = transitionReleaseGateState(sanitizing.state, ReleaseGateEvent.WriteArtifact);
  if (artifactWritten.state !== ReleaseGateState.ArtifactWritten) {
    throw new Error("release result artifact state transition failed");
  }
  const outputPath = createReleaseGateResultPath(root, new Date(result.completedAt));
  const artifactResult = {
    ...result,
    state: artifactWritten.state,
  };
  let content = renderReleaseGateResult(artifactResult);

  try {
    sanitizeReleaseGateArtifact(content, deniedFixtures);
  } catch {
    content = renderReleaseGateResult({
      ...artifactResult,
      status: "failed",
      failureCategory: "release_result_sanitizer_failed",
      failedStepId: "artifact_sanitizer",
      failureMessage: "Release result sanitizer blocked unsafe artifact content.",
      commandResults: result.commandResults.map((step) => ({
        id: step.id,
        command: step.command,
        status: step.status,
        exitCode: step.exitCode,
        signal: step.signal,
        durationMs: step.durationMs,
      })),
    });
    sanitizeReleaseGateArtifact(content, deniedFixtures);
  }

  await mkdir(join(root, ".tasks", "release", "results"), { recursive: true });
  const latestPath = join(root, ".tasks", "release", "results", "latest-phase002-release-gate.md");
  await writeFile(outputPath, content);
  await writeFile(latestPath, content);
  const written = await readFile(outputPath, "utf8");
  const latest = await readFile(latestPath, "utf8");
  sanitizeReleaseGateArtifact(written, deniedFixtures);
  sanitizeReleaseGateArtifact(latest, deniedFixtures);
  return outputPath;
}

export function createReleaseGateResultPath(root, date) {
  return join(
    root,
    ".tasks",
    "release",
    "results",
    `phase002-release-gate-${formatTimestamp(date)}.md`,
  );
}

export function sanitizeReleaseGateArtifact(content, deniedFixtures) {
  const findings = [];
  for (const fixture of deniedFixtures) {
    if (typeof fixture.value === "string" && fixture.value.length > 0 && content.includes(fixture.value)) {
      findings.push(fixture.id);
    }
  }
  if (findings.length > 0) {
    throw new ReleaseGateArtifactSanitizerError(findings);
  }
}

export function summarizePerformanceReport(body) {
  if (!body.trim()) {
    return "unavailable";
  }
  const containsP95 = /p95 300ms/i.test(body);
  const paths = [
    "permission-aware current document lookup",
    "permission-aware document search",
    "document comments list",
    "audit event list",
    "asset metadata list",
  ].filter((token) => body.includes(token));
  return containsP95
    ? `p95 300ms target documented; paths=${paths.join(", ")}`
    : `p95 summary missing; paths=${paths.join(", ")}`;
}

export function summarizeSensitiveScan(securityStep) {
  if (!securityStep) {
    return "not_run";
  }
  return securityStep.status === "passed"
    ? "passed"
    : `failed; step=${securityStep.id}; exit_code=${securityStep.exitCode}`;
}

export function summarizeGitStatusText(statusText) {
  const summary = { modified: 0, added: 0, deleted: 0, renamed: 0, untracked: 0, other: 0 };
  for (const line of statusText.split(/\r?\n/)) {
    if (!line.trim()) {
      continue;
    }
    const code = line.slice(0, 2);
    if (code === "??") {
      summary.untracked += 1;
    } else if (code.includes("M")) {
      summary.modified += 1;
    } else if (code.includes("A")) {
      summary.added += 1;
    } else if (code.includes("D")) {
      summary.deleted += 1;
    } else if (code.includes("R")) {
      summary.renamed += 1;
    } else {
      summary.other += 1;
    }
  }
  return summary;
}

async function readManifest(manifestPath) {
  if (!existsSync(manifestPath)) {
    throw new Error(`Missing release manifest: ${manifestPath}`);
  }
  const raw = await readFile(manifestPath, "utf8");
  const parsed = JSON.parse(raw);
  if (!Array.isArray(parsed.commands) || parsed.commands.length === 0) {
    throw new Error("Release manifest must contain at least one command.");
  }
  if (!Array.isArray(parsed.requiredReports) || parsed.requiredReports.length === 0) {
    throw new Error("Release manifest must contain requiredReports.");
  }
  return parsed;
}

function validatePrerequisites(manifest, root) {
  const requiredCompletedTaskCount =
    Number.isInteger(manifest.requiredCompletedTaskCount) && manifest.requiredCompletedTaskCount >= 0
      ? manifest.requiredCompletedTaskCount
      : defaultRequiredCompletedTaskCount;
  for (let index = 1; index <= requiredCompletedTaskCount; index += 1) {
    const taskPath = join(root, ".tasks", `task${String(index).padStart(3, "0")}.md`);
    assertFile(taskPath, root);
    const taskBody = readTextSync(taskPath);
    if (taskBody.includes("- [ ]")) {
      throw new Error(`Previous task still has unchecked checklist items: ${relative(taskPath, root)}`);
    }
  }

  for (const file of manifest.requiredFiles ?? []) {
    assertFile(join(root, file), root);
  }
}

function validateReportDocuments(manifest, root) {
  for (const report of manifest.requiredReports) {
    const reportPath = join(root, report.path);
    assertFile(reportPath, root);
    const body = readTextSync(reportPath);
    for (const token of report.requiredText ?? []) {
      if (!body.includes(token)) {
        throw new Error(`${report.path} is missing required text: ${token}`);
      }
    }
  }
}

function validateReleaseStep(step) {
  if (!step.id || !Array.isArray(step.command) || step.command.length === 0) {
    throw new Error("Release command step must include id and command array.");
  }
}

function spawnCommand(command, args, root) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: root,
      stdio: "inherit",
    });
    child.on("error", reject);
    child.on("exit", (code, signal) => {
      resolve({
        exitCode: code ?? 1,
        signal,
      });
    });
  });
}

async function readGitStatusSummary(root) {
  const result = await captureCommand("git", ["status", "--short"], root);
  if (result.exitCode !== 0) {
    return { modified: 0, added: 0, deleted: 0, renamed: 0, untracked: 0, other: 1 };
  }
  return summarizeGitStatusText(result.output);
}

function captureCommand(command, args, root) {
  return new Promise((resolve) => {
    const chunks = [];
    const child = spawn(command, args, {
      cwd: root,
      stdio: ["ignore", "pipe", "pipe"],
    });
    child.stdout.on("data", (chunk) => chunks.push(chunk.toString("utf8")));
    child.stderr.on("data", (chunk) => chunks.push(chunk.toString("utf8")));
    child.on("error", () => {
      resolve({ exitCode: 1, output: "" });
    });
    child.on("exit", (code) => {
      resolve({ exitCode: code ?? 1, output: chunks.join("") });
    });
  });
}

function loadDeniedFixtures(securityManifestPath) {
  if (!existsSync(securityManifestPath)) {
    return [];
  }
  try {
    const manifest = JSON.parse(readFileSync(securityManifestPath, "utf8"));
    return Array.isArray(manifest.deniedFixtures) ? manifest.deniedFixtures : [];
  } catch {
    return [];
  }
}

function assertFile(path, root) {
  if (!existsSync(path)) {
    throw new Error(`Missing required file: ${relative(path, root)}`);
  }
}

function readTextSync(path) {
  return readFileSync(path, "utf8");
}

function relative(path, root) {
  return path.startsWith(`${root}/`) ? path.slice(root.length + 1) : path;
}

function formatTimestamp(date) {
  const parts = [
    date.getFullYear(),
    date.getMonth() + 1,
    date.getDate(),
    date.getHours(),
    date.getMinutes(),
    date.getSeconds(),
  ].map((value) => String(value).padStart(2, "0"));
  return `${parts[0]}${parts[1]}${parts[2]}-${parts[3]}${parts[4]}${parts[5]}`;
}

function formatGitStatusSummary(summary) {
  return `modified=${summary.modified} added=${summary.added} deleted=${summary.deleted} renamed=${summary.renamed} untracked=${summary.untracked} other=${summary.other}`;
}

function stableErrorMessage(error) {
  if (!(error instanceof Error)) {
    return "Release gate failed.";
  }
  if (/unchecked checklist/.test(error.message)) {
    return "Previous task checklist is incomplete.";
  }
  if (/Missing required file/.test(error.message)) {
    return "Required release file is missing.";
  }
  if (/missing required text/.test(error.message)) {
    return "Required release report text is missing.";
  }
  if (/manifest/i.test(error.message)) {
    return "Release manifest is invalid.";
  }
  return "Release gate failed.";
}

class ReleaseGateArtifactSanitizerError extends Error {
  constructor(findings) {
    super("Release result artifact contains denied fixture.");
    this.findings = findings;
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
