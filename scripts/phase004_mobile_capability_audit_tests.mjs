import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  MobileCapabilityAuditErrorCode,
  MobileCapabilityAuditEvent,
  MobileCapabilityAuditState,
  analyzeMobileCapabilitySources,
  renderMobileCapabilityAuditMarkdown,
  transitionMobileCapabilityAuditState,
} from "./phase004_mobile_capability_audit.mjs";

const completeSources = {
  "packages/client-core/src/index.ts":
    "PlatformFeatureSupport knowledgeGraphSupport canvasSupport realtimeCollaborationSupport supportsCanvasFullEdit createPlatformCapabilityMatrix",
  "packages/client-core/tests/mobile_read_contract_tests.ts":
    "platform capability matrix documents web desktop and mobile differences without domain rules knowledgeGraphSupport canvasSupport realtimeCollaborationSupport supportsCanvasFullEdit",
  "apps/mobile/src/index.ts":
    "requestCanvasEdit approveReviewRequest rejectReviewRequest createMobilePushNotificationPayload transitionMobileNotificationDeliveryState MOBILE_UNSUPPORTED_CANVAS_EDIT MOBILE_NOTIFICATION_INVALID_TRANSITION",
  "apps/mobile/tests/mobile_read_skeleton_tests.ts":
    "mobile skeleton maps review approve and reject decisions without raw body data mobile skeleton exposes explicit unsupported Canvas full edit action",
  "apps/mobile/tests/mobile_push_notification_tests.ts":
    "mobile push payload excludes sensitive document comment token and canvas data mobile notification delivery state machine exposes queued sent failed and retry transitions",
  "apps/mobile/tests/mobile_read_product_smoke.ts":
    "mobile_review_decision_product_smoke=passed mobile_canvas_unsupported_product_smoke=passed mobile_push_payload_product_smoke=passed mobile_read_product_smoke=passed",
  "scripts/run_mobile_read_product_smoke.mjs":
    "requiredSmokeMarkers assertRequiredMarkersPresent mobile_review_decision_product_smoke=passed mobile_canvas_unsupported_product_smoke=passed mobile_push_payload_product_smoke=passed",
  ".tmp/mobile-read-product-smoke-output.txt":
    "mobile_review_decision_product_smoke=passed mobile_canvas_unsupported_product_smoke=passed mobile_push_payload_product_smoke=passed mobile_read_product_smoke=passed",
  ".tasks/release/security-log-policy-manifest.json":
    "phase004_mobile_push_payload_contract mobile_push_document_body_fixture mobile_push_comment_body_fixture mobile_push_session_token_fixture mobile_push_raw_canvas_state_fixture push_document_body push_comment_body push_session_token push_raw_canvas_state",
  "scripts/security_log_scanner_tests.mjs":
    "active security manifest includes Phase 004 mobile push artifacts and denied fixtures phase004_mobile_push_payload_contract",
};

test("mobile capability audit marks complete fixture as covered", () => {
  const audit = analyzeMobileCapabilitySources({ sources: completeSources });

  assert.equal(audit.summary.totalTargets, 4);
  assert.equal(audit.summary.covered, 4);
  assert.equal(audit.summary.targetsNeedingWork, 0);
  assert.equal(audit.nextImplementationTarget, null);
});

test("mobile capability audit selects product smoke when output artifact is missing", () => {
  const {
    ".tmp/mobile-read-product-smoke-output.txt": _output,
    ...sources
  } = completeSources;

  const audit = analyzeMobileCapabilitySources({ sources });
  const target = audit.targets.find((entry) => entry.id === "mobile_phase004_product_smoke");

  assert.equal(target.status, "missing");
  assert.ok(target.missingFiles.includes(".tmp/mobile-read-product-smoke-output.txt"));
  assert.equal(audit.nextImplementationTarget.id, "mobile_phase004_product_smoke");
});

test("mobile capability audit selects security policy when push fixtures are missing", () => {
  const sources = {
    ...completeSources,
    ".tasks/release/security-log-policy-manifest.json": "phase004_mobile_push_payload_contract",
    "scripts/security_log_scanner_tests.mjs":
      "active security manifest includes Phase 004 mobile push artifacts and denied fixtures",
  };

  const audit = analyzeMobileCapabilitySources({ sources });
  const target = audit.targets.find((entry) => entry.id === "mobile_push_security_policy");

  assert.equal(target.status, "missing");
  assert.ok(target.missingEvidence.includes("mobile_push_document_body_fixture"));
  assert.ok(target.missingEvidence.includes("push_raw_canvas_state"));
});

test("mobile capability audit fails with stable code when source set is empty", () => {
  assert.throws(
    () => analyzeMobileCapabilitySources({ sources: {} }),
    (error) => error.code === MobileCapabilityAuditErrorCode.SourceSetEmpty,
  );
});

test("mobile capability audit state machine rejects invalid transitions", () => {
  assert.equal(
    transitionMobileCapabilityAuditState(
      MobileCapabilityAuditState.NotStarted,
      MobileCapabilityAuditEvent.Start,
    ),
    MobileCapabilityAuditState.ReadingSource,
  );
  assert.equal(
    transitionMobileCapabilityAuditState(
      MobileCapabilityAuditState.ReadingSource,
      MobileCapabilityAuditEvent.SourceLoaded,
    ),
    MobileCapabilityAuditState.Auditing,
  );
  assert.equal(
    transitionMobileCapabilityAuditState(
      MobileCapabilityAuditState.Auditing,
      MobileCapabilityAuditEvent.AuditComplete,
    ),
    MobileCapabilityAuditState.Reported,
  );
  assert.throws(
    () =>
      transitionMobileCapabilityAuditState(
        MobileCapabilityAuditState.NotStarted,
        MobileCapabilityAuditEvent.ReportWritten,
      ),
    (error) => error.code === MobileCapabilityAuditErrorCode.InvalidTransition,
  );
});

test("mobile capability markdown records phase targets and next action", () => {
  const {
    ".tmp/mobile-read-product-smoke-output.txt": _output,
    ...sources
  } = completeSources;
  const audit = analyzeMobileCapabilitySources({ sources });
  const markdown = renderMobileCapabilityAuditMarkdown(audit);

  assert.match(markdown, /# Phase 004 Mobile Capability Coverage Audit/);
  assert.match(markdown, /Phase 004\.6/);
  assert.match(markdown, /mobile_phase004_product_smoke/);
  assert.match(markdown, /missing/);
});

test("package scripts expose mobile capability audit runners", async () => {
  const packageJson = JSON.parse(await readFile("package.json", "utf8"));
  const testRunner = await readFile("scripts/run_phase004_mobile_capability_audit_tests.sh", "utf8");
  const auditRunner = await readFile("scripts/run_phase004_mobile_capability_audit.sh", "utf8");

  assert.equal(
    packageJson.scripts["run:phase004-mobile-capability-audit-tests"],
    "sh scripts/run_phase004_mobile_capability_audit_tests.sh",
  );
  assert.equal(
    packageJson.scripts["run:phase004-mobile-capability-audit"],
    "sh scripts/run_phase004_mobile_capability_audit.sh",
  );
  assert.match(testRunner, /node --test scripts\/phase004_mobile_capability_audit_tests\.mjs/);
  assert.match(auditRunner, /node scripts\/phase004_mobile_capability_audit\.mjs/);
});
