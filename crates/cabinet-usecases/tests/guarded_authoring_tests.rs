use std::collections::HashMap;

use cabinet_domain::document::{DocumentBodyPolicy, DocumentId, DocumentPath};
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_version::{
    CurrentDocumentVersionPointerError, CurrentDocumentVersionPointerPort,
};
use cabinet_ports::document_repository::{
    CurrentDocumentRecord, DocumentRepository, DocumentRepositoryError,
};
use cabinet_ports::version_store::{
    HistoryPage, HistoryPageRequest, VersionRecord, VersionSnapshot, VersionStore,
    VersionStoreError,
};
use cabinet_usecases::document::{
    CreateDocumentProductEvent, DocumentChangeEvent, DocumentChangeEventPublisher,
    DocumentProductLogger,
};
use cabinet_usecases::guarded_authoring::{
    GuardedAuthoringError, GuardedAuthoringUsecase, GuardedCreateDocumentInput,
    GuardedGetCurrentDocumentInput, GuardedUpdateDocumentInput,
};

#[test]
fn guarded_create_sets_initial_pointer_and_current_read_returns_version_without_history() {
    let mut fixture = Fixture::default();
    let usecase = guarded_usecase();

    let created = usecase
        .create(
            valid_create(),
            &mut fixture.documents,
            &mut fixture.versions,
            &mut fixture.pointer,
            &mut fixture.events,
            &mut fixture.logger,
        )
        .expect("create");
    let current = usecase
        .get_current(
            GuardedGetCurrentDocumentInput::new("workspace-1", "doc-1"),
            &fixture.documents,
            &fixture.pointer,
        )
        .expect("current");

    assert_eq!(created.document_id(), "doc-1");
    assert_eq!(created.current_version_id(), "v1");
    assert_eq!(current.current_version_id(), "v1");
    assert_eq!(current.record().body().as_str(), "body one");
    assert_eq!(fixture.versions.history_call_count, 0);
    assert_eq!(fixture.pointer.set_count, 1);
}

#[test]
fn guarded_update_compares_expected_before_writes_and_advances_pointer() {
    let mut fixture = Fixture::default();
    let usecase = guarded_usecase();
    usecase
        .create(
            valid_create(),
            &mut fixture.documents,
            &mut fixture.versions,
            &mut fixture.pointer,
            &mut fixture.events,
            &mut fixture.logger,
        )
        .expect("create");

    let updated = usecase
        .update(
            GuardedUpdateDocumentInput::new(
                "workspace-1",
                "doc-1",
                "body two",
                "v1",
                "v2",
                "snapshot-v2",
                "local-user",
                "Updated",
            ),
            &mut fixture.documents,
            &mut fixture.versions,
            &mut fixture.pointer,
            &mut fixture.events,
            &mut fixture.logger,
        )
        .expect("update");

    assert_eq!(updated.current_version_id(), "v2");
    assert_eq!(fixture.versions.records.len(), 2);
    assert_eq!(
        fixture.documents.body("workspace-1", "doc-1"),
        Some("body two")
    );
    assert_eq!(fixture.pointer.current("workspace-1", "doc-1"), Some("v2"));
}

#[test]
fn guarded_update_rejects_stale_expected_version_before_primary_writes() {
    let mut fixture = Fixture::default();
    let usecase = guarded_usecase();
    usecase
        .create(
            valid_create(),
            &mut fixture.documents,
            &mut fixture.versions,
            &mut fixture.pointer,
            &mut fixture.events,
            &mut fixture.logger,
        )
        .expect("create");
    let write_count = fixture.documents.put_count;
    let version_count = fixture.versions.records.len();

    let error = usecase
        .update(
            GuardedUpdateDocumentInput::new(
                "workspace-1",
                "doc-1",
                "must not persist",
                "stale",
                "v2",
                "snapshot-v2",
                "local-user",
                "Updated",
            ),
            &mut fixture.documents,
            &mut fixture.versions,
            &mut fixture.pointer,
            &mut fixture.events,
            &mut fixture.logger,
        )
        .expect_err("stale update");

    assert_eq!(error, GuardedAuthoringError::VersionConflict);
    assert_eq!(fixture.documents.put_count, write_count);
    assert_eq!(fixture.versions.records.len(), version_count);
    assert_eq!(
        fixture.documents.body("workspace-1", "doc-1"),
        Some("body one")
    );
}

