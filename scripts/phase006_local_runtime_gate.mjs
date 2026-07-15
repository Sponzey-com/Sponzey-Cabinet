import { readFile, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

export const LocalRuntimeGateState = Object.freeze({
  Pending: "Pending",
  ReadingSources: "ReadingSources",
  ValidatingEvidence: "ValidatingEvidence",
  WritingReport: "WritingReport",
  Passed: "Passed",
  Failed: "Failed",
});

export const LocalRuntimeGateEvent = Object.freeze({
  Start: "Start",
  SourcesLoaded: "SourcesLoaded",
  EvidenceValidated: "EvidenceValidated",
  ReportWritten: "ReportWritten",
  Fail: "Fail",
});

export const LocalRuntimeGateErrorCode = Object.freeze({
  RequiredEvidenceMissing: "PHASE006_LOCAL_RUNTIME_REQUIRED_EVIDENCE_MISSING",
  ForbiddenRunbookText: "PHASE006_LOCAL_RUNTIME_FORBIDDEN_RUNBOOK_TEXT",
  SourceReadFailed: "PHASE006_LOCAL_RUNTIME_SOURCE_READ_FAILED",
  InvalidTransition: "PHASE006_LOCAL_RUNTIME_INVALID_TRANSITION",
});

const requiredTargets = Object.freeze([
  target("phase006_plan_validation_prerequisite", "Phase 006 plan validation prerequisite", {
    requiredFiles: [".tasks/phase006-plan-validation-result.md"],
    evidence: ["phase006_plan_validation=passed"],
  }),
  target("local_desktop_runbook", "Local desktop clean install and recovery runbook", {
    requiredFiles: [".tasks/release/local-desktop-runbook.md"],
    evidence: [
      "clean install",
      "startup repair",
      "index rebuild",
      "read-only recovery",
      "no external DB",
      "no external search server",
      "no Git CLI",
      "no Node.js",
      "no manual env",
      "no server URL",
      "Product Log",
      "Field Debug Log",
      "Development Log",
    ],
    forbiddenText: [
      "requires external DB",
      "requires external search server",
      "requires Git CLI",
      "requires Node.js",
      "requires manual env",
      "requires server URL",
    ],
  }),
  target("first_run_state_machine", "First-run state machine and initializer contract", {
    requiredFiles: [
      "crates/cabinet-core/src/first_run.rs",
      "crates/cabinet-core/tests/first_run_tests.rs",
      "crates/cabinet-core/tests/first_run_initializer_tests.rs",
    ],
    evidence: [
      "FirstRunState",
      "FirstRunEvent",
      "FirstRunInitializer",
      "FirstRunPlan",
      "FirstRunStore",
      "transition_first_run",
      "FirstRunProductEvent",
      "first_run_plan_includes_all_local_store_directories",
      "first_run_transitions_to_completed_through_explicit_events",
      "first_run_rejects_invalid_transition",
      "first_run_initializer_completes_clean_profile",
      "first_run_initializer_is_idempotent_for_existing_profile",
      "first_run_initializer_returns_retryable_failed_outcome_when_store_creation_fails",
    ],
  }),
  target("local_setup_health_contract", "Local setup health checker contract", {
    requiredFiles: [
      "crates/cabinet-adapters/src/local_setup_health.rs",
      "crates/cabinet-adapters/tests/local_setup_health_checker_tests.rs",
    ],
    evidence: [
      "LocalSetupHealthChecker",
      "LocalSetupHealthStatus",
      "LocalSetupHealthRole",
      "LocalSetupHealthIssueKind",
      "MissingFirstRunMarker",
      "local_setup_health_checker_reports_healthy_first_run_profile",
      "local_setup_health_checker_reports_missing_required_directory",
      "local_setup_health_checker_reports_path_that_is_not_directory",
      "local_setup_health_checker_reports_missing_first_run_marker",
    ],
  }),
  target("clean_install_smoke", "clean install without external services", {
    requiredFiles: ["crates/cabinet-platform/tests/clean_install_smoke.rs"],
    evidence: [
      "clean_install_smoke_initializes_local_profile_once_without_external_services",
      "run_clean_install_smoke",
      "created_directories",
    ],
  }),
  target("startup_repair_smoke", "startup repair and corrupted index rebuild", {
    requiredFiles: ["crates/cabinet-platform/tests/startup_repair_smoke.rs"],
    evidence: [
      "startup_repair_smoke_rebuilds_corrupted_indexes_without_losing_current_workspace_data",
      "startup_repair_state_machine_rejects_invalid_transition",
      "run_startup_repair_smoke",
      "corrupted_index_rebuilt",
      "product_log_sensitive_data_absent",
    ],
  }),
]);

export function transitionLocalRuntimeGateState(currentState, event, detail = {}) {
  if (currentState === LocalRuntimeGateState.Pending && event === LocalRuntimeGateEvent.Start) {
    return { state: LocalRuntimeGateState.ReadingSources };
  }
  if (
    currentState === LocalRuntimeGateState.ReadingSources &&
    event === LocalRuntimeGateEvent.SourcesLoaded
  ) {
    return { state: LocalRuntimeGateState.ValidatingEvidence };
  }
  if (
    currentState === LocalRuntimeGateState.ValidatingEvidence &&
    event === LocalRuntimeGateEvent.EvidenceValidated
  ) {
    return { state: LocalRuntimeGateState.WritingReport };
  }
  if (
    currentState === LocalRuntimeGateState.WritingReport &&
    event === LocalRuntimeGateEvent.ReportWritten
  ) {
    return { state: LocalRuntimeGateState.Passed };
  }
  if (
    [
      LocalRuntimeGateState.ReadingSources,
      LocalRuntimeGateState.ValidatingEvidence,
      LocalRuntimeGateState.WritingReport,
    ].includes(currentState) &&
    event === LocalRuntimeGateEvent.Fail
  ) {
    return {
      state: LocalRuntimeGateState.Failed,
      errorCode: detail.errorCode ?? LocalRuntimeGateErrorCode.RequiredEvidenceMissing,
      findingId: detail.findingId,
    };
  }
  return {
    state: LocalRuntimeGateState.Failed,
    errorCode: LocalRuntimeGateErrorCode.InvalidTransition,
  };
}

export function analyzeLocalRuntimeEvidence({ sources }) {
  if (!sources || Object.keys(sources).length === 0) {
    return failedResult({
      errorCode: LocalRuntimeGateErrorCode.SourceReadFailed,
      missingEvidence: [{ targetId: "source_set", missing: ["sources"] }],
    });
  }

  const targetResults = requiredTargets.map((entry) => analyzeTarget(entry, sources));
  const forbidden = targetResults.find((entry) => entry.forbiddenFound.length > 0);
  if (forbidden) {
    return failedResult({
      errorCode: LocalRuntimeGateErrorCode.ForbiddenRunbookText,
      missingEvidence: [
        {
          targetId: forbidden.id,
          missing: forbidden.forbiddenFound,
        },
      ],
      targetResults,
    });
  }

  const missingEvidence = targetResults
    .filter((entry) => entry.missing.length > 0)
    .map((entry) => ({ targetId: entry.id, missing: entry.missing }));
  if (missingEvidence.length > 0) {
    return failedResult({
      errorCode: LocalRuntimeGateErrorCode.RequiredEvidenceMissing,
      missingEvidence,
      targetResults,
    });
  }

  return {
    passed: true,
    marker: "phase006_local_runtime_gate=passed",
    state: LocalRuntimeGateState.Passed,
    summary: {
      requiredTargets: requiredTargets.length,
      missingRequiredEvidence: 0,
    },
    targetResults,
    missingEvidence: [],
  };
}

export function renderLocalRuntimeGateMarkdown(result) {
  const lines = [
    "# Phase 006 Local Runtime Gate Result",
    "",
    result.marker,
    "",
    "- phase: `Phase 006`",
    "- gate: `Local Desktop Runtime and First-Run Product Contract`",
    `- status: \`${result.passed ? "passed" : "failed"}\``,
    `- state: \`${result.state}\``,
  ];
  if (!result.passed) {
    lines.push(`- error code: \`${result.errorCode}\``);
  }
  lines.push(
    `- required targets: \`${result.summary.requiredTargets}\``,
    `- missing required evidence: \`${result.summary.missingRequiredEvidence}\``,
    "",
    "## Evidence",
    "",
    "| Target | Status | Description |",
    "| --- | --- | --- |",
  );
  for (const targetResult of result.targetResults ?? []) {
    lines.push(
      `| \`${targetResult.id}\` | \`${targetResult.status}\` | ${targetResult.description} |`,
    );
  }
  lines.push(
    "",
    "## Commands",
    "",
    "- `npm run run:phase006-local-runtime-gate-tests`",
    "- `npm run run:phase006-local-runtime-gate`",
    "- `cargo test -p cabinet-core first_run`",
    "- `cargo test -p cabinet-adapters local_setup_health`",
    "- `cargo test -p cabinet-platform clean_install_smoke`",
    "- `cargo test -p cabinet-platform startup_repair_smoke`",
    "- `npm run run:security-log-scanner`",
    "",
    "## Sensitive Data Exclusion",
    "",
    "This artifact records target ids, marker status, counts, and stable error codes only. It does not record document body, asset content, AI prompt, AI answer, provider key, token, credential, or personal absolute path.",
    "",
    "## Follow-Up Limitation",
    "",
    "React/Tauri setup health UI and explicit read-only recovery action mapping remain for a later Phase 006 task.",
    "",
  );
  return lines.join("\n");
}

export async function runLocalRuntimeGate({ root = process.cwd() } = {}) {
  let state = transitionLocalRuntimeGateState(
    LocalRuntimeGateState.Pending,
    LocalRuntimeGateEvent.Start,
  );
  try {
    const sources = {};
    for (const filePath of collectRequiredFiles()) {
      sources[filePath] = await readFile(`${root}/${filePath}`, "utf8");
    }
    state = transitionLocalRuntimeGateState(state.state, LocalRuntimeGateEvent.SourcesLoaded);

    const result = analyzeLocalRuntimeEvidence({ sources });
    if (!result.passed) {
      state = transitionLocalRuntimeGateState(state.state, LocalRuntimeGateEvent.Fail, {
        errorCode: result.errorCode,
        findingId: result.missingEvidence[0]?.targetId,
      });
      return { ...result, state: state.state };
    }
    state = transitionLocalRuntimeGateState(state.state, LocalRuntimeGateEvent.EvidenceValidated);
    state = transitionLocalRuntimeGateState(state.state, LocalRuntimeGateEvent.ReportWritten);
    return { ...result, state: state.state };
  } catch {
    state = transitionLocalRuntimeGateState(state.state, LocalRuntimeGateEvent.Fail, {
      errorCode: LocalRuntimeGateErrorCode.SourceReadFailed,
    });
    return failedResult({
      errorCode: state.errorCode,
      state: state.state,
      missingEvidence: [{ targetId: "source_read", missing: ["required source file"] }],
    });
  }
}

function analyzeTarget(entry, sources) {
  const texts = entry.requiredFiles.map((filePath) => sources[filePath] ?? "");
  const missingFiles = entry.requiredFiles.filter((filePath) => !(filePath in sources));
  const missingEvidence = entry.evidence.filter(
    (needle) => !texts.some((text) => text.includes(needle)),
  );
  const forbiddenFound = entry.forbiddenText.filter((needle) =>
    texts.some((text) => text.includes(needle)),
  );
  const missing = [...missingFiles, ...missingEvidence];
  return {
    id: entry.id,
    description: entry.description,
    status: missing.length === 0 && forbiddenFound.length === 0 ? "covered" : "missing",
    missing,
    forbiddenFound,
  };
}

function failedResult({ errorCode, state = LocalRuntimeGateState.Failed, missingEvidence, targetResults = [] }) {
  return {
    passed: false,
    marker: "phase006_local_runtime_gate=failed",
    state,
    errorCode,
    summary: {
      requiredTargets: requiredTargets.length,
      missingRequiredEvidence: missingEvidence.length,
    },
    targetResults,
    missingEvidence,
  };
}

function target(id, description, { requiredFiles, evidence, forbiddenText = [] }) {
  return {
    id,
    description,
    requiredFiles,
    evidence,
    forbiddenText,
  };
}

function collectRequiredFiles() {
  return [...new Set(requiredTargets.flatMap((entry) => entry.requiredFiles))];
}

async function runCli() {
  const result = await runLocalRuntimeGate();
  const markdown = renderLocalRuntimeGateMarkdown(result);
  await writeFile(".tasks/phase006-local-runtime-gate-result.md", markdown);
  if (result.passed) {
    console.log(result.marker);
    console.log(`gate_state=${result.state}`);
    console.log(`required_targets=${result.summary.requiredTargets}`);
    return;
  }
  console.error(result.marker);
  console.error(`gate_state=${result.state}`);
  console.error(`error_code=${result.errorCode}`);
  process.exit(1);
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runCli();
}
