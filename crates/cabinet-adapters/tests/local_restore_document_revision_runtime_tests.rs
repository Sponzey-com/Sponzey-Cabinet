use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

use cabinet_adapters::durable_asset_association_catalog::DurableAssetAssociationCatalog;
use cabinet_adapters::local_create_document_revision_runtime::{
    LOCAL_DOCUMENT_POINTER_ROOT, LOCAL_DOCUMENT_VERSION_ROOT, LocalCreateDocumentRevisionRuntime,
};
use cabinet_adapters::local_current_document_revision_projection::LOCAL_CURRENT_DOCUMENT_REVISION_IDENTITY_ROOT;
use cabinet_adapters::local_current_document_version_pointer::LocalCurrentDocumentVersionPointer;
use cabinet_adapters::local_mutate_document_attachments_runtime::LocalMutateDocumentAttachmentsRuntime;
use cabinet_adapters::local_restore_document_revision_runtime::LocalRestoreDocumentRevisionRuntime;
use cabinet_adapters::local_update_document_revision_runtime::LocalUpdateDocumentRevisionRuntime;
use cabinet_adapters::local_version_store::LocalVersionStore;
use cabinet_domain::asset::AssetId;
use cabinet_domain::document::{DocumentBodyPolicy, DocumentId};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_association_catalog::AssetAssociationCatalog;
use cabinet_ports::current_document_version::CurrentDocumentVersionPointerPort;
use cabinet_ports::version_store::{HistoryPageRequest, VersionStore};
use cabinet_usecases::create_document_revision::CreateDocumentRevisionInput;
use cabinet_usecases::document_revision_commit::CommitDocumentRevisionOutcomeKind;
use cabinet_usecases::mutate_document_attachments::MutateDocumentAttachmentsInput;
use cabinet_usecases::restore_document_revision::{
    RestoreDocumentRevisionError, RestoreDocumentRevisionInput,
};
use cabinet_usecases::restore_product_log::{RestoreProductEvent, RestoreProductLogger};
use cabinet_usecases::update_document_revision::UpdateDocumentRevisionInput;

#[test]
fn local_restore_commits_new_revision_projects_attachments_and_replays_after_restart() {
    let temp = TempRoot::new();
    seed_asset_object(&temp);
    let mut create = LocalCreateDocumentRevisionRuntime::new(temp.path.clone(), policy());
    let created = create
        .execute(CreateDocumentRevisionInput::new(
            "create-1",
            "workspace-1",
            "doc-1",
            "# 과거 제목\n과거 본문",
            "local-user",
            "Create",
        ))
        .unwrap();
    let mut attachments = LocalMutateDocumentAttachmentsRuntime::new(temp.path.clone(), policy());
    let linked = attachments
        .execute(MutateDocumentAttachmentsInput::link(
            "link-1",
            "workspace-1",
            "doc-1",
            created.version_id().as_str(),
            asset_id().as_str(),
            "설계 자료",
            "local-user",
            "Attach",
        ))
        .unwrap();
    let target_version = linked.version_id().clone();
    let unlinked = attachments
        .execute(MutateDocumentAttachmentsInput::unlink(
            "unlink-1",
            "workspace-1",
            "doc-1",
            target_version.as_str(),
            asset_id().as_str(),
            "local-user",
            "Detach",
        ))
        .unwrap();
    assert!(associations(&temp).is_empty());
    let command = RestoreDocumentRevisionInput::new(
        "restore-1",
        "workspace-1",
        "doc-1",
        target_version.as_str(),
        unlinked.version_id().as_str(),
        "local-user",
        "Restore",
    );

    let mut logger = RecordingRestoreLogger::default();
    let restored = LocalRestoreDocumentRevisionRuntime::new(temp.path.clone(), policy())
        .execute_with_logger(command.clone(), &mut logger)
        .expect("restore");
    assert_eq!(
        logger.events,
        vec![
            RestoreProductEvent::Requested,
            RestoreProductEvent::PrimaryCommitted
        ]
    );
    assert_eq!(restored.kind(), CommitDocumentRevisionOutcomeKind::Fresh);
    assert_ne!(restored.version_id(), &target_version);
    assert_eq!(current_version(&temp), *restored.version_id());
    assert_eq!(history_count(&temp), 4);
    assert_eq!(
        associations(&temp),
        vec![(asset_id().as_str().to_string(), "설계 자료".to_string())]
    );

    let replayed = LocalRestoreDocumentRevisionRuntime::new(temp.path.clone(), policy())
        .execute(command)
        .expect("replay");
    assert_eq!(replayed.kind(), CommitDocumentRevisionOutcomeKind::Replayed);
    assert_eq!(replayed.version_id(), restored.version_id());
    assert_eq!(history_count(&temp), 4);
}

