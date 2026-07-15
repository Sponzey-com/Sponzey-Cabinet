import { readFile, writeFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { pathToFileURL } from "node:url";

export const Phase008ReleaseGateState = Object.freeze({
  Pending: "Pending",
  ReadingEvidence: "ReadingEvidence",
  RunningSecurityScan: "RunningSecurityScan",
  RunningRunbookValidation: "RunningRunbookValidation",
  ValidatingPerformance: "ValidatingPerformance",
  WritingResult: "WritingResult",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase008ReleaseGateEvent = Object.freeze({
  Start: "Start",
  EvidenceRead: "EvidenceRead",
  SecurityScanPassed: "SecurityScanPassed",
  RunbookValidationPassed: "RunbookValidationPassed",
  PerformanceValidated: "PerformanceValidated",
  ResultWritten: "ResultWritten",
  Fail: "Fail",
});

export const Phase008ReleaseGateErrorCode = Object.freeze({
  RequiredEvidenceMissing: "PHASE008_RELEASE_REQUIRED_EVIDENCE_MISSING",
  SecurityScanFailed: "PHASE008_RELEASE_SECURITY_SCAN_FAILED",
  RunbookValidationFailed: "PHASE008_RELEASE_RUNBOOK_VALIDATION_FAILED",
  PerformanceBudgetMissing: "PHASE008_RELEASE_PERFORMANCE_BUDGET_MISSING",
  SourceReadFailed: "PHASE008_RELEASE_SOURCE_READ_FAILED",
  InvalidTransition: "PHASE008_RELEASE_INVALID_TRANSITION",
});

const requiredEvidence = [
  evidence("phase008_plan_validation", ".tasks/phase008-plan-validation-result.md", ["phase008_plan_validation=passed"]),
  evidence("phase008_native_bootstrap_gate", ".tasks/phase008-native-bootstrap-gate-result.md", ["phase008_native_bootstrap_gate=passed"]),
  evidence("phase008_command_bridge_gate", ".tasks/phase008-command-bridge-gate-result.md", ["phase008_command_bridge_gate=passed"]),
  evidence("phase008_document_runtime_gate", ".tasks/phase008-document-runtime-gate-result.md", ["phase008_document_runtime_gate=passed"]),
  evidence("phase008_projection_index_gate", ".tasks/phase008-projection-index-gate-result.md", ["phase008_projection_index_gate=passed"]),
  evidence("phase008_asset_lifecycle_gate", ".tasks/phase008-asset-lifecycle-gate-result.md", ["phase008_asset_lifecycle_gate=passed"]),
  evidence("phase008_recovery_backup_gate", ".tasks/phase008-recovery-backup-gate-result.md", ["phase008_recovery_backup_gate=passed"]),
  evidence("phase008_native_product_smoke_gate", ".tasks/phase008-native-product-smoke-gate-result.md", ["phase008_native_product_smoke_gate=passed"]),
  evidence("phase008_performance_budget", ".tasks/release/performance-budget-phase008.md", ["phase008_performance_budget=passed"], Phase008ReleaseGateErrorCode.PerformanceBudgetMissing),
  evidence("phase008_security_manifest", ".tasks/release/security-log-policy-manifest.json", [
    "phase008_raw_document_body_fixture",
    "phase008_asset_binary_content_fixture",
    "phase008_ai_prompt_fixture",
    "phase008_ai_answer_fixture",
    "phase008_provider_key_fixture",
    "phase008_token_fixture",
    "phase008_credential_fixture",
    "phase008_secret_fixture",
    "phase008_raw_local_absolute_path_fixture",
    "phase008_native_product_smoke_gate_result",
    "phase008_product_log_event_matrix",
    "phase008_local_desktop_runbook",
  ]),
  evidence("phase008_product_log_matrix", ".tasks/release/product-log-event-matrix.md", [
    "Product Log",
    "Field Debug Log",
    "Development Log",
    "app.start.completed",
    "workspace.ready",
    "document.save.completed",
    "backup.created",
  ]),
  evidence("phase008_local_desktop_runbook", ".tasks/release/local-desktop-runbook.md", [
    "Phase 008 Local Desktop Runbook",
    "clean install",
    "startup repair",
    "backup",
    "restore",
    "import preview",
    "p95 300ms",
  ]),
  evidence("phase008_runbook_manifest", ".tasks/release/runbook-validation-manifest.json", ["phase008_local_desktop_release"]),
  evidence("phase008_release_tooling", "package.json", [
    "run:phase008-release-gate-tests",
    "run:phase008-release-gate",
  ]),
];

const commandOrder = [
  {
    key: "security",
    event: Phase008ReleaseGateEvent.SecurityScanPassed,
    errorCode: Phase008ReleaseGateErrorCode.SecurityScanFailed,
    command: "npm run run:phase008-release-security-scan",
    spawn: ["npm", ["run", "run:phase008-release-security-scan"]],
  },
  {
    key: "runbook",
    event: Phase008ReleaseGateEvent.RunbookValidationPassed,
    errorCode: Phase008ReleaseGateErrorCode.RunbookValidationFailed,
    command: "npm run run:phase008-release-runbook-validation",
    spawn: ["npm", ["run", "run:phase008-release-runbook-validation"]],
  },
];

export function transitionPhase008ReleaseGateState(currentState, event, detail = {}) {
  if (currentState === Phase008ReleaseGateState.Pending && event === Phase008ReleaseGateEvent.Start) {
    return { state: Phase008ReleaseGateState.ReadingEvidence };
  }
  if (currentState === Phase008ReleaseGateState.ReadingEvidence && event === Phase008ReleaseGateEvent.EvidenceRead) {
    return { state: Phase008ReleaseGateState.RunningSecurityScan };
  }
  if (
    currentState === Phase008ReleaseGateState.RunningSecurityScan &&
    event === Phase008ReleaseGateEvent.SecurityScanPassed
  ) {
    return { state: Phase008ReleaseGateState.RunningRunbookValidation };
  }
  if (
    currentState === Phase008ReleaseGateState.RunningRunbookValidation &&
    event === Phase008ReleaseGateEvent.RunbookValidationPassed
  ) {
    return { state: Phase008ReleaseGateState.ValidatingPerformance };
  }
  if (
    currentState === Phase008ReleaseGateState.ValidatingPerformance &&
    event === Phase008ReleaseGateEvent.PerformanceValidated
  ) {
    return { state: Phase008ReleaseGateState.WritingResult };
  }
  if (currentState === Phase008ReleaseGateState.WritingResult && event === Phase008ReleaseGateEvent.ResultWritten) {
    return { state: Phase008ReleaseGateState.Passed };
  }
  if (
    [
      Phase008ReleaseGateState.ReadingEvidence,
      Phase008ReleaseGateState.RunningSecurityScan,
      Phase008ReleaseGateState.RunningRunbookValidation,
      Phase008ReleaseGateState.ValidatingPerformance,
      Phase008ReleaseGateState.WritingResult,
    ].includes(currentState) &&
    event === Phase008ReleaseGateEvent.Fail
  ) {
    return {
      state: Phase008ReleaseGateState.Failed,
      errorCode: detail.errorCode ?? Phase008ReleaseGateErrorCode.RequiredEvidenceMissing,
      findingId: detail.findingId,
      failedCommandExitCode: detail.failedCommandExitCode,
    };
  }
  return { state: Phase008ReleaseGateState.Failed, errorCode: Phase008ReleaseGateErrorCode.InvalidTransition };
}

export function evaluatePhase008ReleaseGate({ sources, commandResults }) {
  let state = transitionPhase008ReleaseGateState(
    Phase008ReleaseGateState.Pending,
    Phase008ReleaseGateEvent.Start,
  );
  const missingEvidence = findMissingEvidence(sources);
  if (missingEvidence) {
    state = transitionPhase008ReleaseGateState(state.state, Phase008ReleaseGateEvent.Fail, {
      errorCode: missingEvidence.errorCode,
      findingId: missingEvidence.id,
    });
    return failedResult(state, commandResults);
  }

  state = transitionPhase008ReleaseGateState(state.state, Phase008ReleaseGateEvent.EvidenceRead);
  for (const command of commandOrder) {
    const result = commandResults[command.key];
    if (!result?.passed) {
      state = transitionPhase008ReleaseGateState(state.state, Phase008ReleaseGateEvent.Fail, {
        errorCode: command.errorCode,
        findingId: command.key,
        failedCommandExitCode: result?.exitCode,
      });
      return failedResult(state, commandResults);
    }
    state = transitionPhase008ReleaseGateState(state.state, command.event);
  }
  state = transitionPhase008ReleaseGateState(state.state, Phase008ReleaseGateEvent.PerformanceValidated);
  state = transitionPhase008ReleaseGateState(state.state, Phase008ReleaseGateEvent.ResultWritten);

  return {
    passed: true,
    state: state.state,
    evidenceCount: requiredEvidence.length,
    commandCount: commandOrder.length,
  };
}

export function renderPhase008ReleaseGateArtifact(result) {
  const status = result.passed ? "passed" : "failed";
  return [
    "# Phase 008 Final Release Gate Result",
    "",
    result.passed
      ? [
          "phase008_release_gate=passed",
          `validation_state=${result.state}`,
          `validated_evidence_count=${result.evidenceCount}`,
          `validation_command_count=${result.commandCount}`,
        ].join("\n")
      : [
          "phase008_release_gate=failed",
          `validation_state=${result.state}`,
          `error_code=${result.errorCode}`,
          result.findingId ? `finding_id=${result.findingId}` : undefined,
        ].filter(Boolean).join("\n"),
    "",
    "- phase: `Phase 008`",
    "- gate: `Final Native Local Desktop Release`",
    `- status: \`${status}\``,
    "- evidence:",
    ...requiredEvidence.map((entry) => `  - \`${entry.id}\` from \`${entry.filePath}\``),
    "- validation commands:",
    ...commandOrder.map(({ command }) => `  - \`${command}\``),
    "- release boundary: personal local desktop installable knowledge management app only. Server hosting, SaaS, multi-user administration, billing, SSO administration, OS signing, notarization, and app store distribution remain outside this gate.",
    "- sensitive-data exclusion: this artifact records markers, evidence ids, command ids, counts, states, and stable error codes only. It does not record raw document body, asset content, AI prompt, AI answer, provider key, token, credential, secret, raw local absolute path, browser stdout, or package internal file contents.",
    "",
  ].join("\n");
}

function findMissingEvidence(sources) {
  if (!sources || Object.keys(sources).length === 0) {
    return { id: "source_set", errorCode: Phase008ReleaseGateErrorCode.SourceReadFailed };
  }
  for (const entry of requiredEvidence) {
    const text = sources[entry.filePath] ?? "";
    const missingNeedle = entry.needles.find((needle) => !text.includes(needle));
    if (missingNeedle) {
      return { id: entry.id, errorCode: entry.errorCode };
    }
  }
  return undefined;
}

function failedResult(state, commandResults) {
  return {
    passed: false,
    state: state.state,
    errorCode: state.errorCode,
    findingId: state.findingId,
    failedCommandExitCode: state.failedCommandExitCode,
    commandResults,
  };
}

function runCommand({ key, command, spawn }) {
  const [program, args] = spawn;
  const completed = spawnSync(program, args, {
    cwd: process.cwd(),
    encoding: "utf8",
    stdio: "inherit",
  });
  return { key, command, passed: completed.status === 0, exitCode: completed.status ?? 1 };
}

function evidence(id, filePath, needles, errorCode = Phase008ReleaseGateErrorCode.RequiredEvidenceMissing) {
  return { id, filePath, needles, errorCode };
}

async function readRequiredSources() {
  const sources = {};
  for (const entry of requiredEvidence) {
    sources[entry.filePath] = await readFile(entry.filePath, "utf8");
  }
  return sources;
}

async function runPhase008ReleaseGateCli() {
  let result;
  try {
    const sources = await readRequiredSources();
    const missingEvidence = findMissingEvidence(sources);
    const commandResults = missingEvidence
      ? {}
      : Object.fromEntries(commandOrder.map((commandSpec) => [commandSpec.key, runCommand(commandSpec)]));
    result = evaluatePhase008ReleaseGate({ sources, commandResults });
  } catch {
    const state = {
      state: Phase008ReleaseGateState.Failed,
      errorCode: Phase008ReleaseGateErrorCode.SourceReadFailed,
      findingId: "phase008_release_source_read",
    };
    result = failedResult(state, {});
  }
  await writeFile(".tasks/phase008-release-gate-result.md", renderPhase008ReleaseGateArtifact(result));
  if (result.passed) {
    console.log("phase008_release_gate=passed");
    return;
  }
  console.error("phase008_release_gate=failed");
  process.exit(1);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runPhase008ReleaseGateCli();
}
