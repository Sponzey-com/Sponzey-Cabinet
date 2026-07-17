use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_version_store::{
    LocalVersionStore, VERSION_ATTACHMENTS_FILE, VERSION_BODY_FILE, VERSION_DOCUMENTS_DIR,
    VERSION_ENTRY_FILE, VERSION_SNAPSHOTS_DIR,
};
use cabinet_domain::asset::{AssetId, AssetReference};
use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::document_revision::DocumentOperationId;
use cabinet_domain::version::{
    AttachmentSnapshotState, DocumentRevisionNumber, DocumentSnapshotRef, VersionAuthor,
    VersionEntry, VersionId, VersionSummary,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::committed_version_record_reader::{
    CommittedVersionRecordReadError, CommittedVersionRecordReader,
};
use cabinet_ports::version_preparation::VersionPreparationPort;
use cabinet_ports::version_publication::VersionPublicationPort;
use cabinet_ports::version_store::{VersionRecord, VersionSnapshot};

#[test]
fn reads_full_published_record_with_revision_body_and_attachments_after_restart() {
    let temp = TempRoot::new("restart");
    publish_record(&temp.path, "version-1", "제목\n본문", true);
    let restarted = local_store(&temp.path);

    let loaded = restarted
        .get_committed_version_record(&workspace(), &document(), &version("version-1"))
        .expect("read succeeds")
        .expect("record exists");

    assert_eq!(loaded.version_id().as_str(), "version-1");
    assert_eq!(loaded.entry().revision_number().unwrap().value(), 1);
    assert!(loaded.entry().created_at_epoch_ms().is_some());
    assert_eq!(loaded.snapshot().body().as_str(), "제목\n본문");
    let references = loaded
        .snapshot()
        .attachment_state()
        .references()
        .expect("known attachments");
    assert_eq!(references.len(), 1);
    assert_eq!(references[0].label(), "설계 자료");
}

#[test]
fn returns_none_for_missing_published_version() {
    let temp = TempRoot::new("missing");
    let store = local_store(&temp.path);

    let loaded = store
        .get_committed_version_record(&workspace(), &document(), &version("missing"))
        .expect("missing is not an error");

    assert!(loaded.is_none());
}

#[test]
fn rejects_corrupt_entry_body_and_attachment_sidecar_with_stable_error() {
    let cases = [
        VERSION_ENTRY_FILE,
        VERSION_BODY_FILE,
        VERSION_ATTACHMENTS_FILE,
    ];
    for target in cases {
        let temp = TempRoot::new(target);
        publish_record(&temp.path, "version-1", "제목\n본문", true);
        let path = published_dir(&temp.path).join(target);
        if target == VERSION_BODY_FILE {
            fs::write(path, "x".repeat(5000)).unwrap();
        } else {
            fs::write(path, "corrupted\n").unwrap();
        }
        let store = local_store(&temp.path);

        let error = store
            .get_committed_version_record(&workspace(), &document(), &version("version-1"))
            .expect_err("corrupt record must fail");

        assert_eq!(error, CommittedVersionRecordReadError::CorruptedRecord);
        assert_eq!(error.code(), "committed_version_record.corrupted_record");
    }
}

fn publish_record(root: &Path, version_id: &str, body: &str, with_attachment: bool) {
    let mut store = local_store(root);
    let operation = DocumentOperationId::new("operation-1").unwrap();
    store
        .prepare_version(
            &workspace(),
            &operation,
            version_record(version_id, body, with_attachment),
        )
        .unwrap();
    store.publish_prepared(&workspace(), &operation).unwrap();
}

fn version_record(version_id: &str, body: &str, with_attachment: bool) -> VersionRecord {
    let snapshot_ref = DocumentSnapshotRef::new(&format!("snapshot:{version_id}")).unwrap();
    let entry = VersionEntry::new(
        version(version_id),
        document(),
        snapshot_ref.clone(),
        VersionAuthor::new("local-user").unwrap(),
        VersionSummary::new("save").unwrap(),
    )
    .unwrap()
    .with_created_at_epoch_ms(1_700_000_000_000)
    .unwrap()
    .with_revision_number(DocumentRevisionNumber::new(1).unwrap())
    .unwrap();
    let attachments = if with_attachment {
        AttachmentSnapshotState::known(vec![
            AssetReference::new(
                AssetId::from_sha256_hex(&"a".repeat(64)).unwrap(),
                "설계 자료",
            )
            .unwrap(),
        ])
        .unwrap()
    } else {
        AttachmentSnapshotState::known(Vec::new()).unwrap()
    };
    VersionRecord::new(
        entry,
        VersionSnapshot::with_attachment_state(
            document(),
            snapshot_ref,
            DocumentBody::new(body, body_policy()).unwrap(),
            attachments,
        ),
    )
    .unwrap()
}

fn local_store(root: &Path) -> LocalVersionStore {
    LocalVersionStore::with_body_policy(root.to_path_buf(), body_policy())
}

fn body_policy() -> DocumentBodyPolicy {
    DocumentBodyPolicy::new(4096).unwrap()
}

fn workspace() -> WorkspaceId {
    WorkspaceId::new("workspace-1").unwrap()
}

fn document() -> DocumentId {
    DocumentId::new("document-1").unwrap()
}

fn version(value: &str) -> VersionId {
    VersionId::new(value).unwrap()
}

fn published_dir(root: &Path) -> PathBuf {
    root.join("workspace-1")
        .join(VERSION_DOCUMENTS_DIR)
        .join("document-1")
        .join(VERSION_SNAPSHOTS_DIR)
        .join("version-1")
}

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "cabinet-version-record-reader-{name}-{}-{nanos}",
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
