use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_version_store::LocalVersionStore;
use cabinet_domain::asset::{AssetId, AssetReference};
use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::document_revision::DocumentOperationId;
use cabinet_domain::version::{
    AttachmentSnapshotState, DocumentRevisionNumber, DocumentSnapshotRef, VersionAuthor,
    VersionEntry, VersionId, VersionSummary,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::version_preparation::{
    VersionPreparationError, VersionPreparationOutcomeKind, VersionPreparationPort,
};
use cabinet_ports::version_store::{
    HistoryPageRequest, VersionRecord, VersionSnapshot, VersionStore,
};

struct TempRoot(PathBuf);

impl TempRoot {
    fn new(name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "sponzey-version-preparation-{name}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temp root");
        Self(path)
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn prepare_round_trips_after_restart_without_history_or_snapshot_visibility() {
    let temp = TempRoot::new("restart-hidden");
    let workspace = workspace("workspace-1");
    let operation = operation("operation-1");
    let record = record("version-1", "Prepared body", 1, 101, true);
    let document = record.document_id().clone();
    let version = record.version_id().clone();
    let mut store = LocalVersionStore::new(temp.0.clone());

    let outcome = store
        .prepare_version(&workspace, &operation, record.clone())
        .expect("prepare");
    assert_eq!(outcome.kind(), VersionPreparationOutcomeKind::Prepared);

    let restarted = LocalVersionStore::new(temp.0.clone());
    let loaded = restarted
        .load_prepared(&workspace, &operation)
        .expect("load")
        .expect("prepared");
    assert_eq!(loaded.record(), &record);
    assert_eq!(
        loaded
            .record()
            .snapshot()
            .attachment_state()
            .references()
            .expect("known")[0]
            .label(),
        "Diagram"
    );
    assert!(
        restarted
            .get_version_snapshot(&workspace, &document, &version)
            .expect("snapshot query")
            .is_none()
    );
    assert!(
        restarted
            .list_history(
                &workspace,
                &document,
                HistoryPageRequest::first(10).expect("request")
            )
            .expect("history")
            .entries()
            .is_empty()
    );
}

#[test]
fn same_operation_and_record_is_idempotent_but_different_record_conflicts_without_mutation() {
    let temp = TempRoot::new("idempotency");
    let workspace = workspace("workspace-1");
    let operation = operation("operation-1");
    let original_record = record("version-1", "Original", 1, 101, false);
    let mut store = LocalVersionStore::new(temp.0.clone());

    store
        .prepare_version(&workspace, &operation, original_record.clone())
        .expect("first prepare");
    let repeated = store
        .prepare_version(&workspace, &operation, original_record.clone())
        .expect("idempotent prepare");
    let error = store
        .prepare_version(
            &workspace,
            &operation,
            record("version-2", "Different", 1, 101, false),
        )
        .expect_err("conflicting retry");

    assert_eq!(repeated.kind(), VersionPreparationOutcomeKind::Existing);
    assert_eq!(error, VersionPreparationError::Conflict);
    assert_eq!(
        store
            .load_prepared(&workspace, &operation)
            .expect("load")
            .expect("prepared")
            .record(),
        &original_record
    );
    assert_eq!(prepared_manifest_count(&temp.0), 1);
}

#[test]
fn prepare_rejects_record_without_assigned_revision_or_timestamp() {
    let temp = TempRoot::new("invalid-record");
    let workspace = workspace("workspace-1");
    let operation = operation("operation-1");
    let mut store = LocalVersionStore::new(temp.0.clone());
    let document = DocumentId::new("doc-1").expect("document");
    let snapshot_ref = DocumentSnapshotRef::new("snapshot-1").expect("snapshot");
    let legacy_entry = VersionEntry::new(
        VersionId::new("version-1").expect("version"),
        document.clone(),
        snapshot_ref.clone(),
        VersionAuthor::new("writer").expect("author"),
        VersionSummary::new("Updated").expect("summary"),
    )
    .expect("entry");
    let legacy_record = VersionRecord::new(
        legacy_entry,
        VersionSnapshot::new(
            document,
            snapshot_ref,
            DocumentBody::new("Body", DocumentBodyPolicy::new(1024).expect("policy"))
                .expect("body"),
        ),
    )
    .expect("record");

    let error = store
        .prepare_version(&workspace, &operation, legacy_record)
        .expect_err("immutable metadata required");

    assert_eq!(error, VersionPreparationError::InvalidRecord);
    assert_eq!(prepared_manifest_count(&temp.0), 0);
}

#[test]
fn discard_is_idempotent_and_isolated_by_workspace_and_operation() {
    let temp = TempRoot::new("discard");
    let workspace_a = workspace("workspace-a");
    let workspace_b = workspace("workspace-b");
    let operation_a = operation("operation-a");
    let operation_b = operation("operation-b");
    let mut store = LocalVersionStore::new(temp.0.clone());
    store
        .prepare_version(
            &workspace_a,
            &operation_a,
            record("version-a", "A", 1, 101, false),
        )
        .expect("prepare a");
    store
        .prepare_version(
            &workspace_b,
            &operation_b,
            record("version-b", "B", 1, 102, false),
        )
        .expect("prepare b");

    store
        .discard_prepared(&workspace_a, &operation_a)
        .expect("discard");
    store
        .discard_prepared(&workspace_a, &operation_a)
        .expect("repeat discard");

    assert!(
        store
            .load_prepared(&workspace_a, &operation_a)
            .expect("load discarded")
            .is_none()
    );
    assert!(
        store
            .load_prepared(&workspace_b, &operation_b)
            .expect("load retained")
            .is_some()
    );
}

#[test]
fn malformed_manifest_is_reported_as_corruption_without_payload_leakage() {
    let temp = TempRoot::new("corruption");
    let workspace = workspace("workspace-1");
    let operation = operation("operation-1");
    let mut store = LocalVersionStore::new(temp.0.clone());
    store
        .prepare_version(
            &workspace,
            &operation,
            record("version-1", "Private body", 1, 101, false),
        )
        .expect("prepare");
    let manifest = find_file(&temp.0, "manifest.json");
    fs::write(
        &manifest,
        r#"{"schema_version":1,"operation_id":"operation-1","unknown":"Private body"}"#,
    )
    .expect("corrupt manifest");

    let error = store
        .load_prepared(&workspace, &operation)
        .expect_err("corrupt prepared");

    assert_eq!(error, VersionPreparationError::CorruptedPrepared);
    assert!(!format!("{error:?}").contains("Private body"));
}

#[test]
fn missing_prepared_payload_is_reported_as_corruption() {
    let temp = TempRoot::new("missing-payload");
    let workspace = workspace("workspace-1");
    let operation = operation("operation-1");
    let mut store = LocalVersionStore::new(temp.0.clone());
    store
        .prepare_version(
            &workspace,
            &operation,
            record("version-1", "Body", 1, 101, false),
        )
        .expect("prepare");
    fs::remove_file(find_file(&temp.0, "body.md")).expect("remove prepared body");

    let error = store
        .load_prepared(&workspace, &operation)
        .expect_err("missing payload");

    assert_eq!(error, VersionPreparationError::CorruptedPrepared);
}

fn workspace(value: &str) -> WorkspaceId {
    WorkspaceId::new(value).expect("workspace")
}

fn operation(value: &str) -> DocumentOperationId {
    DocumentOperationId::new(value).expect("operation")
}

fn record(
    version: &str,
    body: &str,
    revision: u64,
    timestamp: u64,
    known_attachments: bool,
) -> VersionRecord {
    let document = DocumentId::new("doc-1").expect("document");
    let snapshot_ref =
        DocumentSnapshotRef::new(&format!("snapshot-{version}")).expect("snapshot reference");
    let entry = VersionEntry::new(
        VersionId::new(version).expect("version"),
        document.clone(),
        snapshot_ref.clone(),
        VersionAuthor::new("writer").expect("author"),
        VersionSummary::new("Updated").expect("summary"),
    )
    .expect("entry")
    .with_created_at_epoch_ms(timestamp)
    .expect("timestamp")
    .with_revision_number(DocumentRevisionNumber::new(revision).expect("revision"))
    .expect("assigned revision");
    let attachment_state = if known_attachments {
        AttachmentSnapshotState::known(vec![
            AssetReference::new(
                AssetId::from_sha256_hex(&"a".repeat(64)).expect("asset"),
                "Diagram",
            )
            .expect("reference"),
        ])
        .expect("known snapshot")
    } else {
        AttachmentSnapshotState::legacy_unknown()
    };
    VersionRecord::new(
        entry,
        VersionSnapshot::with_attachment_state(
            document,
            snapshot_ref,
            DocumentBody::new(body, DocumentBodyPolicy::new(1024).expect("policy")).expect("body"),
            attachment_state,
        ),
    )
    .expect("record")
}

fn prepared_manifest_count(root: &Path) -> usize {
    walk_files(root)
        .iter()
        .filter(|path| path.file_name().is_some_and(|name| name == "manifest.json"))
        .count()
}

fn find_file(root: &Path, name: &str) -> PathBuf {
    walk_files(root)
        .into_iter()
        .find(|path| path.file_name().is_some_and(|value| value == name))
        .expect("file")
}

fn walk_files(root: &Path) -> Vec<PathBuf> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory).expect("read directory") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.push(path);
            }
        }
    }
    files
}
