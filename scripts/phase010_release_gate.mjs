import { spawn } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const Phase010ReleaseGateState = Object.freeze({
  Pending: "Pending",
  ReadingMarkers: "ReadingMarkers",
  ValidatingArtifacts: "ValidatingArtifacts",
  RunningReleaseCommands: "RunningReleaseCommands",
  WritingResult: "WritingResult",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase010ReleaseGateEvent = Object.freeze({
  Start: "Start",
  MarkersRead: "MarkersRead",
  ArtifactsValidated: "ArtifactsValidated",
  CommandsPassed: "CommandsPassed",
  ResultWritten: "ResultWritten",
  Fail: "Fail",
});

export const Phase010ReleaseGateErrorCode = Object.freeze({
  MissingMarker: "PHASE010_MISSING_MARKER",
  InvalidScope: "PHASE010_INVALID_SCOPE",
  PackagedSmokeFailed: "PHASE010_PACKAGED_SMOKE_FAILED",
  SecurityScanFailed: "PHASE010_SECURITY_SCAN_FAILED",
  RunbookValidationFailed: "PHASE010_RUNBOOK_VALIDATION_FAILED",
  PerformanceBudgetFailed: "PHASE010_PERFORMANCE_BUDGET_FAILED",
  SourceReadFailed: "PHASE010_SOURCE_READ_FAILED",
  UnsafeArtifactContent: "PHASE010_UNSAFE_ARTIFACT_CONTENT",
  CommandFailed: "PHASE010_RELEASE_COMMAND_FAILED",
  InvalidTransition: "PHASE010_INVALID_TRANSITION",
});

const requiredEvidence = Object.freeze([
  evidence("phase010_archive_validation", ".tasks/phase010-archive-validation-result.md", [
    "phase010_archive_validation=passed",
  ]),
  evidence("phase010_plan_validation", ".tasks/phase010-plan-validation-result.md", [
    "phase010_plan_validation=passed",
  ]),
  evidence("phase010_packaged_launch_gate", ".tasks/phase010-packaged-launch-gate-result.md", [
    "phase010_packaged_launch_gate=passed",
  ]),
  evidence("phase010_first_run_workspace_gate", ".tasks/phase010-first-run-workspace-gate-result.md", [
    "phase010_first_run_workspace_gate=passed",
  ]),
  evidence("phase010_durable_authoring_gate", ".tasks/phase010-durable-authoring-gate-result.md", [
    "phase010_durable_authoring_gate=passed",
  ]),
  evidence("phase010_data_portability_gate", ".tasks/phase010-data-portability-gate-result.md", [
    "phase010_data_portability_gate=passed",
  ]),
  evidence("phase010_index_health_repair_gate", ".tasks/phase010-index-health-repair-gate-result.md", [
    "phase010_index_health_repair_gate=passed",
  ]),
  evidence("phase010_settings_observability_gate", ".tasks/phase010-settings-observability-gate-result.md", [
    "phase010_settings_observability_gate=passed",
    "settings_scope=personal_local_desktop",
    "ai_provider_optional=verified",
    "field_debug_guard=verified",
  ]),
  evidence("phase010_performance_budget", ".tasks/release/performance-budget-phase010.md", [
    "phase010_performance_budget=passed",
    "current document read",
    "history list",
    "search",
    "asset metadata",
  ], Phase010ReleaseGateErrorCode.PerformanceBudgetFailed),
  evidence("phase010_packaged_runtime_manifest", ".tasks/release/packaged-runtime-manifest-phase010.json", [
    "phase010_packaged_runtime_manifest=passed",
    '"devServerRequired": false',
    '"installedNodeRuntimeRequired": false',
    '"externalDbRequired": false',
    '"externalSearchRequired": false',
  ]),
  evidence("phase010_data_portability_manifest", ".tasks/release/data-portability-manifest-phase010.json", [
    "phase010_data_portability_manifest=passed",
    "personal_local_desktop",
    "single_user_local_workspace",
  ]),
  evidence("phase010_product_log_matrix", ".tasks/release/product-log-event-matrix-phase010.md", [
    "phase010_product_log_matrix=passed",
    "Product Log",
    "Field Debug Log",
    "Development Log",
    "settings.opened",
    "field_debug.activation.created",
  ]),
  evidence("phase010_security_log_manifest", ".tasks/release/security-log-policy-manifest-phase010.json", [
    "phase010_security_log_manifest=passed",
    "Product Log",
    "Field Debug Log",
    "Development Log",
    "AUTH_MATERIAL_SAMPLE",
    "RAW_DOC_BODY_SAMPLE",
  ], Phase010ReleaseGateErrorCode.SecurityScanFailed),
  evidence("phase010_runbook", ".tasks/release/local-desktop-runbook-phase010.md", [
    "phase010_runbook=passed",
    "Clean Install",
    "Packaged Launch",
    "Reinstall Preservation",
    "Blank Screen Recovery",
    "Index Repair",
    "Export Import",
    "Backup Restore",
    "Field Debug",
    "Data Export",
  ], Phase010ReleaseGateErrorCode.RunbookValidationFailed),
  evidence("phase010_release_tooling", "package.json", [
    "run:phase010-release-gate-tests",
    "run:phase010-release-gate",
    "run:phase010-packaged-launch-gate",
    "run:phase010-settings-observability-gate",
  ]),
]);

const unsafeSourceTerms = [
  "provider_api_key_fixture",
  "raw_document_body_fixture",
  "personal_absolute_path_fixture",
  "token_fixture",
  "credential_fixture",
  "secret_fixture",
  "raw_prompt_fixture",
  "raw_answer_fixture",
  "/Users/example/private",
  "C:\\Users\\example\\private",
];

const futureOnlyTargets = ["self-hosting", "SaaS", "multi-user", "mobile"];

export function transitionPhase010ReleaseGateState(currentState, event, detail = {}) {
  if (currentState === Phase010ReleaseGateState.Pending && event === Phase010ReleaseGateEvent.Start) {
    return { state: Phase010ReleaseGateState.ReadingMarkers };
  }
  if (
    currentState === Phase010ReleaseGateState.ReadingMarkers &&
    event === Phase010ReleaseGateEvent.MarkersRead
  ) {
    return { state: Phase010ReleaseGateState.ValidatingArtifacts };
  }
  if (
    currentState === Phase010ReleaseGateState.ValidatingArtifacts &&
    event === Phase010ReleaseGateEvent.ArtifactsValidated
  ) {
    return { state: Phase010ReleaseGateState.RunningReleaseCommands };
  }
  if (
    currentState === Phase010ReleaseGateState.RunningReleaseCommands &&
    event === Phase010ReleaseGateEvent.CommandsPassed
  ) {
    return { state: Phase010ReleaseGateState.WritingResult };
  }
  if (
    currentState === Phase010ReleaseGateState.WritingResult &&
    event === Phase010ReleaseGateEvent.ResultWritten
  ) {
    return { state: Phase010ReleaseGateState.Passed };
  }
  if (
    [
      Phase010ReleaseGateState.ReadingMarkers,
      Phase010ReleaseGateState.ValidatingArtifacts,
      Phase010ReleaseGateState.RunningReleaseCommands,
      Phase010ReleaseGateState.WritingResult,
    ].includes(currentState) &&
    event === Phase010ReleaseGateEvent.Fail
  ) {
    return {
      state: Phase010ReleaseGateState.Failed,
      errorCode: detail.errorCode ?? Phase010ReleaseGateErrorCode.MissingMarker,
      findingId: detail.findingId,
      failedStepId: detail.failedStepId,
      failedCommandExitCode: detail.failedCommandExitCode,
    };
  }
  return {
    state: Phase010ReleaseGateState.Failed,
    errorCode: Phase010ReleaseGateErrorCode.InvalidTransition,
  };
}

export function buildPhase010ReleaseCommandPlan() {
  return [
    step("phase010ArchiveValidator", ["npm", "run", "run:phase010-archive-validator"]),
    step("phase010PlanValidator", ["npm", "run", "run:phase010-plan-validator"]),
    step("phase010PackagedLaunchGate", ["npm", "run", "run:phase010-packaged-launch-gate"]),
    step("phase010FirstRunWorkspaceGate", ["npm", "run", "run:phase010-first-run-workspace-gate"]),
    step("phase010DurableAuthoringGate", ["npm", "run", "run:phase010-durable-authoring-gate"]),
    step("phase010DataPortabilityGate", ["npm", "run", "run:phase010-data-portability-gate"]),
    step("phase010IndexHealthRepairGate", ["npm", "run", "run:phase010-index-health-repair-gate"]),
    step("phase010SettingsObservabilityGate", [
      "npm",
      "run",
      "run:phase010-settings-observability-gate",
    ]),
    step("rustWorkspace", ["cargo", "test", "--workspace"]),
    step("activeTypeScriptTests", [
      "node",
      "--test",
      "packages/client-core/tests/local_desktop_command_client_tests.ts",
      "packages/client-core/tests/personal_local_desktop_capability_tests.ts",
      "packages/ui/tests/personal_workspace_shell_model_tests.ts",
      "packages/ui/tests/personal_workspace_home_model_tests.ts",
      "packages/ui/tests/document_authoring_preview_model_tests.ts",
      "packages/ui/tests/local_discovery_panel_model_tests.ts",
      "packages/ui/tests/graph_canvas_panel_model_tests.ts",
      "packages/ui/tests/backup_restore_staging_model_tests.ts",
      "packages/ui/tests/import_preview_model_tests.ts",
      "packages/ui/tests/restore_flow_model_tests.ts",
      "packages/ui/tests/ai_citation_tool_scope_model_tests.ts",
      "packages/ui/tests/ai_query_ui_model_tests.ts",
      "apps/desktop/tests/desktop_local_command_facade_tests.ts",
      "apps/desktop/tests/desktop_personal_workspace_shell_tests.ts",
      "apps/desktop/tests/desktop_personal_workspace_home_tests.ts",
      "apps/desktop/tests/desktop_document_authoring_smoke_tests.ts",
      "apps/desktop/tests/desktop_discovery_smoke_tests.ts",
      "apps/desktop/tests/desktop_backup_restore_staging_smoke_tests.ts",
      "apps/desktop/tests/desktop_import_preview_smoke_tests.ts",
      "apps/desktop/tests/desktop_ai_local_ux_smoke_tests.ts",
    ]),
    step("desktopPackageSmoke", ["npm", "run", "run:desktop-package-smoke"]),
    step("desktopPackagedAppSmoke", ["npm", "run", "run:desktop-packaged-app-smoke"]),
    step("securityScan", [
      "node",
      "scripts/security_log_scanner.mjs",
      ".tasks/release/security-log-policy-manifest-phase010.json",
    ]),
    step("runbookValidation", ["node", "scripts/phase010_settings_observability_self_check.mjs"]),
  ];
}

export function evaluatePhase010ReleaseGate({
  sources,
  commandResults,
  commandPlan = buildPhase010ReleaseCommandPlan(),
}) {
  let state = transitionPhase010ReleaseGateState(
    Phase010ReleaseGateState.Pending,
    Phase010ReleaseGateEvent.Start,
  );

  if (!sources || Object.keys(sources).length === 0) {
    return failedResult(
      transitionPhase010ReleaseGateState(state.state, Phase010ReleaseGateEvent.Fail, {
        errorCode: Phase010ReleaseGateErrorCode.SourceReadFailed,
        findingId: "source_set",
      }),
      commandResults,
      0,
    );
  }

  const missingEvidence = findMissingEvidence(sources);
  if (missingEvidence) {
    return failedResult(
      transitionPhase010ReleaseGateState(state.state, Phase010ReleaseGateEvent.Fail, {
        errorCode: missingEvidence.errorCode,
        findingId: missingEvidence.id,
      }),
      commandResults,
      countSatisfiedEvidence(sources),
    );
  }

  state = transitionPhase010ReleaseGateState(state.state, Phase010ReleaseGateEvent.MarkersRead);
  const scopeFinding = findInvalidScope(commandPlan);
  if (scopeFinding) {
    return failedResult(
      transitionPhase010ReleaseGateState(state.state, Phase010ReleaseGateEvent.Fail, {
        errorCode: Phase010ReleaseGateErrorCode.InvalidScope,
        findingId: scopeFinding,
      }),
      commandResults,
      requiredEvidence.length,
    );
  }
  const unsafeFinding = findUnsafeSource(sources);
  if (unsafeFinding) {
    return failedResult(
      transitionPhase010ReleaseGateState(state.state, Phase010ReleaseGateEvent.Fail, {
        errorCode: Phase010ReleaseGateErrorCode.UnsafeArtifactContent,
        findingId: unsafeFinding,
      }),
      commandResults,
      requiredEvidence.length,
    );
  }

  state = transitionPhase010ReleaseGateState(
    state.state,
    Phase010ReleaseGateEvent.ArtifactsValidated,
  );
  for (const gateStep of commandPlan) {
    if (!commandResults[gateStep.id]?.passed) {
      return failedResult(
        transitionPhase010ReleaseGateState(state.state, Phase010ReleaseGateEvent.Fail, {
          errorCode: commandErrorCode(gateStep.id),
          findingId: gateStep.id,
          failedStepId: gateStep.id,
          failedCommandExitCode: commandResults[gateStep.id]?.exitCode ?? null,
        }),
        commandResults,
        requiredEvidence.length,
      );
    }
  }

  state = transitionPhase010ReleaseGateState(state.state, Phase010ReleaseGateEvent.CommandsPassed);
  state = transitionPhase010ReleaseGateState(state.state, Phase010ReleaseGateEvent.ResultWritten);
  return {
    passed: true,
    state: state.state,
    commandResults,
    evidenceCount: requiredEvidence.length,
    requiredTargets: ["local-desktop"],
    futureOnlyTargets,
  };
}

export async function runPhase010ReleaseGate({
  root = process.cwd(),
  writeArtifacts = true,
  runner = runCommandStep,
  commandPlan = buildPhase010ReleaseCommandPlan(),
} = {}) {
  const sources = await readRequiredSources(root);
  const commandResults = {};
  const preflight = evaluatePhase010ReleaseGate({
    sources,
    commandResults: Object.fromEntries(
      commandPlan.map((gateStep) => [gateStep.id, { passed: true, command: gateStep.command.join(" ") }]),
    ),
    commandPlan,
  });
  if (!preflight.passed) {
    if (writeArtifacts) {
      await writeReleaseArtifact(root, preflight);
    }
    return preflight;
  }

  for (const gateStep of commandPlan) {
    const started = Date.now();
    const execution = await runner(gateStep, { root });
    commandResults[gateStep.id] = {
      command: gateStep.command.join(" "),
      passed: execution.exitCode === 0 && !execution.signal,
      exitCode: execution.exitCode,
      signal: execution.signal ?? null,
      durationMs: execution.durationMs ?? Date.now() - started,
    };
    if (!commandResults[gateStep.id].passed) {
      const partial = evaluatePhase010ReleaseGate({ sources, commandResults, commandPlan });
      if (writeArtifacts) {
        await writeReleaseArtifact(root, partial);
      }
      return partial;
    }
  }

  const refreshedSources = await readRequiredSources(root);
  const result = evaluatePhase010ReleaseGate({
    sources: refreshedSources,
    commandResults,
    commandPlan,
  });
  if (writeArtifacts) {
    await writeReleaseArtifact(root, result);
  }
  return result;
}

export function renderPhase010ReleaseGateArtifact(result) {
  const passed = result.passed === true;
  const lines = [
    "# Phase 010 Final Release Gate Result",
    "",
    passed ? "phase010_release_gate=passed" : "phase010_release_gate=failed",
    `validation_state=${result.state}`,
    `release_scope=${passed ? "personal_local_desktop" : "not_verified"}`,
    `p95_300ms_budget=${passed ? "passed" : "not_verified"}`,
    `future_only_targets=${futureOnlyTargets.join(",")}`,
    "",
    "- phase: `Phase 010.7`",
    "- product target: personal local desktop installable knowledge management app only.",
    "- release required target list: local desktop packaged runtime, first-run workspace, durable authoring, data portability, index repair, settings observability, sanitized logging evidence, and p95 300ms read/search budget.",
    "- future-only target list: self-hosting, SaaS, multi-user, mobile.",
    "- source rule: this gate reads marker files and release artifacts only; it does not infer completion from task checkboxes.",
    "- logging rule: this artifact records marker states, command ids, exit codes, durations, and stable error codes only. It excludes raw document body, asset content, backup package contents, provider raw key, authentication material, raw prompt, raw answer, raw local path, and raw command stdout.",
    "",
    `- evidence_count: ${result.evidenceCount ?? 0}`,
    "- required evidence:",
    ...requiredEvidence.map((item) => `  - ${item.id}: \`${item.path}\``),
    "",
    "## Command Results",
    "",
    "| command | status | exit code | duration ms |",
    "| --- | --- | ---: | ---: |",
    ...Object.entries(result.commandResults ?? {}).map(
      ([id, entry]) =>
        `| \`${id}\` | ${entry.passed ? "passed" : "failed"} | ${entry.exitCode ?? "null"} | ${entry.durationMs ?? 0} |`,
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

async function readRequiredSources(root) {
  const sources = {};
  for (const item of requiredEvidence) {
    try {
      sources[item.path] = await readFile(join(root, item.path), "utf8");
    } catch {
      sources[item.path] = "";
    }
  }
  return sources;
}

async function writeReleaseArtifact(root, result) {
  await mkdir(join(root, ".tasks"), { recursive: true });
  await writeFile(
    join(root, ".tasks", "phase010-release-gate-result.md"),
    renderPhase010ReleaseGateArtifact(result),
  );
}

function findMissingEvidence(sources) {
  for (const item of requiredEvidence) {
    const text = sources[item.path] ?? "";
    for (const marker of item.markers) {
      if (!text.includes(marker)) {
        return item;
      }
    }
  }
  return null;
}

function countSatisfiedEvidence(sources) {
  return requiredEvidence.filter((item) =>
    item.markers.every((marker) => (sources[item.path] ?? "").includes(marker)),
  ).length;
}

function findInvalidScope(commandPlan) {
  for (const gateStep of commandPlan) {
    const command = gateStep.command.join(" ");
    if (command.includes("self-host") || command.includes("mobile") || command.includes("remote") || command.includes("admin")) {
      return gateStep.id;
    }
  }
  return null;
}

function findUnsafeSource(sources) {
  for (const text of Object.values(sources)) {
    const finding = unsafeSourceTerms.find((term) => text.includes(term));
    if (finding) {
      return finding;
    }
  }
  return null;
}

function commandErrorCode(stepId) {
  if (["desktopPackageSmoke", "desktopPackagedAppSmoke", "phase010PackagedLaunchGate"].includes(stepId)) {
    return Phase010ReleaseGateErrorCode.PackagedSmokeFailed;
  }
  if (stepId === "securityScan") {
    return Phase010ReleaseGateErrorCode.SecurityScanFailed;
  }
  if (stepId === "runbookValidation") {
    return Phase010ReleaseGateErrorCode.RunbookValidationFailed;
  }
  return Phase010ReleaseGateErrorCode.CommandFailed;
}

function failedResult(state, commandResults, evidenceCount) {
  return {
    passed: false,
    state: state.state,
    errorCode: state.errorCode,
    findingId: state.findingId,
    failedStepId: state.failedStepId,
    failedCommandExitCode: state.failedCommandExitCode,
    commandResults,
    evidenceCount,
    requiredTargets: ["local-desktop"],
    futureOnlyTargets,
  };
}

function evidence(id, path, markers, errorCode = Phase010ReleaseGateErrorCode.MissingMarker) {
  return { id, path, markers, errorCode };
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
  const result = await runPhase010ReleaseGate({
    root: process.cwd(),
    writeArtifacts: true,
  });
  if (!result.passed) {
    console.error(`${result.errorCode} ${result.findingId ?? ""}`.trim());
    process.exit(1);
  }
  console.log("phase010_release_gate=passed");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    console.error(error);
    process.exit(1);
  });
}
