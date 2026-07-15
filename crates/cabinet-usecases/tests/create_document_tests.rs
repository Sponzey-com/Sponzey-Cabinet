use std::collections::HashMap;

use cabinet_domain::document::{DocumentBodyPolicy, DocumentId, DocumentPath};
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_repository::{
    CurrentDocumentRecord, DocumentRepository, DocumentRepositoryError,
};
use cabinet_ports::version_store::{
    HistoryPage, HistoryPageRequest, VersionRecord, VersionSnapshot, VersionStore,
    VersionStoreError,
};
use cabinet_usecases::document::{
    CreateDocumentError, CreateDocumentInput, CreateDocumentProductEvent, CreateDocumentUsecase,
    DocumentChangeEvent, DocumentChangeEventPublisher, DocumentProductLogger,
};

#[derive(Default)]
struct FakeDocumentRepository {
    records: HashMap<(String, String), CurrentDocumentRecord>,
    fail_put: bool,
}

impl DocumentRepository for FakeDocumentRepository {
    fn put_current(
        &mut self,
        workspace_id: &WorkspaceId,
        record: CurrentDocumentRecord,
    ) -> Result<(), DocumentRepositoryError> {
        if self.fail_put {
            return Err(DocumentRepositoryError::StorageUnavailable);
        }
        let key = (
            workspace_id.as_str().to_string(),
            record.document_id().as_str().to_string(),
        );
        if self.records.contains_key(&key) {
            return Err(DocumentRepositoryError::Conflict);
        }
        self.records.insert(key, record);
        Ok(())
    }

    fn get_current_by_id(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<CurrentDocumentRecord>, DocumentRepositoryError> {
        Ok(self
            .records
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
struct FakeVersionStore {
    records: Vec<VersionRecord>,
}

impl VersionStore for FakeVersionStore {
    fn append_version(
        &mut self,
        _workspace_id: &WorkspaceId,
        record: VersionRecord,
    ) -> Result<(), VersionStoreError> {
        self.records.push(record);
        Ok(())
    }

    fn get_version_snapshot(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        _version_id: &VersionId,
    ) -> Result<Option<VersionSnapshot>, VersionStoreError> {
        Ok(None)
    }

    fn list_history(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        _request: HistoryPageRequest,
    ) -> Result<HistoryPage, VersionStoreError> {
        Ok(HistoryPage::new(Vec::new(), None))
    }
}

#[derive(Default)]
struct FakeDocumentEventPublisher {
    events: Vec<DocumentChangeEvent>,
}

impl DocumentChangeEventPublisher for FakeDocumentEventPublisher {
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
fn create_document_stores_current_snapshot_version_and_change_event() {
    let mut documents = FakeDocumentRepository::default();
    let mut versions = FakeVersionStore::default();
    let mut events = FakeDocumentEventPublisher::default();
    let mut logger = FakeProductLogger::default();
    let usecase = CreateDocumentUsecase::new(DocumentBodyPolicy::new(1024).expect("policy"));

    let output = usecase
        .execute(
            valid_input("# First-line title\nHello body"),
            &mut documents,
            &mut versions,
            &mut events,
            &mut logger,
        )
        .expect("document should be created");

    assert_eq!(output.document_id().as_str(), "doc-1");
    assert_eq!(output.version_id().as_str(), "version-1");
    assert_eq!(versions.records.len(), 1);
    assert_eq!(
        versions.records[0].snapshot().body().as_str(),
        "# First-line title\nHello body"
    );
    assert_eq!(
        documents
            .get_current_by_id(
                &WorkspaceId::new("workspace-1").expect("workspace id"),
                output.document_id(),
            )
            .expect("get current")
            .expect("current")
            .body()
            .as_str(),
        "# First-line title\nHello body"
    );
    assert_eq!(
        events.events,
        vec![DocumentChangeEvent::DocumentCreated {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
            version_id: "version-1".to_string(),
            title: "First-line title".to_string(),
            path: "private/path.md".to_string(),
        }]
    );
    assert_eq!(
        logger.events,
        vec![CreateDocumentProductEvent::DocumentCreated {
            document_id: "doc-1".to_string(),
        }]
    );
}

#[test]
fn create_document_rejects_invalid_body_before_writes_and_logs_failure() {
    let mut documents = FakeDocumentRepository::default();
    let mut versions = FakeVersionStore::default();
    let mut events = FakeDocumentEventPublisher::default();
    let mut logger = FakeProductLogger::default();
    let usecase = CreateDocumentUsecase::new(DocumentBodyPolicy::new(4).expect("policy"));

    let error = usecase
        .execute(
            valid_input("too long"),
            &mut documents,
            &mut versions,
            &mut events,
            &mut logger,
        )
        .expect_err("invalid body must fail");

    assert_eq!(error, CreateDocumentError::InvalidDocumentInput);
    assert!(documents.records.is_empty());
    assert!(versions.records.is_empty());
    assert!(events.events.is_empty());
    assert_eq!(
        logger.events,
        vec![CreateDocumentProductEvent::UsecaseFailed {
            error_code: "document.invalid_input",
        }]
    );
}

#[test]
fn create_document_repository_failure_skips_version_and_change_event() {
    let mut documents = FakeDocumentRepository {
        fail_put: true,
        ..FakeDocumentRepository::default()
    };
    let mut versions = FakeVersionStore::default();
    let mut events = FakeDocumentEventPublisher::default();
    let mut logger = FakeProductLogger::default();
    let usecase = CreateDocumentUsecase::new(DocumentBodyPolicy::new(1024).expect("policy"));

    let error = usecase
        .execute(
            valid_input("Hello body"),
            &mut documents,
            &mut versions,
            &mut events,
            &mut logger,
        )
        .expect_err("storage failure must fail");

    assert_eq!(error, CreateDocumentError::StorageUnavailable);
    assert!(versions.records.is_empty());
    assert!(events.events.is_empty());
}

#[test]
fn create_document_product_log_excludes_document_body_title_and_path() {
    let mut documents = FakeDocumentRepository::default();
    let mut versions = FakeVersionStore::default();
    let mut events = FakeDocumentEventPublisher::default();
    let mut logger = FakeProductLogger::default();
    let usecase = CreateDocumentUsecase::new(DocumentBodyPolicy::new(1024).expect("policy"));

    usecase
        .execute(
            valid_input("private body"),
            &mut documents,
            &mut versions,
            &mut events,
            &mut logger,
        )
        .expect("document should be created");

    let rendered_log = format!("{:?}", logger.events);
    assert!(!rendered_log.contains("private body"));
    assert!(!rendered_log.contains("Private Title"));
    assert!(!rendered_log.contains("private/path.md"));
}

fn valid_input(body: &str) -> CreateDocumentInput {
    CreateDocumentInput::new(
        "workspace-1",
        "doc-1",
        "private/path.md",
        body,
        "version-1",
        "snapshot-1",
        "writer",
        "Initial save",
    )
}
