import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  CanvasCoverageAuditErrorCode,
  CanvasCoverageAuditEvent,
  CanvasCoverageAuditState,
  analyzeCanvasCoverageSources,
  renderCanvasCoverageAuditMarkdown,
  transitionCanvasCoverageAuditState,
} from "./phase004_canvas_coverage_audit.mjs";

const completeSources = {
  "crates/cabinet-domain/src/canvas.rs":
    "Canvas CanvasNode CanvasEdge CanvasEmbed CanvasLifecycleState transition_canvas_lifecycle",
  "crates/cabinet-domain/tests/canvas_tests.rs":
    "canvas_rejects_edge_with_missing_node_reference canvas_accepts_document_attachment_external_link_and_text_card_nodes canvas_embed_uses_stable_reference_without_raw_ui_state canvas_lifecycle_uses_explicit_transitions",
  "crates/cabinet-usecases/src/canvas.rs":
    "CreateCanvasUsecase AddCanvasNodeUsecase ConnectCanvasNodesUsecase EmbedCanvasInDocumentUsecase ConvertDocumentOutlineToCanvasUsecase canvas.created canvas.node.added canvas.edge.connected canvas.embedded",
  "crates/cabinet-usecases/tests/canvas_usecase_tests.rs":
    "create_canvas_requires_write_permission_and_saves_draft_canvas add_canvas_node_updates_existing_canvas_without_raw_ui_state_output connect_canvas_nodes_rejects_missing_node_edge_without_save embed_canvas_in_document_returns_stable_reference_without_raw_ui_state convert_document_outline_to_canvas_preserves_heading_order",
  "crates/cabinet-ports/src/canvas_repository.rs":
    "CanvasRepository CanvasRecord CanvasRepositoryError",
  "crates/cabinet-ports/tests/canvas_repository_contract_tests.rs":
    "canvas_repository_contract_preserves_workspace_canvas_and_lifecycle_state canvas_repository_error_codes_are_stable",
  "crates/cabinet-adapters/src/local_canvas_repository.rs": "LocalCanvasRepository",
  "crates/cabinet-adapters/tests/local_canvas_repository_tests.rs":
    "local_canvas_repository_keeps_workspace_canvases_separate local_canvas_repository_replaces_existing_canvas_record",
  ".tasks/release/security-log-policy-manifest.json":
    "phase004_canvas_domain_model phase004_canvas_usecase_contract phase004_canvas_local_adapter canvas_raw_ui_state_fixture canvas_text_card_fixture canvas_heading_title_fixture canvas_attachment_filename_fixture canvas_raw_ui_state card_text heading_title canvas_attachment_filename phase004_canvas_coverage_audit",
  "scripts/security_log_scanner_tests.mjs":
    "active security manifest includes Phase 004 Canvas artifacts and denied fixtures phase004_canvas_coverage_audit",
  "crates/cabinet-server/tests/canvas_runtime_tests.rs":
    "canvas.create canvas.add_node canvas.embed canvas_relation_graph_projection",
  "packages/client-core/tests/canvas_client_tests.ts": "CanvasApiClient createCanvas addCanvasNode embedCanvas",
  "apps/web/tests/web_canvas_model_tests.ts": "createWebCanvasViewModel WebCanvasNodeModel",
  "scripts/run_phase004_canvas_product_smoke.sh": "phase004_canvas_product_smoke=passed",
  ".tasks/canvas-product-smoke-result.md":
    "phase004_canvas_product_smoke=passed canvas_relation_graph_projection=passed",
};

test("canvas coverage audit marks complete fixture as covered", () => {
  const audit = analyzeCanvasCoverageSources({ sources: completeSources });

  assert.equal(audit.summary.totalTargets, 4);
  assert.equal(audit.summary.covered, 4);
  assert.equal(audit.summary.targetsNeedingWork, 0);
  assert.equal(audit.nextImplementationTarget, null);
});

