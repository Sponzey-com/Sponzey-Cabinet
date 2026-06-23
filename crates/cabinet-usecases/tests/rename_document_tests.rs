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
    CreateDocumentProductEvent, DocumentChangeEvent, DocumentChangeEventPublisher,
    DocumentProductLogger, RenameDocumentError, RenameDocumentInput, RenameDocumentUsecase,
};

#[derive(Default)]
struct FakeDocumentRepository {
    current: HashMap<(String, String), CurrentDocumentRecord>,
    put_count: usize,
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

    fn current_record(&self, workspace_id: &str, document_id: &str) -> &CurrentDocumentRecord {
        self.current
            .get(&(workspace_id.to_string(), document_id.to_string()))
            .expect("current document")
    }
}

impl DocumentRepository for FakeDocumentRepository {
    fn put_current(
        &mut self,
        workspace_id: &WorkspaceId,
        record: CurrentDocumentRecord,
    ) -> Result<(), DocumentRepositoryError> {
        self.put_count += 1;
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
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
    ) -> Result<(), DocumentRepositoryError> {
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
fn rename_document_keeps_identity_and_body_while_replacing_metadata() {
    let mut documents = FakeDocumentRepository::default();
    documents.insert(
        "workspace-1",
        current_record("doc-1", "Old Title", "docs/old.md", "body"),
    );
    let mut publisher = FakeEventPublisher::default();
    let mut logger = FakeProductLogger::default();
    let usecase = RenameDocumentUsecase::new();

    let output = usecase
        .execute(
            RenameDocumentInput::new("workspace-1", "doc-1", "New Title", "docs/new.md"),
            &mut documents,
            &mut publisher,
            &mut logger,
        )
        .expect("rename");

    let record = documents.current_record("workspace-1", "doc-1");
    assert_eq!(output.document_id().as_str(), "doc-1");
    assert_eq!(output.title().as_str(), "New Title");
    assert_eq!(output.path().as_str(), "docs/new.md");
    assert_eq!(record.document_id().as_str(), "doc-1");
    assert_eq!(record.metadata().title().as_str(), "New Title");
    assert_eq!(record.path().as_str(), "docs/new.md");
    assert_eq!(record.body().as_str(), "body");
    assert_eq!(
        publisher.events,
        vec![DocumentChangeEvent::DocumentRenamed {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
            old_path: "docs/old.md".to_string(),
            new_path: "docs/new.md".to_string(),
        }]
    );
    assert_eq!(
        logger.events,
        vec![CreateDocumentProductEvent::DocumentRenamed {
            document_id: "doc-1".to_string(),
        }]
    );
}

#[test]
fn rename_document_rejects_invalid_path_without_writes() {
    let mut documents = FakeDocumentRepository::default();
    documents.insert(
        "workspace-1",
        current_record("doc-1", "Old Title", "docs/old.md", "body"),
    );
    let mut publisher = FakeEventPublisher::default();
    let mut logger = FakeProductLogger::default();
    let usecase = RenameDocumentUsecase::new();

    let error = usecase
        .execute(
            RenameDocumentInput::new("workspace-1", "doc-1", "New Title", "/absolute.md"),
            &mut documents,
            &mut publisher,
            &mut logger,
        )
        .expect_err("invalid path must fail");

    let record = documents.current_record("workspace-1", "doc-1");
    assert_eq!(error, RenameDocumentError::InvalidDocumentInput);
    assert_eq!(record.metadata().title().as_str(), "Old Title");
    assert_eq!(record.path().as_str(), "docs/old.md");
    assert_eq!(documents.put_count, 0);
    assert!(publisher.events.is_empty());
    assert_eq!(
        logger.events,
        vec![CreateDocumentProductEvent::UsecaseFailed {
            error_code: "document.invalid_input",
        }]
    );
}

#[test]
fn rename_document_reports_not_found_for_missing_current_without_writes() {
    let mut documents = FakeDocumentRepository::default();
    let mut publisher = FakeEventPublisher::default();
    let mut logger = FakeProductLogger::default();
    let usecase = RenameDocumentUsecase::new();

    let error = usecase
        .execute(
            RenameDocumentInput::new("workspace-1", "doc-404", "New Title", "docs/new.md"),
            &mut documents,
            &mut publisher,
            &mut logger,
        )
        .expect_err("missing current must fail");

    assert_eq!(error, RenameDocumentError::NotFound);
    assert_eq!(documents.put_count, 0);
    assert!(publisher.events.is_empty());
    assert_eq!(
        logger.events,
        vec![CreateDocumentProductEvent::UsecaseFailed {
            error_code: "document.not_found",
        }]
    );
}

fn current_record(document_id: &str, title: &str, path: &str, body: &str) -> CurrentDocumentRecord {
    let metadata = DocumentMetadata::new(
        DocumentId::new(document_id).expect("document id"),
        DocumentTitle::new(title).expect("title"),
        DocumentPath::new(path).expect("path"),
    )
    .expect("metadata");
    let snapshot = CurrentDocumentSnapshot::new(
        DocumentId::new(document_id).expect("document id"),
        DocumentBody::new(body, DocumentBodyPolicy::new(1024).expect("policy")).expect("body"),
    );
    CurrentDocumentRecord::new(metadata, snapshot).expect("record")
}
