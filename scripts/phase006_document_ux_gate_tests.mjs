import assert from "node:assert/strict";
import test from "node:test";

import {
  DocumentUxGateErrorCode,
  DocumentUxGateEvent,
  DocumentUxGateState,
  analyzeDocumentUxEvidence,
  renderDocumentUxGateMarkdown,
  transitionDocumentUxGateState,
} from "./phase006_document_ux_gate.mjs";

test("document UX gate reports complete evidence as passed", () => {
  const result = analyzeDocumentUxEvidence({ sources: completeSources() });
  const markdown = renderDocumentUxGateMarkdown(result);

  assert.equal(result.passed, true);
  assert.equal(result.marker, "phase006_document_ux_gate=passed");
  assert.equal(result.summary.missingRequiredEvidence, 0);
  assert.match(markdown, /phase006_document_ux_gate=passed/);
  assert.match(markdown, /Markdown preview and restore UX/);
  assert.doesNotMatch(markdown, /phase006-raw-document-body-should-not-log/);
  assert.doesNotMatch(markdown, /provider_api_key_fixture/);
});

test("document UX gate fails when workspace shell prerequisite is missing", () => {
  const sources = completeSources();
  sources[".tasks/phase006-workspace-shell-gate-result.md"] =
    "phase006_workspace_shell_gate=failed";

  const result = analyzeDocumentUxEvidence({ sources });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, DocumentUxGateErrorCode.RequiredEvidenceMissing);
  assert.equal(result.missingEvidence[0].targetId, "workspace_shell_prerequisite");
});

test("document UX gate fails when performance budget marker is missing", () => {
  const sources = completeSources();
  sources[".tasks/release/performance-budget-phase006.md"] =
    "phase006_document_query_budget=failed";

  const result = analyzeDocumentUxEvidence({ sources });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, DocumentUxGateErrorCode.RequiredEvidenceMissing);
  assert.equal(result.missingEvidence[0].targetId, "document_query_performance_budget");
});

test("document UX gate state machine exposes explicit transitions and invalid transition", () => {
  const readingSources = transitionDocumentUxGateState(
    DocumentUxGateState.Pending,
    DocumentUxGateEvent.Start,
  );
  const validatingEvidence = transitionDocumentUxGateState(
    readingSources.state,
    DocumentUxGateEvent.SourcesLoaded,
  );
  const writingReport = transitionDocumentUxGateState(
    validatingEvidence.state,
    DocumentUxGateEvent.EvidenceValidated,
  );
  const passed = transitionDocumentUxGateState(
    writingReport.state,
    DocumentUxGateEvent.ReportWritten,
  );
  const failed = transitionDocumentUxGateState(validatingEvidence.state, DocumentUxGateEvent.Fail, {
    errorCode: DocumentUxGateErrorCode.RequiredEvidenceMissing,
    findingId: "document_query_performance_budget",
  });
  const invalid = transitionDocumentUxGateState(
    DocumentUxGateState.Pending,
    DocumentUxGateEvent.ReportWritten,
  );

  assert.equal(readingSources.state, DocumentUxGateState.ReadingSources);
  assert.equal(validatingEvidence.state, DocumentUxGateState.ValidatingEvidence);
  assert.equal(writingReport.state, DocumentUxGateState.WritingReport);
  assert.equal(passed.state, DocumentUxGateState.Passed);
  assert.equal(failed.state, DocumentUxGateState.Failed);
  assert.equal(failed.findingId, "document_query_performance_budget");
  assert.equal(invalid.errorCode, DocumentUxGateErrorCode.InvalidTransition);
});

function completeSources() {
  return {
    ".tasks/phase006-workspace-shell-gate-result.md": "phase006_workspace_shell_gate=passed",
    ".tasks/release/performance-budget-phase006.md": [
      "phase006_document_query_budget=passed",
      "current_document_read_p95_ms=5",
      "history_read_p95_ms=6",
      "threshold_ms=300",
    ].join("\n"),
    "packages/ui/src/index.ts": [
      "createMarkdownPreviewModel",
      "createDocumentReadingWorkspaceModel",
      "transitionRestoreFlowState",
      "createRestorePreviewRequestFromHistoryEntry",
      "createRestoreApplyCommand",
      "MarkdownTablePreviewBlock",
    ].join("\n"),
    "packages/ui/tests/markdown_preview_model_tests.ts": [
      "markdown preview renders table grid while source remains markdown text",
      "document reading workspace keeps current and history query paths separated",
      "unsafe html",
    ].join("\n"),
    "packages/ui/tests/restore_flow_model_tests.ts": [
      "restore flow state machine accepts only preview before apply",
      "history entry creates restore preview request without document body",
      "restore preview model creates confirmed command only when restore is allowed",
    ].join("\n"),
    "apps/desktop/src/index.ts": [
      "createDesktopDocumentReadingWorkspace",
      "createDesktopRestorePreviewRequest",
      "createDesktopRestoreApplyCommand",
    ].join("\n"),
    "apps/desktop/tests/desktop_document_ux_smoke_tests.ts": [
      "desktop document UX smoke renders markdown preview table and keeps read paths split",
      "desktop restore smoke creates preview request and confirmed command without body payload",
    ].join("\n"),
    "crates/cabinet-usecases/tests/preview_document_restore_tests.rs": [
      "preview_document_restore_returns_diff_without_writes",
      "preview_document_restore_reports_not_found_for_missing_target",
    ].join("\n"),
    "crates/cabinet-usecases/tests/restore_document_version_tests.rs": [
      "restore_document_version_appends_restore_version_updates_current_and_emits_events",
      "restore_document_version_reports_not_found_for_missing_target_without_writes",
    ].join("\n"),
    "crates/cabinet-usecases/tests/compare_document_versions_tests.rs": [
      "compare_current_to_version_uses_current_and_specific_snapshot_without_history_list",
      "compare_two_versions_uses_specific_snapshots_without_current_or_history_list",
    ].join("\n"),
  };
}
