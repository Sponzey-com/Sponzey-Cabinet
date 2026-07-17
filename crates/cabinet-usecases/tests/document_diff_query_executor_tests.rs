use std::cell::Cell;
use std::collections::HashMap;

use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentMetadata, DocumentPath, DocumentTitle,
};
use cabinet_domain::document_diff_query::DocumentDiffQueryTarget;
use cabinet_domain::version::{CurrentDocumentSnapshot, DocumentSnapshotRef, VersionId};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_repository::{
    CurrentDocumentRecord, DocumentRepository, DocumentRepositoryError,
};
use cabinet_ports::version_store::{
    HistoryPage, HistoryPageRequest, VersionRecord, VersionSnapshot, VersionStore,
    VersionStoreError,
};
use cabinet_usecases::document_diff::{DiffComputation, DiffPolicy, DocumentLineDiffService};
use cabinet_usecases::document_diff_query::{
    ExecuteDocumentDiffQueryError, ExecuteDocumentDiffQueryUsecase,
};

#[derive(Default)]
struct FakeDocumentRepository {
    current: Option<CurrentDocumentRecord>,
    reads: Cell<usize>,
    fail: bool,
}

impl DocumentRepository for FakeDocumentRepository {
    fn put_current(
        &mut self,
        _workspace_id: &WorkspaceId,
        _record: CurrentDocumentRecord,
    ) -> Result<(), DocumentRepositoryError> {
        panic!("write must not be called")
    }

    fn get_current_by_id(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
    ) -> Result<Option<CurrentDocumentRecord>, DocumentRepositoryError> {
        self.reads.set(self.reads.get() + 1);
        if self.fail {
            return Err(DocumentRepositoryError::StorageUnavailable);
        }
        Ok(self.current.clone())
    }

    fn get_current_by_path(
        &self,
        _workspace_id: &WorkspaceId,
        _path: &DocumentPath,
    ) -> Result<Option<CurrentDocumentRecord>, DocumentRepositoryError> {
        panic!("path query must not be called")
    }

    fn delete_current(
        &mut self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
    ) -> Result<(), DocumentRepositoryError> {
        panic!("delete must not be called")
    }
}

#[derive(Default)]
struct FakeVersionStore {
    snapshots: HashMap<String, VersionSnapshot>,
    reads: Cell<usize>,
    history_reads: Cell<usize>,
    fail: bool,
}

impl VersionStore for FakeVersionStore {
    fn append_version(
        &mut self,
        _workspace_id: &WorkspaceId,
        _record: VersionRecord,
    ) -> Result<(), VersionStoreError> {
        panic!("write must not be called")
    }

    fn get_version_snapshot(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        version_id: &VersionId,
    ) -> Result<Option<VersionSnapshot>, VersionStoreError> {
        self.reads.set(self.reads.get() + 1);
        if self.fail {
            return Err(VersionStoreError::StorageUnavailable);
        }
        Ok(self.snapshots.get(version_id.as_str()).cloned())
    }

    fn list_history(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        _request: HistoryPageRequest,
    ) -> Result<HistoryPage, VersionStoreError> {
        self.history_reads.set(self.history_reads.get() + 1);
        panic!("history must not be called")
    }
}

#[test]
fn current_to_version_reads_current_and_target_once_without_history() {
    let documents = FakeDocumentRepository {
        current: Some(current_record("doc-1", "new title\nbody\n")),
        ..Default::default()
    };
    let mut versions = FakeVersionStore::default();
    versions.snapshots.insert(
        "version-1".to_string(),
        version_snapshot("doc-1", "version-1", "old title\nbody\n"),
    );
    let target =
        DocumentDiffQueryTarget::current_to_version("workspace-1", "doc-1", "version-1").unwrap();

    let result = ExecuteDocumentDiffQueryUsecase::new()
        .execute(&target, &documents, &versions)
        .unwrap();

    assert!(matches!(result, DiffComputation::Complete(_)));
    assert_eq!(documents.reads.get(), 1);
    assert_eq!(versions.reads.get(), 1);
    assert_eq!(versions.history_reads.get(), 0);
}

