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
    CreateDocumentProductEvent, DeleteDocumentError, DeleteDocumentInput, DeleteDocumentUsecase,
    DocumentChangeEvent, DocumentChangeEventPublisher, DocumentProductLogger,
};

#[derive(Default)]
struct FakeDocumentRepository {
    current: HashMap<(String, String), CurrentDocumentRecord>,
    fail_delete: Cell<bool>,
    delete_count: Cell<usize>,
}

impl FakeDocumentRepository {
    fn insert(&mut self, workspace_id: &str, record: CurrentDocumentRecord) {
        self.current.insert(
            (
                workspace_id.to_string(),
                record.document_id().as_str().to_string(),
            ),
            record,
        );
    }

    fn contains_current(&self, workspace_id: &str, document_id: &str) -> bool {
        self.current
            .contains_key(&(workspace_id.to_string(), document_id.to_string()))
    }
}

impl DocumentRepository for FakeDocumentRepository {
    fn put_current(
        &mut self,
        workspace_id: &WorkspaceId,
        record: CurrentDocumentRecord,
    ) -> Result<(), DocumentRepositoryError> {
        self.current.insert(
            (
                workspace_id.as_str().to_string(),
                record.document_id().as_str().to_string(),
            ),
            record,
        );
        Ok(())
    }

    fn get_current_by_id(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<CurrentDocumentRecord>, DocumentRepositoryError> {
        Ok(self
            .current
            .get(&(
                workspace_id.as_str().to_string(),
                document_id.as_str().to_string(),
            ))
            .cloned())
    }

    fn get_current_by_path(
        &self,
        _workspace_id: &WorkspaceId,
        _path: &DocumentPath,
    ) -> Result<Option<CurrentDocumentRecord>, DocumentRepositoryError> {
        Ok(None)
    }

    fn delete_current(
        &mut self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<(), DocumentRepositoryError> {
        self.delete_count.set(self.delete_count.get() + 1);
        if self.fail_delete.get() {
            return Err(DocumentRepositoryError::StorageUnavailable);
        }
        self.current.remove(&(
            workspace_id.as_str().to_string(),
            document_id.as_str().to_string(),
        ));
        Ok(())
    }
}

#[derive(Default)]
struct FakeEventPublisher {
    events: Vec<DocumentChangeEvent>,
}

impl DocumentChangeEventPublisher for FakeEventPublisher {
    fn publish(&mut self, event: DocumentChangeEvent) {
        self.events.push(event);
    }
}

#[derive(Default)]
struct FakeProductLogger {
    events: Vec<CreateDocumentProductEvent>,
}

impl DocumentProductLogger for FakeProductLogger {
    fn write_product(&mut self, event: CreateDocumentProductEvent) {
        self.events.push(event);
    }
}

#[test]
fn delete_document_removes_current_and_emits_events_without_history_store() {
    let mut documents = FakeDocumentRepository::default();
    documents.insert("workspace-1", current_record("doc-1", "body"));
    let mut publisher = FakeEventPublisher::default();
    let mut logger = FakeProductLogger::default();
    let usecase = DeleteDocumentUsecase::new();

    let output = usecase
        .execute(
            DeleteDocumentInput::new("workspace-1", "doc-1", "version-1"),
            &mut documents,
            &mut publisher,
            &mut logger,
        )
        .expect("delete");

    assert_eq!(output.document_id().as_str(), "doc-1");
    assert!(!documents.contains_current("workspace-1", "doc-1"));
    assert_eq!(documents.delete_count.get(), 1);
    assert_eq!(
        publisher.events,
        vec![DocumentChangeEvent::DocumentDeleted {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
            version_id: "version-1".to_string(),
        }]
    );
    assert_eq!(
        logger.events,
        vec![CreateDocumentProductEvent::DocumentDeleted {
            document_id: "doc-1".to_string(),
        }]
    );
}

#[test]
fn delete_document_reports_not_found_without_delete_call() {
    let mut documents = FakeDocumentRepository::default();
    let mut publisher = FakeEventPublisher::default();
    let mut logger = FakeProductLogger::default();
    let usecase = DeleteDocumentUsecase::new();

    let error = usecase
        .execute(
            DeleteDocumentInput::new("workspace-1", "doc-404", "version-1"),
            &mut documents,
            &mut publisher,
            &mut logger,
        )
        .expect_err("missing current must fail");

    assert_eq!(error, DeleteDocumentError::NotFound);
    assert_eq!(documents.delete_count.get(), 0);
    assert!(publisher.events.is_empty());
    assert_eq!(
        logger.events,
        vec![CreateDocumentProductEvent::UsecaseFailed {
            error_code: "document.not_found",
        }]
    );
}

#[test]
fn delete_document_preserves_current_when_repository_delete_fails() {
    let mut documents = FakeDocumentRepository::default();
    documents.insert("workspace-1", current_record("doc-1", "body"));
    documents.fail_delete.set(true);
    let mut publisher = FakeEventPublisher::default();
    let mut logger = FakeProductLogger::default();
    let usecase = DeleteDocumentUsecase::new();

    let error = usecase
        .execute(
            DeleteDocumentInput::new("workspace-1", "doc-1", "version-1"),
            &mut documents,
            &mut publisher,
            &mut logger,
        )
        .expect_err("delete failure must fail");

    assert_eq!(error, DeleteDocumentError::StorageUnavailable);
    assert!(documents.contains_current("workspace-1", "doc-1"));
    assert_eq!(documents.delete_count.get(), 1);
    assert!(publisher.events.is_empty());
    assert_eq!(
        logger.events,
        vec![CreateDocumentProductEvent::UsecaseFailed {
            error_code: "document.storage_unavailable",
        }]
    );
}

fn current_record(document_id: &str, body: &str) -> CurrentDocumentRecord {
    let metadata = DocumentMetadata::new(
        DocumentId::new(document_id).expect("document id"),
        DocumentTitle::new("Title").expect("title"),
        DocumentPath::new("docs/title.md").expect("path"),
    )
    .expect("metadata");
    let snapshot = CurrentDocumentSnapshot::new(
        DocumentId::new(document_id).expect("document id"),
        DocumentBody::new(body, DocumentBodyPolicy::new(1024).expect("policy")).expect("body"),
    );
    CurrentDocumentRecord::new(metadata, snapshot).expect("record")
}
