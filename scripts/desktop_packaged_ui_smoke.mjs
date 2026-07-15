import { spawn } from "node:child_process";
import { mkdir, mkdtemp, rename, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const PackagedUiSmokeRunnerErrorCode = Object.freeze({
  ProcessFailed: "PHASE012_PACKAGED_UI_PROCESS_FAILED",
  Timeout: "PHASE012_PACKAGED_UI_TIMEOUT",
  MarkerMissing: "PHASE012_PACKAGED_UI_MARKER_MISSING",
  SampleCountInvalid: "PHASE012_PACKAGED_UI_SAMPLE_COUNT_INVALID",
  PerformanceBudgetExceeded: "PHASE012_PACKAGED_UI_PERFORMANCE_BUDGET_EXCEEDED",
  UiErrorReported: "PHASE012_PACKAGED_UI_ERROR_REPORTED",
  ActionCoverageIncomplete: "PHASE012_PACKAGED_UI_ACTION_COVERAGE_INCOMPLETE",
});

export function analyzePackagedUiSmokeOutput(output, exitCode) {
  const values = new Map(String(output).split(/\r?\n/).map((line) => {
    const separator = line.indexOf("=");
    return separator < 0 ? [line, ""] : [line.slice(0, separator), line.slice(separator + 1)];
  }));
  if (values.get("phase012_packaged_ui_smoke") === "failed") {
    const reported = values.get("error_code");
    if (typeof reported === "string" && /^PHASE012_PACKAGED_UI_[A-Z0-9_]+$/.test(reported)) {
      return failed(reported);
    }
  }
  if (exitCode !== 0) return failed(PackagedUiSmokeRunnerErrorCode.ProcessFailed);
  if (values.get("phase012_packaged_ui_smoke") !== "passed") {
    return failed(PackagedUiSmokeRunnerErrorCode.MarkerMissing);
  }
  const sampleCount = Number(values.get("sample_count"));
  const p95Ms = Number(values.get("p95_ms"));
  const errorCount = Number(values.get("error_count"));
  const actionCount = Number(values.get("action_count"));
  const durableReadbackCount = Number(values.get("durable_readback_count"));
  if (sampleCount !== 200) return failed(PackagedUiSmokeRunnerErrorCode.SampleCountInvalid);
  if (errorCount !== 0) return failed(PackagedUiSmokeRunnerErrorCode.UiErrorReported);
  if (actionCount < 15 || durableReadbackCount < 4) {
    return failed(PackagedUiSmokeRunnerErrorCode.ActionCoverageIncomplete);
  }
  if (!Number.isFinite(p95Ms) || p95Ms > 300) {
    return failed(PackagedUiSmokeRunnerErrorCode.PerformanceBudgetExceeded);
  }
  return { passed: true, sampleCount, p95Ms, errorCount, actionCount, durableReadbackCount };
}

export async function executeWithDeadline(operation, timeoutMs, terminate) {
  let timer;
  const timeoutResult = new Promise((resolve) => {
    timer = setTimeout(() => {
      terminate();
      resolve(failed(PackagedUiSmokeRunnerErrorCode.Timeout));
    }, timeoutMs);
  });
  try {
    return await Promise.race([
      Promise.resolve().then(operation).catch(() => failed(PackagedUiSmokeRunnerErrorCode.ProcessFailed)),
      timeoutResult,
    ]);
  } finally {
    clearTimeout(timer);
  }
}

export async function runPackagedUiSmokeProcess(binary, { timeoutMs = 120_000 } = {}) {
  const profile = await mkdtemp(join(tmpdir(), "cabinet-packaged-ui-smoke-"));
  let child;
  try {
    const operation = () => new Promise((resolve) => {
      child = spawn(binary, ["--packaged-ui-smoke", profile], { stdio: ["ignore", "pipe", "pipe"] });
      let output = "";
      const collect = (chunk) => {
        if (output.length < 64 * 1024) output += chunk.toString("utf8");
      };
      child.stdout.on("data", collect);
      child.stderr.on("data", collect);
      child.on("error", () => resolve(failed(PackagedUiSmokeRunnerErrorCode.ProcessFailed)));
      child.on("close", (code) => resolve(analyzePackagedUiSmokeOutput(output, code)));
    });
    return await executeWithDeadline(operation, timeoutMs, () => child?.kill("SIGKILL"));
  } finally {
    await rm(profile, { recursive: true, force: true });
  }
}

function failed(errorCode) {
  return { passed: false, errorCode };
}

async function main() {
  const artifact = ".tasks/release/packaged-ui-smoke-phase012.md";
  await rm(artifact, { force: true });
  const binary = process.argv[2];
  if (!binary) {
    console.log("phase012_packaged_ui_smoke=failed");
    console.log(`error_code=${PackagedUiSmokeRunnerErrorCode.ProcessFailed}`);
    process.exitCode = 1;
    return;
  }
  const result = await runPackagedUiSmokeProcess(binary);
  console.log(`phase012_packaged_ui_smoke=${result.passed ? "passed" : "failed"}`);
  if (result.passed) {
    console.log(`sample_count=${result.sampleCount}`);
    console.log(`p95_ms=${result.p95Ms}`);
    console.log(`error_count=${result.errorCount}`);
    console.log(`action_count=${result.actionCount}`);
    console.log(`durable_readback_count=${result.durableReadbackCount}`);
    await writeEvidenceAtomically(artifact, result);
  } else {
    console.log(`error_code=${result.errorCode}`);
    process.exitCode = 1;
  }
}

async function writeEvidenceAtomically(path, result) {
  await mkdir(".tasks/release", { recursive: true });
  const temporary = `${path}.tmp`;
  const text = [
    "# Phase 012 Packaged UI Smoke",
    "",
    "phase012_packaged_ui_smoke=passed",
    `sample_count=${result.sampleCount}`,
    `p95_ms=${result.p95Ms}`,
    `error_count=${result.errorCount}`,
    `action_count=${result.actionCount}`,
    `durable_readback_count=${result.durableReadbackCount}`,
    "surfaces=home,graph,canvas,assets",
    "native_query_generation_required=true",
    "document_body_excluded=true",
    "asset_bytes_excluded=true",
    "absolute_path_excluded=true",
    "",
  ].join("\n");
  await writeFile(temporary, text, "utf8");
  await rename(temporary, path);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await main();
}
