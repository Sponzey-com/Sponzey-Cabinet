use std::cell::Cell;
use std::collections::HashMap;

use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentMetadata, DocumentPath, DocumentTitle,
};
use cabinet_domain::version::{CurrentDocumentSnapshot, DocumentSnapshotRef, VersionId};
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
    DocumentProductLogger, RestoreDocumentVersionError, RestoreDocumentVersionInput,
    RestoreDocumentVersionState, RestoreDocumentVersionUsecase,
};

#[derive(Default)]
struct FakeDocumentRepository {
    current: HashMap<(String, String), CurrentDocumentRecord>,
    fail_put: Cell<bool>,
    put_count: Cell<usize>,
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
        self.put_count.set(self.put_count.get() + 1);
        if self.fail_put.get() {
            return Err(DocumentRepositoryError::StorageUnavailable);
        }
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
    snapshots: HashMap<(String, String, String), VersionSnapshot>,
    appended: Vec<VersionRecord>,
}

impl FakeVersionStore {
    fn insert(&mut self, workspace_id: &str, document_id: &str, version_id: &str, body: &str) {
        self.snapshots.insert(
            (
                workspace_id.to_string(),
                document_id.to_string(),
                version_id.to_string(),
            ),
            version_snapshot(document_id, version_id, body),
        );
    }
}

impl VersionStore for FakeVersionStore {
    fn append_version(
        &mut self,
        workspace_id: &WorkspaceId,
        record: VersionRecord,
    ) -> Result<(), VersionStoreError> {
        self.snapshots.insert(
            (
                workspace_id.as_str().to_string(),
                record.document_id().as_str().to_string(),
                record.version_id().as_str().to_string(),
            ),
            record.snapshot().clone(),
        );
        self.appended.push(record);
        Ok(())
    }

    fn get_version_snapshot(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        version_id: &VersionId,
    ) -> Result<Option<VersionSnapshot>, VersionStoreError> {
        Ok(self
            .snapshots
            .get(&(
                workspace_id.as_str().to_string(),
                document_id.as_str().to_string(),
                version_id.as_str().to_string(),
            ))
            .cloned())
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
fn restore_document_version_appends_restore_version_updates_current_and_emits_events() {
    let mut documents = FakeDocumentRepository::default();
    documents.insert("workspace-1", current_record("doc-1", "current body"));
    let mut versions = FakeVersionStore::default();
    versions.insert(
        "workspace-1",
        "doc-1",
        "version-1",
        "# Restored title\nrestored body",
    );
    let mut publisher = FakeEventPublisher::default();
    let mut logger = FakeProductLogger::default();
    let usecase = RestoreDocumentVersionUsecase::new();

    let output = usecase
        .execute(
            RestoreDocumentVersionInput::new(
                "workspace-1",
                "doc-1",
                "version-1",
                "version-restore-1",
                "snapshot-restore-1",
                "alice",
                "Restore version-1",
            ),
            &mut documents,
            &mut versions,
            &mut publisher,
            &mut logger,
        )
        .expect("restore");

    assert_eq!(output.final_state(), RestoreDocumentVersionState::Completed);
    assert_eq!(output.restored_version_id().as_str(), "version-restore-1");
    assert_eq!(
        documents.current_body("workspace-1", "doc-1"),
        "# Restored title\nrestored body"
    );
    assert_eq!(
        documents.current_title("workspace-1", "doc-1"),
        "Restored title"
    );
    assert_eq!(versions.appended.len(), 1);
    assert_eq!(
        versions.appended[0].entry().version_id().as_str(),
        "version-restore-1"
    );
    assert_eq!(
        versions.appended[0].snapshot().body().as_str(),
        "# Restored title\nrestored body"
    );
    assert_eq!(
        publisher.events,
        vec![DocumentChangeEvent::DocumentRestored {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
            target_version_id: "version-1".to_string(),
            restored_version_id: "version-restore-1".to_string(),
        }]
    );
    assert_eq!(
        logger.events,
        vec![CreateDocumentProductEvent::DocumentRestored {
            document_id: "doc-1".to_string(),
            restored_version_id: "version-restore-1".to_string(),
        }]
    );
}

#[test]
fn restore_document_version_reports_not_found_for_missing_target_without_writes() {
    let mut documents = FakeDocumentRepository::default();
    documents.insert("workspace-1", current_record("doc-1", "current body"));
    let mut versions = FakeVersionStore::default();
    let mut publisher = FakeEventPublisher::default();
    let mut logger = FakeProductLogger::default();
    let usecase = RestoreDocumentVersionUsecase::new();

    let error = usecase
        .execute(
            RestoreDocumentVersionInput::new(
                "workspace-1",
                "doc-1",
                "version-404",
                "version-restore-1",
                "snapshot-restore-1",
                "alice",
                "Restore missing version",
            ),
            &mut documents,
            &mut versions,
            &mut publisher,
            &mut logger,
        )
        .expect_err("missing target must fail");

    assert_eq!(
        error,
        RestoreDocumentVersionError::NotFound {
            final_state: RestoreDocumentVersionState::Failed
        }
    );
    assert_eq!(
        documents.current_body("workspace-1", "doc-1"),
        "current body"
    );
    assert_eq!(documents.put_count.get(), 0);
    assert!(versions.appended.is_empty());
    assert!(publisher.events.is_empty());
    assert_eq!(
        logger.events,
        vec![CreateDocumentProductEvent::UsecaseFailed {
            error_code: "document.restore_target_not_found"
        }]
    );
}

#[test]
fn restore_document_version_preserves_current_when_current_update_fails() {
    let mut documents = FakeDocumentRepository::default();
    documents.insert("workspace-1", current_record("doc-1", "current body"));
    documents.fail_put.set(true);
    let mut versions = FakeVersionStore::default();
    versions.insert("workspace-1", "doc-1", "version-1", "restored body");
    let mut publisher = FakeEventPublisher::default();
    let mut logger = FakeProductLogger::default();
    let usecase = RestoreDocumentVersionUsecase::new();

    let error = usecase
        .execute(
            RestoreDocumentVersionInput::new(
                "workspace-1",
                "doc-1",
                "version-1",
                "version-restore-1",
                "snapshot-restore-1",
                "alice",
                "Restore version-1",
            ),
            &mut documents,
            &mut versions,
            &mut publisher,
            &mut logger,
        )
        .expect_err("current update must fail");

    assert_eq!(
        error,
        RestoreDocumentVersionError::StorageUnavailable {
            final_state: RestoreDocumentVersionState::Failed
        }
    );
    assert_eq!(
        documents.current_body("workspace-1", "doc-1"),
        "current body"
    );
    assert_eq!(documents.put_count.get(), 1);
    assert_eq!(versions.appended.len(), 1);
    assert!(publisher.events.is_empty());
    assert_eq!(
        logger.events,
        vec![CreateDocumentProductEvent::UsecaseFailed {
            error_code: "document.storage_unavailable"
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

fn version_snapshot(document_id: &str, version_id: &str, body: &str) -> VersionSnapshot {
    VersionSnapshot::new(
        DocumentId::new(document_id).expect("document id"),
        DocumentSnapshotRef::new(&format!("snapshot-{version_id}")).expect("snapshot ref"),
        DocumentBody::new(body, DocumentBodyPolicy::new(1024).expect("policy")).expect("body"),
    )
}
