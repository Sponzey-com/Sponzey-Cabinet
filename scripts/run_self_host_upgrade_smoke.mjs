import { spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";

const root = process.cwd();
const outputArtifactPath = join(root, ".tmp", "self-host-upgrade-smoke-output.txt");

export function buildSelfHostUpgradeSmokePlan() {
  return {
    name: "run_self_host_upgrade_smoke",
    steps: [
      step("migration_state_machine", [
        "cargo",
        "test",
        "-p",
        "cabinet-core",
        "--test",
        "migration_tests",
      ], "migration_state_machine=verified"),
      step("data_preservation_smoke", [
        "cargo",
        "test",
        "-p",
        "cabinet-platform",
        "--test",
        "data_preservation_smoke",
      ], "data_preservation_smoke=passed"),
      step("phase002_migration_fixture_smoke", [
        "cargo",
        "test",
        "-p",
        "cabinet-platform",
        "--test",
        "phase002_migration_fixture_smoke",
      ], "phase002_migration_fixture_smoke=passed"),
    ],
  };
}

export function validateUpgradeSmokeResults(results) {
  for (const result of results) {
    if (result.exitCode !== 0) {
      throw new Error(`upgrade smoke step failed: ${result.step.id}`);
    }
    const combinedOutput = `${result.stdout}\n${result.stderr}`;
    if (!combinedOutput.includes(result.step.successMarker)) {
      throw new Error(`upgrade smoke marker was not found: ${result.step.successMarker}`);
    }
    assertSensitiveOutputClean(combinedOutput);
  }

  return {
    passed: true,
    stepIds: results.map((result) => result.step.id),
  };
}

export function renderUpgradeSmokeSummary(result) {
  return [
    "run_self_host_upgrade_smoke=true",
    "migration_state_machine=verified",
    "upgrade_migration_smoke=passed",
    `step_count=${result.stepIds.length}`,
    `steps=${result.stepIds.join(",")}`,
  ].join("\n");
}

export function assertSensitiveOutputClean(output) {
  const forbiddenPatterns = [
    /token=/i,
    /secret/i,
    /credential/i,
    /password/i,
    /fixture document body should not be logged/i,
    /asset content should not be logged/i,
  ];
  const matched = forbiddenPatterns.find((pattern) => pattern.test(output));
  if (matched) {
    throw new Error(`sensitive output detected: ${matched}`);
  }
}

async function runCli() {
  const plan = buildSelfHostUpgradeSmokePlan();
  const results = [];
  for (const currentStep of plan.steps) {
    const result = await runCommand(currentStep.command);
    const stdoutWithMarker =
      result.exitCode === 0
        ? `${result.stdout}\n${currentStep.successMarker}\n`
        : result.stdout;
    const stepResult = {
      ...result,
      stdout: stdoutWithMarker,
      step: currentStep,
    };
    results.push(stepResult);
    if (result.exitCode !== 0) {
      break;
    }
  }

  const combinedOutput = results
    .map((result) => [`# ${result.step.id}`, result.stdout, result.stderr].join("\n"))
    .join("\n");
  await mkdir(dirname(outputArtifactPath), { recursive: true });
  await writeFile(outputArtifactPath, combinedOutput, "utf8");

  const validation = validateUpgradeSmokeResults(results);
  const summary = renderUpgradeSmokeSummary(validation);
  console.log(summary);
  console.log(`output_artifact=${outputArtifactPath}`);
}

function step(id, command, successMarker) {
  return {
    id,
    command,
    successMarker,
  };
}

function runCommand(command) {
  const [bin, ...args] = command;
  return new Promise((resolve) => {
    const child = spawn(bin, args, {
      cwd: root,
      env: process.env,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    child.on("close", (exitCode, signal) => {
      resolve({
        exitCode: exitCode ?? 1,
        signal,
        stdout,
        stderr,
      });
    });
  });
}

if (process.argv[1]?.endsWith("run_self_host_upgrade_smoke.mjs")) {
  try {
    await runCli();
  } catch (error) {
    console.error("upgrade_migration_smoke=failed");
    console.error(`message=${error instanceof Error ? error.message : String(error)}`);
    process.exit(1);
  }
}