#[test]
fn guarded_authoring_reports_pointer_update_failure_as_repair_required_without_raw_content() {
    let mut fixture = Fixture::default();
    fixture.pointer.fail_set = true;

    let error = guarded_usecase()
        .create(
            valid_create(),
            &mut fixture.documents,
            &mut fixture.versions,
            &mut fixture.pointer,
            &mut fixture.events,
            &mut fixture.logger,
        )
        .expect_err("pointer failure");

    assert_eq!(error, GuardedAuthoringError::PointerUpdateFailed);
    assert_eq!(error.code(), "guarded_authoring.pointer_update_failed");
    assert_eq!(
        fixture.documents.body("workspace-1", "doc-1"),
        Some("body one")
    );
    assert_eq!(fixture.versions.records.len(), 1);
    assert!(!format!("{error:?}").contains("body one"));
    assert!(!format!("{error:?}").contains("/Users/"));
}

fn guarded_usecase() -> GuardedAuthoringUsecase {
    GuardedAuthoringUsecase::new(DocumentBodyPolicy::new(1024).expect("policy"))
}

fn valid_create() -> GuardedCreateDocumentInput {
    GuardedCreateDocumentInput::new(
        "workspace-1",
        "doc-1",
        "notes/source.md",
        "body one",
        "v1",
        "snapshot-v1",
        "local-user",
        "Created",
    )
}

#[derive(Default)]
struct Fixture {
    documents: FakeDocuments,
    versions: FakeVersions,
    pointer: FakePointer,
    events: NoopEvents,
    logger: NoopLogger,
}

#[derive(Default)]
struct FakeDocuments {
    records: HashMap<(String, String), CurrentDocumentRecord>,
    put_count: usize,
}

impl FakeDocuments {
    fn body(&self, workspace: &str, document: &str) -> Option<&str> {
        self.records
            .get(&(workspace.to_string(), document.to_string()))
            .map(|record| record.body().as_str())
    }
}

impl DocumentRepository for FakeDocuments {
    fn put_current(
        &mut self,
        workspace_id: &WorkspaceId,
        record: CurrentDocumentRecord,
    ) -> Result<(), DocumentRepositoryError> {
        self.put_count += 1;
        self.records.insert(
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
            .records
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
        Ok(self
            .records
            .values()
            .find(|record| {
                record.path() == path
                    && self.records.contains_key(&(
                        workspace_id.as_str().to_string(),
                        record.document_id().as_str().to_string(),
                    ))
            })
            .cloned())
    }
    fn delete_current(
        &mut self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<(), DocumentRepositoryError> {
        self.records.remove(&(
            workspace_id.as_str().to_string(),
            document_id.as_str().to_string(),
        ));
        Ok(())
    }
}

#[derive(Default)]
struct FakeVersions {
    records: Vec<VersionRecord>,
    history_call_count: usize,
}

impl VersionStore for FakeVersions {
    fn append_version(
        &mut self,
        _workspace_id: &WorkspaceId,
        record: VersionRecord,
    ) -> Result<(), VersionStoreError> {
        if self
            .records
            .iter()
            .any(|existing| existing.version_id() == record.version_id())
        {
            return Err(VersionStoreError::Conflict);
        }
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
struct FakePointer {
    values: HashMap<(String, String), VersionId>,
    fail_set: bool,
    set_count: usize,
}

impl FakePointer {
    fn current(&self, workspace: &str, document: &str) -> Option<&str> {
        self.values
            .get(&(workspace.to_string(), document.to_string()))
            .map(VersionId::as_str)
    }
}

impl CurrentDocumentVersionPointerPort for FakePointer {
    fn load_current_version(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<VersionId>, CurrentDocumentVersionPointerError> {
        Ok(self
            .values
            .get(&(
                workspace_id.as_str().to_string(),
                document_id.as_str().to_string(),
            ))
            .cloned())
    }
    fn compare_and_set_current_version(
        &mut self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        expected: Option<&VersionId>,
        next: VersionId,
    ) -> Result<(), CurrentDocumentVersionPointerError> {
        if self.fail_set {
            return Err(CurrentDocumentVersionPointerError::StorageUnavailable);
        }
        let key = (
            workspace_id.as_str().to_string(),
            document_id.as_str().to_string(),
        );
        if self.values.get(&key) != expected {
            return Err(CurrentDocumentVersionPointerError::Conflict);
        }
        self.values.insert(key, next);
        self.set_count += 1;
        Ok(())
    }
}

#[derive(Default)]
struct NoopEvents;
impl DocumentChangeEventPublisher for NoopEvents {
    fn publish(&mut self, _event: DocumentChangeEvent) {}
}

#[derive(Default)]
struct NoopLogger;
impl DocumentProductLogger for NoopLogger {
    fn write_product(&mut self, _event: CreateDocumentProductEvent) {}
}