#[test]
fn version_pair_reads_only_two_specific_snapshots() {
    let documents = FakeDocumentRepository::default();
    let mut versions = FakeVersionStore::default();
    versions.snapshots.insert(
        "version-1".to_string(),
        version_snapshot("doc-1", "version-1", "left\n"),
    );
    versions.snapshots.insert(
        "version-2".to_string(),
        version_snapshot("doc-1", "version-2", "right\n"),
    );
    let target =
        DocumentDiffQueryTarget::versions("workspace-1", "doc-1", "version-1", "version-2")
            .unwrap();

    let result = ExecuteDocumentDiffQueryUsecase::new()
        .execute(&target, &documents, &versions)
        .unwrap();

    assert!(matches!(result, DiffComputation::Complete(_)));
    assert_eq!(documents.reads.get(), 0);
    assert_eq!(versions.reads.get(), 2);
    assert_eq!(versions.history_reads.get(), 0);
}

#[test]
fn executor_preserves_too_large_from_injected_policy() {
    let documents = FakeDocumentRepository::default();
    let mut versions = FakeVersionStore::default();
    versions.snapshots.insert(
        "version-1".to_string(),
        version_snapshot("doc-1", "version-1", "left body\n"),
    );
    versions.snapshots.insert(
        "version-2".to_string(),
        version_snapshot("doc-1", "version-2", "right body\n"),
    );
    let target =
        DocumentDiffQueryTarget::versions("workspace-1", "doc-1", "version-1", "version-2")
            .unwrap();
    let service = DocumentLineDiffService::with_policy(DiffPolicy::new(0, 4, 10, 10).unwrap());

    let result = ExecuteDocumentDiffQueryUsecase::with_diff_service(service)
        .execute(&target, &documents, &versions)
        .unwrap();

    assert!(matches!(result, DiffComputation::TooLarge(_)));
}

#[test]
fn missing_and_storage_failures_are_typed() {
    let target =
        DocumentDiffQueryTarget::current_to_version("workspace-1", "doc-1", "version-1").unwrap();
    let documents = FakeDocumentRepository::default();
    let versions = FakeVersionStore::default();

    assert_eq!(
        ExecuteDocumentDiffQueryUsecase::new()
            .execute(&target, &documents, &versions)
            .unwrap_err(),
        ExecuteDocumentDiffQueryError::NotFound
    );

    let documents = FakeDocumentRepository {
        fail: true,
        ..Default::default()
    };
    let error = ExecuteDocumentDiffQueryUsecase::new()
        .execute(&target, &documents, &versions)
        .unwrap_err();
    assert_eq!(error, ExecuteDocumentDiffQueryError::StorageUnavailable);
    assert_eq!(error.code(), "document.storage_unavailable");
}

fn current_record(document_id: &str, body: &str) -> CurrentDocumentRecord {
    let metadata = DocumentMetadata::new(
        DocumentId::new(document_id).unwrap(),
        DocumentTitle::new("Title").unwrap(),
        DocumentPath::new("docs/title.md").unwrap(),
    )
    .unwrap();
    let snapshot = CurrentDocumentSnapshot::new(
        DocumentId::new(document_id).unwrap(),
        DocumentBody::new(body, DocumentBodyPolicy::new(1024).unwrap()).unwrap(),
    );
    CurrentDocumentRecord::new(metadata, snapshot).unwrap()
}

fn version_snapshot(document_id: &str, version_id: &str, body: &str) -> VersionSnapshot {
    VersionSnapshot::new(
        DocumentId::new(document_id).unwrap(),
        DocumentSnapshotRef::new(&format!("snapshot-{version_id}")).unwrap(),
        DocumentBody::new(body, DocumentBodyPolicy::new(1024).unwrap()).unwrap(),
    )
}
