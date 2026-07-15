import assert from "node:assert/strict";
import test from "node:test";

import {
  GraphCoverageAuditErrorCode,
  GraphCoverageAuditEvent,
  GraphCoverageAuditState,
  analyzeGraphCoverageSources,
  renderGraphCoverageAuditMarkdown,
  transitionGraphCoverageAuditState,
} from "./phase004_graph_coverage_audit.mjs";

const completeSources = {
  "crates/cabinet-domain/src/graph.rs":
    "KnowledgeGraph GraphNode GraphEdge GraphProjectionStatus",
  "crates/cabinet-domain/tests/graph_tests.rs":
    "knowledge_graph_rejects_edge_with_missing_node_reference graph_projection_status_uses_explicit_transitions",
  "crates/cabinet-ports/src/graph_projection.rs":
    "GraphProjectionStore GraphProjectionRecord",
  "crates/cabinet-adapters/src/local_graph_projection.rs":
    "LocalGraphProjectionStore GraphProjectionStore",
  "crates/cabinet-usecases/src/graph.rs":
    "PermissionAwareGraphUsecase PermissionAwareGraphInput PermissionAwareGraphStats",
  "crates/cabinet-usecases/tests/permission_aware_graph_tests.rs":
    "permission_aware_graph_filters_denied_document_nodes_and_edges graph.projection_not_found",
  "crates/cabinet-server/src/composition.rs":
    "/api/workspaces/{workspaceId}/documents/{documentId}/graph graph.get_local",
  "crates/cabinet-server/src/runtime.rs":
    "HandlerKind::GraphLocal PermissionAwareGraphUsecase graph_projection_store graph.query.completed",
  "crates/cabinet-server/tests/server_runtime_wiring_tests.rs":
    "graph_handler_uses_projection_port_and_filters_denied_documents GraphProjectionStore",
  "scripts/run_self_host_e2e_smoke.mjs":
    "graph_visible_node_returned graph_hidden_node_filtered graph_under_300ms_target product_log_event=graph.query.completed",
  "packages/client-core/src/index.ts":
    "KnowledgeGraphQuery KnowledgeGraphView createKnowledgeGraphQuery getKnowledgeGraph",
  "packages/client-core/tests/collaboration_api_client_tests.ts":
    "self-host collaboration API client maps knowledge graph route without client-side filtering",
  "apps/web/src/index.ts":
    "createWebGraphViewModel KnowledgeGraphView",
  "apps/web/tests/web_graph_model_tests.ts":
    "web graph model maps API graph result without duplicating permission filtering",
  "apps/desktop/src/index.ts":
    "readDesktopRemoteKnowledgeGraph readRemoteKnowledgeGraph createKnowledgeGraphQuery",
  "scripts/run_desktop_remote_product_smoke.sh":
    "node scripts/run_desktop_remote_product_smoke.mjs",
  "scripts/run_desktop_remote_product_smoke.mjs":
    "runDesktopSmoke assertSensitiveOutputClean",
  "apps/desktop/tests/desktop_remote_product_smoke.ts":
    "desktop_remote_product_step_passed=remote_graph_flow doc-hidden observedMs",
  "crates/cabinet-platform/tests/query_performance_benchmarks.rs":
    "PermissionAwareGraphLookup PermissionAwareGraphUsecase LocalGraphProjectionStore",
};

test("graph coverage audit marks complete graph fixture as covered", () => {
  const audit = analyzeGraphCoverageSources({ sources: completeSources });

  assert.equal(audit.summary.totalTargets, 5);
  assert.equal(audit.summary.covered, 5);
  assert.equal(audit.summary.targetsNeedingWork, 0);
  assert.equal(audit.nextImplementationTarget, null);
});

test("graph coverage audit selects desktop graph smoke when product evidence is missing", () => {
  const {
    "scripts/run_desktop_remote_product_smoke.sh": _desktopRunner,
    ...sources
  } = completeSources;

  const audit = analyzeGraphCoverageSources({ sources });
  const desktop = audit.targets.find((target) => target.id === "graph_desktop_product_smoke");

  assert.equal(desktop.status, "missing");
  assert.equal(audit.nextImplementationTarget.id, "graph_desktop_product_smoke");
});

test("graph coverage audit selects performance evidence when benchmark target is missing", () => {
  const sources = {
    ...completeSources,
    "crates/cabinet-platform/tests/query_performance_benchmarks.rs":
      "GraphLiteProjectionUsecase LinkBacklinkLookup",
  };

  const audit = analyzeGraphCoverageSources({ sources });
  const performance = audit.targets.find((target) => target.id === "graph_performance_evidence");

  assert.equal(performance.status, "missing");
  assert.deepEqual(performance.missingEvidence, [
    "PermissionAwareGraphLookup",
    "PermissionAwareGraphUsecase",
    "LocalGraphProjectionStore",
  ]);
});

test("graph coverage audit fails with stable code when source set is empty", () => {
  assert.throws(
    () => analyzeGraphCoverageSources({ sources: {} }),
    (error) => error.code === GraphCoverageAuditErrorCode.SourceSetEmpty,
  );
});

test("graph coverage audit state machine rejects invalid transitions", () => {
  assert.equal(
    transitionGraphCoverageAuditState(
      GraphCoverageAuditState.NotStarted,
      GraphCoverageAuditEvent.Start,
    ),
    GraphCoverageAuditState.ReadingSource,
  );
  assert.equal(
    transitionGraphCoverageAuditState(
      GraphCoverageAuditState.ReadingSource,
      GraphCoverageAuditEvent.SourceLoaded,
    ),
    GraphCoverageAuditState.Auditing,
  );
  assert.equal(
    transitionGraphCoverageAuditState(
      GraphCoverageAuditState.Auditing,
      GraphCoverageAuditEvent.AuditComplete,
    ),
    GraphCoverageAuditState.Reported,
  );
  assert.throws(
    () =>
      transitionGraphCoverageAuditState(
        GraphCoverageAuditState.NotStarted,
        GraphCoverageAuditEvent.ReportWritten,
      ),
    (error) => error.code === GraphCoverageAuditErrorCode.InvalidTransition,
  );
});

test("graph coverage markdown records phase targets and next action", () => {
  const {
    "apps/desktop/tests/desktop_remote_product_smoke.ts": _desktopSmoke,
    ...sources
  } = completeSources;
  const audit = analyzeGraphCoverageSources({ sources });
  const markdown = renderGraphCoverageAuditMarkdown(audit);

  assert.match(markdown, /# Phase 004 Graph Coverage Audit/);
  assert.match(markdown, /Phase 004\.2/);
  assert.match(markdown, /graph_desktop_product_smoke/);
  assert.match(markdown, /missing/);
});
