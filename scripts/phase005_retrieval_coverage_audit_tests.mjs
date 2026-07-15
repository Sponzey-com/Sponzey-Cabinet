import assert from "node:assert/strict";
import test from "node:test";

import {
  RetrievalCoverageAuditErrorCode,
  RetrievalCoverageAuditEvent,
  RetrievalCoverageAuditState,
  analyzeRetrievalCoverageSources,
  renderRetrievalCoverageAuditMarkdown,
  transitionRetrievalCoverageAuditState,
} from "./phase005_retrieval_coverage_audit.mjs";

const completeSources = {
  "crates/cabinet-domain/src/retrieval.rs":
    "RetrievalQuery RetrievalScope RetrievalCandidate CitationSpan ContextBudget RetrievalPipelineState",
  "crates/cabinet-domain/tests/retrieval_tests.rs":
    "retrieval_candidate_uses_references_without_raw_body_or_denied_source retrieval_pipeline_uses_explicit_transitions",
  "crates/cabinet-ports/src/retrieval.rs":
    "RetrievalSourcePort RetrievalPermissionPort RetrievalPortError",
  "crates/cabinet-usecases/src/retrieval.rs":
    "BuildRetrievalContextUsecase BuildRetrievalContextInput RetrievalContextStats",
  "crates/cabinet-usecases/tests/retrieval_usecase_tests.rs":
    "build_retrieval_context_filters_permission_denied_candidates build_retrieval_context_truncates_candidates_over_context_budget",
  "crates/cabinet-adapters/src/local_retrieval_source.rs":
    "LocalRetrievalSource LocalRetrievalSourceRecord RetrievalSourcePort",
  "crates/cabinet-adapters/tests/local_retrieval_source_tests.rs":
    "local_retrieval_source_returns_matching_candidates_by_query_and_source_kind local_retrieval_source_excludes_source_kinds_outside_scope",
};

test("retrieval coverage audit marks complete retrieval fixture as covered", () => {
  const audit = analyzeRetrievalCoverageSources({ sources: completeSources });

  assert.equal(audit.summary.totalTargets, 3);
  assert.equal(audit.summary.covered, 3);
  assert.equal(audit.summary.targetsNeedingWork, 0);
  assert.equal(audit.nextImplementationTarget, null);
});

test("retrieval coverage audit selects fake adapter when adapter evidence is missing", () => {
  const {
    "crates/cabinet-adapters/src/local_retrieval_source.rs": _adapter,
    ...sources
  } = completeSources;

  const audit = analyzeRetrievalCoverageSources({ sources });
  const adapter = audit.targets.find((target) => target.id === "retrieval_fake_source_adapter");

  assert.equal(adapter.status, "missing");
  assert.equal(audit.nextImplementationTarget.id, "retrieval_fake_source_adapter");
});

test("retrieval coverage audit fails with stable code when source set is empty", () => {
  assert.throws(
    () => analyzeRetrievalCoverageSources({ sources: {} }),
    (error) => error.code === RetrievalCoverageAuditErrorCode.SourceSetEmpty,
  );
});

test("retrieval coverage audit state machine rejects invalid transitions", () => {
  assert.equal(
    transitionRetrievalCoverageAuditState(
      RetrievalCoverageAuditState.NotStarted,
      RetrievalCoverageAuditEvent.Start,
    ),
    RetrievalCoverageAuditState.ReadingSource,
  );
  assert.equal(
    transitionRetrievalCoverageAuditState(
      RetrievalCoverageAuditState.ReadingSource,
      RetrievalCoverageAuditEvent.SourceLoaded,
    ),
    RetrievalCoverageAuditState.Auditing,
  );
  assert.equal(
    transitionRetrievalCoverageAuditState(
      RetrievalCoverageAuditState.Auditing,
      RetrievalCoverageAuditEvent.AuditComplete,
    ),
    RetrievalCoverageAuditState.Reported,
  );
  assert.throws(
    () =>
      transitionRetrievalCoverageAuditState(
        RetrievalCoverageAuditState.NotStarted,
        RetrievalCoverageAuditEvent.ReportWritten,
      ),
    (error) => error.code === RetrievalCoverageAuditErrorCode.InvalidTransition,
  );
});

test("retrieval coverage markdown records passed marker when all targets are covered", () => {
  const audit = analyzeRetrievalCoverageSources({ sources: completeSources });
  const markdown = renderRetrievalCoverageAuditMarkdown(audit);

  assert.match(markdown, /# Phase 005 Retrieval Coverage Audit/);
  assert.match(markdown, /phase005_retrieval_coverage_audit=passed/);
  assert.match(markdown, /retrieval_domain_contract/);
  assert.doesNotMatch(markdown, /raw document body should not be logged/);
});
