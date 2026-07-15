import assert from "node:assert/strict";
import test from "node:test";

import {
  DurableDependencyManifestAuditErrorCode,
  DurableDependencyManifestAuditEvent,
  DurableDependencyManifestAuditState,
  analyzeDurableDependencyManifestSources,
  renderDurableDependencyManifestAuditMarkdown,
  transitionDurableDependencyManifestAuditState,
} from "./phase003_durable_dependency_manifest_audit.mjs";

const completeRuntimeSource = `
impl RuntimeDependencyManifest {
    pub fn phase003_self_host() -> Self {
        use RuntimeDependencyDurability::{
            DurableLocal, External, Policy, RuntimeUtility, VolatileLocal,
        };
        Self {
            dependencies: vec![
                dependency(
                    "document_repository",
                    "LocalDocumentRepository",
                    DurableLocal,
                ),
                dependency("version_store", "LocalVersionStore", DurableLocal),
                dependency(
                    "document_asset_metadata_store",
                    "LocalDocumentAssetRepository",
                    DurableLocal,
                ),
                dependency("object_storage", "LocalObjectStorage", DurableLocal),
                dependency("search_index", "LocalSearchIndex", DurableLocal),
                dependency("link_index", "LocalLinkIndex", DurableLocal),
                dependency("session_store", "LocalSessionStore", DurableLocal),
                dependency("user_repository", "LocalUserRepository", DurableLocal),
                dependency("group_repository", "LocalGroupRepository", DurableLocal),
                dependency(
                    "permission_policy_repository",
                    "LocalPermissionPolicyRepository",
                    DurableLocal,
                ),
                dependency("comment_repository", "LocalCommentRepository", DurableLocal),
                dependency(
                    "review_workflow_repository",
                    "LocalReviewWorkflowRepository",
                    DurableLocal,
                ),
                dependency(
                    "document_lock_repository",
                    "LocalDocumentLockRepository",
                    DurableLocal,
                ),
                dependency("audit_store", "LocalAuditLogStore", DurableLocal),
                dependency("backup_store", "LocalBackupStore", DurableLocal),
                dependency("auth_policy", "AuthSessionPolicy", Policy),
                dependency("clock", "SystemClock", RuntimeUtility),
            ],
        }
    }
}
`;

test("durable dependency manifest audit accepts complete Phase 003 self-host manifest", () => {
  const audit = analyzeDurableDependencyManifestSources({
    runtimeSource: completeRuntimeSource,
  });

  assert.equal(audit.summary.requiredDurableDependencies, 15);
  assert.equal(audit.summary.durableLocalWired, 15);
  assert.equal(audit.summary.missingDurableDependencies, 0);
  assert.equal(audit.summary.wrongImplementation, 0);
  assert.equal(audit.summary.wrongDurability, 0);
  assert.equal(audit.findings.length, 0);
  assert.equal(audit.nextImplementationTarget, null);
});

test("durable dependency manifest audit reports missing and mismatched durable dependencies", () => {
  const audit = analyzeDurableDependencyManifestSources({
    runtimeSource: completeRuntimeSource
      .replace('dependency("backup_store", "LocalBackupStore", DurableLocal),', "")
      .replace(
        'dependency("audit_store", "LocalAuditLogStore", DurableLocal),',
        'dependency("audit_store", "FakeAuditLogStore", DurableLocal),',
      )
      .replace(
        'dependency("session_store", "LocalSessionStore", DurableLocal),',
        'dependency("session_store", "LocalSessionStore", VolatileLocal),',
      ),
  });

  assert.equal(audit.summary.durableLocalWired, 12);
  assert.equal(audit.summary.missingDurableDependencies, 1);
  assert.equal(audit.summary.wrongImplementation, 1);
  assert.equal(audit.summary.wrongDurability, 1);
  assert.deepEqual(
    audit.findings.map((finding) => finding.id),
    [
      "PHASE003_DURABLE_DEPENDENCY_MISSING",
      "PHASE003_DURABLE_DEPENDENCY_WRONG_IMPLEMENTATION",
      "PHASE003_DURABLE_DEPENDENCY_WRONG_DURABILITY",
    ],
  );
  assert.equal(audit.nextImplementationTarget.dependency, "backup_store");
});

test("durable dependency manifest audit rejects empty runtime source with stable code", () => {
  assert.throws(
    () => analyzeDurableDependencyManifestSources({ runtimeSource: "" }),
    (error) => error.code === DurableDependencyManifestAuditErrorCode.ManifestEmpty,
  );
});

test("durable dependency manifest audit state machine rejects invalid transitions", () => {
  assert.equal(
    transitionDurableDependencyManifestAuditState(
      DurableDependencyManifestAuditState.NotStarted,
      DurableDependencyManifestAuditEvent.Start,
    ),
    DurableDependencyManifestAuditState.ReadingSource,
  );
  assert.equal(
    transitionDurableDependencyManifestAuditState(
      DurableDependencyManifestAuditState.ReadingSource,
      DurableDependencyManifestAuditEvent.SourceLoaded,
    ),
    DurableDependencyManifestAuditState.Auditing,
  );
  assert.equal(
    transitionDurableDependencyManifestAuditState(
      DurableDependencyManifestAuditState.Auditing,
      DurableDependencyManifestAuditEvent.AuditComplete,
    ),
    DurableDependencyManifestAuditState.Reported,
  );
  assert.throws(
    () =>
      transitionDurableDependencyManifestAuditState(
        DurableDependencyManifestAuditState.NotStarted,
        DurableDependencyManifestAuditEvent.ReportWritten,
      ),
    (error) => error.code === DurableDependencyManifestAuditErrorCode.InvalidTransition,
  );
});

test("durable dependency manifest audit markdown keeps next dependency explicit", () => {
  const audit = analyzeDurableDependencyManifestSources({
    runtimeSource: completeRuntimeSource,
  });
  const markdown = renderDurableDependencyManifestAuditMarkdown(audit);

  assert.match(markdown, /# Phase 003 Durable Dependency Manifest Audit/);
  assert.match(markdown, /DurableLocal/);
  assert.match(markdown, /LocalBackupStore/);
  assert.match(markdown, /No durable dependency manifest gap was detected/);
  assert.match(markdown, /next dependency: `none`/);
});
