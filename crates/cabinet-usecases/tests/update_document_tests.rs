use std::collections::HashMap;

use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentMetadata, DocumentPath, DocumentTitle,
};
use cabinet_domain::version::{CurrentDocumentSnapshot, VersionId};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_repository::{
    CurrentDocumentRecord, DocumentRepository, DocumentRepositoryError,
};
use cabinet_ports::version_store::{
    HistoryPage, HistoryPageRequest, VersionRecord, VersionSnapshot, VersionStore,
    VersionStoreError,
};
use cabinet_usecases::document::{
    CreateDocumentProductEvent, DocumentChangeEvent, DocumentChangeEventPublisher,
    DocumentProductLogger, UpdateDocumentError, UpdateDocumentInput, UpdateDocumentUsecase,
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

    fn current_body(&self, workspace_id: &str, document_id: &str) -> String {
        self.current
            .get(&(workspace_id.to_string(), document_id.to_string()))
            .expect("current document")
            .body()
            .as_str()
            .to_string()
    }

    fn current_title(&self, workspace_id: &str, document_id: &str) -> String {
        self.current
            .get(&(workspace_id.to_string(), document_id.to_string()))
            .expect("current document")
            .metadata()
            .title()
            .as_str()
            .to_string()
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
struct FakeVersionStore {
    appended: Vec<VersionRecord>,
    fail_conflict: bool,
}

impl VersionStore for FakeVersionStore {
    fn append_version(
        &mut self,
        _workspace_id: &WorkspaceId,
        record: VersionRecord,
    ) -> Result<(), VersionStoreError> {
        if self.fail_conflict {
            return Err(VersionStoreError::Conflict);
        }
        self.appended.push(record);
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
fn update_document_appends_version_then_updates_current_and_emits_events() {
    let mut documents = FakeDocumentRepository::default();
    documents.insert("workspace-1", current_record("doc-1", "old body"));
    let mut versions = FakeVersionStore::default();
    let mut publisher = FakeEventPublisher::default();
    let mut logger = FakeProductLogger::default();
    let usecase = UpdateDocumentUsecase::new(DocumentBodyPolicy::new(1024).expect("policy"));

    let output = usecase
        .execute(
            UpdateDocumentInput::new(
                "workspace-1",
                "doc-1",
                "# 새 문서 제목\r\nbody",
                "version-2",
                "snapshot-2",
                "alice",
                "Update body",
            ),
            &mut documents,
            &mut versions,
            &mut publisher,
            &mut logger,
        )
        .expect("update");

    assert_eq!(output.version_id().as_str(), "version-2");
    assert_eq!(
        documents.current_body("workspace-1", "doc-1"),
        "# 새 문서 제목\nbody"
    );
    assert_eq!(documents.current_title("workspace-1", "doc-1"), "새 문서 제목");
    assert_eq!(versions.appended.len(), 1);
    assert_eq!(
        versions.appended[0].snapshot().body().as_str(),
        "# 새 문서 제목\nbody"
    );
    assert_eq!(
        publisher.events,
        vec![DocumentChangeEvent::DocumentUpdated {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
            version_id: "version-2".to_string(),
        }]
    );
    assert_eq!(
        logger.events,
        vec![CreateDocumentProductEvent::DocumentUpdated {
            document_id: "doc-1".to_string(),
            version_id: "version-2".to_string(),
        }]
    );
}

#[test]
fn update_document_rejects_invalid_body_without_writes() {
    let mut documents = FakeDocumentRepository::default();
    documents.insert("workspace-1", current_record("doc-1", "old body"));
    let mut versions = FakeVersionStore::default();
    let mut publisher = FakeEventPublisher::default();
    let mut logger = FakeProductLogger::default();
    let usecase = UpdateDocumentUsecase::new(DocumentBodyPolicy::new(4).expect("policy"));

    let error = usecase
        .execute(
            UpdateDocumentInput::new(
                "workspace-1",
                "doc-1",
                "too long",
                "version-2",
                "snapshot-2",
                "alice",
                "Update body",
            ),
            &mut documents,
            &mut versions,
            &mut publisher,
            &mut logger,
        )
        .expect_err("invalid body must fail");

    assert_eq!(error, UpdateDocumentError::InvalidDocumentInput);
    assert_eq!(documents.current_body("workspace-1", "doc-1"), "old body");
    assert_eq!(documents.put_count, 0);
    assert!(versions.appended.is_empty());
    assert!(publisher.events.is_empty());
    assert_eq!(
        logger.events,
        vec![CreateDocumentProductEvent::UsecaseFailed {
            error_code: "document.invalid_input",
        }]
    );
}

#[test]
fn update_document_preserves_current_when_version_append_conflicts() {
    let mut documents = FakeDocumentRepository::default();
    documents.insert("workspace-1", current_record("doc-1", "old body"));
    let mut versions = FakeVersionStore {
        fail_conflict: true,
        ..FakeVersionStore::default()
    };
    let mut publisher = FakeEventPublisher::default();
    let mut logger = FakeProductLogger::default();
    let usecase = UpdateDocumentUsecase::new(DocumentBodyPolicy::new(1024).expect("policy"));

    let error = usecase
        .execute(
            UpdateDocumentInput::new(
                "workspace-1",
                "doc-1",
                "updated body",
                "version-2",
                "snapshot-2",
                "alice",
                "Update body",
            ),
            &mut documents,
            &mut versions,
            &mut publisher,
            &mut logger,
        )
        .expect_err("duplicate version must fail");

    assert_eq!(error, UpdateDocumentError::VersionAlreadyExists);
    assert_eq!(documents.current_body("workspace-1", "doc-1"), "old body");
    assert_eq!(documents.put_count, 0);
    assert!(versions.appended.is_empty());
    assert!(publisher.events.is_empty());
    assert_eq!(
        logger.events,
        vec![CreateDocumentProductEvent::UsecaseFailed {
            error_code: "document.version_already_exists",
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
