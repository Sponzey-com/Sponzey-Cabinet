import assert from "node:assert/strict";
import test from "node:test";

import {
  LocalRuntimeGateErrorCode,
  LocalRuntimeGateEvent,
  LocalRuntimeGateState,
  analyzeLocalRuntimeEvidence,
  renderLocalRuntimeGateMarkdown,
  transitionLocalRuntimeGateState,
} from "./phase006_local_runtime_gate.mjs";

test("local runtime gate reports complete evidence as passed", () => {
  const result = analyzeLocalRuntimeEvidence({
    sources: completeSources(),
  });
  const markdown = renderLocalRuntimeGateMarkdown(result);

  assert.equal(result.passed, true);
  assert.equal(result.marker, "phase006_local_runtime_gate=passed");
  assert.equal(result.summary.missingRequiredEvidence, 0);
  assert.match(markdown, /phase006_local_runtime_gate=passed/);
  assert.match(markdown, /clean install without external services/);
  assert.match(markdown, /startup repair and corrupted index rebuild/);
  assert.doesNotMatch(markdown, /phase006-raw-document-body-should-not-log/);
});

test("local runtime gate fails when first-run state machine evidence is missing", () => {
  const sources = completeSources();
  sources["crates/cabinet-core/src/first_run.rs"] = sources[
    "crates/cabinet-core/src/first_run.rs"
  ].replace("FirstRunState", "FirstRunMode");

  const result = analyzeLocalRuntimeEvidence({ sources });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, LocalRuntimeGateErrorCode.RequiredEvidenceMissing);
  assert.equal(result.missingEvidence[0].targetId, "first_run_state_machine");
  assert.equal(result.marker, "phase006_local_runtime_gate=failed");
});

test("local runtime gate fails when runbook uses forbidden local setup dependency", () => {
  const sources = completeSources();
  sources[".tasks/release/local-desktop-runbook.md"] += "\nDefault setup requires external DB.\n";

  const result = analyzeLocalRuntimeEvidence({ sources });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, LocalRuntimeGateErrorCode.ForbiddenRunbookText);
  assert.equal(result.missingEvidence[0].targetId, "local_desktop_runbook");
});

test("local runtime gate fails when plan validation marker is missing", () => {
  const sources = completeSources();
  sources[".tasks/phase006-plan-validation-result.md"] = "phase006_plan_validation=failed";

  const result = analyzeLocalRuntimeEvidence({ sources });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, LocalRuntimeGateErrorCode.RequiredEvidenceMissing);
  assert.equal(result.missingEvidence[0].targetId, "phase006_plan_validation_prerequisite");
});

test("local runtime gate state machine exposes explicit transitions and invalid transition", () => {
  const readingSources = transitionLocalRuntimeGateState(
    LocalRuntimeGateState.Pending,
    LocalRuntimeGateEvent.Start,
  );
  const validatingEvidence = transitionLocalRuntimeGateState(
    readingSources.state,
    LocalRuntimeGateEvent.SourcesLoaded,
  );
  const writingReport = transitionLocalRuntimeGateState(
    validatingEvidence.state,
    LocalRuntimeGateEvent.EvidenceValidated,
  );
  const passed = transitionLocalRuntimeGateState(
    writingReport.state,
    LocalRuntimeGateEvent.ReportWritten,
  );
  const failed = transitionLocalRuntimeGateState(validatingEvidence.state, LocalRuntimeGateEvent.Fail, {
    errorCode: LocalRuntimeGateErrorCode.RequiredEvidenceMissing,
    findingId: "first_run_state_machine",
  });
  const invalid = transitionLocalRuntimeGateState(
    LocalRuntimeGateState.Pending,
    LocalRuntimeGateEvent.ReportWritten,
  );

  assert.equal(readingSources.state, LocalRuntimeGateState.ReadingSources);
  assert.equal(validatingEvidence.state, LocalRuntimeGateState.ValidatingEvidence);
  assert.equal(writingReport.state, LocalRuntimeGateState.WritingReport);
  assert.equal(passed.state, LocalRuntimeGateState.Passed);
  assert.equal(failed.state, LocalRuntimeGateState.Failed);
  assert.equal(failed.findingId, "first_run_state_machine");
  assert.equal(invalid.errorCode, LocalRuntimeGateErrorCode.InvalidTransition);
});

function completeSources() {
  return {
    ".tasks/phase006-plan-validation-result.md": "phase006_plan_validation=passed",
    ".tasks/release/local-desktop-runbook.md": [
      "# Local Desktop Runbook",
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
      "Product Log Field Debug Log Development Log",
    ].join("\n"),
    "crates/cabinet-core/src/first_run.rs": [
      "FirstRunState",
      "FirstRunEvent",
      "FirstRunInitializer",
      "FirstRunPlan",
      "FirstRunStore",
      "transition_first_run",
      "FirstRunProductEvent",
    ].join("\n"),
    "crates/cabinet-core/tests/first_run_tests.rs": [
      "first_run_plan_includes_all_local_store_directories",
      "first_run_transitions_to_completed_through_explicit_events",
      "first_run_rejects_invalid_transition",
    ].join("\n"),
    "crates/cabinet-core/tests/first_run_initializer_tests.rs": [
      "first_run_initializer_completes_clean_profile",
      "first_run_initializer_is_idempotent_for_existing_profile",
      "first_run_initializer_returns_retryable_failed_outcome_when_store_creation_fails",
    ].join("\n"),
    "crates/cabinet-adapters/src/local_setup_health.rs": [
      "LocalSetupHealthChecker",
      "LocalSetupHealthStatus",
      "LocalSetupHealthRole",
      "LocalSetupHealthIssueKind",
      "MissingFirstRunMarker",
    ].join("\n"),
    "crates/cabinet-adapters/tests/local_setup_health_checker_tests.rs": [
      "local_setup_health_checker_reports_healthy_first_run_profile",
      "local_setup_health_checker_reports_missing_required_directory",
      "local_setup_health_checker_reports_path_that_is_not_directory",
      "local_setup_health_checker_reports_missing_first_run_marker",
    ].join("\n"),
    "crates/cabinet-platform/tests/clean_install_smoke.rs": [
      "clean_install_smoke_initializes_local_profile_once_without_external_services",
      "run_clean_install_smoke",
      "created_directories",
    ].join("\n"),
    "crates/cabinet-platform/tests/startup_repair_smoke.rs": [
      "startup_repair_smoke_rebuilds_corrupted_indexes_without_losing_current_workspace_data",
      "startup_repair_state_machine_rejects_invalid_transition",
      "run_startup_repair_smoke",
      "corrupted_index_rebuilt",
      "product_log_sensitive_data_absent",
    ].join("\n"),
  };
}
