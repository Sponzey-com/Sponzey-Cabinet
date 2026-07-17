use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_version_store::{
    LocalVersionStore, VERSION_ATTACHMENTS_FILE, VERSION_BODY_FILE, VERSION_DOCUMENTS_DIR,
    VERSION_ENTRY_FILE, VERSION_HISTORY_FILE, VERSION_SNAPSHOTS_DIR,
};
use cabinet_domain::asset::{AssetId, AssetReference};
use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::version::{
    AttachmentSnapshotState, DocumentRevisionNumber, DocumentSnapshotRef, VersionAuthor,
    VersionEntry, VersionId, VersionSummary,
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
    assert!(snapshot.attachment_state().is_legacy_unknown());
    assert!(version_entry_path(&temp, &workspace_id, &document_id, &version_id).is_file());
    assert!(version_body_path(&temp, &workspace_id, &document_id, &version_id).is_file());
    assert!(!version_attachments_path(&temp, &workspace_id, &document_id, &version_id).exists());
}

#[test]
fn local_version_store_round_trips_known_attachment_snapshot_after_restart() {
    let temp = TempVersionRoot::new("known-attachments");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let record = version_record_with_attachments(
        "doc-1",
        "version-1",
        "snapshot-1",
        "Version body",
        vec![
            asset_reference('b', "Second"),
            asset_reference('a', "First"),
        ],
    );
    let document_id = record.document_id().clone();
    let version_id = record.version_id().clone();

    LocalVersionStore::new(temp.path.clone())
        .append_version(&workspace_id, record)
        .expect("append known version");
    let sidecar = fs::read_to_string(version_attachments_path(
        &temp,
        &workspace_id,
        &document_id,
        &version_id,
    ))
    .expect("attachment sidecar");
    let restarted = LocalVersionStore::new(temp.path.clone());
    let snapshot = restarted
        .get_version_snapshot(&workspace_id, &document_id, &version_id)
        .expect("read version")
        .expect("snapshot");

    let references = snapshot
        .attachment_state()
        .references()
        .expect("known references");
    assert_eq!(references[0].label(), "First");
    assert_eq!(references[1].label(), "Second");
    assert!(sidecar.contains("\"schema_version\":1"));
    assert!(sidecar.contains("\"state\":\"known\""));
    assert!(!sidecar.contains("byte_size"));
    assert!(!sidecar.contains("media_type"));
    assert!(!sidecar.contains("file_name"));
    assert!(!sidecar.contains("body"));
}

#[test]
fn local_version_store_preserves_known_empty_attachment_snapshot() {
    let temp = TempVersionRoot::new("known-empty-attachments");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let record = version_record_with_attachments(
        "doc-1",
        "version-1",
        "snapshot-1",
        "Version body",
        Vec::new(),
    );
    let document_id = record.document_id().clone();
    let version_id = record.version_id().clone();
    let mut store = LocalVersionStore::new(temp.path.clone());

    store
        .append_version(&workspace_id, record)
        .expect("append known empty version");
    let snapshot = store
        .get_version_snapshot(&workspace_id, &document_id, &version_id)
        .expect("read version")
        .expect("snapshot");

    assert_eq!(snapshot.attachment_state().references(), Some(&[][..]));
}

#[test]
fn local_version_store_round_trips_assigned_revision_number() {
    let temp = TempVersionRoot::new("assigned-revision");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let record =
        version_record_with_revision("doc-1", "version-1", "snapshot-1", "Version body", 1);
    let document_id = record.document_id().clone();
    let version_id = record.version_id().clone();
    let mut store = LocalVersionStore::new(temp.path.clone());

    store
        .append_version(&workspace_id, record)
        .expect("append assigned version");
    let restarted = LocalVersionStore::new(temp.path.clone());
    let history = restarted
        .list_history(
            &workspace_id,
            &document_id,
            HistoryPageRequest::first(10).expect("request"),
        )
        .expect("history");

    assert_eq!(
        history.entries()[0]
            .revision_number()
            .map(|number| number.value()),
        Some(1)
    );
    assert!(
        fs::read_to_string(version_entry_path(
            &temp,
            &workspace_id,
            &document_id,
            &version_id,
        ))
        .expect("entry")
        .contains("revision_number=1\n")
    );
}

