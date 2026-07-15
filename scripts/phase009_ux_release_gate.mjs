import { spawnSync } from "node:child_process";
import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const Phase009UxReleaseGateState = Object.freeze({
  Pending: "Pending",
  ReadingMarkers: "ReadingMarkers",
  ValidatingArtifacts: "ValidatingArtifacts",
  RunningSecurityScan: "RunningSecurityScan",
  RunningRunbookValidation: "RunningRunbookValidation",
  RunningProductSmoke: "RunningProductSmoke",
  WritingResult: "WritingResult",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase009UxReleaseGateEvent = Object.freeze({
  Start: "Start",
  MarkersRead: "MarkersRead",
  ArtifactsValidated: "ArtifactsValidated",
  SecurityScanPassed: "SecurityScanPassed",
  RunbookValidationPassed: "RunbookValidationPassed",
  ProductSmokePassed: "ProductSmokePassed",
  ResultWritten: "ResultWritten",
  Fail: "Fail",
});

export const Phase009UxReleaseGateErrorCode = Object.freeze({
  MissingMarker: "PHASE009_MISSING_MARKER",
  InvalidScope: "PHASE009_INVALID_SCOPE",
  PerformanceBudgetFailed: "PHASE009_PERFORMANCE_BUDGET_FAILED",
  SecurityLogScanFailed: "PHASE009_SECURITY_LOG_SCAN_FAILED",
  RunbookMissing: "PHASE009_RUNBOOK_MISSING",
  ProductSmokeFailed: "PHASE009_PRODUCT_SMOKE_FAILED",
  SourceReadFailed: "PHASE009_SOURCE_READ_FAILED",
  InvalidTransition: "PHASE009_INVALID_TRANSITION",
});

const requiredEvidence = Object.freeze([
  evidence("phase009_current_inventory", ".tasks/phase009-current-implementation-inventory.md", [
    "phase009_current_inventory=passed",
    "product_scope: `personal_local_desktop`",
  ]),
  evidence("phase009_plan_validation", ".tasks/phase009-plan-validation-result.md", [
    "phase009_plan_validation=passed",
    "scope lock: personal local desktop only",
    "completion evidence: marker artifacts only",
  ]),
  evidence("phase009_desktop_launch_gate", ".tasks/phase009-desktop-launch-gate-result.md", [
    "phase009_desktop_launch_gate=passed",
    "product app command: `npm run run:desktop-app`",
    "DESKTOP_BLANK_SCREEN_DETECTED",
  ]),
  evidence("phase009_command_runtime_gate", ".tasks/phase009-command-runtime-gate-result.md", [
    "phase009_command_runtime_gate=passed",
    "command_count=17",
    "LocalDesktopCommandState",
  ]),
  evidence("phase009_document_authoring_gate", ".tasks/phase009-document-authoring-gate-result.md", [
    "phase009_document_authoring_gate=passed",
    "DocumentEditorState",
    "p95 budget impact",
  ]),
  evidence("phase009_discovery_assets_gate", ".tasks/phase009-discovery-assets-gate-result.md", [
    "phase009_discovery_assets_gate=passed",
    "search result, backlink group, unresolved link group, graph node/edge, and asset metadata panel",
    "p95 budget impact",
  ]),
  evidence("phase009_recovery_backup_ux_gate", ".tasks/phase009-recovery-backup-ux-gate-result.md", [
    "phase009_recovery_backup_ux_gate=passed",
    "backup summary, import preview, restore confirmation, and recovery action panels",
    "runbook follow-up",
  ]),
  evidence("phase009_performance_budget", ".tasks/release/performance-budget-phase009.md", [
    "phase009_performance_budget=passed",
    "current_document_read",
    "history_list",
    "search",
    "p95 target",
    "300ms",
  ], Phase009UxReleaseGateErrorCode.PerformanceBudgetFailed),
  evidence("phase009_product_log_matrix", ".tasks/release/product-log-event-matrix-phase009.md", [
    "phase009_product_log_matrix=passed",
    "Product Log",
    "Field Debug Log",
    "Development Log",
    "document.save.completed",
    "backup.created",
  ]),
  evidence("phase009_security_manifest", ".tasks/release/security-log-policy-manifest-phase009.json", [
    "phase009_security_log_manifest=passed",
    "__CABINET_DENY_PROVIDER_KEY__",
    "__CABINET_DENY_TOKEN__",
    "__CABINET_DENY_CREDENTIAL__",
    "__CABINET_DENY_DOCUMENT_BODY__",
    "__CABINET_DENY_ASSET_CONTENT__",
    "__CABINET_DENY_LOCAL_PATH__",
  ]),
  evidence("phase009_local_desktop_runbook", ".tasks/release/local-desktop-runbook-phase009.md", [
    "phase009_runbook=passed",
    "Product Launch",
    "Blank Screen Recovery",
    "run_desktop_shell.sh is an internal shell smoke path",
    "Field Debug requires explicit scope and expiry",
  ], Phase009UxReleaseGateErrorCode.RunbookMissing),
  evidence("phase009_runbook_manifest", ".tasks/release/runbook-validation-manifest-phase009.json", [
    "phase009_local_desktop",
  ], Phase009UxReleaseGateErrorCode.RunbookMissing),
  evidence("phase009_release_tooling", "package.json", [
    "run:phase009-ux-release-gate-tests",
    "run:phase009-ux-release-gate",
    "run:phase009-release-evidence",
  ]),
]);

const commandOrder = Object.freeze([
  command(
    "security",
    "node scripts/security_log_scanner.mjs .tasks/release/security-log-policy-manifest-phase009.json",
    [process.execPath, ["scripts/security_log_scanner.mjs", ".tasks/release/security-log-policy-manifest-phase009.json"]],
    Phase009UxReleaseGateEvent.SecurityScanPassed,
    Phase009UxReleaseGateErrorCode.SecurityLogScanFailed,
  ),
  command(
    "runbook",
    "node scripts/runbook_validator.mjs .tasks/release/runbook-validation-manifest-phase009.json",
    [process.execPath, ["scripts/runbook_validator.mjs", ".tasks/release/runbook-validation-manifest-phase009.json"]],
    Phase009UxReleaseGateEvent.RunbookValidationPassed,
    Phase009UxReleaseGateErrorCode.RunbookMissing,
  ),
  command(
    "productSmoke",
    "npm run run:desktop-dist-browser-smoke",
    ["npm", ["run", "run:desktop-dist-browser-smoke"]],
    Phase009UxReleaseGateEvent.ProductSmokePassed,
    Phase009UxReleaseGateErrorCode.ProductSmokeFailed,
  ),
]);

export function transitionPhase009UxReleaseGateState(currentState, event, detail = {}) {
  if (currentState === Phase009UxReleaseGateState.Pending && event === Phase009UxReleaseGateEvent.Start) {
    return { state: Phase009UxReleaseGateState.ReadingMarkers };
  }
  if (
    currentState === Phase009UxReleaseGateState.ReadingMarkers &&
    event === Phase009UxReleaseGateEvent.MarkersRead
  ) {
    return { state: Phase009UxReleaseGateState.ValidatingArtifacts };
  }
  if (
    currentState === Phase009UxReleaseGateState.ValidatingArtifacts &&
    event === Phase009UxReleaseGateEvent.ArtifactsValidated
  ) {
    return { state: Phase009UxReleaseGateState.RunningSecurityScan };
  }
  if (
    currentState === Phase009UxReleaseGateState.RunningSecurityScan &&
    event === Phase009UxReleaseGateEvent.SecurityScanPassed
  ) {
    return { state: Phase009UxReleaseGateState.RunningRunbookValidation };
  }
  if (
    currentState === Phase009UxReleaseGateState.RunningRunbookValidation &&
    event === Phase009UxReleaseGateEvent.RunbookValidationPassed
  ) {
    return { state: Phase009UxReleaseGateState.RunningProductSmoke };
  }
  if (
    currentState === Phase009UxReleaseGateState.RunningProductSmoke &&
    event === Phase009UxReleaseGateEvent.ProductSmokePassed
  ) {
    return { state: Phase009UxReleaseGateState.WritingResult };
  }
  if (
    currentState === Phase009UxReleaseGateState.WritingResult &&
    event === Phase009UxReleaseGateEvent.ResultWritten
  ) {
    return { state: Phase009UxReleaseGateState.Passed };
  }
  if (
    [
      Phase009UxReleaseGateState.ReadingMarkers,
      Phase009UxReleaseGateState.ValidatingArtifacts,
      Phase009UxReleaseGateState.RunningSecurityScan,
      Phase009UxReleaseGateState.RunningRunbookValidation,
      Phase009UxReleaseGateState.RunningProductSmoke,
      Phase009UxReleaseGateState.WritingResult,
    ].includes(currentState) &&
    event === Phase009UxReleaseGateEvent.Fail
  ) {
    return {
      state: Phase009UxReleaseGateState.Failed,
      errorCode: detail.errorCode ?? Phase009UxReleaseGateErrorCode.MissingMarker,
      findingId: detail.findingId,
      failedCommandExitCode: detail.failedCommandExitCode,
    };
  }
  return {
    state: Phase009UxReleaseGateState.Failed,
    errorCode: Phase009UxReleaseGateErrorCode.InvalidTransition,
  };
}

export function evaluatePhase009UxReleaseGate({ sources, commandResults }) {
  let state = transitionPhase009UxReleaseGateState(
    Phase009UxReleaseGateState.Pending,
    Phase009UxReleaseGateEvent.Start,
  );

  if (!sources || Object.keys(sources).length === 0) {
    return failedResult(
      transitionPhase009UxReleaseGateState(state.state, Phase009UxReleaseGateEvent.Fail, {
        errorCode: Phase009UxReleaseGateErrorCode.SourceReadFailed,
        findingId: "source_set",
      }),
      commandResults,
    );
  }

  const missingEvidence = findMissingEvidence(sources);
  if (missingEvidence) {
    return failedResult(
      transitionPhase009UxReleaseGateState(state.state, Phase009UxReleaseGateEvent.Fail, {
        errorCode: missingEvidence.errorCode,
        findingId: missingEvidence.id,
      }),
      commandResults,
    );
  }

  state = transitionPhase009UxReleaseGateState(state.state, Phase009UxReleaseGateEvent.MarkersRead);
  const scopeFinding = findInvalidScope(sources);
  if (scopeFinding) {
    return failedResult(
      transitionPhase009UxReleaseGateState(state.state, Phase009UxReleaseGateEvent.Fail, {
        errorCode: Phase009UxReleaseGateErrorCode.InvalidScope,
        findingId: scopeFinding,
      }),
      commandResults,
    );
  }

  state = transitionPhase009UxReleaseGateState(state.state, Phase009UxReleaseGateEvent.ArtifactsValidated);
  for (const commandSpec of commandOrder) {
    const result = commandResults?.[commandSpec.key];
    if (!result?.passed) {
      return failedResult(
        transitionPhase009UxReleaseGateState(state.state, Phase009UxReleaseGateEvent.Fail, {
          errorCode: commandSpec.errorCode,
          findingId: commandSpec.key,
          failedCommandExitCode: result?.exitCode,
        }),
        commandResults,
      );
    }
    state = transitionPhase009UxReleaseGateState(state.state, commandSpec.event);
  }
  state = transitionPhase009UxReleaseGateState(state.state, Phase009UxReleaseGateEvent.ResultWritten);

  return {
    passed: true,
    state: state.state,
    evidenceCount: requiredEvidence.length,
    commandCount: commandOrder.length,
    evidenceResults: requiredEvidence.map((entry) => ({
      id: entry.id,
      filePath: entry.filePath,
      status: "covered",
    })),
    commandResults,
  };
}

export function renderPhase009UxReleaseGateArtifact(result) {
  const status = result.passed ? "passed" : "failed";
  const marker = result.passed ? "phase009_ux_release_gate=passed" : "phase009_ux_release_gate=failed";
  const lines = [
    "# Phase 009 UX Release Gate Result",
    "",
    marker,
    `validation_state=${result.state}`,
  ];

  if (result.passed) {
    lines.push(
      `validated_evidence_count=${result.evidenceCount}`,
      `validation_command_count=${result.commandCount}`,
    );
  } else {
    lines.push(
      `error_code=${result.errorCode}`,
      result.findingId ? `finding_id=${result.findingId}` : undefined,
      result.failedCommandExitCode === undefined
        ? undefined
        : `failed_command_exit_code=${result.failedCommandExitCode}`,
    );
  }

  lines.push(
    "",
    "- phase: `Phase 009`",
    "- gate: `Final User-Visible Local Desktop Product UX Release`",
    `- status: \`${status}\``,
    "- release boundary: personal local desktop installable knowledge management app only. Server hosting, SaaS, multi-user administration, tenant administration, billing, SSO, real-time collaboration, mobile implementation, OS signing, notarization, and app store distribution remain outside this gate.",
    "- changed layers: `release-tooling`, `marker-artifacts`, `desktop-product-smoke`, `security-log-policy`, `runbook-validation`.",
    "- p95 300ms paths: documented and validated by release evidence for current document, history list, search, backlink, graph projection, asset metadata, and backup/restore status queries.",
    "- completion evidence: marker artifacts, release evidence artifacts, security scan, runbook validation, and desktop product smoke only. Task checkbox text is not release evidence.",
    "",
    "## Required Evidence",
    "",
    "| Evidence | Source | Status |",
    "| --- | --- | --- |",
    ...requiredEvidence.map((entry) => `| \`${entry.id}\` | \`${entry.filePath}\` | \`${result.passed ? "covered" : "required"}\` |`),
    "",
    "## Validation Commands",
    "",
    ...commandOrder.map((entry) => `- \`${entry.command}\``),
    "",
    "## Sensitive Data Exclusion",
    "",
    "This artifact records marker names, evidence ids, command ids, counts, states, and stable error codes only. It does not record raw document body, rendered document HTML, asset content, asset bytes, graph dump, backup package contents, AI prompt, AI answer, provider key, token, credential, secret, raw local absolute path, browser stdout, or package internal file contents.",
    "",
  );

  return lines.filter((line) => line !== undefined).join("\n");
}

export async function runPhase009UxReleaseGate({ root = process.cwd() } = {}) {
  let result;
  try {
    const sources = await readRequiredSources(root);
    const missingEvidence = findMissingEvidence(sources);
    const commandResults = missingEvidence
      ? {}
      : Object.fromEntries(commandOrder.map((commandSpec) => [commandSpec.key, runCommand(commandSpec)]));
    result = evaluatePhase009UxReleaseGate({ sources, commandResults });
  } catch {
    result = failedResult({
      state: Phase009UxReleaseGateState.Failed,
      errorCode: Phase009UxReleaseGateErrorCode.SourceReadFailed,
      findingId: "phase009_release_source_read",
    }, {});
  }
  await writeFile(
    join(root, ".tasks/phase009-ux-release-gate-result.md"),
    renderPhase009UxReleaseGateArtifact(result),
  );
  return result;
}

export function collectPhase009UxReleaseGateRequiredFiles() {
  return [...new Set(requiredEvidence.map((entry) => entry.filePath))];
}

function findMissingEvidence(sources) {
  for (const entry of requiredEvidence) {
    const text = sources[entry.filePath] ?? "";
    const missingNeedle = entry.needles.find((needle) => !text.includes(needle));
    if (missingNeedle) {
      return { id: entry.id, errorCode: entry.errorCode };
    }
  }
  return undefined;
}

function findInvalidScope(sources) {
  const inventory = sources[".tasks/phase009-current-implementation-inventory.md"] ?? "";
  const planValidation = sources[".tasks/phase009-plan-validation-result.md"] ?? "";
  if (!inventory.includes("product_scope: `personal_local_desktop`")) {
    return "phase009_product_scope";
  }
  if (!planValidation.includes("scope lock: personal local desktop only")) {
    return "phase009_scope_lock";
  }
  const disallowedActiveMarkers = [
    "phase009_server_release_gate=passed",
    "phase009_saas_release_gate=passed",
    "phase009_multi_user_release_gate=passed",
  ];
  const combinedSources = Object.values(sources).join("\n");
  const disallowed = disallowedActiveMarkers.find((needle) => combinedSources.includes(needle));
  return disallowed ? "phase009_out_of_scope_marker" : undefined;
}

function failedResult(state, commandResults) {
  return {
    passed: false,
    state: state.state,
    errorCode: state.errorCode,
    findingId: state.findingId,
    failedCommandExitCode: state.failedCommandExitCode,
    evidenceCount: requiredEvidence.length,
    commandCount: commandOrder.length,
    commandResults,
  };
}

function runCommand(commandSpec) {
  const [program, args] = commandSpec.spawn;
  const completed = spawnSync(program, args, {
    cwd: process.cwd(),
    encoding: "utf8",
    stdio: "inherit",
  });
  return {
    key: commandSpec.key,
    command: commandSpec.command,
    passed: completed.status === 0,
    exitCode: completed.status ?? 1,
  };
}

function command(key, commandText, spawn, event, errorCode) {
  return { key, command: commandText, spawn, event, errorCode };
}

function evidence(
  id,
  filePath,
  needles,
  errorCode = Phase009UxReleaseGateErrorCode.MissingMarker,
) {
  return { id, filePath, needles, errorCode };
}

async function readRequiredSources(root) {
  const sources = {};
  for (const filePath of collectPhase009UxReleaseGateRequiredFiles()) {
    sources[filePath] = await readFile(join(root, filePath), "utf8");
  }
  return sources;
}

async function runCli() {
  const result = await runPhase009UxReleaseGate();
  if (result.passed) {
    console.log("phase009_ux_release_gate=passed");
    return;
  }
  console.error("phase009_ux_release_gate=failed");
  console.error(`error_code=${result.errorCode}`);
  if (result.findingId) {
    console.error(`finding_id=${result.findingId}`);
  }
  process.exit(1);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runCli();
}
