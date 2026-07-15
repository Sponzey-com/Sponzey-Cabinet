use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_version_store::{
    LocalVersionStore, VERSION_BODY_FILE, VERSION_DOCUMENTS_DIR, VERSION_ENTRY_FILE,
    VERSION_HISTORY_FILE, VERSION_SNAPSHOTS_DIR,
};
use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::version::{
    DocumentSnapshotRef, VersionAuthor, VersionEntry, VersionId, VersionSummary,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::version_store::{
    HistoryPageRequest, VersionRecord, VersionSnapshot, VersionStore, VersionStoreError,
};

struct TempVersionRoot {
    path: PathBuf,
}

impl TempVersionRoot {
    fn new(test_name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let path = PathBuf::from("/tmp").join(format!(
            "sponzey-cabinet-version-store-{test_name}-{}-{nanos}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("temp version root");
        Self { path }
    }
}

impl Drop for TempVersionRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn local_version_store_appends_and_reads_specific_snapshot() {
    let temp = TempVersionRoot::new("snapshot");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let record = version_record("doc-1", "version-1", "snapshot-1", "Version body");
    let document_id = record.document_id().clone();
    let version_id = record.version_id().clone();
    let mut store = LocalVersionStore::new(temp.path.clone());

    store
        .append_version(&workspace_id, record)
        .expect("append version");
    let snapshot = store
        .get_version_snapshot(&workspace_id, &document_id, &version_id)
        .expect("get snapshot")
        .expect("snapshot");

    assert_eq!(snapshot.body().as_str(), "Version body");
    assert!(version_entry_path(&temp, &workspace_id, &document_id, &version_id).is_file());
    assert!(version_body_path(&temp, &workspace_id, &document_id, &version_id).is_file());
}

#[test]
fn local_version_store_paginates_history_with_cursor() {
    let temp = TempVersionRoot::new("history");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let document_id = DocumentId::new("doc-1").expect("document id");
    let mut store = LocalVersionStore::new(temp.path.clone());

    for version_number in 1..=3 {
        store
            .append_version(
                &workspace_id,
                version_record(
                    "doc-1",
                    &format!("version-{version_number}"),
                    &format!("snapshot-{version_number}"),
                    "Version body",
                ),
            )
            .expect("append version");
    }

    let first_page = store
        .list_history(
            &workspace_id,
            &document_id,
            HistoryPageRequest::first(2).expect("request"),
        )
        .expect("first page");
    let second_page = store
        .list_history(
            &workspace_id,
            &document_id,
            HistoryPageRequest::after(first_page.next_cursor().cloned().expect("next cursor"), 2)
                .expect("request"),
        )
        .expect("second page");

    assert_eq!(first_page.entries().len(), 2);
    assert_eq!(first_page.entries()[0].version_id().as_str(), "version-1");
    assert_eq!(first_page.next_cursor().expect("next").as_str(), "2");
    assert_eq!(second_page.entries().len(), 1);
    assert_eq!(second_page.entries()[0].version_id().as_str(), "version-3");
    assert!(second_page.next_cursor().is_none());
    assert_eq!(
        fs::read_to_string(history_path(&temp, &workspace_id, &document_id)).expect("history"),
        "version-1\nversion-2\nversion-3\n"
    );
}

#[test]
fn local_version_store_persists_injected_creation_time_and_reads_legacy_unknown() {
    let temp = TempVersionRoot::new("created-at");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let document_id = DocumentId::new("doc-1").expect("document id");
    let policy = DocumentBodyPolicy::new(1024).expect("policy");
    let mut store = LocalVersionStore::with_body_policy_and_clock(
        temp.path.clone(),
        policy,
        || 1_721_000_000_123,
    );
    store
        .append_version(
            &workspace_id,
            version_record("doc-1", "version-1", "snapshot-1", "Version body"),
        )
        .expect("append version");
    drop(store);

    let restarted = LocalVersionStore::with_body_policy_and_clock(
        temp.path.clone(),
        policy,
        || 1_999_000_000_000,
    );
    let persisted = restarted
        .list_history(
            &workspace_id,
            &document_id,
            HistoryPageRequest::first(10).expect("request"),
        )
        .expect("history");
    assert_eq!(
        persisted.entries()[0].created_at_epoch_ms(),
        Some(1_721_000_000_123)
    );

    let entry_path = version_entry_path(
        &temp,
        &workspace_id,
        &document_id,
        &VersionId::new("version-1").expect("version"),
    );
    let legacy = fs::read_to_string(&entry_path)
        .expect("entry")
        .lines()
        .filter(|line| !line.starts_with("created_at_epoch_ms="))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    fs::write(entry_path, legacy).expect("legacy entry");
    let legacy_page = restarted
        .list_history(
            &workspace_id,
            &document_id,
            HistoryPageRequest::first(10).expect("request"),
        )
        .expect("legacy history");
    assert_eq!(legacy_page.entries()[0].created_at_epoch_ms(), None);
}

#[test]
fn local_version_store_rejects_duplicate_version_id() {
    let temp = TempVersionRoot::new("duplicate");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let mut store = LocalVersionStore::new(temp.path.clone());

    store
        .append_version(
            &workspace_id,
            version_record("doc-1", "version-1", "snapshot-1", "Version body"),
        )
        .expect("append version");
    let error = store
        .append_version(
            &workspace_id,
            version_record("doc-1", "version-1", "snapshot-1", "Version body"),
        )
        .expect_err("duplicate must fail");

    assert_eq!(error, VersionStoreError::Conflict);
    assert_eq!(error.code(), "version_store.conflict");
}

#[test]
fn local_version_store_reports_corrupted_version_metadata() {
    let temp = TempVersionRoot::new("corrupt");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let record = version_record("doc-1", "version-1", "snapshot-1", "Version body");
    let document_id = record.document_id().clone();
    let version_id = record.version_id().clone();
    let mut store = LocalVersionStore::new(temp.path.clone());

    store
        .append_version(&workspace_id, record)
        .expect("append version");
    fs::write(
        version_entry_path(&temp, &workspace_id, &document_id, &version_id),
        "not valid metadata",
    )
    .expect("write corrupt metadata");

    let error = store
        .get_version_snapshot(&workspace_id, &document_id, &version_id)
        .expect_err("corrupt metadata must fail");

    assert_eq!(error, VersionStoreError::CorruptedHistory);
}

fn version_record(
    document_id: &str,
    version_id: &str,
    snapshot_ref: &str,
    body: &str,
) -> VersionRecord {
    VersionRecord::new(
        VersionEntry::new(
            VersionId::new(version_id).expect("version id"),
            DocumentId::new(document_id).expect("document id"),
            DocumentSnapshotRef::new(snapshot_ref).expect("snapshot ref"),
            VersionAuthor::new("writer").expect("author"),
            VersionSummary::new("Saved document").expect("summary"),
        )
        .expect("version entry"),
        VersionSnapshot::new(
            DocumentId::new(document_id).expect("snapshot document id"),
            DocumentSnapshotRef::new(snapshot_ref).expect("snapshot ref"),
            DocumentBody::new(body, DocumentBodyPolicy::new(1024).expect("policy")).expect("body"),
        ),
    )
    .expect("version record")
}

fn version_entry_path(
    temp: &TempVersionRoot,
    workspace_id: &WorkspaceId,
    document_id: &DocumentId,
    version_id: &VersionId,
) -> PathBuf {
    version_dir(temp, workspace_id, document_id, version_id).join(VERSION_ENTRY_FILE)
}

fn version_body_path(
    temp: &TempVersionRoot,
    workspace_id: &WorkspaceId,
    document_id: &DocumentId,
    version_id: &VersionId,
) -> PathBuf {
    version_dir(temp, workspace_id, document_id, version_id).join(VERSION_BODY_FILE)
}

fn version_dir(
    temp: &TempVersionRoot,
    workspace_id: &WorkspaceId,
    document_id: &DocumentId,
    version_id: &VersionId,
) -> PathBuf {
    temp.path
        .join(workspace_id.as_str())
        .join(VERSION_DOCUMENTS_DIR)
        .join(document_id.as_str())
        .join(VERSION_SNAPSHOTS_DIR)
        .join(version_id.as_str())
}

fn history_path(
    temp: &TempVersionRoot,
    workspace_id: &WorkspaceId,
    document_id: &DocumentId,
) -> PathBuf {
    temp.path
        .join(workspace_id.as_str())
        .join(VERSION_DOCUMENTS_DIR)
        .join(document_id.as_str())
        .join(VERSION_HISTORY_FILE)
}
