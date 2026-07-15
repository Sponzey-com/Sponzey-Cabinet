import assert from "node:assert/strict";
import test from "node:test";

import {
  HardeningCoverageAuditErrorCode,
  HardeningCoverageAuditEvent,
  HardeningCoverageAuditState,
  analyzeHardeningCoverageSources,
  renderHardeningCoverageAuditMarkdown,
  transitionHardeningCoverageAuditState,
} from "./phase003_hardening_coverage_audit.mjs";

const COMPLETE_SOURCES = Object.freeze({
  "scripts/security_log_scanner.mjs":
    "security_log_scan=passed Product Log Field Debug Log Development Log deniedFixtures scanTargets SensitiveFixtureFound",
  "scripts/security_log_scanner_tests.mjs":
    "security scan validates Product Log Field Debug Log Development Log denied fixtures",
  "scripts/run_security_log_scanner.sh":
    "node scripts/security_log_scanner.mjs .tasks/release/security-log-policy-manifest.json",
  ".tasks/release/security-log-policy-manifest.json":
    "Product Log Field Debug Log Development Log deniedFixtures scanTargets document_body token secret credential",
  "scripts/runbook_validator.mjs":
    "runbook_validation=passed requiredSections requiredPhrases forbiddenText",
  "scripts/runbook_validator_tests.mjs": "runbook validation rejects edit .env raw-token-example",
  "scripts/run_runbook_validator.sh":
    "node scripts/runbook_validator.mjs .tasks/release/runbook-validation-manifest.json",
  ".tasks/release/runbook-validation-manifest.json":
    "requiredSections requiredPhrases forbiddenText runbooks backup-restore field-debug-approval upgrade-migration",
  ".tasks/release/runbooks/backup-restore.md":
    "Backup Restore Product Log Field Debug Log recovery smoke no manual environment edit",
  ".tasks/release/runbooks/field-debug-approval.md":
    "Field Debug approval TTL revoke scoped admin approval",
  ".tasks/release/runbooks/upgrade-migration.md":
    "Upgrade Migration migration state machine data preservation rollback",
  ".tasks/release/product-log-event-matrix.md":
    "Product Log Field Debug Log Development Log stable error code event name sensitive data denied",
  "crates/cabinet-domain/src/field_debug.rs": "FieldDebugSession Requested Approved Expired Revoked",
  "crates/cabinet-domain/tests/field_debug_tests.rs":
    "field_debug_session_approval_requires_scope_ttl_and_admin revoke expires",
  "crates/cabinet-usecases/src/field_debug.rs":
    "FieldDebugSessionPolicy max_ttl_seconds approve revoke expire",
  "crates/cabinet-usecases/tests/field_debug_usecase_tests.rs":
    "field_debug_request_requires_scope approve expires revoke",
  "crates/cabinet-core/src/server_config.rs": "DevelopmentLogMode disabled development_log_mode",
  "crates/cabinet-core/tests/server_config_tests.rs":
    "development_log_mode defaults to disabled rejects invalid development log mode",
  "scripts/phase003_recovery_coverage_audit.mjs":
    "startup repair restore staging backup restore recovery coverage",
  ".tasks/phase003/recovery-coverage-audit.md":
    "targets needing work | 0 startup repair corrupted indexes backup restore",
  "scripts/phase003_release_gate.mjs":
    "phase003_release_gate=passed production hardening complete security_log_scan runbook_validation packaging_gate product_smoke_gate",
  "scripts/phase003_release_gate_tests.mjs":
    "release gate short-circuits final release artifact production hardening complete",
  ".tasks/phase003/final-release-gate-result.md":
    "Phase 003 Final Release Gate Result status: `passed` production hardening complete",
});

test("hardening coverage audit marks complete fixture as covered", () => {
  const audit = analyzeHardeningCoverageSources({ sources: COMPLETE_SOURCES });

  assert.equal(audit.phase, "Phase 003.5");
  assert.equal(audit.summary.totalTargets, 7);
  assert.equal(audit.summary.covered, 7);
  assert.equal(audit.summary.partial, 0);
  assert.equal(audit.summary.missing, 0);
  assert.equal(audit.summary.targetsNeedingWork, 0);
});

test("hardening coverage audit classifies missing active security manifest as partial next target", () => {
  const { ".tasks/release/security-log-policy-manifest.json": _manifest, ...sources } =
    COMPLETE_SOURCES;

  const audit = analyzeHardeningCoverageSources({ sources });
  const security = audit.targets.find(
    (target) => target.id === "security_log_scanner_active_manifest",
  );

  assert.equal(security.status, "partial");
  assert.equal(audit.nextImplementationTarget.id, "security_log_scanner_active_manifest");
});

test("hardening coverage audit reports missing final release gate when scripts are absent", () => {
  const {
    "scripts/phase003_release_gate.mjs": _gate,
    "scripts/phase003_release_gate_tests.mjs": _gateTests,
    ".tasks/phase003/final-release-gate-result.md": _result,
    ...sources
  } = COMPLETE_SOURCES;

  const audit = analyzeHardeningCoverageSources({ sources });
  const releaseGate = audit.targets.find((target) => target.id === "final_release_gate_artifact");

  assert.equal(releaseGate.status, "missing");
  assert.equal(releaseGate.missingFiles.includes("scripts/phase003_release_gate.mjs"), true);
});

test("hardening coverage audit fails with stable code when source set is empty", () => {
  assert.throws(
    () => analyzeHardeningCoverageSources({ sources: {} }),
    (error) => error.code === HardeningCoverageAuditErrorCode.SourceSetEmpty,
  );
});

test("hardening coverage audit state machine rejects invalid transitions", () => {
  assert.throws(
    () =>
      transitionHardeningCoverageAuditState(
        HardeningCoverageAuditState.Pending,
        HardeningCoverageAuditEvent.ReportWritten,
      ),
    (error) => error.code === HardeningCoverageAuditErrorCode.InvalidTransition,
  );
});

test("hardening coverage markdown records summary and next target", () => {
  const { ".tasks/release/security-log-policy-manifest.json": _manifest, ...sources } =
    COMPLETE_SOURCES;
  const audit = analyzeHardeningCoverageSources({ sources });
  const markdown = renderHardeningCoverageAuditMarkdown(audit);

  assert.match(markdown, /Phase 003 Hardening Coverage Audit/);
  assert.match(markdown, /security_log_scanner_active_manifest/);
  assert.match(markdown, /partial/);
  assert.match(markdown, /targets needing work/);
});
