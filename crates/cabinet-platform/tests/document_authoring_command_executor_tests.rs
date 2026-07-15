use std::collections::HashMap;

use cabinet_domain::document::{DocumentBodyPolicy, DocumentId, DocumentPath};
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_platform::document_authoring_command::{
    DocumentAuthoringCommandExecutor, DocumentAuthoringCommandFailure,
    DocumentAuthoringCommandRequest, DocumentAuthoringCommandResult,
};
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

#[test]
fn command_executor_creates_reads_and_updates_with_owned_safe_results() {
    let mut fixture = Fixture::default();
    let executor = executor();

    let created = execute(&executor, create_request(), &mut fixture).expect("create");
    let current = execute(&executor, get_request(), &mut fixture).expect("get");
    let current_debug = format!("{current:?}");
    let updated = execute(&executor, update_request("v1", "v2"), &mut fixture).expect("update");

    assert_eq!(
        created,
        DocumentAuthoringCommandResult::Created {
            document_id: "doc-1".to_string(),
            current_version_id: "v1".to_string(),
        }
    );
    assert_eq!(
        current,
        DocumentAuthoringCommandResult::Current {
            document_id: "doc-1".to_string(),
            title: "Source".to_string(),
            path: "notes/source.md".to_string(),
            body: "# Source\nbody one".to_string(),
            current_version_id: "v1".to_string(),
        }
    );
    assert_eq!(
        updated,
        DocumentAuthoringCommandResult::Updated {
            document_id: "doc-1".to_string(),
            current_version_id: "v2".to_string(),
        }
    );
    assert_eq!(fixture.versions.history_call_count, 0);
    assert_eq!(
        fixture.documents.body("workspace-1", "doc-1"),
        Some("body two")
    );
    assert!(!current_debug.contains("body one"));
    assert!(!current_debug.contains("notes/source.md"));
}

#[test]
fn command_executor_maps_stale_version_before_primary_writes() {
    let mut fixture = Fixture::default();
    let executor = executor();
    execute(&executor, create_request(), &mut fixture).expect("create");
    let document_writes = fixture.documents.put_count;
    let version_writes = fixture.versions.records.len();

    let failure =
        execute(&executor, update_request("stale", "v2"), &mut fixture).expect_err("conflict");

    assert_eq!(
        failure,
        DocumentAuthoringCommandFailure {
            error_code: "DOCUMENT_AUTHORING_VERSION_CONFLICT",
            retryable: false,
            repair_required: false,
        }
    );
    assert_eq!(fixture.documents.put_count, document_writes);
    assert_eq!(fixture.versions.records.len(), version_writes);
}

#[test]
fn command_executor_maps_invalid_missing_and_body_limit_errors() {
    let mut fixture = Fixture::default();
    let executor = executor();

    let invalid = execute(
        &executor,
        DocumentAuthoringCommandRequest::GetCurrent {
            workspace_id: "".to_string(),
            document_id: "doc-1".to_string(),
        },
        &mut fixture,
    )
    .expect_err("invalid");
    let missing = execute(&executor, get_request(), &mut fixture).expect_err("missing");
    let too_large = execute(
        &DocumentAuthoringCommandExecutor::new(
            DocumentBodyPolicy::new(3).expect("small body policy"),
        ),
        create_request(),
        &mut fixture,
    )
    .expect_err("body limit");

    assert_eq!(invalid.error_code, "DOCUMENT_AUTHORING_INVALID_INPUT");
    assert_eq!(missing.error_code, "DOCUMENT_AUTHORING_NOT_FOUND");
    assert_eq!(too_large.error_code, "DOCUMENT_AUTHORING_INVALID_INPUT");
    assert!(!invalid.retryable);
    assert!(!missing.retryable);
}

#[test]
fn command_executor_maps_pointer_failure_to_sanitized_repair_required_failure() {
    let mut fixture = Fixture::default();
    fixture.pointer.fail_set = true;

    let failure = execute(&executor(), create_request(), &mut fixture).expect_err("pointer");
    let debug = format!("{failure:?}");

    assert_eq!(
        failure.error_code,
        "DOCUMENT_AUTHORING_POINTER_UPDATE_FAILED"
    );
    assert!(failure.repair_required);
    assert!(failure.retryable);
    assert!(!debug.contains("body one"));
    assert!(!debug.contains("notes/source.md"));
    assert!(!debug.contains("/Users/"));
}

fn executor() -> DocumentAuthoringCommandExecutor {
    DocumentAuthoringCommandExecutor::new(DocumentBodyPolicy::new(1024).expect("policy"))
}

fn execute(
    executor: &DocumentAuthoringCommandExecutor,
    request: DocumentAuthoringCommandRequest,
    fixture: &mut Fixture,
) -> Result<DocumentAuthoringCommandResult, DocumentAuthoringCommandFailure> {
    executor.execute(
        request,
        &mut fixture.documents,
        &mut fixture.versions,
        &mut fixture.pointer,
        &mut fixture.events,
        &mut fixture.logger,
    )
}

fn create_request() -> DocumentAuthoringCommandRequest {
    DocumentAuthoringCommandRequest::Create {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        path: "notes/source.md".to_string(),
        body: "# Source\nbody one".to_string(),
        version_id: "v1".to_string(),
        snapshot_ref: "snapshot-v1".to_string(),
        author: "local-user".to_string(),
        summary: "Created".to_string(),
    }
}

fn update_request(expected_version_id: &str, version_id: &str) -> DocumentAuthoringCommandRequest {
    DocumentAuthoringCommandRequest::Update {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        body: "body two".to_string(),
        expected_version_id: expected_version_id.to_string(),
        version_id: version_id.to_string(),
        snapshot_ref: format!("snapshot-{version_id}"),
        author: "local-user".to_string(),
        summary: "Updated".to_string(),
    }
}

fn get_request() -> DocumentAuthoringCommandRequest {
    DocumentAuthoringCommandRequest::GetCurrent {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
    }
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
            .iter()
            .find(|((workspace, _), record)| {
                workspace == workspace_id.as_str() && record.path() == path
            })
            .map(|(_, record)| record.clone()))
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
