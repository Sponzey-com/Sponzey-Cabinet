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
use cabinet_usecases::document::{
    GetCurrentDocumentError, GetCurrentDocumentInput, GetCurrentDocumentUsecase,
};

#[derive(Default)]
struct FakeDocumentRepository {
    by_id: HashMap<(String, String), CurrentDocumentRecord>,
    by_path: HashMap<(String, String), CurrentDocumentRecord>,
    current_read_count: Cell<usize>,
    history_scan_count: Cell<usize>,
}

impl FakeDocumentRepository {
    fn insert(&mut self, workspace_id: &WorkspaceId, record: CurrentDocumentRecord) {
        self.by_id.insert(
            (
                workspace_id.as_str().to_string(),
                record.document_id().as_str().to_string(),
            ),
            record.clone(),
        );
        self.by_path.insert(
            (
                workspace_id.as_str().to_string(),
                record.path().as_str().to_string(),
            ),
            record,
        );
    }
}

impl DocumentRepository for FakeDocumentRepository {
    fn put_current(
        &mut self,
        workspace_id: &WorkspaceId,
        record: CurrentDocumentRecord,
    ) -> Result<(), DocumentRepositoryError> {
        self.insert(workspace_id, record);
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
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
    ) -> Result<(), DocumentRepositoryError> {
        Ok(())
    }
}

#[test]
fn get_current_document_by_id_uses_current_repository_without_history_scan() {
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let mut repository = FakeDocumentRepository::default();
    repository.insert(
        &workspace_id,
        current_record("doc-1", "Private Title", "private/path.md", "Current body"),
    );
    let usecase = GetCurrentDocumentUsecase::new();

    let output = usecase
        .execute(
            GetCurrentDocumentInput::by_id("workspace-1", "doc-1"),
            &repository,
        )
        .expect("current document");

    assert_eq!(output.record().body().as_str(), "Current body");
    assert_eq!(repository.current_read_count.get(), 1);
    assert_eq!(repository.history_scan_count.get(), 0);
}

#[test]
fn get_current_document_by_path_uses_current_repository_without_history_scan() {
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let mut repository = FakeDocumentRepository::default();
    repository.insert(
        &workspace_id,
        current_record("doc-1", "Private Title", "private/path.md", "Current body"),
    );
    let usecase = GetCurrentDocumentUsecase::new();

    let output = usecase
        .execute(
            GetCurrentDocumentInput::by_path("workspace-1", "private/path.md"),
            &repository,
        )
        .expect("current document");

    assert_eq!(output.record().document_id().as_str(), "doc-1");
    assert_eq!(repository.current_read_count.get(), 1);
    assert_eq!(repository.history_scan_count.get(), 0);
}

#[test]
fn get_current_document_reports_not_found_for_missing_current_snapshot() {
    let repository = FakeDocumentRepository::default();
    let usecase = GetCurrentDocumentUsecase::new();

    let error = usecase
        .execute(
            GetCurrentDocumentInput::by_id("workspace-1", "missing-doc"),
            &repository,
        )
        .expect_err("missing current must fail");

    assert_eq!(error, GetCurrentDocumentError::NotFound);
    assert_eq!(repository.current_read_count.get(), 1);
    assert_eq!(repository.history_scan_count.get(), 0);
}

#[test]
fn get_current_document_rejects_invalid_input_before_repository_read() {
    let repository = FakeDocumentRepository::default();
    let usecase = GetCurrentDocumentUsecase::new();

    let error = usecase
        .execute(
            GetCurrentDocumentInput::by_path("workspace-1", "../bad.md"),
            &repository,
        )
        .expect_err("invalid path must fail");

    assert_eq!(error, GetCurrentDocumentError::InvalidInput);
    assert_eq!(repository.current_read_count.get(), 0);
    assert_eq!(repository.history_scan_count.get(), 0);
}

fn current_record(id: &str, title: &str, path: &str, body: &str) -> CurrentDocumentRecord {
    let metadata = DocumentMetadata::new(
        DocumentId::new(id).expect("document id"),
        DocumentTitle::new(title).expect("title"),
        DocumentPath::new(path).expect("path"),
    )
    .expect("metadata");
    let snapshot = CurrentDocumentSnapshot::new(
        DocumentId::new(id).expect("snapshot id"),
        DocumentBody::new(body, DocumentBodyPolicy::new(1024).expect("policy")).expect("body"),
    );
    CurrentDocumentRecord::new(metadata, snapshot).expect("record")
}
