import assert from "node:assert/strict";
import test from "node:test";

import {
  PersistenceGapAuditErrorCode,
  PersistenceGapAuditEvent,
  PersistenceGapAuditState,
  analyzePersistenceSources,
  renderPersistenceGapAuditMarkdown,
  transitionPersistenceGapAuditState,
} from "./phase003_persistence_gap_audit.mjs";

const fixtureSources = {
  "crates/cabinet-ports/src/document_repository.rs": "pub trait DocumentRepository {}",
  "crates/cabinet-adapters/src/local_document_repository.rs":
    "pub struct LocalDocumentRepository; impl DocumentRepository for LocalDocumentRepository {}",
  "crates/cabinet-adapters/tests/local_document_repository_tests.rs":
    "LocalDocumentRepository persists current documents",
  "crates/cabinet-ports/src/auth.rs": "pub trait SessionStore {}",
  "crates/cabinet-adapters/src/local_auth.rs":
    "pub struct InMemorySessionStore; impl SessionStore for InMemorySessionStore {}",
  "crates/cabinet-adapters/tests/local_auth_adapter_tests.rs":
    "InMemorySessionStore validates auth contract",
  "crates/cabinet-ports/src/audit_log.rs": "pub trait AuditLogStore {}",
  "crates/cabinet-ports/tests/audit_log_store_contract_tests.rs":
    "AuditLogStore contract test",
  "crates/cabinet-ports/src/group_repository.rs": "pub trait GroupRepository {}",
  "crates/cabinet-core/src/migration.rs":
    "pub enum MigrationState { NotStarted, Locked, Running, Completed, Failed }",
  "crates/cabinet-core/tests/migration_tests.rs": "migration state machine tests",
  "crates/cabinet-platform/tests/phase002_migration_fixture_smoke.rs":
    "phase002 migration fixture smoke",
};

test("persistence gap audit classifies durable, volatile, contract-only, and missing targets", () => {
  const audit = analyzePersistenceSources({ sources: fixtureSources });

  assert.equal(audit.summary.totalTargets > 0, true);
  assert.equal(
    audit.targets.find((target) => target.id === "document_current_store").status,
    "durable adapter wired",
  );
  assert.equal(
    audit.targets.find((target) => target.id === "session_store").status,
    "volatile adapter only",
  );
  assert.equal(
    audit.targets.find((target) => target.id === "audit_log_store").status,
    "contract complete only",
  );
  assert.equal(
    audit.targets.find((target) => target.id === "group_repository").status,
    "port defined only",
  );
  assert.equal(audit.findings[0].id, "PHASE003_PERSISTENCE_GAP");
  assert.equal(audit.nextImplementationTarget.id, "session_store");
});

test("persistence gap audit fails with stable code when no source is provided", () => {
  assert.throws(
    () => analyzePersistenceSources({ sources: {} }),
    (error) => error.code === PersistenceGapAuditErrorCode.SourceSetEmpty,
  );
});

test("persistence gap audit treats explicit durable evidence as stronger than volatile evidence", () => {
  const audit = analyzePersistenceSources({
    sources: {
      ...fixtureSources,
      "crates/cabinet-adapters/src/local_auth.rs":
        "pub struct InMemorySessionStore; pub struct LocalSessionStore; impl SessionStore for LocalSessionStore {}",
    },
  });

  assert.equal(
    audit.targets.find((target) => target.id === "session_store").status,
    "durable adapter wired",
  );
  assert.equal(audit.nextImplementationTarget.id, "user_repository");
});

test("persistence gap audit state machine rejects invalid transitions", () => {
  assert.equal(
    transitionPersistenceGapAuditState(
      PersistenceGapAuditState.NotStarted,
      PersistenceGapAuditEvent.Start,
    ),
    PersistenceGapAuditState.ReadingSource,
  );
  assert.equal(
    transitionPersistenceGapAuditState(
      PersistenceGapAuditState.ReadingSource,
      PersistenceGapAuditEvent.SourceLoaded,
    ),
    PersistenceGapAuditState.Auditing,
  );
  assert.equal(
    transitionPersistenceGapAuditState(
      PersistenceGapAuditState.Auditing,
      PersistenceGapAuditEvent.AuditComplete,
    ),
    PersistenceGapAuditState.Reported,
  );
  assert.equal(
    transitionPersistenceGapAuditState(
      PersistenceGapAuditState.Reported,
      PersistenceGapAuditEvent.ReportWritten,
    ),
    PersistenceGapAuditState.Reported,
  );
  assert.throws(
    () =>
      transitionPersistenceGapAuditState(
        PersistenceGapAuditState.NotStarted,
        PersistenceGapAuditEvent.ReportWritten,
      ),
    (error) => error.code === PersistenceGapAuditErrorCode.InvalidTransition,
  );
});

test("persistence gap audit markdown keeps phase terms and next task target explicit", () => {
  const audit = analyzePersistenceSources({ sources: fixtureSources });
  const markdown = renderPersistenceGapAuditMarkdown(audit);

  assert.match(markdown, /# Phase 003 Persistence Gap Audit/);
  assert.match(markdown, /Phase 003\.2/);
  assert.match(markdown, /durable adapter wired/);
  assert.match(markdown, /volatile adapter only/);
  assert.match(markdown, /contract complete only/);
  assert.match(markdown, /Next implementation target/);
  assert.match(markdown, /session_store/);
});
