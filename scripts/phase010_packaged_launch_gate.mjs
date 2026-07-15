import { spawn } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const Phase010PackagedLaunchState = Object.freeze({
  Pending: "Pending",
  ReadingPrerequisites: "ReadingPrerequisites",
  ValidatingEvidence: "ValidatingEvidence",
  RunningPackageSmoke: "RunningPackageSmoke",
  RunningPackagedAppSmoke: "RunningPackagedAppSmoke",
  WritingResult: "WritingResult",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase010PackagedLaunchEvent = Object.freeze({
  Start: "Start",
  PrerequisitesRead: "PrerequisitesRead",
  EvidenceValidated: "EvidenceValidated",
  PackageSmokePassed: "PackageSmokePassed",
  PackagedAppSmokePassed: "PackagedAppSmokePassed",
  ResultWritten: "ResultWritten",
  Fail: "Fail",
});

export const Phase010PackagedLaunchErrorCode = Object.freeze({
  PlanValidationMarkerMissing: "PHASE010_PLAN_VALIDATION_MARKER_MISSING",
  SourceReadFailed: "PHASE010_PACKAGED_SOURCE_READ_FAILED",
  DevServerDependencyDetected: "PHASE010_PACKAGED_DEV_SERVER_DEPENDENCY_DETECTED",
  InstalledRuntimeDependencyDetected: "PHASE010_PACKAGED_INSTALLED_RUNTIME_DEPENDENCY_DETECTED",
  PackageScriptMissing: "PHASE010_PACKAGED_SCRIPT_MISSING",
  PackageSmokeFailed: "PHASE010_PACKAGE_SMOKE_FAILED",
  PackagedAppSmokeFailed: "PHASE010_PACKAGED_APP_SMOKE_FAILED",
  ArtifactWriteFailed: "PHASE010_PACKAGED_ARTIFACT_WRITE_FAILED",
  InvalidTransition: "PHASE010_PACKAGED_LAUNCH_INVALID_TRANSITION",
});

const sourcePaths = Object.freeze([
  "scripts/run_desktop_package_smoke.sh",
  "scripts/run_desktop_packaged_app_smoke.sh",
  "scripts/run_desktop_tauri_build.sh",
  "package.json",
]);

const devServerForbiddenTerms = Object.freeze([
  "scripts/run_web_app.mjs",
  "run:desktop-app",
  "SPONZEY_CABINET_REQUIRE_EXACT_PORT",
  "localhost:5173",
  "127.0.0.1:5173",
]);

const installedRuntimeForbiddenTerms = Object.freeze([
  "node runtime required for installed app",
  "node_runtime_required=true",
  "installed_node_runtime_required=true",
  "external db required",
  "external search required",
  "git cli required",
  "manual env required",
  "edit .env",
  "postgres://",
  "meilisearch",
]);

export function transitionPhase010PackagedLaunchState(currentState, event, detail = {}) {
  if (
    currentState === Phase010PackagedLaunchState.Pending &&
    event === Phase010PackagedLaunchEvent.Start
  ) {
    return { state: Phase010PackagedLaunchState.ReadingPrerequisites };
  }
  if (
    currentState === Phase010PackagedLaunchState.ReadingPrerequisites &&
    event === Phase010PackagedLaunchEvent.PrerequisitesRead
  ) {
    return { state: Phase010PackagedLaunchState.ValidatingEvidence };
  }
  if (
    currentState === Phase010PackagedLaunchState.ValidatingEvidence &&
    event === Phase010PackagedLaunchEvent.EvidenceValidated
  ) {
    return { state: Phase010PackagedLaunchState.RunningPackageSmoke };
  }
  if (
    currentState === Phase010PackagedLaunchState.RunningPackageSmoke &&
    event === Phase010PackagedLaunchEvent.PackageSmokePassed
  ) {
    return { state: Phase010PackagedLaunchState.RunningPackagedAppSmoke };
  }
  if (
    currentState === Phase010PackagedLaunchState.RunningPackagedAppSmoke &&
    event === Phase010PackagedLaunchEvent.PackagedAppSmokePassed
  ) {
    return { state: Phase010PackagedLaunchState.WritingResult };
  }
  if (
    currentState === Phase010PackagedLaunchState.WritingResult &&
    event === Phase010PackagedLaunchEvent.ResultWritten
  ) {
    return { state: Phase010PackagedLaunchState.Passed };
  }
  if (
    [
      Phase010PackagedLaunchState.ReadingPrerequisites,
      Phase010PackagedLaunchState.ValidatingEvidence,
      Phase010PackagedLaunchState.RunningPackageSmoke,
      Phase010PackagedLaunchState.RunningPackagedAppSmoke,
      Phase010PackagedLaunchState.WritingResult,
    ].includes(currentState) &&
    event === Phase010PackagedLaunchEvent.Fail
  ) {
    return {
      state: Phase010PackagedLaunchState.Failed,
      errorCode: detail.errorCode ?? Phase010PackagedLaunchErrorCode.SourceReadFailed,
      findingId: detail.findingId,
      failedStepId: detail.failedStepId,
    };
  }
  return {
    state: Phase010PackagedLaunchState.Failed,
    errorCode: Phase010PackagedLaunchErrorCode.InvalidTransition,
  };
}

export function buildPhase010PackagedLaunchCommandPlan() {
  return [
    step("desktop_package_smoke", ["npm", "run", "run:desktop-package-smoke"]),
    step("desktop_packaged_app_smoke", ["npm", "run", "run:desktop-packaged-app-smoke"]),
  ];
}

export function validatePhase010PackagedLaunchSources(sources) {
  for (const path of sourcePaths.filter((sourcePath) => sourcePath !== "package.json")) {
    if (!Object.hasOwn(sources, path)) {
      return failedSource(Phase010PackagedLaunchErrorCode.SourceReadFailed, path);
    }
  }

  for (const path of sourcePaths.filter((sourcePath) => sourcePath !== "package.json")) {
    const text = sources[path];
    for (const term of devServerForbiddenTerms) {
      if (text.includes(term)) {
        return failedSource(Phase010PackagedLaunchErrorCode.DevServerDependencyDetected, path);
      }
    }
    for (const term of installedRuntimeForbiddenTerms) {
      if (text.toLowerCase().includes(term)) {
        return failedSource(
          Phase010PackagedLaunchErrorCode.InstalledRuntimeDependencyDetected,
          path,
        );
      }
    }
  }

  const packageSmoke = sources["scripts/run_desktop_package_smoke.sh"];
  if (
    !packageSmoke.includes("node scripts/build_desktop_assets.mjs") ||
    !packageSmoke.includes("cargo build -p cabinet-desktop-shell") ||
    !packageSmoke.includes("target/debug/cabinet-desktop-shell --packaged-smoke")
  ) {
    return failedSource(
      Phase010PackagedLaunchErrorCode.PackageScriptMissing,
      "scripts/run_desktop_package_smoke.sh",
    );
  }

  const packagedAppSmoke = sources["scripts/run_desktop_packaged_app_smoke.sh"];
  if (
    !packagedAppSmoke.includes("scripts/run_desktop_tauri_build.sh") ||
    !packagedAppSmoke.includes("--packaged-smoke")
  ) {
    return failedSource(
      Phase010PackagedLaunchErrorCode.PackageScriptMissing,
      "scripts/run_desktop_packaged_app_smoke.sh",
    );
  }

  let packageJson;
  try {
    packageJson = JSON.parse(sources["package.json"]);
  } catch {
    return failedSource(Phase010PackagedLaunchErrorCode.PackageScriptMissing, "package.json");
  }
  const scripts = packageJson.scripts ?? {};
  if (
    scripts["run:desktop-package-smoke"] !== "sh scripts/run_desktop_package_smoke.sh" ||
    scripts["run:desktop-packaged-app-smoke"] !==
      "sh scripts/run_desktop_packaged_app_smoke.sh"
  ) {
    return failedSource(Phase010PackagedLaunchErrorCode.PackageScriptMissing, "package.json");
  }

  return {
    passed: true,
    devServerRequired: false,
    installedNodeRuntimeRequired: false,
    externalDbRequired: false,
    externalSearchRequired: false,
    gitCliRequired: false,
    manualEnvironmentRequired: false,
    sourceCount: sourcePaths.length,
  };
}

export async function runPhase010PackagedLaunchGate({
  root = process.cwd(),
  writeArtifact = true,
  runner = runCommandStep,
  steps = buildPhase010PackagedLaunchCommandPlan(),
} = {}) {
  let state = transitionPhase010PackagedLaunchState(
    Phase010PackagedLaunchState.Pending,
    Phase010PackagedLaunchEvent.Start,
  ).state;

  try {
    const planMarkerPath = ".tasks/phase010-plan-validation-result.md";
    const planMarkerText = await readFile(join(root, planMarkerPath), "utf8");
    if (!planMarkerText.includes("phase010_plan_validation=passed")) {
      return failedResult(
        transitionPhase010PackagedLaunchState(state, Phase010PackagedLaunchEvent.Fail, {
          errorCode: Phase010PackagedLaunchErrorCode.PlanValidationMarkerMissing,
          findingId: planMarkerPath,
        }),
        [],
      );
    }

    state = transitionPhase010PackagedLaunchState(
      state,
      Phase010PackagedLaunchEvent.PrerequisitesRead,
    ).state;

    const sources = {};
    for (const path of sourcePaths) {
      sources[path] = await readFile(join(root, path), "utf8");
    }
    const sourceValidation = validatePhase010PackagedLaunchSources(sources);
    if (!sourceValidation.passed) {
      return failedResult(
        transitionPhase010PackagedLaunchState(state, Phase010PackagedLaunchEvent.Fail, {
          errorCode: sourceValidation.errorCode,
          findingId: sourceValidation.findingId,
        }),
        [],
        sourceValidation,
      );
    }

    state = transitionPhase010PackagedLaunchState(
      state,
      Phase010PackagedLaunchEvent.EvidenceValidated,
    ).state;

    const commandResults = [];
    for (const gateStep of steps) {
      const started = Date.now();
      const execution = await runner(gateStep, { root });
      const status = execution.exitCode === 0 && !execution.signal ? "passed" : "failed";
      const commandResult = {
        id: gateStep.id,
        command: gateStep.command.join(" "),
        status,
        exitCode: execution.exitCode,
        signal: execution.signal ?? null,
        durationMs: execution.durationMs ?? Date.now() - started,
      };
      commandResults.push(commandResult);

      if (status !== "passed") {
        const errorCode =
          gateStep.id === "desktop_package_smoke"
            ? Phase010PackagedLaunchErrorCode.PackageSmokeFailed
            : Phase010PackagedLaunchErrorCode.PackagedAppSmokeFailed;
        return failedResult(
          transitionPhase010PackagedLaunchState(state, Phase010PackagedLaunchEvent.Fail, {
            errorCode,
            findingId: gateStep.id,
            failedStepId: gateStep.id,
          }),
          commandResults,
          sourceValidation,
        );
      }

      state = transitionPhase010PackagedLaunchState(
        state,
        gateStep.id === "desktop_package_smoke"
          ? Phase010PackagedLaunchEvent.PackageSmokePassed
          : Phase010PackagedLaunchEvent.PackagedAppSmokePassed,
      ).state;
    }

    const result = {
      passed: true,
      state: Phase010PackagedLaunchState.Passed,
      sourceValidation,
      commandResults,
    };

    if (writeArtifact) {
      await writePackagedLaunchArtifacts(root, result);
    }

    state = transitionPhase010PackagedLaunchState(
      state,
      Phase010PackagedLaunchEvent.ResultWritten,
    ).state;
    return { ...result, state };
  } catch (error) {
    return failedResult(
      transitionPhase010PackagedLaunchState(state, Phase010PackagedLaunchEvent.Fail, {
        errorCode: Phase010PackagedLaunchErrorCode.SourceReadFailed,
        findingId: error.path ?? error.message,
      }),
      [],
    );
  }
}

export function renderPhase010PackagedRuntimeManifest(result) {
  const passed = result.passed === true;
  const sourceValidation = result.sourceValidation ?? {};
  return `${JSON.stringify(
    {
      schemaVersion: 1,
      marker: passed
        ? "phase010_packaged_runtime_manifest=passed"
        : "phase010_packaged_runtime_manifest=failed",
      productScope: "personal_local_desktop",
      runtimeKind: "tauri_desktop_packaged_runtime",
      devServerRequired: sourceValidation.devServerRequired ?? false,
      installedNodeRuntimeRequired: sourceValidation.installedNodeRuntimeRequired ?? false,
      externalDbRequired: sourceValidation.externalDbRequired ?? false,
      externalSearchRequired: sourceValidation.externalSearchRequired ?? false,
      gitCliRequired: sourceValidation.gitCliRequired ?? false,
      manualEnvironmentRequired: sourceValidation.manualEnvironmentRequired ?? false,
      commands: sanitizeCommandResults(result.commandResults ?? []),
      sensitiveDataPolicy:
        "Records command ids, statuses, exit codes, and durations only. Raw stdout, document body, asset bytes, provider key, token, credential, secret, and raw local path are excluded.",
    },
    null,
    2,
  )}\n`;
}

export function renderPhase010PackagedLaunchArtifact(result) {
  const passed = result.passed === true;
  const marker = passed
    ? "phase010_packaged_launch_gate=passed"
    : "phase010_packaged_launch_gate=failed";
  const lines = [
    "# Phase 010 Packaged Launch Gate Result",
    "",
    marker,
    `validation_state=${result.state}`,
    "",
    "- phase: `Phase 010.1`",
    "- gate: `Packaged Desktop Launch and Bundled Asset Gate`",
    `- status: \`${passed ? "passed" : "failed"}\``,
    "- prerequisites:",
    "  - `.tasks/phase010-plan-validation-result.md` with `phase010_plan_validation=passed`",
    "- validation commands:",
    "  - `npm run run:phase010-packaged-launch-gate-tests`",
    "  - `npm run run:desktop-package-smoke`",
    "  - `npm run run:desktop-packaged-app-smoke`",
    "  - `npm run run:phase010-packaged-launch-gate`",
    "- changed layers: `release-tooling`, `task-tooling`.",
    "- p95 300ms path impact: none. This gate validates packaged startup evidence only.",
    "- product log events: `desktop.packaged.launch.started`, `desktop.packaged.launch.ready`, `desktop.packaged.launch.failed`.",
    "- scope lock: personal local desktop only. Server hosting, SaaS, multi-user, mobile implementation, SSO, billing, admin console, and collaboration are future/out-of-scope for Phase 010.",
    "- completion evidence: marker artifacts, packaged runtime manifest, and smoke command results only. Task checkbox text is not release evidence.",
    "- sensitive data exclusion: this artifact records command ids, status, exit code, duration, marker names, scopes, and stable error codes only. It does not record raw command stdout, raw document body, asset content, AI prompt, AI answer, provider key, token, credential, secret, or personal absolute path.",
    "",
    "## Command Results",
    "",
    "| command | status | exit code | duration ms |",
    "| --- | --- | ---: | ---: |",
    ...sanitizeCommandResults(result.commandResults ?? []).map(
      (entry) =>
        `| \`${entry.id}\` | ${entry.status} | ${entry.exitCode ?? "null"} | ${entry.durationMs ?? 0} |`,
    ),
  ];

  if (!passed) {
    lines.push("");
    lines.push(`- error_code: \`${result.errorCode}\``);
    lines.push(`- finding_id: \`${result.findingId ?? "unknown"}\``);
    if (result.failedStepId) {
      lines.push(`- failed_step: \`${result.failedStepId}\``);
    }
  }

  lines.push("");
  return lines.join("\n");
}

async function writePackagedLaunchArtifacts(root, result) {
  await mkdir(join(root, ".tasks", "release"), { recursive: true });
  await writeFile(
    join(root, ".tasks", "release", "packaged-runtime-manifest-phase010.json"),
    renderPhase010PackagedRuntimeManifest(result),
  );
  await writeFile(
    join(root, ".tasks", "phase010-packaged-launch-gate-result.md"),
    renderPhase010PackagedLaunchArtifact(result),
  );
}

function failedResult(failedTransition, commandResults, sourceValidation = {}) {
  return {
    passed: false,
    state: Phase010PackagedLaunchState.Failed,
    errorCode: failedTransition.errorCode,
    findingId: failedTransition.findingId,
    failedStepId: failedTransition.failedStepId,
    commandResults,
    sourceValidation,
  };
}

function failedSource(errorCode, findingId) {
  return {
    passed: false,
    errorCode,
    findingId,
    devServerRequired: errorCode === Phase010PackagedLaunchErrorCode.DevServerDependencyDetected,
    installedNodeRuntimeRequired:
      errorCode === Phase010PackagedLaunchErrorCode.InstalledRuntimeDependencyDetected,
    externalDbRequired: false,
    externalSearchRequired: false,
    gitCliRequired: false,
    manualEnvironmentRequired: false,
  };
}

function sanitizeCommandResults(commandResults) {
  return commandResults.map((entry) => ({
    id: entry.id,
    command: entry.command,
    status: entry.status,
    exitCode: entry.exitCode,
    signal: entry.signal ?? null,
    durationMs: entry.durationMs,
  }));
}

function step(id, command) {
  return { id, command };
}

function runCommandStep(gateStep, { root }) {
  return new Promise((resolve, reject) => {
    const started = Date.now();
    const [command, ...args] = gateStep.command;
    const child = spawn(command, args, { cwd: root, stdio: "inherit" });
    child.on("error", reject);
    child.on("exit", (exitCode, signal) =>
      resolve({ exitCode, signal, durationMs: Date.now() - started }),
    );
  });
}

async function main() {
  const result = await runPhase010PackagedLaunchGate({
    root: process.cwd(),
    writeArtifact: true,
  });
  if (!result.passed) {
    console.error(`${result.errorCode} ${result.findingId ?? ""}`.trim());
    process.exit(1);
  }
  console.log("phase010_packaged_launch_gate=passed");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    console.error(error);
    process.exit(1);
  });
}
