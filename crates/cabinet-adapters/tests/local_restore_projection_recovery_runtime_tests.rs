use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_create_document_revision_runtime::{
    LOCAL_DOCUMENT_VERSION_ROOT, LocalCreateDocumentRevisionRuntime,
};
use cabinet_adapters::local_current_document_revision_projection::LOCAL_CURRENT_DOCUMENT_REVISION_IDENTITY_ROOT;
use cabinet_adapters::local_document_repository::LocalDocumentRepository;
use cabinet_adapters::local_restore_document_revision_runtime::LocalRestoreDocumentRevisionRuntime;
use cabinet_adapters::local_restore_projection_recovery_runtime::LocalRestoreProjectionRecoveryRuntime;
use cabinet_adapters::local_update_document_revision_runtime::LocalUpdateDocumentRevisionRuntime;
use cabinet_adapters::local_version_store::LocalVersionStore;
use cabinet_domain::document::{DocumentBodyPolicy, DocumentId};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_repository::DocumentRepository;
use cabinet_ports::version_store::{HistoryPageRequest, VersionStore};
use cabinet_usecases::create_document_revision::CreateDocumentRevisionInput;
use cabinet_usecases::restore_document_revision::{
    RestoreDocumentRevisionError, RestoreDocumentRevisionInput,
};
use cabinet_usecases::update_document_revision::UpdateDocumentRevisionInput;

#[test]
fn restart_recovery_replays_current_projection_without_new_history() {
    let temp = TempRoot::new("restart");
    let mut create = LocalCreateDocumentRevisionRuntime::new(temp.path.clone(), policy());
    let target = create
        .execute(CreateDocumentRevisionInput::new(
            "create",
            "workspace-1",
            "doc-1",
            "old body",
            "local-user",
            "Create",
        ))
        .unwrap();
    let mut update = LocalUpdateDocumentRevisionRuntime::new(temp.path.clone(), policy());
    let current = update
        .execute(UpdateDocumentRevisionInput::new(
            "update",
            "workspace-1",
            "doc-1",
            target.version_id().as_str(),
            "new body",
            "local-user",
            "Update",
        ))
        .unwrap();
    let blocker = projection_identity_path(&temp);
    fs::remove_file(&blocker).unwrap();
    fs::create_dir(&blocker).unwrap();
    let error = LocalRestoreDocumentRevisionRuntime::new(temp.path.clone(), policy())
        .execute(RestoreDocumentRevisionInput::new(
            "restore",
            "workspace-1",
            "doc-1",
            target.version_id().as_str(),
            current.version_id().as_str(),
            "local-user",
            "Restore",
        ))
        .expect_err("projection failure");
    assert_eq!(error, RestoreDocumentRevisionError::RecoveryRequired);
    assert_eq!(history_count(&temp), 3);
    fs::remove_dir(blocker).unwrap();

    let recovered = LocalRestoreProjectionRecoveryRuntime::new(temp.path.clone(), policy())
        .recover(100)
        .expect("startup recovery");

    assert_eq!(recovered.recovered().len(), 1);
    assert_eq!(recovered.skipped_stale_count(), 0);
    assert_eq!(history_count(&temp), 3);
    assert_eq!(current_body(&temp).as_deref(), Some("old body"));

    let repeated = LocalRestoreProjectionRecoveryRuntime::new(temp.path.clone(), policy())
        .recover(100)
        .expect("idempotent restart");
    assert_eq!(repeated.recovered().len(), 1);
    assert_eq!(history_count(&temp), 3);
}

#[test]
fn restart_recovery_skips_restore_that_is_no_longer_authoritative_current() {
    let temp = TempRoot::new("stale");
    let mut create = LocalCreateDocumentRevisionRuntime::new(temp.path.clone(), policy());
    let target = create
        .execute(CreateDocumentRevisionInput::new(
            "create",
            "workspace-1",
            "doc-1",
            "old body",
            "local-user",
            "Create",
        ))
        .unwrap();
    let mut update = LocalUpdateDocumentRevisionRuntime::new(temp.path.clone(), policy());
    let current = update
        .execute(UpdateDocumentRevisionInput::new(
            "update",
            "workspace-1",
            "doc-1",
            target.version_id().as_str(),
            "new body",
            "local-user",
            "Update",
        ))
        .unwrap();
    let restored = LocalRestoreDocumentRevisionRuntime::new(temp.path.clone(), policy())
        .execute(RestoreDocumentRevisionInput::new(
            "restore",
            "workspace-1",
            "doc-1",
            target.version_id().as_str(),
            current.version_id().as_str(),
            "local-user",
            "Restore",
        ))
        .unwrap();
    let newest = update
        .execute(UpdateDocumentRevisionInput::new(
            "update-after-restore",
            "workspace-1",
            "doc-1",
            restored.version_id().as_str(),
            "newest body",
            "local-user",
            "Update",
        ))
        .unwrap();

    let output = LocalRestoreProjectionRecoveryRuntime::new(temp.path.clone(), policy())
        .recover(100)
        .expect("skip stale");

    assert!(output.recovered().is_empty());
    assert_eq!(output.skipped_stale_count(), 1);
    assert_eq!(current_body(&temp).as_deref(), Some("newest body"));
    assert_eq!(history_count(&temp), 4);
    assert_ne!(newest.version_id(), restored.version_id());
}

fn current_body(temp: &TempRoot) -> Option<String> {
    LocalDocumentRepository::with_body_policy(temp.path.join("authoring-current"), policy())
        .get_current_by_id(&workspace(), &document())
        .unwrap()
        .map(|record| record.body().as_str().to_string())
}

fn history_count(temp: &TempRoot) -> usize {
    LocalVersionStore::with_body_policy(temp.path.join(LOCAL_DOCUMENT_VERSION_ROOT), policy())
        .list_history(
            &workspace(),
            &document(),
            HistoryPageRequest::first(20).unwrap(),
        )
        .unwrap()
        .entries()
        .len()
}

fn projection_identity_path(temp: &TempRoot) -> PathBuf {
    temp.path
        .join(LOCAL_CURRENT_DOCUMENT_REVISION_IDENTITY_ROOT)
        .join(hex("workspace-1"))
        .join(hex("doc-1"))
        .join("current.projection")
}

fn hex(value: &str) -> String {
    value.bytes().map(|byte| format!("{byte:02x}")).collect()
}

fn workspace() -> WorkspaceId {
    WorkspaceId::new("workspace-1").unwrap()
}
fn document() -> DocumentId {
    DocumentId::new("doc-1").unwrap()
}
fn policy() -> DocumentBodyPolicy {
    DocumentBodyPolicy::new(4096).unwrap()
}

struct TempRoot {
    path: PathBuf,
}
impl TempRoot {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "sponzey-restore-projection-recovery-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}
impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
