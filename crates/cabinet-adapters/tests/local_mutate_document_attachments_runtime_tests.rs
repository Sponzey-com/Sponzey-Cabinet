use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_asset_association_catalog::DurableAssetAssociationCatalog;
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
use cabinet_adapters::local_imported_asset_document_revision_linker::LocalImportedAssetDocumentRevisionLinker;
use cabinet_adapters::local_mutate_document_attachments_runtime::LocalMutateDocumentAttachmentsRuntime;
use cabinet_adapters::local_version_store::LocalVersionStore;
use cabinet_domain::asset::{AssetAssociation, AssetId, AssetReference};
use cabinet_domain::attachment_snapshot_mutation::AttachmentSnapshotDelta;
use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::document_revision::DocumentOperationId;
use cabinet_domain::version::{
    AttachmentSnapshotState, DocumentRevisionNumber, DocumentSnapshotRef, VersionAuthor,
    VersionEntry, VersionId, VersionSummary,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_association_catalog::AssetAssociationCatalog;
use cabinet_ports::committed_version_record_reader::CommittedVersionRecordReader;
use cabinet_ports::current_document_version::CurrentDocumentVersionPointerPort;
use cabinet_ports::document_repository::DocumentRepository;
use cabinet_ports::document_revision_commit::{
    DocumentOperationJournalPort, DocumentOperationJournalState,
};
use cabinet_ports::imported_asset_document_link::{
    ImportedAssetDocumentLinkError, ImportedAssetDocumentLinkOutcome, ImportedAssetDocumentLinkPort,
};
use cabinet_ports::version_store::{
    HistoryPageRequest, VersionRecord, VersionSnapshot, VersionStore,
};
use cabinet_usecases::mutate_document_attachments::{
    MutateDocumentAttachmentsError, MutateDocumentAttachmentsInput,
    MutateDocumentAttachmentsOutcomeKind,
};
use cabinet_usecases::project_current_document_revision::{
    ProjectCurrentDocumentRevisionInput, ProjectCurrentDocumentRevisionUsecase,
};

#[test]
fn local_link_survives_restart_and_same_operation_replays_without_duplicate_history() {
    let temp = TempRoot::new("link-restart");
    seed_current(&temp, known(Vec::new()));
    let command = link_input("operation-link", "version-1", 'a', "설계 자료");

    let fresh = runtime(&temp).execute(command.clone()).expect("fresh link");
    assert_eq!(fresh.kind(), MutateDocumentAttachmentsOutcomeKind::Fresh);
    assert_eq!(fresh.delta(), AttachmentSnapshotDelta::Linked);
    assert_eq!(fresh.revision_number().value(), 2);
    let linked_version = fresh.version_id().clone();
    assert_current_snapshot(&temp, &linked_version, &[reference('a', "설계 자료")], 2);
    assert_associations(&temp, &[('a', "설계 자료")]);

    let replayed = runtime(&temp).execute(command).expect("restart replay");
    assert_eq!(
        replayed.kind(),
        MutateDocumentAttachmentsOutcomeKind::Replayed
    );
    assert_eq!(replayed.version_id(), &linked_version);
    assert_current_snapshot(&temp, &linked_version, &[reference('a', "설계 자료")], 2);
    assert_associations(&temp, &[('a', "설계 자료")]);
    assert_eq!(projected_path(&temp).as_deref(), Some("notes/original.md"));

    let journal = LocalDocumentOperationJournal::new(temp.path.join(LOCAL_DOCUMENT_JOURNAL_ROOT));
    assert_eq!(
        journal
            .load_operation(&DocumentOperationId::new("operation-link").unwrap())
            .unwrap()
            .unwrap()
            .state(),
        DocumentOperationJournalState::Committed
    );
}

#[test]
fn local_unlink_preserves_other_attachment_and_document_body() {
    let temp = TempRoot::new("unlink");
    seed_current(&temp, known(vec![reference('a', "A"), reference('b', "B")]));

    let output = runtime(&temp)
        .execute(unlink_input("operation-unlink", "version-1", 'a'))
        .expect("unlink");

    assert_eq!(output.delta(), AttachmentSnapshotDelta::Unlinked);
    assert_current_snapshot(&temp, output.version_id(), &[reference('b', "B")], 2);
    assert_associations(&temp, &[('b', "B")]);
    let record = current_record(&temp);
    assert_eq!(record.snapshot().body().as_str(), "첫 번째 문서\n본문\n");
}

#[test]
fn stale_mutation_leaves_durable_state_unchanged() {
    let stale = TempRoot::new("stale");
    seed_current(&stale, known(Vec::new()));
    let first = runtime(&stale)
        .execute(link_input("operation-first", "version-1", 'a', "A"))
        .expect("first");
    let current = first.version_id().clone();

    let stale_error = runtime(&stale)
        .execute(link_input("operation-stale", "version-1", 'b', "B"))
        .expect_err("stale");
    assert_eq!(stale_error, MutateDocumentAttachmentsError::CommitConflict);
    assert_current_snapshot(&stale, &current, &[reference('a', "A")], 2);
}

#[test]
fn legacy_current_uses_empty_durable_association_baseline_and_appends_new_version() {
    let legacy = TempRoot::new("legacy-empty-baseline");
    seed_current(&legacy, AttachmentSnapshotState::legacy_unknown());
    let legacy_body = fs::read(version_body_path(&legacy, "version-1")).unwrap();

    let linked = runtime(&legacy)
        .execute(link_input("operation-legacy", "version-1", 'a', "A"))
        .expect("legacy baseline migration");

    assert_eq!(linked.delta(), AttachmentSnapshotDelta::Linked);
    assert_current_snapshot(&legacy, linked.version_id(), &[reference('a', "A")], 2);
    assert_associations(&legacy, &[('a', "A")]);
    assert_eq!(
        fs::read(version_body_path(&legacy, "version-1")).unwrap(),
        legacy_body
    );
    assert!(!version_attachments_path(&legacy, "version-1").exists());
}

#[test]
fn legacy_current_preserves_existing_durable_associations_in_new_version() {
    let legacy = TempRoot::new("legacy-existing-baseline");
    seed_current(&legacy, AttachmentSnapshotState::legacy_unknown());
    DurableAssetAssociationCatalog::new(legacy.path.clone())
        .link(
            &workspace(),
            AssetAssociation::new(asset_id('a'), document(), "Existing").unwrap(),
        )
        .unwrap();

    let linked = runtime(&legacy)
        .execute(link_input("operation-legacy", "version-1", 'b', "New"))
        .expect("preserved legacy baseline");

    assert_current_snapshot(
        &legacy,
        linked.version_id(),
        &[reference('a', "Existing"), reference('b', "New")],
        2,
    );
    assert_associations(&legacy, &[('a', "Existing"), ('b', "New")]);
}

#[test]
fn projection_failure_retry_repairs_projection_without_duplicate_revision() {
    let temp = TempRoot::new("projection-retry");
    seed_current(&temp, known(Vec::new()));
    let identity = projection_identity_path(&temp);
    fs::remove_file(&identity).expect("remove identity");
    fs::create_dir(&identity).expect("block projection identity");
    let command = link_input("operation-link", "version-1", 'a', "A");

    let error = runtime(&temp)
        .execute(command.clone())
        .expect_err("projection failure");
    assert_eq!(error, MutateDocumentAttachmentsError::RecoveryRequired);
    assert_eq!(history_count(&temp), 2);

    fs::remove_dir(identity).expect("remove blocker");
    let repaired = runtime(&temp).execute(command).expect("repair replay");
    assert_eq!(
        repaired.kind(),
        MutateDocumentAttachmentsOutcomeKind::Replayed
    );
    assert_eq!(history_count(&temp), 2);
    assert_current_snapshot(&temp, repaired.version_id(), &[reference('a', "A")], 2);
    assert_eq!(projected_path(&temp).as_deref(), Some("notes/original.md"));
}

#[test]
fn attachment_projection_failure_replays_primary_commit_and_resumes_association() {
    let temp = TempRoot::new("association-retry");
    seed_current(&temp, known(Vec::new()));
    let blocked_asset_root = association_asset_root(&temp, 'a');
    fs::create_dir_all(blocked_asset_root.parent().unwrap()).unwrap();
    fs::write(&blocked_asset_root, b"block asset association directory").unwrap();
    let command = link_input("operation-link", "version-1", 'a', "A");

    let error = runtime(&temp)
        .execute(command.clone())
        .expect_err("association projection failure");
    assert_eq!(error, MutateDocumentAttachmentsError::RecoveryRequired);
    assert_eq!(history_count(&temp), 2);

    fs::remove_file(blocked_asset_root).unwrap();
    let repaired = runtime(&temp).execute(command).expect("association repair");
    assert_eq!(
        repaired.kind(),
        MutateDocumentAttachmentsOutcomeKind::Replayed
    );
    assert_eq!(history_count(&temp), 2);
    assert_associations(&temp, &[('a', "A")]);
    assert_current_snapshot(&temp, repaired.version_id(), &[reference('a', "A")], 2);
}

#[test]
fn startup_recovery_skips_stale_attachment_candidate_and_reapplies_current_unlink() {
    let temp = TempRoot::new("startup-current-guard");
    seed_current(&temp, known(Vec::new()));
    let linked = runtime(&temp)
        .execute(link_input("operation-link", "version-1", 'a', "A"))
        .expect("link");
    let unlinked = runtime(&temp)
        .execute(unlink_input(
            "operation-unlink",
            linked.version_id().as_str(),
            'a',
        ))
        .expect("unlink");
    assert_associations(&temp, &[]);

    DurableAssetAssociationCatalog::new(temp.path.clone())
        .link(
            &workspace(),
            AssetAssociation::new(asset_id('a'), document(), "stale association").unwrap(),
        )
        .unwrap();
    assert_associations(&temp, &[('a', "stale association")]);

    let recovered = runtime(&temp).recover_committed(1000).expect("recover");
    assert_eq!(recovered.recovered().len(), 1);
    assert_eq!(recovered.skipped_stale_count(), 1);
    assert_eq!(recovered.recovered()[0].version_id(), unlinked.version_id());
    assert_associations(&temp, &[]);
    assert_eq!(history_count(&temp), 3);

    let repeated = runtime(&temp).recover_committed(1000).expect("repeat");
    assert_eq!(repeated.recovered().len(), 1);
    assert_eq!(repeated.skipped_stale_count(), 1);
    assert_associations(&temp, &[]);
    assert_eq!(history_count(&temp), 3);
}

#[test]
fn imported_asset_linker_commits_once_replays_and_rejects_stale_current() {
    let temp = TempRoot::new("imported-linker");
    seed_current(&temp, known(Vec::new()));
    let association = AssetAssociation::new(asset_id('a'), document(), "Imported spec").unwrap();
    let mut linker = LocalImportedAssetDocumentRevisionLinker::new(
        runtime(&temp),
        "attachment-import-1",
        "version-1",
        "local-user",
        "첨부 파일 가져오기",
    );

    assert_eq!(
        linker
            .link_imported_asset(&workspace(), association.clone())
            .unwrap(),
        ImportedAssetDocumentLinkOutcome::Linked
    );
    assert_eq!(
        linker
            .link_imported_asset(&workspace(), association)
            .unwrap(),
        ImportedAssetDocumentLinkOutcome::AlreadyLinked
    );
    assert_eq!(history_count(&temp), 2);
    assert_associations(&temp, &[('a', "Imported spec")]);

    let stale_association =
        AssetAssociation::new(asset_id('b'), document(), "Stale asset").unwrap();
    let mut stale = LocalImportedAssetDocumentRevisionLinker::new(
        runtime(&temp),
        "attachment-import-stale",
        "version-1",
        "local-user",
        "첨부 파일 가져오기",
    );
    assert_eq!(
        stale
            .link_imported_asset(&workspace(), stale_association)
            .unwrap_err(),
        ImportedAssetDocumentLinkError::CurrentConflict
    );
    assert_eq!(history_count(&temp), 2);
    assert_associations(&temp, &[('a', "Imported spec")]);
}

fn runtime(temp: &TempRoot) -> LocalMutateDocumentAttachmentsRuntime {
    LocalMutateDocumentAttachmentsRuntime::new(temp.path.clone(), body_policy())
}

fn link_input(
    operation: &str,
    expected: &str,
    asset: char,
    label: &str,
) -> MutateDocumentAttachmentsInput {
    MutateDocumentAttachmentsInput::link(
        operation,
        "workspace-1",
        "doc-1",
        expected,
        asset_id(asset).as_str(),
        label,
        "local-user",
        "첨부 파일 연결",
    )
}

fn unlink_input(operation: &str, expected: &str, asset: char) -> MutateDocumentAttachmentsInput {
    MutateDocumentAttachmentsInput::unlink(
        operation,
        "workspace-1",
        "doc-1",
        expected,
        asset_id(asset).as_str(),
        "local-user",
        "첨부 파일 해제",
    )
}

fn seed_current(temp: &TempRoot, attachment_state: AttachmentSnapshotState) {
    let record = version_one(attachment_state);
    let mut versions = LocalVersionStore::with_body_policy(
        temp.path.join(LOCAL_DOCUMENT_VERSION_ROOT),
        body_policy(),
    );
    versions
        .append_version(&workspace(), record.clone())
        .expect("seed version");
    let mut pointer =
        LocalCurrentDocumentVersionPointer::new(temp.path.join(LOCAL_DOCUMENT_POINTER_ROOT));
    pointer
        .compare_and_set_current_version(&workspace(), &document(), None, version_id("version-1"))
        .expect("seed pointer");
    let mut projection =
        LocalCurrentDocumentRevisionProjectionWriter::new(temp.path.clone(), body_policy());
    ProjectCurrentDocumentRevisionUsecase::new()
        .execute(
            ProjectCurrentDocumentRevisionInput::new("workspace-1", "notes/original.md", record),
            &mut projection,
        )
        .expect("seed projection");
}

fn version_one(attachment_state: AttachmentSnapshotState) -> VersionRecord {
    let snapshot_ref = DocumentSnapshotRef::new("snapshot-1").unwrap();
    let entry = VersionEntry::new(
        version_id("version-1"),
        document(),
        snapshot_ref.clone(),
        VersionAuthor::new("local-user").unwrap(),
        VersionSummary::new("Seed").unwrap(),
    )
    .unwrap()
    .with_created_at_epoch_ms(1)
    .unwrap()
    .with_revision_number(DocumentRevisionNumber::new(1).unwrap())
    .unwrap();
    let snapshot = VersionSnapshot::with_attachment_state(
        document(),
        snapshot_ref,
        DocumentBody::new("첫 번째 문서\n본문\n", body_policy()).unwrap(),
        attachment_state,
    );
    VersionRecord::new(entry, snapshot).unwrap()
}

fn assert_current_snapshot(
    temp: &TempRoot,
    expected_version: &VersionId,
    expected_references: &[AssetReference],
    expected_history: usize,
) {
    let pointer =
        LocalCurrentDocumentVersionPointer::new(temp.path.join(LOCAL_DOCUMENT_POINTER_ROOT));
    assert_eq!(
        pointer
            .load_current_version(&workspace(), &document())
            .unwrap()
            .as_ref(),
        Some(expected_version)
    );
    assert_eq!(history_count(temp), expected_history);
    assert_eq!(
        current_record(temp)
            .snapshot()
            .attachment_state()
            .references()
            .unwrap(),
        expected_references
    );
}

fn current_record(temp: &TempRoot) -> VersionRecord {
    let pointer =
        LocalCurrentDocumentVersionPointer::new(temp.path.join(LOCAL_DOCUMENT_POINTER_ROOT));
    let current = pointer
        .load_current_version(&workspace(), &document())
        .unwrap()
        .unwrap();
    LocalVersionStore::with_body_policy(temp.path.join(LOCAL_DOCUMENT_VERSION_ROOT), body_policy())
        .get_committed_version_record(&workspace(), &document(), &current)
        .unwrap()
        .unwrap()
}

fn history_count(temp: &TempRoot) -> usize {
    LocalVersionStore::with_body_policy(temp.path.join(LOCAL_DOCUMENT_VERSION_ROOT), body_policy())
        .list_history(
            &workspace(),
            &document(),
            HistoryPageRequest::first(10).unwrap(),
        )
        .unwrap()
        .entries()
        .len()
}

fn version_body_path(temp: &TempRoot, version: &str) -> PathBuf {
    temp.path
        .join(LOCAL_DOCUMENT_VERSION_ROOT)
        .join("workspace-1")
        .join("documents")
        .join("doc-1")
        .join("snapshots")
        .join(version)
        .join("body.md")
}

fn version_attachments_path(temp: &TempRoot, version: &str) -> PathBuf {
    version_body_path(temp, version).with_file_name("attachments.json")
}

fn projected_path(temp: &TempRoot) -> Option<String> {
    LocalDocumentRepository::with_body_policy(
        temp.path.join(LOCAL_CURRENT_DOCUMENT_PROJECTION_ROOT),
        body_policy(),
    )
    .get_current_by_id(&workspace(), &document())
    .unwrap()
    .map(|record| record.path().as_str().to_string())
}

fn assert_associations(temp: &TempRoot, expected: &[(char, &str)]) {
    let associations = DurableAssetAssociationCatalog::new(temp.path.clone())
        .list_assets(&workspace(), &document(), 500)
        .unwrap();
    let actual = associations
        .iter()
        .map(|association| {
            (
                association.asset_id().as_str().to_string(),
                association.label().to_string(),
            )
        })
        .collect::<Vec<_>>();
    let expected = expected
        .iter()
        .map(|(asset, label)| (asset_id(*asset).as_str().to_string(), (*label).to_string()))
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
}

fn association_asset_root(temp: &TempRoot, character: char) -> PathBuf {
    temp.path
        .join("assets/associations")
        .join(hex("workspace-1"))
        .join("by-asset")
        .join(asset_id(character).as_str())
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

fn known(references: Vec<AssetReference>) -> AttachmentSnapshotState {
    AttachmentSnapshotState::known(references).unwrap()
}

fn reference(asset: char, label: &str) -> AssetReference {
    AssetReference::new(asset_id(asset), label).unwrap()
}

fn asset_id(value: char) -> AssetId {
    AssetId::from_sha256_hex(&std::iter::repeat_n(value, 64).collect::<String>()).unwrap()
}

fn version_id(value: &str) -> VersionId {
    VersionId::new(value).unwrap()
}

fn workspace() -> WorkspaceId {
    WorkspaceId::new("workspace-1").unwrap()
}

fn document() -> DocumentId {
    DocumentId::new("doc-1").unwrap()
}

fn body_policy() -> DocumentBodyPolicy {
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
            "sponzey-local-attachment-runtime-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = remove_dir_all(&self.path);
    }
}

fn remove_dir_all(path: &Path) -> std::io::Result<()> {
    fs::remove_dir_all(path)
}
