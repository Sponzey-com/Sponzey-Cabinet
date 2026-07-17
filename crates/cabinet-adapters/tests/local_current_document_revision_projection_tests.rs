use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_create_document_revision_runtime::LOCAL_DOCUMENT_POINTER_ROOT;
use cabinet_adapters::local_current_document_revision_projection::{
    LOCAL_CURRENT_DOCUMENT_PROJECTION_ROOT, LOCAL_CURRENT_DOCUMENT_REVISION_IDENTITY_ROOT,
    LocalCurrentDocumentRevisionProjectionWriter,
};
use cabinet_adapters::local_current_document_version_pointer::LocalCurrentDocumentVersionPointer;
use cabinet_adapters::local_document_repository::LocalDocumentRepository;
use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentMetadata, DocumentPath, DocumentTitle,
};
use cabinet_domain::version::{
    AttachmentSnapshotState, CurrentDocumentSnapshot, DocumentRevisionNumber, DocumentSnapshotRef,
    VersionAuthor, VersionEntry, VersionId, VersionSummary,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_version::CurrentDocumentVersionPointerPort;
use cabinet_ports::document_repository::{
    CurrentDocumentRecord, DocumentRepository, DocumentRepositoryError,
};
use cabinet_ports::version_store::{VersionRecord, VersionSnapshot};
use cabinet_usecases::project_current_document_revision::{
    ProjectCurrentDocumentRevisionError, ProjectCurrentDocumentRevisionInput,
    ProjectCurrentDocumentRevisionUsecase,
};

#[test]
fn applies_current_projection_and_replays_idempotently_after_restart() {
    let temp = TempRoot::new("apply-restart");
    set_pointer(&temp.path, None, "version-1");
    let input = projection_input("version-1", 1, "# 첫 제목\n본문", "notes/doc.md");
    let mut writer = local_writer(&temp.path);

    let first = ProjectCurrentDocumentRevisionUsecase::new()
        .execute(input.clone(), &mut writer)
        .expect("first projection");
    drop(writer);
    let mut restarted = local_writer(&temp.path);
    let replayed = ProjectCurrentDocumentRevisionUsecase::new()
        .execute(input, &mut restarted)
        .expect("restart replay");

    assert_eq!(format!("{:?}", first.outcome()), "Applied");
    assert_eq!(format!("{:?}", replayed.outcome()), "AlreadyCurrent");
    let current = current_record(&temp.path).expect("current record");
    assert_eq!(current.metadata().title().as_str(), "첫 제목");
    assert_eq!(current.path().as_str(), "notes/doc.md");
    assert_eq!(current.body().as_str(), "# 첫 제목\n본문");
    assert!(identity_path(&temp.path).is_file());
}

#[test]
fn applies_higher_revision_and_rejects_stale_or_same_revision_conflict_without_changes() {
    let temp = TempRoot::new("revision-guards");
    set_pointer(&temp.path, None, "version-1");
    let mut writer = local_writer(&temp.path);
    ProjectCurrentDocumentRevisionUsecase::new()
        .execute(
            projection_input("version-1", 1, "첫 제목\n본문 1", "doc.md"),
            &mut writer,
        )
        .unwrap();
    set_pointer(&temp.path, Some("version-1"), "version-2");
    ProjectCurrentDocumentRevisionUsecase::new()
        .execute(
            projection_input("version-2", 2, "두 번째 제목\n본문 2", "doc.md"),
            &mut writer,
        )
        .expect("revision 2 projection");
    let before = projection_bytes(&temp.path);

    let stale = ProjectCurrentDocumentRevisionUsecase::new()
        .execute(
            projection_input("version-1", 1, "오래된 제목\n오래된 본문", "doc.md"),
            &mut writer,
        )
        .expect_err("stale pointer must fail");
    assert_eq!(stale, ProjectCurrentDocumentRevisionError::StaleRevision);
    assert_eq!(projection_bytes(&temp.path), before);

    set_pointer(&temp.path, Some("version-2"), "version-2-conflict");
    let conflict = ProjectCurrentDocumentRevisionUsecase::new()
        .execute(
            projection_input("version-2-conflict", 2, "충돌 제목\n충돌 본문", "doc.md"),
            &mut writer,
        )
        .expect_err("same revision different version must fail");
    assert_eq!(
        conflict,
        ProjectCurrentDocumentRevisionError::RevisionConflict
    );
    assert_eq!(projection_bytes(&temp.path), before);
    assert_eq!(
        current_record(&temp.path)
            .unwrap()
            .metadata()
            .title()
            .as_str(),
        "두 번째 제목"
    );
}

#[test]
fn rejects_corrupt_identity_without_mutating_current_projection() {
    let temp = TempRoot::new("corrupt-sidecar");
    set_pointer(&temp.path, None, "version-1");
    let mut writer = local_writer(&temp.path);
    ProjectCurrentDocumentRevisionUsecase::new()
        .execute(
            projection_input("version-1", 1, "정상 제목\n본문", "doc.md"),
            &mut writer,
        )
        .unwrap();
    fs::write(identity_path(&temp.path), "not-a-valid-projection\n").unwrap();
    let before = projection_bytes(&temp.path);

    let error = ProjectCurrentDocumentRevisionUsecase::new()
        .execute(
            projection_input("version-1", 1, "변조 제목\n변조 본문", "doc.md"),
            &mut writer,
        )
        .expect_err("corrupt identity must fail");

    assert_eq!(
        error,
        ProjectCurrentDocumentRevisionError::CorruptedProjection
    );
    assert_eq!(projection_bytes(&temp.path), before);
    assert_eq!(
        current_record(&temp.path).unwrap().body().as_str(),
        "정상 제목\n본문"
    );
}

#[test]
fn adds_revision_identity_to_legacy_current_record_without_losing_content_or_path() {
    let temp = TempRoot::new("legacy-current");
    let workspace = WorkspaceId::new("workspace-1").unwrap();
    let legacy = legacy_current("기존 제목\n기존 본문", "legacy/doc.md");
    let mut repository = LocalDocumentRepository::with_body_policy(
        temp.path.join(LOCAL_CURRENT_DOCUMENT_PROJECTION_ROOT),
        body_policy(),
    );
    repository.put_current(&workspace, legacy).unwrap();
    assert!(!identity_path(&temp.path).exists());
    set_pointer(&temp.path, None, "version-1");

    let mut writer = local_writer(&temp.path);
    let output = ProjectCurrentDocumentRevisionUsecase::new()
        .execute(
            projection_input("version-1", 1, "기존 제목\n기존 본문", "legacy/doc.md"),
            &mut writer,
        )
        .expect("legacy projection migration");

    assert_eq!(format!("{:?}", output.outcome()), "Applied");
    let migrated = current_record(&temp.path).unwrap();
    assert_eq!(migrated.path().as_str(), "legacy/doc.md");
    assert_eq!(migrated.body().as_str(), "기존 제목\n기존 본문");
    assert!(identity_path(&temp.path).is_file());
}

fn local_writer(root: &Path) -> LocalCurrentDocumentRevisionProjectionWriter {
    LocalCurrentDocumentRevisionProjectionWriter::new(root.to_path_buf(), body_policy())
}

fn set_pointer(root: &Path, expected: Option<&str>, next: &str) {
    let workspace = WorkspaceId::new("workspace-1").unwrap();
    let document = DocumentId::new("document-1").unwrap();
    let expected = expected.map(|value| VersionId::new(value).unwrap());
    LocalCurrentDocumentVersionPointer::new(root.join(LOCAL_DOCUMENT_POINTER_ROOT))
        .compare_and_set_current_version(
            &workspace,
            &document,
            expected.as_ref(),
            VersionId::new(next).unwrap(),
        )
        .unwrap();
}

fn projection_input(
    version_id: &str,
    revision: u64,
    body: &str,
    path: &str,
) -> ProjectCurrentDocumentRevisionInput {
    ProjectCurrentDocumentRevisionInput::new(
        "workspace-1",
        path,
        version_record(version_id, revision, body),
    )
}

fn version_record(version_id: &str, revision: u64, body: &str) -> VersionRecord {
    let document = DocumentId::new("document-1").unwrap();
    let snapshot_ref = DocumentSnapshotRef::new(&format!("snapshot:{version_id}")).unwrap();
    let entry = VersionEntry::new(
        VersionId::new(version_id).unwrap(),
        document.clone(),
        snapshot_ref.clone(),
        VersionAuthor::new("local-user").unwrap(),
        VersionSummary::new("save").unwrap(),
    )
    .unwrap()
    .with_revision_number(DocumentRevisionNumber::new(revision).unwrap())
    .unwrap();
    let snapshot = VersionSnapshot::with_attachment_state(
        document,
        snapshot_ref,
        DocumentBody::new(body, body_policy()).unwrap(),
        AttachmentSnapshotState::known(Vec::new()).unwrap(),
    );
    VersionRecord::new(entry, snapshot).unwrap()
}

fn legacy_current(body: &str, path: &str) -> CurrentDocumentRecord {
    let document = DocumentId::new("document-1").unwrap();
    CurrentDocumentRecord::new(
        DocumentMetadata::new(
            document.clone(),
            DocumentTitle::from_markdown_text(body),
            DocumentPath::new(path).unwrap(),
        )
        .unwrap(),
        CurrentDocumentSnapshot::new(document, DocumentBody::new(body, body_policy()).unwrap()),
    )
    .unwrap()
}

fn current_record(root: &Path) -> Result<CurrentDocumentRecord, DocumentRepositoryError> {
    LocalDocumentRepository::with_body_policy(
        root.join(LOCAL_CURRENT_DOCUMENT_PROJECTION_ROOT),
        body_policy(),
    )
    .get_current_by_id(
        &WorkspaceId::new("workspace-1").unwrap(),
        &DocumentId::new("document-1").unwrap(),
    )?
    .ok_or(DocumentRepositoryError::CorruptedMetadata)
}

fn identity_path(root: &Path) -> PathBuf {
    root.join(LOCAL_CURRENT_DOCUMENT_REVISION_IDENTITY_ROOT)
        .join(hex("workspace-1"))
        .join(hex("document-1"))
        .join("current.projection")
}

fn projection_bytes(root: &Path) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let current_root = root
        .join(LOCAL_CURRENT_DOCUMENT_PROJECTION_ROOT)
        .join("workspace-1/documents/by-id/document-1");
    (
        fs::read(current_root.join("metadata.txt")).unwrap(),
        fs::read(current_root.join("body.md")).unwrap(),
        fs::read(identity_path(root)).unwrap(),
    )
}

fn body_policy() -> DocumentBodyPolicy {
    DocumentBodyPolicy::new(4096).unwrap()
}

fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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
            "cabinet-current-projection-{name}-{}-{nanos}",
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
