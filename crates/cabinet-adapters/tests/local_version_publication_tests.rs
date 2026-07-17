use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_version_store::{
    LocalVersionStore, VERSION_ATTACHMENTS_FILE, VERSION_BODY_FILE, VERSION_DOCUMENTS_DIR,
    VERSION_ENTRY_FILE, VERSION_HISTORY_FILE, VERSION_PREPARED_DIR, VERSION_SNAPSHOTS_DIR,
};
use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::document_revision::DocumentOperationId;
use cabinet_domain::version::{
    DocumentRevisionNumber, DocumentSnapshotRef, VersionAuthor, VersionEntry, VersionId,
    VersionSummary,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::version_preparation::VersionPreparationPort;
use cabinet_ports::version_publication::{VersionPublicationError, VersionPublicationPort};
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
            "sponzey-version-publication-{name}-{}-{nanos}",
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
fn first_publication_exposes_snapshot_and_single_history_entry() {
    let temp = TempRoot::new("first");
    let workspace = workspace();
    let operation = operation("operation-1");
    let record = record("version-1", "First", 1);
    let document = record.document_id().clone();
    let version = record.version_id().clone();
    let mut store = LocalVersionStore::new(temp.0.clone());
    store
        .prepare_version(&workspace, &operation, record.clone())
        .expect("prepare");

    let published = store
        .publish_prepared(&workspace, &operation)
        .expect("publish");

    assert_eq!(published.version_id(), &version);
    assert_eq!(published.revision_number().value(), 1);
    assert_eq!(
        store
            .get_version_snapshot(&workspace, &document, &version)
            .expect("snapshot")
            .expect("published")
            .body()
            .as_str(),
        "First"
    );
    assert_eq!(history(&store, &workspace, &document).len(), 1);
    assert!(
        store
            .load_prepared(&workspace, &operation)
            .expect("prepared retained")
            .is_some()
    );
}

#[test]
fn repeated_and_restart_publication_do_not_duplicate_history() {
    let temp = TempRoot::new("restart-idempotent");
    let workspace = workspace();
    let operation = operation("operation-1");
    let record = record("version-1", "First", 1);
    let document = record.document_id().clone();
    let mut store = LocalVersionStore::new(temp.0.clone());
    store
        .prepare_version(&workspace, &operation, record)
        .expect("prepare");
    let first = store
        .publish_prepared(&workspace, &operation)
        .expect("first publish");
    let second = store
        .publish_prepared(&workspace, &operation)
        .expect("same process retry");
    let mut restarted = LocalVersionStore::new(temp.0.clone());
    let third = restarted
        .publish_prepared(&workspace, &operation)
        .expect("restart retry");

    assert_eq!(first, second);
    assert_eq!(second, third);
    assert_eq!(history(&restarted, &workspace, &document).len(), 1);
    assert_eq!(history_file_lines(&temp.0), 1);
}

#[test]
fn missing_prepared_operation_does_not_write_history() {
    let temp = TempRoot::new("missing");
    let workspace = workspace();
    let mut store = LocalVersionStore::new(temp.0.clone());

    let error = store
        .publish_prepared(&workspace, &operation("missing-operation"))
        .expect_err("not prepared");

    assert_eq!(error, VersionPublicationError::NotPrepared);
    assert_eq!(history_file_lines(&temp.0), 0);
}

#[test]
fn revision_mismatch_preserves_prepared_and_committed_state() {
    let temp = TempRoot::new("revision-conflict");
    let workspace = workspace();
    let operation = operation("operation-1");
    let record = record("version-2", "Second", 2);
    let document = record.document_id().clone();
    let version = record.version_id().clone();
    let mut store = LocalVersionStore::new(temp.0.clone());
    store
        .prepare_version(&workspace, &operation, record)
        .expect("prepare");

    let error = store
        .publish_prepared(&workspace, &operation)
        .expect_err("revision conflict");

    assert_eq!(error, VersionPublicationError::Conflict);
    assert!(
        store
            .load_prepared(&workspace, &operation)
            .expect("prepared")
            .is_some()
    );
    assert!(
        store
            .get_version_snapshot(&workspace, &document, &version)
            .expect("snapshot")
            .is_none()
    );
    assert!(history(&store, &workspace, &document).is_empty());
}

#[test]
fn snapshot_only_interruption_resumes_history_publication() {
    let temp = TempRoot::new("snapshot-only");
    let workspace = workspace();
    let operation = operation("operation-1");
    let record = record("version-1", "First", 1);
    let document = record.document_id().clone();
    let version = record.version_id().clone();
    let mut store = LocalVersionStore::new(temp.0.clone());
    store
        .prepare_version(&workspace, &operation, record)
        .expect("prepare");
    copy_prepared_payload_to_final(&temp.0, &workspace, &operation, &document, &version);

    let published = store
        .publish_prepared(&workspace, &operation)
        .expect("resume publication");

    assert_eq!(published.version_id(), &version);
    assert_eq!(history(&store, &workspace, &document).len(), 1);
}

#[test]
fn existing_committed_payload_mismatch_is_conflict_without_overwrite() {
    let temp = TempRoot::new("payload-conflict");
    let workspace = workspace();
    let operation = operation("operation-1");
    let prepared = record("version-1", "Prepared", 1);
    let document = prepared.document_id().clone();
    let version = prepared.version_id().clone();
    let mut store = LocalVersionStore::new(temp.0.clone());
    store
        .prepare_version(&workspace, &operation, prepared)
        .expect("prepare");
    store
        .append_version(&workspace, record("version-1", "Existing", 1))
        .expect("existing committed");

    let error = store
        .publish_prepared(&workspace, &operation)
        .expect_err("payload mismatch");

    assert_eq!(error, VersionPublicationError::Conflict);
    assert_eq!(
        store
            .get_version_snapshot(&workspace, &document, &version)
            .expect("snapshot")
            .expect("existing")
            .body()
            .as_str(),
        "Existing"
    );
    assert_eq!(history(&store, &workspace, &document).len(), 1);
}

fn workspace() -> WorkspaceId {
    WorkspaceId::new("workspace-1").expect("workspace")
}

fn operation(value: &str) -> DocumentOperationId {
    DocumentOperationId::new(value).expect("operation")
}

fn record(version: &str, body: &str, revision: u64) -> VersionRecord {
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
    .with_created_at_epoch_ms(100 + revision)
    .expect("timestamp")
    .with_revision_number(DocumentRevisionNumber::new(revision).expect("revision"))
    .expect("assigned revision");
    VersionRecord::new(
        entry,
        VersionSnapshot::new(
            document,
            snapshot_ref,
            DocumentBody::new(body, DocumentBodyPolicy::new(1024).expect("policy")).expect("body"),
        ),
    )
    .expect("record")
}

fn history(
    store: &LocalVersionStore,
    workspace: &WorkspaceId,
    document: &DocumentId,
) -> Vec<VersionEntry> {
    store
        .list_history(
            workspace,
            document,
            HistoryPageRequest::first(10).expect("request"),
        )
        .expect("history")
        .entries()
        .to_vec()
}

fn history_file_lines(root: &Path) -> usize {
    walk_files(root)
        .into_iter()
        .find(|path| {
            path.file_name()
                .is_some_and(|name| name == VERSION_HISTORY_FILE)
        })
        .map(|path| fs::read_to_string(path).expect("history").lines().count())
        .unwrap_or(0)
}

fn copy_prepared_payload_to_final(
    root: &Path,
    workspace: &WorkspaceId,
    operation: &DocumentOperationId,
    document: &DocumentId,
    version: &VersionId,
) {
    let prepared = root
        .join(workspace.as_str())
        .join(VERSION_PREPARED_DIR)
        .join(operation.as_str());
    let final_dir = root
        .join(workspace.as_str())
        .join(VERSION_DOCUMENTS_DIR)
        .join(document.as_str())
        .join(VERSION_SNAPSHOTS_DIR)
        .join(version.as_str());
    fs::create_dir_all(&final_dir).expect("final dir");
    for file in [
        VERSION_ENTRY_FILE,
        VERSION_BODY_FILE,
        VERSION_ATTACHMENTS_FILE,
    ] {
        let source = prepared.join(file);
        if source.exists() {
            fs::copy(source, final_dir.join(file)).expect("copy payload");
        }
    }
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
