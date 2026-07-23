use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_create_document_revision_runtime::{
    LOCAL_DOCUMENT_POINTER_ROOT, LOCAL_DOCUMENT_VERSION_ROOT,
};
use cabinet_adapters::local_current_document_version_pointer::LocalCurrentDocumentVersionPointer;
use cabinet_adapters::local_document_store_migration::{
    AUTHORITATIVE_DOCUMENT_MIGRATION_MARKER, LEGACY_DOCUMENT_POINTER_ROOT,
    LEGACY_DOCUMENT_VERSION_ROOT, LocalDocumentStoreMigration, LocalDocumentStoreMigrationError,
    LocalDocumentStoreMigrationOutcome,
};
use cabinet_adapters::local_version_store::LocalVersionStore;
use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::version::{
    AttachmentSnapshotState, DocumentRevisionNumber, DocumentSnapshotRef, VersionAuthor,
    VersionEntry, VersionId, VersionSummary,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_version::CurrentDocumentVersionPointerPort;
use cabinet_ports::version_store::{VersionRecord, VersionSnapshot, VersionStore};

#[test]
fn completed_migration_does_not_compare_advanced_authoritative_store_with_legacy_source() {
    let temp = TempRoot::new("advanced-authoritative");
    seed_version_and_pointer(
        &temp.path.join(LEGACY_DOCUMENT_VERSION_ROOT),
        &temp.path.join(LEGACY_DOCUMENT_POINTER_ROOT),
        "version-1",
        1,
        None,
    );
    let migration = LocalDocumentStoreMigration::new(temp.path.clone(), policy());
    assert_eq!(
        migration.execute().unwrap(),
        LocalDocumentStoreMigrationOutcome::Migrated
    );

    seed_version_and_pointer(
        &temp.path.join(LOCAL_DOCUMENT_VERSION_ROOT),
        &temp.path.join(LOCAL_DOCUMENT_POINTER_ROOT),
        "version-2",
        2,
        Some("version-1"),
    );
    let authoritative_pointer = current_pointer(&temp.path.join(LOCAL_DOCUMENT_POINTER_ROOT));

    assert_eq!(
        migration.execute().unwrap(),
        LocalDocumentStoreMigrationOutcome::AlreadyMigrated
    );
    assert_eq!(
        current_pointer(&temp.path.join(LOCAL_DOCUMENT_POINTER_ROOT)),
        authoritative_pointer
    );
}

#[test]
fn invalid_completed_marker_never_skips_migration_validation() {
    let temp = TempRoot::new("invalid-marker");
    fs::create_dir_all(temp.path.join(LOCAL_DOCUMENT_VERSION_ROOT)).unwrap();
    fs::create_dir_all(temp.path.join(LOCAL_DOCUMENT_POINTER_ROOT)).unwrap();
    fs::write(
        temp.path.join(AUTHORITATIVE_DOCUMENT_MIGRATION_MARKER),
        "schema=1\nstate=partial\n",
    )
    .unwrap();

    assert_eq!(
        LocalDocumentStoreMigration::new(temp.path.clone(), policy())
            .execute()
            .unwrap_err(),
        LocalDocumentStoreMigrationError::CorruptedLegacy
    );
}

#[test]
fn completed_marker_requires_both_authoritative_roots() {
    let temp = TempRoot::new("missing-authoritative-root");
    fs::create_dir_all(temp.path.join(LOCAL_DOCUMENT_VERSION_ROOT)).unwrap();
    fs::write(
        temp.path.join(AUTHORITATIVE_DOCUMENT_MIGRATION_MARKER),
        "schema=1\nstate=completed\n",
    )
    .unwrap();

    assert_eq!(
        LocalDocumentStoreMigration::new(temp.path.clone(), policy())
            .execute()
            .unwrap_err(),
        LocalDocumentStoreMigrationError::CorruptedLegacy
    );
}

fn seed_version_and_pointer(
    version_root: &PathBuf,
    pointer_root: &PathBuf,
    version: &str,
    revision: u64,
    expected: Option<&str>,
) {
    let workspace = WorkspaceId::new("workspace-1").unwrap();
    let document = DocumentId::new("doc-1").unwrap();
    let version_id = VersionId::new(version).unwrap();
    let snapshot_ref = DocumentSnapshotRef::new(&format!("snapshot-{revision}")).unwrap();
    let entry = VersionEntry::new(
        version_id.clone(),
        document.clone(),
        snapshot_ref.clone(),
        VersionAuthor::new("local-user").unwrap(),
        VersionSummary::new("Migration fixture").unwrap(),
    )
    .unwrap()
    .with_created_at_epoch_ms(revision)
    .unwrap()
    .with_revision_number(DocumentRevisionNumber::new(revision).unwrap())
    .unwrap();
    let snapshot = VersionSnapshot::with_attachment_state(
        document.clone(),
        snapshot_ref,
        DocumentBody::new("# Migration fixture\n", policy()).unwrap(),
        AttachmentSnapshotState::known(Vec::new()).unwrap(),
    );
    LocalVersionStore::with_body_policy(version_root.clone(), policy())
        .append_version(&workspace, VersionRecord::new(entry, snapshot).unwrap())
        .unwrap();
    let expected = expected.map(|value| VersionId::new(value).unwrap());
    LocalCurrentDocumentVersionPointer::new(pointer_root.clone())
        .compare_and_set_current_version(&workspace, &document, expected.as_ref(), version_id)
        .unwrap();
}

fn current_pointer(root: &PathBuf) -> VersionId {
    LocalCurrentDocumentVersionPointer::new(root.clone())
        .load_current_version(
            &WorkspaceId::new("workspace-1").unwrap(),
            &DocumentId::new("doc-1").unwrap(),
        )
        .unwrap()
        .unwrap()
}

fn policy() -> DocumentBodyPolicy {
    DocumentBodyPolicy::new(1024 * 1024).unwrap()
}

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "sponzey-cabinet-document-migration-{name}-{}-{nonce}",
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
