use std::cell::Cell;
use std::collections::HashMap;

use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentMetadata, DocumentPath, DocumentTitle,
};
use cabinet_domain::version::CurrentDocumentSnapshot;
use cabinet_domain::version::{DocumentSnapshotRef, VersionId};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_repository::{
    CurrentDocumentRecord, DocumentRepository, DocumentRepositoryError,
};
use cabinet_ports::version_store::{
    HistoryPage, HistoryPageRequest, VersionRecord, VersionSnapshot, VersionStore,
    VersionStoreError,
};
use cabinet_usecases::document::{
    CompareDocumentVersionsError, CompareDocumentVersionsInput, CompareDocumentVersionsUsecase,
    LineDiffKind,
};

#[derive(Default)]
struct FakeDocumentRepository {
    current: HashMap<(String, String), CurrentDocumentRecord>,
    current_read_count: Cell<usize>,
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
}

impl DocumentRepository for FakeDocumentRepository {
    fn put_current(
        &mut self,
        workspace_id: &WorkspaceId,
        record: CurrentDocumentRecord,
    ) -> Result<(), DocumentRepositoryError> {
        self.insert(workspace_id.as_str(), record);
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
    snapshot_read_count: Cell<usize>,
    history_list_count: Cell<usize>,
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
        _workspace_id: &WorkspaceId,
        _record: VersionRecord,
    ) -> Result<(), VersionStoreError> {
        Ok(())
    }

    fn get_version_snapshot(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        version_id: &VersionId,
    ) -> Result<Option<VersionSnapshot>, VersionStoreError> {
        self.snapshot_read_count
            .set(self.snapshot_read_count.get() + 1);
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
        self.history_list_count
            .set(self.history_list_count.get() + 1);
        Ok(HistoryPage::new(Vec::new(), None))
    }
}

#[test]
fn compare_current_to_version_uses_current_and_specific_snapshot_without_history_list() {
    let mut documents = FakeDocumentRepository::default();
    documents.insert(
        "workspace-1",
        current_record("doc-1", "line 1\nline 2 changed\n"),
    );
    let mut versions = FakeVersionStore::default();
    versions.insert("workspace-1", "doc-1", "version-1", "line 1\nline 2\n");
    let usecase = CompareDocumentVersionsUsecase::new();

    let output = usecase
        .execute(
            CompareDocumentVersionsInput::current_to_version("workspace-1", "doc-1", "version-1"),
            &documents,
            &versions,
        )
        .expect("diff");

    assert!(
        output.lines().iter().any(|line| {
            line.kind() == LineDiffKind::Removed && line.text() == "line 2 changed"
        })
    );
    assert!(
        output
            .lines()
            .iter()
            .any(|line| { line.kind() == LineDiffKind::Added && line.text() == "line 2" })
    );
    assert_eq!(documents.current_read_count.get(), 1);
    assert_eq!(versions.snapshot_read_count.get(), 1);
    assert_eq!(versions.history_list_count.get(), 0);
}

#[test]
fn compare_two_versions_uses_specific_snapshots_without_current_or_history_list() {
    let documents = FakeDocumentRepository::default();
    let mut versions = FakeVersionStore::default();
    versions.insert("workspace-1", "doc-1", "version-1", "line 1\nline 2\n");
    versions.insert("workspace-1", "doc-1", "version-2", "line 1\nline 3\n");
    let usecase = CompareDocumentVersionsUsecase::new();

    let output = usecase
        .execute(
            CompareDocumentVersionsInput::versions(
                "workspace-1",
                "doc-1",
                "version-1",
                "version-2",
            ),
            &documents,
            &versions,
        )
        .expect("diff");

    assert!(
        output
            .lines()
            .iter()
            .any(|line| line.kind() == LineDiffKind::Removed && line.text() == "line 2")
    );
    assert!(
        output
            .lines()
            .iter()
            .any(|line| line.kind() == LineDiffKind::Added && line.text() == "line 3")
    );
    assert_eq!(documents.current_read_count.get(), 0);
    assert_eq!(versions.snapshot_read_count.get(), 2);
    assert_eq!(versions.history_list_count.get(), 0);
}

#[test]
fn compare_document_versions_reports_not_found_for_missing_target() {
    let documents = FakeDocumentRepository::default();
    let versions = FakeVersionStore::default();
    let usecase = CompareDocumentVersionsUsecase::new();

    let error = usecase
        .execute(
            CompareDocumentVersionsInput::current_to_version("workspace-1", "doc-1", "version-404"),
            &documents,
            &versions,
        )
        .expect_err("missing target must fail");

    assert_eq!(error, CompareDocumentVersionsError::NotFound);
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