#[test]
fn local_version_store_assigns_next_revision_number_during_append() {
    let temp = TempVersionRoot::new("append-revision-allocation");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let document_id = DocumentId::new("doc-1").expect("document id");
    let mut store = LocalVersionStore::new(temp.path.clone());

    store
        .append_version(
            &workspace_id,
            version_record("doc-1", "version-1", "snapshot-1", "First"),
        )
        .expect("append first version");
    store
        .append_version(
            &workspace_id,
            version_record("doc-1", "version-2", "snapshot-2", "Second"),
        )
        .expect("append second version");
    let page = store
        .list_history(
            &workspace_id,
            &document_id,
            HistoryPageRequest::first(10).expect("request"),
        )
        .expect("history");

    assert_eq!(
        page.entries()
            .iter()
            .map(|entry| entry.revision_number().expect("assigned").value())
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
}

#[test]
fn local_version_store_rejects_explicit_revision_mismatch_before_write() {
    let temp = TempVersionRoot::new("append-revision-mismatch");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let document_id = DocumentId::new("doc-1").expect("document id");
    let version_id = VersionId::new("version-1").expect("version id");
    let mut store = LocalVersionStore::new(temp.path.clone());

    let error = store
        .append_version(
            &workspace_id,
            version_record_with_revision("doc-1", "version-1", "snapshot-1", "Body", 2),
        )
        .expect_err("mismatched explicit revision must fail");

    assert_eq!(error, VersionStoreError::Conflict);
    assert!(!version_entry_path(&temp, &workspace_id, &document_id, &version_id).exists());
}

#[test]
fn revision_migration_assigns_history_order_and_is_byte_stable_on_rerun() {
    let temp = TempVersionRoot::new("revision-migration");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let document_id = DocumentId::new("doc-1").expect("document id");
    let mut store = LocalVersionStore::new(temp.path.clone());
    for number in 1..=3 {
        let record = if number == 1 {
            version_record_with_attachments(
                "doc-1",
                "version-1",
                "snapshot-1",
                "Version body",
                vec![asset_reference('a', "Diagram")],
            )
        } else {
            version_record(
                "doc-1",
                &format!("version-{number}"),
                &format!("snapshot-{number}"),
                "Version body",
            )
        };
        store
            .append_version(&workspace_id, record)
            .expect("append legacy version");
    }
    for number in 1..=3 {
        remove_revision_number(&version_entry_path(
            &temp,
            &workspace_id,
            &document_id,
            &VersionId::new(&format!("version-{number}")).expect("version"),
        ));
    }
    let version_one = VersionId::new("version-1").expect("version");
    let history_before = fs::read(history_path(&temp, &workspace_id, &document_id))
        .expect("history before migration");
    let body_before = fs::read(version_body_path(
        &temp,
        &workspace_id,
        &document_id,
        &version_one,
    ))
    .expect("body before migration");
    let attachments_before = fs::read(version_attachments_path(
        &temp,
        &workspace_id,
        &document_id,
        &version_one,
    ))
    .expect("attachments before migration");

    let first = store.migrate_revision_numbers().expect("first migration");
    let bytes_after_first = (1..=3)
        .map(|number| {
            fs::read(version_entry_path(
                &temp,
                &workspace_id,
                &document_id,
                &VersionId::new(&format!("version-{number}")).expect("version"),
            ))
            .expect("entry bytes")
        })
        .collect::<Vec<_>>();
    let second = store
        .migrate_revision_numbers()
        .expect("idempotent migration");
    let bytes_after_second = (1..=3)
        .map(|number| {
            fs::read(version_entry_path(
                &temp,
                &workspace_id,
                &document_id,
                &VersionId::new(&format!("version-{number}")).expect("version"),
            ))
            .expect("entry bytes")
        })
        .collect::<Vec<_>>();
    let history = store
        .list_history(
            &workspace_id,
            &document_id,
            HistoryPageRequest::first(10).expect("request"),
        )
        .expect("history");

    assert_eq!(first.documents_scanned(), 1);
    assert_eq!(first.entries_scanned(), 3);
    assert_eq!(first.entries_assigned(), 3);
    assert_eq!(second.entries_assigned(), 0);
    assert_eq!(bytes_after_first, bytes_after_second);
    assert_eq!(
        fs::read(history_path(&temp, &workspace_id, &document_id))
            .expect("history after migration"),
        history_before
    );
    assert_eq!(
        fs::read(version_body_path(
            &temp,
            &workspace_id,
            &document_id,
            &version_one,
        ))
        .expect("body after migration"),
        body_before
    );
    assert_eq!(
        fs::read(version_attachments_path(
            &temp,
            &workspace_id,
            &document_id,
            &version_one,
        ))
        .expect("attachments after migration"),
        attachments_before
    );
    assert_eq!(
        history
            .entries()
            .iter()
            .map(|entry| entry.revision_number().expect("assigned").value())
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
}

#[test]
fn revision_migration_completes_valid_partial_assignment() {
    let temp = TempVersionRoot::new("partial-revision-migration");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let mut store = LocalVersionStore::new(temp.path.clone());
    store
        .append_version(
            &workspace_id,
            version_record_with_revision("doc-1", "version-1", "snapshot-1", "First", 1),
        )
        .expect("append assigned version");
    store
        .append_version(
            &workspace_id,
            version_record("doc-1", "version-2", "snapshot-2", "Second"),
        )
        .expect("append legacy version");
    remove_revision_number(&version_entry_path(
        &temp,
        &workspace_id,
        &DocumentId::new("doc-1").expect("document id"),
        &VersionId::new("version-2").expect("version id"),
    ));

    let report = store.migrate_revision_numbers().expect("partial migration");

    assert_eq!(report.entries_scanned(), 2);
    assert_eq!(report.entries_assigned(), 1);
}

#[test]
fn revision_migration_rejects_mismatch_before_writing_any_entry() {
    let temp = TempVersionRoot::new("invalid-revision-migration");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let mut store = LocalVersionStore::new(temp.path.clone());
    store
        .append_version(
            &workspace_id,
            version_record("doc-a", "version-1", "snapshot-1", "Legacy"),
        )
        .expect("append legacy version");
    store
        .append_version(
            &workspace_id,
            version_record("doc-b", "version-1", "snapshot-1", "Invalid assignment"),
        )
        .expect("append version");
    let doc_a = DocumentId::new("doc-a").expect("document id");
    let doc_b = DocumentId::new("doc-b").expect("document id");
    let version = VersionId::new("version-1").expect("version id");
    let paths = [
        version_entry_path(&temp, &workspace_id, &doc_a, &version),
        version_entry_path(&temp, &workspace_id, &doc_b, &version),
    ];
    remove_revision_number(&paths[0]);
    replace_revision_number(&paths[1], 2);
    let before = paths
        .iter()
        .map(|path| fs::read(path).expect("entry bytes"))
        .collect::<Vec<_>>();

    let error = store
        .migrate_revision_numbers()
        .expect_err("mismatch must fail preflight");
    let after = paths
        .iter()
        .map(|path| fs::read(path).expect("entry bytes"))
        .collect::<Vec<_>>();

    assert_eq!(error, VersionStoreError::CorruptedHistory);
    assert_eq!(before, after);
}

#[test]
fn local_version_store_rejects_corrupt_attachment_sidecar() {
    let temp = TempVersionRoot::new("corrupt-attachments");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let record = version_record_with_attachments(
        "doc-1",
        "version-1",
        "snapshot-1",
        "Version body",
        vec![asset_reference('a', "Diagram")],
    );
    let document_id = record.document_id().clone();
    let version_id = record.version_id().clone();
    let mut store = LocalVersionStore::new(temp.path.clone());
    store
        .append_version(&workspace_id, record)
        .expect("append version");
    let sidecar_path = version_attachments_path(&temp, &workspace_id, &document_id, &version_id);
    let duplicate_hash = "d".repeat(64);
    let corrupt_fixtures = [
        "not-json".to_string(),
        r#"{"schema_version":2,"state":"known","references":[]}"#.to_string(),
        r#"{"schema_version":1,"state":"unknown","references":[]}"#.to_string(),
        r#"{"schema_version":1,"state":"known","references":[{"asset_id":"invalid","label":"A"}]}"#
            .to_string(),
        format!(
            r#"{{"schema_version":1,"state":"known","references":[{{"asset_id":"{duplicate_hash}","label":"A"}},{{"asset_id":"{duplicate_hash}","label":"B"}}]}}"#
        ),
    ];

    for fixture in corrupt_fixtures {
        fs::write(&sidecar_path, fixture).expect("write corrupt sidecar");
        assert_eq!(
            store
                .get_version_snapshot(&workspace_id, &document_id, &version_id)
                .expect_err("corrupt sidecar must fail"),
            VersionStoreError::CorruptedHistory
        );
    }
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
    let mut store =
        LocalVersionStore::with_body_policy_and_clock(temp.path.clone(), policy, || {
            1_721_000_000_123
        });
    store
        .append_version(
            &workspace_id,
            version_record("doc-1", "version-1", "snapshot-1", "Version body"),
        )
        .expect("append version");
    drop(store);

    let restarted =
        LocalVersionStore::with_body_policy_and_clock(temp.path.clone(), policy, || {
            1_999_000_000_000
        });
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

fn version_record_with_attachments(
    document_id: &str,
    version_id: &str,
    snapshot_ref: &str,
    body: &str,
    references: Vec<AssetReference>,
) -> VersionRecord {
    let attachment_state =
        AttachmentSnapshotState::known(references).expect("known attachment snapshot");
    VersionRecord::new(
        VersionEntry::new(
            VersionId::new(version_id).expect("version id"),
            DocumentId::new(document_id).expect("document id"),
            DocumentSnapshotRef::new(snapshot_ref).expect("snapshot ref"),
            VersionAuthor::new("writer").expect("author"),
            VersionSummary::new("Saved document").expect("summary"),
        )
        .expect("version entry"),
        VersionSnapshot::with_attachment_state(
            DocumentId::new(document_id).expect("snapshot document id"),
            DocumentSnapshotRef::new(snapshot_ref).expect("snapshot ref"),
            DocumentBody::new(body, DocumentBodyPolicy::new(1024).expect("policy")).expect("body"),
            attachment_state,
        ),
    )
    .expect("version record")
}

fn version_record_with_revision(
    document_id: &str,
    version_id: &str,
    snapshot_ref: &str,
    body: &str,
    revision_number: u64,
) -> VersionRecord {
    let entry = VersionEntry::new(
        VersionId::new(version_id).expect("version id"),
        DocumentId::new(document_id).expect("document id"),
        DocumentSnapshotRef::new(snapshot_ref).expect("snapshot ref"),
        VersionAuthor::new("writer").expect("author"),
        VersionSummary::new("Saved document").expect("summary"),
    )
    .expect("version entry")
    .with_revision_number(DocumentRevisionNumber::new(revision_number).expect("revision"))
    .expect("assign revision");
    VersionRecord::new(
        entry,
        VersionSnapshot::new(
            DocumentId::new(document_id).expect("snapshot document id"),
            DocumentSnapshotRef::new(snapshot_ref).expect("snapshot ref"),
            DocumentBody::new(body, DocumentBodyPolicy::new(1024).expect("policy")).expect("body"),
        ),
    )
    .expect("version record")
}

fn asset_reference(hash_character: char, label: &str) -> AssetReference {
    AssetReference::new(
        AssetId::from_sha256_hex(&hash_character.to_string().repeat(64)).expect("asset id"),
        label,
    )
    .expect("asset reference")
}

fn remove_revision_number(path: &PathBuf) {
    let content = fs::read_to_string(path).expect("entry");
    let legacy = content
        .lines()
        .filter(|line| !line.starts_with("revision_number="))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    fs::write(path, legacy).expect("legacy entry");
}

fn replace_revision_number(path: &PathBuf, value: u64) {
    let content = fs::read_to_string(path).expect("entry");
    let mut lines = content
        .lines()
        .filter(|line| !line.starts_with("revision_number="))
        .map(str::to_string)
        .collect::<Vec<_>>();
    lines.push(format!("revision_number={value}"));
    fs::write(path, lines.join("\n") + "\n").expect("entry");
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

fn version_attachments_path(
    temp: &TempVersionRoot,
    workspace_id: &WorkspaceId,
    document_id: &DocumentId,
    version_id: &VersionId,
) -> PathBuf {
    version_dir(temp, workspace_id, document_id, version_id).join(VERSION_ATTACHMENTS_FILE)
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