test("canvas coverage audit selects runtime UI smoke when product evidence is missing", () => {
  const {
    "crates/cabinet-server/tests/canvas_runtime_tests.rs": _serverTests,
    "packages/client-core/tests/canvas_client_tests.ts": _clientTests,
    "apps/web/tests/web_canvas_model_tests.ts": _webTests,
    "scripts/run_phase004_canvas_product_smoke.sh": _smokeRunner,
    ".tasks/canvas-product-smoke-result.md": _smokeResult,
    ...sources
  } = completeSources;

  const audit = analyzeCanvasCoverageSources({ sources });
  const target = audit.targets.find((entry) => entry.id === "canvas_runtime_ui_product_smoke");

  assert.equal(target.status, "missing");
  assert.equal(audit.nextImplementationTarget.id, "canvas_runtime_ui_product_smoke");
});

test("canvas coverage audit selects security target when scanner evidence is missing", () => {
  const sources = {
    ...completeSources,
    ".tasks/release/security-log-policy-manifest.json": "phase004_canvas_domain_model",
    "scripts/security_log_scanner_tests.mjs":
      "active security manifest includes Phase 004 Canvas artifacts and denied fixtures",
  };

  const audit = analyzeCanvasCoverageSources({ sources });
  const target = audit.targets.find((entry) => entry.id === "canvas_security_log_policy_evidence");

  assert.equal(target.status, "missing");
  assert.ok(target.missingEvidence.includes("canvas_raw_ui_state_fixture"));
  assert.ok(target.missingEvidence.includes("phase004_canvas_coverage_audit"));
});

test("canvas coverage audit fails with stable code when source set is empty", () => {
  assert.throws(
    () => analyzeCanvasCoverageSources({ sources: {} }),
    (error) => error.code === CanvasCoverageAuditErrorCode.SourceSetEmpty,
  );
});

test("canvas coverage audit state machine rejects invalid transitions", () => {
  assert.equal(
    transitionCanvasCoverageAuditState(
      CanvasCoverageAuditState.NotStarted,
      CanvasCoverageAuditEvent.Start,
    ),
    CanvasCoverageAuditState.ReadingSource,
  );
  assert.equal(
    transitionCanvasCoverageAuditState(
      CanvasCoverageAuditState.ReadingSource,
      CanvasCoverageAuditEvent.SourceLoaded,
    ),
    CanvasCoverageAuditState.Auditing,
  );
  assert.equal(
    transitionCanvasCoverageAuditState(
      CanvasCoverageAuditState.Auditing,
      CanvasCoverageAuditEvent.AuditComplete,
    ),
    CanvasCoverageAuditState.Reported,
  );
  assert.throws(
    () =>
      transitionCanvasCoverageAuditState(
        CanvasCoverageAuditState.NotStarted,
        CanvasCoverageAuditEvent.ReportWritten,
      ),
    (error) => error.code === CanvasCoverageAuditErrorCode.InvalidTransition,
  );
});

test("canvas coverage markdown records phase targets and next action", () => {
  const {
    "scripts/run_phase004_canvas_product_smoke.sh": _smokeRunner,
    ".tasks/canvas-product-smoke-result.md": _smokeResult,
    ...sources
  } = completeSources;
  const audit = analyzeCanvasCoverageSources({ sources });
  const markdown = renderCanvasCoverageAuditMarkdown(audit);

  assert.match(markdown, /# Phase 004 Canvas Coverage Audit/);
  assert.match(markdown, /Phase 004\.5/);
  assert.match(markdown, /canvas_runtime_ui_product_smoke/);
  assert.match(markdown, /missing/);
});

test("package scripts expose canvas coverage audit runners", async () => {
  const packageJson = JSON.parse(await readFile("package.json", "utf8"));
  const testRunner = await readFile("scripts/run_phase004_canvas_coverage_audit_tests.sh", "utf8");
  const auditRunner = await readFile("scripts/run_phase004_canvas_coverage_audit.sh", "utf8");

  assert.equal(
    packageJson.scripts["run:phase004-canvas-coverage-audit-tests"],
    "sh scripts/run_phase004_canvas_coverage_audit_tests.sh",
  );
  assert.equal(
    packageJson.scripts["run:phase004-canvas-coverage-audit"],
    "sh scripts/run_phase004_canvas_coverage_audit.sh",
  );
  assert.match(testRunner, /node --test scripts\/phase004_canvas_coverage_audit_tests\.mjs/);
  assert.match(auditRunner, /node scripts\/phase004_canvas_coverage_audit\.mjs/);
});
