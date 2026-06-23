use std::cell::Cell;
use std::collections::HashMap;

use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentMetadata, DocumentPath, DocumentTitle,
};
use cabinet_domain::version::CurrentDocumentSnapshot;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_repository::{
    CurrentDocumentRecord, DocumentRepository, DocumentRepositoryError,
};

#[derive(Default)]
struct FakeDocumentRepository {
    by_id: HashMap<(String, String), CurrentDocumentRecord>,
    by_path: HashMap<(String, String), CurrentDocumentRecord>,
    current_read_count: Cell<usize>,
    history_scan_count: Cell<usize>,
}

impl FakeDocumentRepository {
    fn history_scan_count(&self) -> usize {
        self.history_scan_count.get()
    }

    fn current_read_count(&self) -> usize {
        self.current_read_count.get()
    }
}

impl DocumentRepository for FakeDocumentRepository {
    fn put_current(
        &mut self,
        workspace_id: &WorkspaceId,
        record: CurrentDocumentRecord,
    ) -> Result<(), DocumentRepositoryError> {
        let id_key = (
            workspace_id.as_str().to_string(),
            record.document_id().as_str().to_string(),
        );
        let path_key = (
            workspace_id.as_str().to_string(),
            record.path().as_str().to_string(),
        );
        self.by_id.insert(id_key, record.clone());
        self.by_path.insert(path_key, record);
        Ok(())
    }

    fn get_current_by_id(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<CurrentDocumentRecord>, DocumentRepositoryError> {
        self.current_read_count
            .set(self.current_read_count.get() + 1);
        Ok(self
            .by_id
            .get(&(
                workspace_id.as_str().to_string(),
                document_id.as_str().to_string(),
            ))
            .cloned())
    }

    fn get_current_by_path(
        &self,
        workspace_id: &WorkspaceId,
        path: &DocumentPath,
    ) -> Result<Option<CurrentDocumentRecord>, DocumentRepositoryError> {
        self.current_read_count
            .set(self.current_read_count.get() + 1);
        Ok(self
            .by_path
            .get(&(workspace_id.as_str().to_string(), path.as_str().to_string()))
            .cloned())
    }

    fn delete_current(
        &mut self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<(), DocumentRepositoryError> {
        if let Some(record) = self.by_id.remove(&(
            workspace_id.as_str().to_string(),
            document_id.as_str().to_string(),
        )) {
            self.by_path.remove(&(
                workspace_id.as_str().to_string(),
                record.path().as_str().to_string(),
            ));
        }
        Ok(())
    }
}

#[test]
fn current_document_record_rejects_mismatched_metadata_and_snapshot_identity() {
    let metadata = document_metadata("doc-1", "Decision Log", "docs/decision-log.md");
    let snapshot = CurrentDocumentSnapshot::new(
        DocumentId::new("doc-2").expect("snapshot id"),
        document_body("Body"),
    );

    let error =
        CurrentDocumentRecord::new(metadata, snapshot).expect_err("mismatched identity must fail");

    assert_eq!(error, DocumentRepositoryError::MismatchedDocumentIdentity);
    assert_eq!(
        error.code(),
        "document_repository.mismatched_document_identity"
    );
}

#[test]
fn repository_contract_reads_current_snapshot_by_id_without_history_scan() {
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let record = current_record(
        "doc-1",
        "Decision Log",
        "docs/decision-log.md",
        "Current body",
    );
    let document_id = record.document_id().clone();
    let mut repository = FakeDocumentRepository::default();

    repository
        .put_current(&workspace_id, record)
        .expect("put current");
    let loaded = repository
        .get_current_by_id(&workspace_id, &document_id)
        .expect("get current")
        .expect("current record");

    assert_eq!(loaded.body().as_str(), "Current body");
    assert_eq!(repository.current_read_count(), 1);
    assert_eq!(repository.history_scan_count(), 0);
}

#[test]
fn repository_contract_reads_current_snapshot_by_path_without_history_scan() {
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let record = current_record(
        "doc-1",
        "Decision Log",
        "docs/decision-log.md",
        "Current body",
    );
    let document_path = record.path().clone();
    let mut repository = FakeDocumentRepository::default();

    repository
        .put_current(&workspace_id, record)
        .expect("put current");
    let loaded = repository
        .get_current_by_path(&workspace_id, &document_path)
        .expect("get current")
        .expect("current record");

    assert_eq!(loaded.document_id().as_str(), "doc-1");
    assert_eq!(repository.current_read_count(), 1);
    assert_eq!(repository.history_scan_count(), 0);
}

#[test]
fn repository_contract_reports_missing_current_snapshot_as_none() {
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let document_id = DocumentId::new("missing-doc").expect("document id");
    let repository = FakeDocumentRepository::default();

    let loaded = repository
        .get_current_by_id(&workspace_id, &document_id)
        .expect("get current");

    assert!(loaded.is_none());
    assert_eq!(repository.current_read_count(), 1);
    assert_eq!(repository.history_scan_count(), 0);
}

fn current_record(id: &str, title: &str, path: &str, body: &str) -> CurrentDocumentRecord {
    let metadata = document_metadata(id, title, path);
    let snapshot = CurrentDocumentSnapshot::new(
        DocumentId::new(id).expect("snapshot id"),
        document_body(body),
    );
    CurrentDocumentRecord::new(metadata, snapshot).expect("current record")
}

fn document_metadata(id: &str, title: &str, path: &str) -> DocumentMetadata {
    DocumentMetadata::new(
        DocumentId::new(id).expect("document id"),
        DocumentTitle::new(title).expect("document title"),
        DocumentPath::new(path).expect("document path"),
    )
    .expect("metadata")
}

fn document_body(value: &str) -> DocumentBody {
    DocumentBody::new(value, DocumentBodyPolicy::new(1024).expect("policy")).expect("body")
}
