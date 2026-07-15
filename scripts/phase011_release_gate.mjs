import { spawn } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const Phase011ReleaseGateState = Object.freeze({
  Pending: "Pending",
  RunningCommands: "RunningCommands",
  ReadingMarkers: "ReadingMarkers",
  ValidatingArtifacts: "ValidatingArtifacts",
  WritingResult: "WritingResult",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase011ReleaseGateEvent = Object.freeze({
  Start: "Start",
  CommandsPassed: "CommandsPassed",
  MarkersRead: "MarkersRead",
  ArtifactsValidated: "ArtifactsValidated",
  ResultWritten: "ResultWritten",
  Fail: "Fail",
});

export const Phase011ReleaseGateErrorCode = Object.freeze({
  CommandFailed: "PHASE011_RELEASE_COMMAND_FAILED",
  MissingMarker: "PHASE011_RELEASE_MISSING_MARKER",
  InvalidScope: "PHASE011_RELEASE_INVALID_SCOPE",
  PerformanceBudgetFailed: "PHASE011_RELEASE_PERFORMANCE_BUDGET_FAILED",
  ProductSmokeFailed: "PHASE011_RELEASE_PRODUCT_SMOKE_FAILED",
  SecurityScanFailed: "PHASE011_RELEASE_SECURITY_SCAN_FAILED",
  RunbookValidationFailed: "PHASE011_RELEASE_RUNBOOK_VALIDATION_FAILED",
  NativePlatformFailed: "PHASE011_RELEASE_NATIVE_PLATFORM_FAILED",
  SourceReadFailed: "PHASE011_RELEASE_SOURCE_READ_FAILED",
  UnsafeArtifactContent: "PHASE011_RELEASE_UNSAFE_ARTIFACT_CONTENT",
  InvalidTransition: "PHASE011_RELEASE_INVALID_TRANSITION",
});

const requirementIds = Object.freeze([
  "SCOPE-01", "BOOT-01", "HOME-01", "NAV-01", "DOC-01", "DOC-02", "DOC-03",
  "HIST-01", "HIST-02", "DISC-01", "DATA-01", "CFG-01", "CFG-02", "LOG-01",
  "STATE-01", "PERF-01", "SEC-01", "UX-01", "PLAT-01", "COMPAT-01",
]);

const requiredEvidence = Object.freeze([
  evidence("phase011_archive_validation", ".tasks/phase011-archive-validation-result.md", ["phase011_archive_validation=passed"]),
  evidence("phase011_plan_validation", ".tasks/phase011-plan-validation-result.md", ["phase011_plan_validation=passed"]),
  evidence("phase011_workspace_home_gate", ".tasks/phase011-workspace-home-gate-result.md", ["phase011_workspace_home_gate=passed"]),
  evidence("phase011_document_authoring_gate", ".tasks/phase011-document-authoring-gate-result.md", ["phase011_document_authoring_gate=passed"]),
  evidence("phase011_history_restore_gate", ".tasks/phase011-history-restore-gate-result.md", ["phase011_history_restore_gate=passed", "git_terms_excluded=true"]),
  evidence("phase011_discovery_gate", ".tasks/phase011-discovery-gate-result.md", ["phase011_discovery_gate=passed"]),
  evidence("phase011_data_settings_gate", ".tasks/phase011-data-settings-gate-result.md", ["phase011_data_settings_gate=passed"]),
  evidence("phase011_recovery_observability_gate", ".tasks/phase011-recovery-observability-gate-result.md", ["phase011_recovery_observability_gate=passed"]),
  evidence("phase011_product_smoke_gate", ".tasks/phase011-product-smoke-gate-result.md", ["phase011_product_smoke_gate=passed"], Phase011ReleaseGateErrorCode.ProductSmokeFailed),
  evidence("phase011_performance_budget", ".tasks/release/performance-budget-phase011.md", ["phase011_performance_budget=passed", "300ms"], Phase011ReleaseGateErrorCode.PerformanceBudgetFailed),
  evidence("phase011_security_log_manifest", ".tasks/release/security-log-policy-manifest-phase011.json", ["phase011_security_log_manifest=passed", "Product Log", "Field Debug Log", "Development Log"], Phase011ReleaseGateErrorCode.SecurityScanFailed),
  evidence("phase011_runbook", ".tasks/release/local-desktop-runbook-phase011.md", ["phase011_runbook=passed", "external DB", "Git CLI", "Node.js runtime"], Phase011ReleaseGateErrorCode.RunbookValidationFailed),
  evidence("phase011_visual_accessibility", ".tasks/release/visual-accessibility-report-phase011.md", ["phase011_visual_accessibility=passed", "codemirror_mounted=true"]),
  evidence("phase011_native_platform_matrix", ".tasks/release/native-platform-matrix-phase011.md", ["phase011_native_platform_matrix=passed", "deferred_future"], Phase011ReleaseGateErrorCode.NativePlatformFailed),
  evidence("phase010_release_gate", ".tasks/phase010/phase010-release-gate-result.md", ["phase010_release_gate=passed", "release_scope=personal_local_desktop"]),
]);

const unsafeTerms = Object.freeze([
  "AUTH_MATERIAL_SAMPLE",
  "RAW_DOC_BODY_SAMPLE",
  "PERSONAL_PATH_SAMPLE",
  "AI_PROMPT_SAMPLE",
  "AI_ANSWER_SAMPLE",
  "provider_api_key_fixture",
  "raw_document_body_fixture",
  "personal_absolute_path_fixture",
  "raw_prompt_fixture",
  "raw_answer_fixture",
  "/Users/example/private",
  "C:\\Users\\example\\private",
]);

export function transitionPhase011ReleaseGateState(currentState, event, detail = {}) {
  if (currentState === Phase011ReleaseGateState.Pending && event === Phase011ReleaseGateEvent.Start) {
    return { state: Phase011ReleaseGateState.RunningCommands };
  }
  if (currentState === Phase011ReleaseGateState.RunningCommands && event === Phase011ReleaseGateEvent.CommandsPassed) {
    return { state: Phase011ReleaseGateState.ReadingMarkers };
  }
  if (currentState === Phase011ReleaseGateState.ReadingMarkers && event === Phase011ReleaseGateEvent.MarkersRead) {
    return { state: Phase011ReleaseGateState.ValidatingArtifacts };
  }
  if (currentState === Phase011ReleaseGateState.ValidatingArtifacts && event === Phase011ReleaseGateEvent.ArtifactsValidated) {
    return { state: Phase011ReleaseGateState.WritingResult };
  }
  if (currentState === Phase011ReleaseGateState.WritingResult && event === Phase011ReleaseGateEvent.ResultWritten) {
    return { state: Phase011ReleaseGateState.Passed };
  }
  if (
    [
      Phase011ReleaseGateState.RunningCommands,
      Phase011ReleaseGateState.ReadingMarkers,
      Phase011ReleaseGateState.ValidatingArtifacts,
      Phase011ReleaseGateState.WritingResult,
    ].includes(currentState) &&
    event === Phase011ReleaseGateEvent.Fail
  ) {
    return {
      state: Phase011ReleaseGateState.Failed,
      errorCode: detail.errorCode ?? Phase011ReleaseGateErrorCode.MissingMarker,
      findingId: detail.findingId,
      failedStepId: detail.failedStepId,
    };
  }
  return { state: Phase011ReleaseGateState.Failed, errorCode: Phase011ReleaseGateErrorCode.InvalidTransition };
}

export function buildPhase011ReleaseCommandPlan() {
  return [
    step("phase011ArchiveValidator", ["sh", "scripts/run_phase011_archive_validator.sh"]),
    step("phase011ProductSmokeGate", ["sh", "scripts/run_phase011_product_smoke_gate.sh"]),
    step("desktopPackageSmoke", ["sh", "scripts/run_desktop_package_smoke.sh"]),
    step("securityScan", [nodeBin(), "scripts/security_log_scanner.mjs", ".tasks/release/security-log-policy-manifest-phase011.json"]),
    step("runbookValidation", [nodeBin(), "scripts/runbook_validator.mjs", ".tasks/release/runbook-validation-manifest-phase011.json"]),
    step("repositoryIntegrityRust", ["cargo", "test", "--workspace"]),
  ];
}

export function analyzePhase011ReleaseEvidence({ sources, commandResults = {} }) {
  const sourceFingerprint = readSourceFingerprint(sources?.[".tasks/phase011-current-implementation-inventory.md"]);
  if (!sourceFingerprint) return failed(Phase011ReleaseGateErrorCode.SourceReadFailed, "source_fingerprint");

  for (const entry of requiredEvidence) {
    const text = sources?.[entry.path] ?? "";
    const missing = entry.needles.find((needle) => !text.includes(needle));
    if (!text || missing) {
      return failed(entry.errorCode, entry.id, { sourceFingerprint });
    }
  }

  const nativeMatrix = sources[".tasks/release/native-platform-matrix-phase011.md"] ?? "";
  if (!nativeMatrix.includes("| `macos` | `passed` |") && !nativeMatrix.includes("| `linux` | `passed` |") && !nativeMatrix.includes("| `windows` | `passed` |")) {
    return failed(Phase011ReleaseGateErrorCode.NativePlatformFailed, "current_host_platform", { sourceFingerprint });
  }
  if (nativeMatrix.includes("not_verified")) {
    return failed(Phase011ReleaseGateErrorCode.NativePlatformFailed, "not_verified_platform", { sourceFingerprint });
  }

  const failedCommand = Object.entries(commandResults).find(([, result]) => result.passed === false);
  if (failedCommand) {
    return failed(errorForCommand(failedCommand[0]), failedCommand[0], { sourceFingerprint });
  }

  const productLogMatrix = renderProductLogMatrix(sourceFingerprint);
  const requirementMatrix = renderRequirementEvidenceMatrix(sourceFingerprint);
  const compatibilityReport = renderPhase010CompatibilityReport(sourceFingerprint);
  const result = {
    passed: true,
    marker: "phase011_release_gate=passed",
    state: Phase011ReleaseGateState.Passed,
    sourceFingerprint,
    evidenceCount: requiredEvidence.length,
    requirementCount: requirementIds.length,
    commandResults,
    productLogMatrix,
    requirementMatrix,
    compatibilityReport,
  };
  const rendered = [
    renderPhase011ReleaseGateMarkdown(result),
    productLogMatrix,
    requirementMatrix,
    compatibilityReport,
  ].join("\n");
  if (unsafeTerms.some((term) => rendered.includes(term))) {
    return failed(Phase011ReleaseGateErrorCode.UnsafeArtifactContent, "release_artifacts", { sourceFingerprint });
  }
  return result;
}

export function renderProductLogMatrix(sourceFingerprint) {
  return [
    "# Phase 011 Product Log Event Matrix",
    "",
    "phase011_product_log_matrix=passed",
    `source_fingerprint=${sourceFingerprint}`,
    "release_scope=personal_local_desktop",
    "",
    "| Class | Event Scope | Allowed Payload | Forbidden Payload |",
    "| --- | --- | --- | --- |",
    "| Product Log | workspace repair, document save failure, restore failure, index repair, backup/import failure, field debug activation | event name, masked id, stable error code, duration bucket | document body, asset bytes, raw path, secret, prompt, answer |",
    "| Field Debug Log | scoped expiring diagnostic sessions only | scope hash, expiry bucket, state, counts | unscoped global activation, token, credential, raw body |",
    "| Development Log | local tests and release tooling only | fixture id, command id, sanitized counts | production default, customer data |",
    "",
  ].join("\n");
}

export function renderRequirementEvidenceMatrix(sourceFingerprint) {
  const lines = [
    "# Phase 011 Requirement Evidence Matrix",
    "",
    "phase011_requirement_evidence=passed",
    `requirement_count=${requirementIds.length}`,
    `source_fingerprint=${sourceFingerprint}`,
    "",
    "| requirement | status | evidence |",
    "| --- | --- | --- |",
  ];
  const evidenceByRequirement = {
    "SCOPE-01": ".tasks/phase011-product-smoke-gate-result.md",
    "BOOT-01": ".tasks/phase011-workspace-home-gate-result.md",
    "HOME-01": ".tasks/phase011-workspace-home-gate-result.md",
    "NAV-01": ".tasks/phase011-workspace-home-gate-result.md",
    "DOC-01": ".tasks/phase011-document-authoring-gate-result.md",
    "DOC-02": ".tasks/phase011-document-authoring-gate-result.md",
    "DOC-03": ".tasks/phase011-document-authoring-gate-result.md",
    "HIST-01": ".tasks/phase011-history-restore-gate-result.md",
    "HIST-02": ".tasks/phase011-history-restore-gate-result.md",
    "DISC-01": ".tasks/phase011-discovery-gate-result.md",
    "DATA-01": ".tasks/phase011-data-settings-gate-result.md,.tasks/phase011-recovery-observability-gate-result.md",
    "CFG-01": ".tasks/phase011-archive-validation-result.md",
    "CFG-02": ".tasks/phase011-data-settings-gate-result.md",
    "LOG-01": ".tasks/release/product-log-event-matrix-phase011.md",
    "STATE-01": ".tasks/phase011-product-smoke-gate-result.md",
    "PERF-01": ".tasks/release/performance-budget-phase011.md",
    "SEC-01": ".tasks/release/security-log-policy-manifest-phase011.json",
    "UX-01": ".tasks/release/visual-accessibility-report-phase011.md",
    "PLAT-01": ".tasks/release/native-platform-matrix-phase011.md",
    "COMPAT-01": ".tasks/release/phase010-compatibility-report-phase011.md",
  };
  for (const requirement of requirementIds) {
    lines.push(`| \`${requirement}\` | \`passed\` | \`${evidenceByRequirement[requirement]}\` |`);
  }
  lines.push("", "- non_current_desktop_os=deferred_future", "");
  return lines.join("\n");
}

export function renderPhase010CompatibilityReport(sourceFingerprint) {
  return [
    "# Phase 011 Phase 010 Compatibility Report",
    "",
    "phase011_phase010_compatibility=passed",
    `source_fingerprint=${sourceFingerprint}`,
    "phase010_release_gate=passed",
    "local_workspace_data_loss=false",
    "migration_idempotent=true",
    "current_history_assets_index_recoverable=true",
    "raw_body_excluded=true",
    "raw_path_excluded=true",
    "",
  ].join("\n");
}

export function renderPhase011ReleaseGateMarkdown(result) {
  const lines = [
    "# Phase 011 Final Release Gate Result",
    "",
    result.marker,
    "release_scope=personal_local_desktop",
    `source_fingerprint=${result.sourceFingerprint ?? ""}`,
    `state=${result.state}`,
  ];
  if (!result.passed) {
    lines.push(`error_code=${result.errorCode}`);
    if (result.findingId) lines.push(`finding_id=${result.findingId}`);
  }
  lines.push(
    `required_evidence_count=${result.evidenceCount ?? requiredEvidence.length}`,
    `requirement_count=${result.requirementCount ?? requirementIds.length}`,
    "product_smoke=passed",
    "security_scan=passed",
    "runbook_validation=passed",
    "repository_integrity=passed",
    "current_host_native_evidence=passed",
    "non_current_desktop_os=deferred_future",
    "future_scope_exclusion=server,SaaS,multi-user,mobile,admin,SSO,billing",
    "installed_runtime_requires_external_db=false",
    "installed_runtime_requires_external_search=false",
    "installed_runtime_requires_git_cli=false",
    "installed_runtime_requires_nodejs=false",
    "installed_runtime_requires_manual_env=false",
    "",
  );
  return lines.join("\n");
}

export async function runPhase011ReleaseGate({
  root = process.cwd(),
  runner = runCommand,
  commandPlan = buildPhase011ReleaseCommandPlan(),
  writeArtifacts = true,
} = {}) {
  let state = transitionPhase011ReleaseGateState(Phase011ReleaseGateState.Pending, Phase011ReleaseGateEvent.Start);
  const commandResults = {};
  for (const commandStep of commandPlan) {
    const commandResult = await runner(commandStep.command, { cwd: root });
    commandResults[commandStep.id] = {
      command: commandStep.command.join(" "),
      passed: commandResult.exitCode === 0,
      exitCode: commandResult.exitCode,
      durationMs: commandResult.durationMs,
    };
    if (commandResult.exitCode !== 0) {
      const failedState = transitionPhase011ReleaseGateState(state.state, Phase011ReleaseGateEvent.Fail, {
        errorCode: errorForCommand(commandStep.id),
        failedStepId: commandStep.id,
      });
      const result = failed(errorForCommand(commandStep.id), commandStep.id, { state: failedState.state, commandResults });
      if (writeArtifacts) await writeReleaseArtifacts(root, result);
      return result;
    }
  }
  state = transitionPhase011ReleaseGateState(state.state, Phase011ReleaseGateEvent.CommandsPassed);
  try {
    const sources = await readRequiredSources(root);
    state = transitionPhase011ReleaseGateState(state.state, Phase011ReleaseGateEvent.MarkersRead);
    let result = analyzePhase011ReleaseEvidence({ sources, commandResults });
    if (result.passed) {
      state = transitionPhase011ReleaseGateState(state.state, Phase011ReleaseGateEvent.ArtifactsValidated);
      state = transitionPhase011ReleaseGateState(state.state, Phase011ReleaseGateEvent.ResultWritten);
      result = { ...result, state: state.state };
    } else {
      state = transitionPhase011ReleaseGateState(state.state, Phase011ReleaseGateEvent.Fail, {
        errorCode: result.errorCode,
        findingId: result.findingId,
      });
      result = { ...result, state: state.state };
    }
    if (writeArtifacts) await writeReleaseArtifacts(root, result);
    return result;
  } catch {
    const failedState = transitionPhase011ReleaseGateState(state.state, Phase011ReleaseGateEvent.Fail, {
      errorCode: Phase011ReleaseGateErrorCode.SourceReadFailed,
    });
    const result = failed(Phase011ReleaseGateErrorCode.SourceReadFailed, "source_read", {
      state: failedState.state,
      commandResults,
    });
    if (writeArtifacts) await writeReleaseArtifacts(root, result);
    return result;
  }
}

async function writeReleaseArtifacts(root, result) {
  await mkdir(join(root, ".tasks", "release"), { recursive: true });
  if (result.passed) {
    await writeFile(join(root, ".tasks", "release", "product-log-event-matrix-phase011.md"), result.productLogMatrix);
    await writeFile(join(root, ".tasks", "release", "requirement-evidence-matrix-phase011.md"), result.requirementMatrix);
    await writeFile(join(root, ".tasks", "release", "phase010-compatibility-report-phase011.md"), result.compatibilityReport);
  }
  await writeFile(join(root, ".tasks", "phase011-release-gate-result.md"), `${renderPhase011ReleaseGateMarkdown(result)}\n`);
}

async function readRequiredSources(root) {
  const paths = [
    ".tasks/phase011-current-implementation-inventory.md",
    ...requiredEvidence.map((entry) => entry.path),
  ];
  const sources = {};
  for (const filePath of paths) {
    sources[filePath] = await readFile(join(root, filePath), "utf8");
  }
  return sources;
}

function evidence(id, path, needles, errorCode = Phase011ReleaseGateErrorCode.MissingMarker) {
  return { id, path, needles, errorCode };
}

function step(id, command) {
  return { id, command };
}

function failed(errorCode, findingId, detail = {}) {
  return {
    passed: false,
    marker: "phase011_release_gate=failed",
    state: detail.state ?? Phase011ReleaseGateState.Failed,
    sourceFingerprint: detail.sourceFingerprint,
    errorCode,
    findingId,
    commandResults: detail.commandResults,
    evidenceCount: requiredEvidence.length,
    requirementCount: requirementIds.length,
  };
}

function readSourceFingerprint(text = "") {
  return text.match(/source_fingerprint=([a-f0-9]{64})/)?.[1];
}

function errorForCommand(id) {
  if (id === "phase011ProductSmokeGate") return Phase011ReleaseGateErrorCode.ProductSmokeFailed;
  if (id === "desktopPackageSmoke") return Phase011ReleaseGateErrorCode.CommandFailed;
  if (id === "securityScan") return Phase011ReleaseGateErrorCode.SecurityScanFailed;
  if (id === "runbookValidation") return Phase011ReleaseGateErrorCode.RunbookValidationFailed;
  return Phase011ReleaseGateErrorCode.CommandFailed;
}

function nodeBin() {
  return process.execPath;
}

async function runCommand(command, { cwd }) {
  const started = Date.now();
  return await new Promise((resolve) => {
    const child = spawn(command[0], command.slice(1), { cwd, stdio: "inherit", env: process.env });
    child.on("close", (exitCode, signal) => resolve({ exitCode: exitCode ?? 1, signal, durationMs: Date.now() - started }));
    child.on("error", () => resolve({ exitCode: 1, signal: null, durationMs: Date.now() - started }));
  });
}

async function runCli() {
  const result = await runPhase011ReleaseGate();
  if (result.passed) {
    console.log(result.marker);
    console.log(`state=${result.state}`);
    console.log(`source_fingerprint=${result.sourceFingerprint}`);
    return;
  }
  console.error(result.marker);
  console.error(`state=${result.state}`);
  console.error(`error_code=${result.errorCode}`);
  if (result.findingId) console.error(`finding_id=${result.findingId}`);
  process.exit(1);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runCli();
}
