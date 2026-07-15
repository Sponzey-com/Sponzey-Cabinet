import { spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";

const root = process.cwd();
const outputArtifactPath = join(root, ".tmp", "self-host-server-package-smoke-output.txt");

export function buildSelfHostServerPackageSmokePlan() {
  return {
    buildCommandLabel: "cargo build -p cabinet-server",
    buildCommand: ["cargo", "build", "-p", "cabinet-server"],
    smokeCommand: ["target/debug/cabinet-server", "--self-host-package-smoke"],
  };
}

export function assertPackageSmokeOutput(output) {
  if (!output.includes("server_package_smoke=passed")) {
    throw new Error("server_package_smoke marker was not found");
  }
  if (!/route_count=([1-9][0-9]*)/.test(output)) {
    throw new Error("positive route_count marker was not found");
  }
  if (!output.includes("health_status_code=200")) {
    throw new Error("health_status_code=200 marker was not found");
  }
  if (!output.includes("default_profile_without_external_services=true")) {
    throw new Error("default profile marker was not found");
  }
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
  const plan = buildSelfHostServerPackageSmokePlan();
  const build = await runCommand(plan.buildCommand);
  if (build.exitCode !== 0) {
    process.stdout.write(build.stdout);
    process.stderr.write(build.stderr);
    throw new Error(`server package build failed with exit code ${build.exitCode}`);
  }

  const smoke = await runCommand(plan.smokeCommand);
  const combinedOutput = [smoke.stdout, smoke.stderr].filter(Boolean).join("\n");
  await mkdir(dirname(outputArtifactPath), { recursive: true });
  await writeFile(outputArtifactPath, combinedOutput, "utf8");

  if (smoke.exitCode !== 0) {
    process.stdout.write(smoke.stdout);
    process.stderr.write(smoke.stderr);
    throw new Error(`server package smoke failed with exit code ${smoke.exitCode}`);
  }

  assertPackageSmokeOutput(combinedOutput);
  assertSensitiveOutputClean(combinedOutput);

  process.stdout.write(smoke.stdout);
  console.log(`output_artifact=${outputArtifactPath}`);
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

if (process.argv[1]?.endsWith("run_self_host_server_package_smoke.mjs")) {
  try {
    await runCli();
  } catch (error) {
    console.error("server_package_smoke=failed");
    console.error(`message=${error instanceof Error ? error.message : String(error)}`);
    process.exit(1);
  }
}
