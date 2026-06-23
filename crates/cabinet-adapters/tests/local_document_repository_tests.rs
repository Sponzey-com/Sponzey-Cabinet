use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_document_repository::{
    DOCUMENT_BODY_FILE, DOCUMENT_METADATA_FILE, DOCUMENTS_BY_ID_DIR, DOCUMENTS_BY_PATH_DIR,
    LOCAL_DOCUMENTS_DIR, LocalDocumentRepository,
};
use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentMetadata, DocumentPath, DocumentTitle,
};
use cabinet_domain::version::CurrentDocumentSnapshot;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_repository::{
    CurrentDocumentRecord, DocumentRepository, DocumentRepositoryError,
};

struct TempWorkspaceRoot {
    path: PathBuf,
}

impl TempWorkspaceRoot {
    fn new(test_name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let path = PathBuf::from("/tmp").join(format!(
            "sponzey-cabinet-doc-repo-{test_name}-{}-{nanos}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("temp workspace root");
        Self { path }
    }
}

impl Drop for TempWorkspaceRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn local_document_repository_writes_and_reads_current_snapshot_by_id() {
    let temp = TempWorkspaceRoot::new("read-by-id");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let record = current_record(
        "doc-1",
        "Decision Log",
        "docs/decision-log.md",
        "Current body",
    );
    let document_id = record.document_id().clone();
    let mut repository = LocalDocumentRepository::new(temp.path.clone());

    repository
        .put_current(&workspace_id, record)
        .expect("put current");
    let loaded = repository
        .get_current_by_id(&workspace_id, &document_id)
        .expect("get current")
        .expect("current record");

    assert_eq!(loaded.document_id().as_str(), "doc-1");
    assert_eq!(loaded.path().as_str(), "docs/decision-log.md");
    assert_eq!(loaded.body().as_str(), "Current body");
    assert!(document_metadata_path(&temp, &workspace_id, &document_id).is_file());
    assert!(document_body_path(&temp, &workspace_id, &document_id).is_file());
}

#[test]
fn local_document_repository_reads_current_snapshot_by_path_with_direct_lookup_index() {
    let temp = TempWorkspaceRoot::new("read-by-path");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let record = current_record(
        "doc-1",
        "Decision Log",
        "docs/decision-log.md",
        "Current body",
    );
    let document_path = record.path().clone();
    let mut repository = LocalDocumentRepository::new(temp.path.clone());

    repository
        .put_current(&workspace_id, record)
        .expect("put current");
    let loaded = repository
        .get_current_by_path(&workspace_id, &document_path)
        .expect("get current")
        .expect("current record");

    assert_eq!(loaded.document_id().as_str(), "doc-1");
    assert_eq!(loaded.body().as_str(), "Current body");
    assert_eq!(
        fs::read_to_string(path_index_path(&temp, &workspace_id, &document_path))
            .expect("path index"),
        "doc-1\n"
    );
}

#[test]
fn local_document_repository_delete_removes_id_and_path_lookup() {
    let temp = TempWorkspaceRoot::new("delete");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let record = current_record(
        "doc-1",
        "Decision Log",
        "docs/decision-log.md",
        "Current body",
    );
    let document_id = record.document_id().clone();
    let document_path = record.path().clone();
    let mut repository = LocalDocumentRepository::new(temp.path.clone());

    repository
        .put_current(&workspace_id, record)
        .expect("put current");
    repository
        .delete_current(&workspace_id, &document_id)
        .expect("delete current");

    assert!(
        repository
            .get_current_by_id(&workspace_id, &document_id)
            .expect("get by id")
            .is_none()
    );
    assert!(
        repository
            .get_current_by_path(&workspace_id, &document_path)
            .expect("get by path")
            .is_none()
    );
}

#[test]
fn local_document_repository_reports_corrupted_metadata() {
    let temp = TempWorkspaceRoot::new("corrupt");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let record = current_record(
        "doc-1",
        "Decision Log",
        "docs/decision-log.md",
        "Current body",
    );
    let document_id = record.document_id().clone();
    let mut repository = LocalDocumentRepository::new(temp.path.clone());

    repository
        .put_current(&workspace_id, record)
        .expect("put current");
    fs::write(
        document_metadata_path(&temp, &workspace_id, &document_id),
        "not valid metadata",
    )
    .expect("write corrupt metadata");

    let error = repository
        .get_current_by_id(&workspace_id, &document_id)
        .expect_err("corrupted metadata must fail");

    assert_eq!(error, DocumentRepositoryError::CorruptedMetadata);
    assert_eq!(error.code(), "document_repository.corrupted_metadata");
}

fn current_record(id: &str, title: &str, path: &str, body: &str) -> CurrentDocumentRecord {
    let metadata = DocumentMetadata::new(
        DocumentId::new(id).expect("document id"),
        DocumentTitle::new(title).expect("document title"),
        DocumentPath::new(path).expect("document path"),
    )
    .expect("metadata");
    let snapshot = CurrentDocumentSnapshot::new(
        DocumentId::new(id).expect("snapshot id"),
        DocumentBody::new(body, DocumentBodyPolicy::new(1024).expect("policy")).expect("body"),
    );
    CurrentDocumentRecord::new(metadata, snapshot).expect("current record")
}

fn document_metadata_path(
    temp: &TempWorkspaceRoot,
    workspace_id: &WorkspaceId,
    document_id: &DocumentId,
) -> PathBuf {
    temp.path
        .join(workspace_id.as_str())
        .join(LOCAL_DOCUMENTS_DIR)
        .join(DOCUMENTS_BY_ID_DIR)
        .join(document_id.as_str())
        .join(DOCUMENT_METADATA_FILE)
}

fn document_body_path(
    temp: &TempWorkspaceRoot,
    workspace_id: &WorkspaceId,
    document_id: &DocumentId,
) -> PathBuf {
    temp.path
        .join(workspace_id.as_str())
        .join(LOCAL_DOCUMENTS_DIR)
        .join(DOCUMENTS_BY_ID_DIR)
        .join(document_id.as_str())
        .join(DOCUMENT_BODY_FILE)
}

fn path_index_path(
    temp: &TempWorkspaceRoot,
    workspace_id: &WorkspaceId,
    document_path: &DocumentPath,
) -> PathBuf {
    temp.path
        .join(workspace_id.as_str())
        .join(LOCAL_DOCUMENTS_DIR)
        .join(DOCUMENTS_BY_PATH_DIR)
        .join(format!("{}.ref", document_path.as_str()))
}