#[test]
fn missing_target_asset_blocks_restore_before_new_history_or_pointer_write() {
    let temp = TempRoot::new();
    let mut create = LocalCreateDocumentRevisionRuntime::new(temp.path.clone(), policy());
    let created = create
        .execute(CreateDocumentRevisionInput::new(
            "create-missing",
            "workspace-1",
            "doc-1",
            "body",
            "local-user",
            "Create",
        ))
        .unwrap();
    let mut attachments = LocalMutateDocumentAttachmentsRuntime::new(temp.path.clone(), policy());
    let target = attachments
        .execute(MutateDocumentAttachmentsInput::link(
            "link-missing",
            "workspace-1",
            "doc-1",
            created.version_id().as_str(),
            asset_id().as_str(),
            "누락 자료",
            "local-user",
            "Attach",
        ))
        .unwrap();
    let current = attachments
        .execute(MutateDocumentAttachmentsInput::unlink(
            "unlink-missing",
            "workspace-1",
            "doc-1",
            target.version_id().as_str(),
            asset_id().as_str(),
            "local-user",
            "Detach",
        ))
        .unwrap();

    let mut logger = RecordingRestoreLogger::default();
    let error = LocalRestoreDocumentRevisionRuntime::new(temp.path.clone(), policy())
        .execute_with_logger(
            RestoreDocumentRevisionInput::new(
                "restore-missing",
                "workspace-1",
                "doc-1",
                target.version_id().as_str(),
                current.version_id().as_str(),
                "local-user",
                "Restore",
            ),
            &mut logger,
        )
        .expect_err("missing asset");

    assert_eq!(error, RestoreDocumentRevisionError::MissingDependency);
    assert_eq!(
        logger.events,
        vec![
            RestoreProductEvent::Requested,
            RestoreProductEvent::BlockedMissingAsset
        ]
    );
    assert_eq!(current_version(&temp), *current.version_id());
    assert_eq!(history_count(&temp), 3);
    assert!(associations(&temp).is_empty());
}

#[test]
fn stale_local_restore_does_not_publish_revision_or_attachment_projection() {
    let temp = TempRoot::new();
    let mut create = LocalCreateDocumentRevisionRuntime::new(temp.path.clone(), policy());
    let created = create
        .execute(CreateDocumentRevisionInput::new(
            "create-1",
            "workspace-1",
            "doc-1",
            "body",
            "local-user",
            "Create",
        ))
        .unwrap();
    let before = current_version(&temp);

    let mut logger = RecordingRestoreLogger::default();
    let error = LocalRestoreDocumentRevisionRuntime::new(temp.path.clone(), policy())
        .execute_with_logger(
            RestoreDocumentRevisionInput::new(
                "restore-stale",
                "workspace-1",
                "doc-1",
                created.version_id().as_str(),
                "stale-version",
                "local-user",
                "Restore",
            ),
            &mut logger,
        )
        .expect_err("stale");

    assert_eq!(error, RestoreDocumentRevisionError::CommitConflict);
    assert_eq!(
        logger.events,
        vec![
            RestoreProductEvent::Requested,
            RestoreProductEvent::Conflict
        ]
    );
    assert_eq!(current_version(&temp), before);
    assert_eq!(history_count(&temp), 1);
}

#[test]
fn projection_failure_logs_primary_commit_before_recovery_required() {
    let temp = TempRoot::new();
    let mut create = LocalCreateDocumentRevisionRuntime::new(temp.path.clone(), policy());
    let target = create
        .execute(CreateDocumentRevisionInput::new(
            "create-recovery-log",
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
            "update-recovery-log",
            "workspace-1",
            "doc-1",
            target.version_id().as_str(),
            "current body",
            "local-user",
            "Update",
        ))
        .unwrap();
    let identity = projection_identity_path(&temp);
    fs::remove_file(&identity).unwrap();
    fs::create_dir(&identity).unwrap();
    let mut logger = RecordingRestoreLogger::default();

    let error = LocalRestoreDocumentRevisionRuntime::new(temp.path.clone(), policy())
        .execute_with_logger(
            RestoreDocumentRevisionInput::new(
                "restore-recovery-log",
                "workspace-1",
                "doc-1",
                target.version_id().as_str(),
                current.version_id().as_str(),
                "local-user",
                "Restore",
            ),
            &mut logger,
        )
        .expect_err("projection must require recovery");

    assert_eq!(error, RestoreDocumentRevisionError::RecoveryRequired);
    assert_eq!(
        logger.events,
        vec![
            RestoreProductEvent::Requested,
            RestoreProductEvent::PrimaryCommitted,
            RestoreProductEvent::RecoveryRequired,
        ]
    );
    assert_eq!(history_count(&temp), 3);
}

fn current_version(temp: &TempRoot) -> cabinet_domain::version::VersionId {
    LocalCurrentDocumentVersionPointer::new(temp.path.join(LOCAL_DOCUMENT_POINTER_ROOT))
        .load_current_version(&workspace(), &document())
        .unwrap()
        .unwrap()
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

fn associations(temp: &TempRoot) -> Vec<(String, String)> {
    DurableAssetAssociationCatalog::new(temp.path.clone())
        .list_assets(&workspace(), &document(), 20)
        .unwrap()
        .into_iter()
        .map(|value| {
            (
                value.asset_id().as_str().to_string(),
                value.label().to_string(),
            )
        })
        .collect()
}

fn asset_id() -> AssetId {
    AssetId::from_sha256_hex(&"a".repeat(64)).unwrap()
}

fn seed_asset_object(temp: &TempRoot) {
    let id = asset_id();
    let workspace_hex = "workspace-1"
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let path = temp
        .path
        .join("assets/objects")
        .join(workspace_hex)
        .join(&id.as_str()[..2])
        .join(format!("{}.bin", id.as_str()));
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, b"asset").unwrap();
}

fn projection_identity_path(temp: &TempRoot) -> PathBuf {
    temp.path
        .join(LOCAL_CURRENT_DOCUMENT_REVISION_IDENTITY_ROOT)
        .join(hex("workspace-1"))
        .join(hex("doc-1"))
        .join("current.projection")
}

fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[derive(Default)]
struct RecordingRestoreLogger {
    events: Vec<RestoreProductEvent>,
}

impl RestoreProductLogger for RecordingRestoreLogger {
    fn write_restore_product(&mut self, event: RestoreProductEvent) {
        self.events.push(event);
    }
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
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "sponzey-local-restore-runtime-{}-{nonce}-{}",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed),
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
