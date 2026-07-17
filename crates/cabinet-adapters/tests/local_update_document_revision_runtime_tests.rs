use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_create_document_revision_runtime::{
    LOCAL_DOCUMENT_JOURNAL_ROOT, LOCAL_DOCUMENT_POINTER_ROOT, LOCAL_DOCUMENT_VERSION_ROOT,
};
use cabinet_adapters::local_current_document_revision_projection::{
    LOCAL_CURRENT_DOCUMENT_PROJECTION_ROOT, LOCAL_CURRENT_DOCUMENT_REVISION_IDENTITY_ROOT,
    LocalCurrentDocumentRevisionProjectionWriter,
};
use cabinet_adapters::local_current_document_version_pointer::LocalCurrentDocumentVersionPointer;
use cabinet_adapters::local_document_operation_journal::LocalDocumentOperationJournal;
use cabinet_adapters::local_document_repository::LocalDocumentRepository;
use cabinet_adapters::local_update_document_revision_runtime::LocalUpdateDocumentRevisionRuntime;
use cabinet_adapters::local_version_store::LocalVersionStore;
use cabinet_domain::asset::{AssetId, AssetReference};
use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::document_revision::DocumentOperationId;
use cabinet_domain::version::{
    AttachmentSnapshotState, DocumentRevisionNumber, DocumentSnapshotRef, VersionAuthor,
    VersionEntry, VersionId, VersionSummary,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_version::CurrentDocumentVersionPointerPort;
use cabinet_ports::document_repository::DocumentRepository;
use cabinet_ports::document_revision_commit::{
    DocumentOperationJournalPort, DocumentOperationJournalState,
};
use cabinet_ports::version_store::{
    HistoryPageRequest, VersionRecord, VersionSnapshot, VersionStore,
};
use cabinet_usecases::document_revision_commit::CommitDocumentRevisionOutcomeKind;
use cabinet_usecases::project_current_document_revision::{
    ProjectCurrentDocumentRevisionInput, ProjectCurrentDocumentRevisionUsecase,
};
use cabinet_usecases::update_document_revision::{
    UpdateDocumentRevisionError, UpdateDocumentRevisionInput,
};

#[test]
fn local_update_preserves_attachment_and_replays_after_restart() {
    let temp = TempRoot::new("update-restart");
    seed_revision_one(
        &temp,
        AttachmentSnapshotState::known(vec![asset_reference()]).expect("known"),
    );
    let command = input("operation-update-1", "version-1", "# Updated\r\nbody");
    let mut runtime = build_runtime(&temp);

    let updated = runtime.execute(command.clone()).expect("update");
    assert_eq!(updated.kind(), CommitDocumentRevisionOutcomeKind::Fresh);
    assert_eq!(updated.revision_number().value(), 2);
    let updated_version = updated.version_id().clone();
    drop(runtime);
    assert_updated_state(&temp, &updated_version, "# Updated\nbody", 2);

    let mut restarted = build_runtime(&temp);
    let replayed = restarted.execute(command).expect("restart replay");
    assert_eq!(replayed.kind(), CommitDocumentRevisionOutcomeKind::Replayed);
    assert_eq!(replayed.version_id(), &updated_version);
    assert_eq!(replayed.revision_number().value(), 2);
    drop(restarted);
    assert_updated_state(&temp, &updated_version, "# Updated\nbody", 2);

    let journal = LocalDocumentOperationJournal::new(temp.path.join(LOCAL_DOCUMENT_JOURNAL_ROOT));
    assert_eq!(
        journal
            .load_operation(&DocumentOperationId::new("operation-update-1").expect("operation"))
            .expect("journal")
            .expect("record")
            .state(),
        DocumentOperationJournalState::Committed
    );
}

#[test]
fn stale_or_missing_update_does_not_change_current_revision() {
    let temp = TempRoot::new("stale-missing");
    seed_revision_one(&temp, AttachmentSnapshotState::legacy_unknown());
    let mut runtime = build_runtime(&temp);
    let updated = runtime
        .execute(input("operation-update-1", "version-1", "current body"))
        .expect("first update");
    let current_version = updated.version_id().clone();

    let stale_error = runtime
        .execute(input("operation-stale", "version-1", "stale body"))
        .expect_err("stale expected current");
    assert_eq!(stale_error, UpdateDocumentRevisionError::CommitConflict);
    let missing_error = runtime
        .execute(input("operation-missing", "missing-version", "missing"))
        .expect_err("missing snapshot");
    assert_eq!(missing_error, UpdateDocumentRevisionError::NotFound);
    drop(runtime);

    assert_updated_state(&temp, &current_version, "current body", 2);
    let versions = LocalVersionStore::new(temp.path.join(LOCAL_DOCUMENT_VERSION_ROOT));
    let snapshot = versions
        .get_version_snapshot(&workspace(), &document(), &current_version)
        .expect("snapshot")
        .expect("current snapshot");
    assert!(snapshot.attachment_state().is_legacy_unknown());
}

#[test]
fn projection_failure_is_recovery_required_and_retry_preserves_path_without_new_revision() {
    let temp = TempRoot::new("projection-recovery");
    seed_revision_one(&temp, AttachmentSnapshotState::known(Vec::new()).unwrap());
    let identity = identity_path(&temp);
    fs::remove_file(&identity).expect("remove seed identity");
    fs::create_dir(&identity).expect("block identity file");
    let command = input("operation-update-1", "version-1", "복구된 제목\n새 본문");
    let mut runtime = build_runtime(&temp);

    let error = runtime
        .execute(command.clone())
        .expect_err("post-primary projection failure");
    assert_eq!(error, UpdateDocumentRevisionError::RecoveryRequired);
    assert_history_count(&temp, 2);

    fs::remove_dir(identity).expect("remove blocker");
    drop(runtime);
    let repaired = build_runtime(&temp)
        .execute(command)
        .expect("retry repairs projection");

    assert_eq!(repaired.kind(), CommitDocumentRevisionOutcomeKind::Replayed);
    assert_eq!(repaired.revision_number().value(), 2);
    assert_history_count(&temp, 2);
    let current = projected_current(&temp).expect("repaired projection");
    assert_eq!(current.metadata().title().as_str(), "복구된 제목");
    assert_eq!(current.path().as_str(), "notes/original.md");
    assert_eq!(current.body().as_str(), "복구된 제목\n새 본문");
}

#[test]
fn missing_current_projection_stops_before_primary_update() {
    let temp = TempRoot::new("missing-current");
    seed_version_and_pointer(&temp, AttachmentSnapshotState::known(Vec::new()).unwrap());
    let mut runtime = build_runtime(&temp);

    let error = runtime
        .execute(input("operation-update-1", "version-1", "새 제목\n본문"))
        .expect_err("missing current projection");

    assert_eq!(error, UpdateDocumentRevisionError::RecoveryRequired);
    assert_history_count(&temp, 1);
    assert_eq!(current_pointer(&temp).unwrap().as_str(), "version-1");
}

fn seed_revision_one(temp: &TempRoot, attachment_state: AttachmentSnapshotState) {
    let record = seed_version_and_pointer(temp, attachment_state);
    let mut projection = LocalCurrentDocumentRevisionProjectionWriter::new(
        temp.path.clone(),
        DocumentBodyPolicy::new(1024).unwrap(),
    );
    ProjectCurrentDocumentRevisionUsecase::new()
        .execute(
            ProjectCurrentDocumentRevisionInput::new("workspace-1", "notes/original.md", record),
            &mut projection,
        )
        .expect("seed current projection");
}

fn seed_version_and_pointer(
    temp: &TempRoot,
    attachment_state: AttachmentSnapshotState,
) -> VersionRecord {
    let mut versions = LocalVersionStore::new(temp.path.join(LOCAL_DOCUMENT_VERSION_ROOT));
    let version = VersionId::new("version-1").expect("version");
    let snapshot_ref = DocumentSnapshotRef::new("snapshot-1").expect("snapshot");
    let entry = VersionEntry::new(
        version.clone(),
        document(),
        snapshot_ref.clone(),
        VersionAuthor::new("local-user").expect("author"),
        VersionSummary::new("Seed").expect("summary"),
    )
    .expect("entry")
    .with_created_at_epoch_ms(1)
    .expect("timestamp")
    .with_revision_number(DocumentRevisionNumber::new(1).expect("revision"))
    .expect("revision assignment");
    let snapshot = VersionSnapshot::with_attachment_state(
        document(),
        snapshot_ref,
        DocumentBody::new("seed body", DocumentBodyPolicy::new(1024).expect("policy"))
            .expect("body"),
        attachment_state,
    );
    let record = VersionRecord::new(entry, snapshot).expect("record");
    versions
        .append_version(&workspace(), record.clone())
        .expect("seed version");
    let mut pointer =
        LocalCurrentDocumentVersionPointer::new(temp.path.join(LOCAL_DOCUMENT_POINTER_ROOT));
    pointer
        .compare_and_set_current_version(&workspace(), &document(), None, version)
        .expect("seed pointer");
    record
}

fn assert_updated_state(
    temp: &TempRoot,
    expected_version: &VersionId,
    expected_body: &str,
    expected_history_len: usize,
) {
    let pointer =
        LocalCurrentDocumentVersionPointer::new(temp.path.join(LOCAL_DOCUMENT_POINTER_ROOT));
    assert_eq!(
        pointer
            .load_current_version(&workspace(), &document())
            .expect("pointer")
            .as_ref(),
        Some(expected_version)
    );
    let versions = LocalVersionStore::new(temp.path.join(LOCAL_DOCUMENT_VERSION_ROOT));
    let history = versions
        .list_history(
            &workspace(),
            &document(),
            HistoryPageRequest::first(10).expect("request"),
        )
        .expect("history");
    assert_eq!(history.entries().len(), expected_history_len);
    assert_eq!(
        history.entries().last().expect("latest").version_id(),
        expected_version
    );
    let snapshot = versions
        .get_version_snapshot(&workspace(), &document(), expected_version)
        .expect("snapshot")
        .expect("current snapshot");
    assert_eq!(snapshot.body().as_str(), expected_body);
    if !snapshot.attachment_state().is_legacy_unknown() {
        assert_eq!(
            snapshot
                .attachment_state()
                .references()
                .expect("known attachments"),
            &[asset_reference()]
        );
    }
    let current = projected_current(temp).expect("current projection");
    assert_eq!(current.body().as_str(), expected_body);
    assert_eq!(current.path().as_str(), "notes/original.md");
}

fn projected_current(
    temp: &TempRoot,
) -> Option<cabinet_ports::document_repository::CurrentDocumentRecord> {
    LocalDocumentRepository::new(temp.path.join(LOCAL_CURRENT_DOCUMENT_PROJECTION_ROOT))
        .get_current_by_id(&workspace(), &document())
        .expect("projection read")
}

fn current_pointer(temp: &TempRoot) -> Option<VersionId> {
    LocalCurrentDocumentVersionPointer::new(temp.path.join(LOCAL_DOCUMENT_POINTER_ROOT))
        .load_current_version(&workspace(), &document())
        .unwrap()
}

fn assert_history_count(temp: &TempRoot, expected: usize) {
    let history = LocalVersionStore::new(temp.path.join(LOCAL_DOCUMENT_VERSION_ROOT))
        .list_history(
            &workspace(),
            &document(),
            HistoryPageRequest::first(10).unwrap(),
        )
        .unwrap();
    assert_eq!(history.entries().len(), expected);
}

fn identity_path(temp: &TempRoot) -> PathBuf {
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

fn build_runtime(temp: &TempRoot) -> LocalUpdateDocumentRevisionRuntime {
    LocalUpdateDocumentRevisionRuntime::new(
        temp.path.clone(),
        DocumentBodyPolicy::new(1024).expect("policy"),
    )
}

fn input(operation: &str, expected: &str, body: &str) -> UpdateDocumentRevisionInput {
    UpdateDocumentRevisionInput::new(
        operation,
        "workspace-1",
        "doc-1",
        expected,
        body,
        "local-user",
        "Update document",
    )
}

fn asset_reference() -> AssetReference {
    AssetReference::new(
        AssetId::from_sha256_hex(&"a".repeat(64)).expect("asset"),
        "Attachment",
    )
    .expect("reference")
}

fn workspace() -> WorkspaceId {
    WorkspaceId::new("workspace-1").expect("workspace")
}

fn document() -> DocumentId {
    DocumentId::new("doc-1").expect("document")
}

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "sponzey-local-update-revision-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temp root");
        Self { path }
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
