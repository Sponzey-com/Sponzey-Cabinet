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
use cabinet_usecases::document_diff::{DiffPolicy, DocumentLineDiffService, DocumentTitleDelta};

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
fn compare_rejects_invalid_target_before_repository_access() {
    let documents = FakeDocumentRepository::default();
    let versions = FakeVersionStore::default();

    let error = CompareDocumentVersionsUsecase::new()
        .execute(
            CompareDocumentVersionsInput::current_to_version("", "doc-1", "version-1"),
            &documents,
            &versions,
        )
        .unwrap_err();

    assert_eq!(error, CompareDocumentVersionsError::InvalidInput);
    assert_eq!(error.code(), "document.invalid_input");
    assert_eq!(documents.current_read_count.get(), 0);
    assert_eq!(versions.snapshot_read_count.get(), 0);
    assert_eq!(versions.history_list_count.get(), 0);
}

#[test]
fn compare_middle_insertion_keeps_following_lines_equal() {
    let documents = FakeDocumentRepository::default();
    let mut versions = FakeVersionStore::default();
    versions.insert(
        "workspace-1",
        "doc-1",
        "version-1",
        "첫 줄\n둘째 줄\n셋째 줄\n",
    );
    versions.insert(
        "workspace-1",
        "doc-1",
        "version-2",
        "첫 줄\n새 줄\n둘째 줄\n셋째 줄\n",
    );

    let output = CompareDocumentVersionsUsecase::new()
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

    assert_eq!(
        output
            .lines()
            .iter()
            .map(|line| (line.kind(), line.text()))
            .collect::<Vec<_>>(),
        vec![
            (LineDiffKind::Equal, "첫 줄"),
            (LineDiffKind::Added, "새 줄"),
            (LineDiffKind::Equal, "둘째 줄"),
            (LineDiffKind::Equal, "셋째 줄"),
        ]
    );
}

#[test]
fn current_to_version_and_version_to_version_share_the_same_diff_contract() {
    let mut documents = FakeDocumentRepository::default();
    documents.insert(
        "workspace-1",
        current_record("doc-1", "첫 줄\n현재 줄\n마지막 줄\n"),
    );
    let mut versions = FakeVersionStore::default();
    versions.insert(
        "workspace-1",
        "doc-1",
        "version-current",
        "첫 줄\n현재 줄\n마지막 줄\n",
    );
    versions.insert(
        "workspace-1",
        "doc-1",
        "version-target",
        "새 제목\n추가 줄\n현재 줄\n마지막 줄\n",
    );
    let usecase = CompareDocumentVersionsUsecase::new();

    let current_to_version = usecase
        .execute(
            CompareDocumentVersionsInput::current_to_version(
                "workspace-1",
                "doc-1",
                "version-target",
            ),
            &documents,
            &versions,
        )
        .expect("current to version");
    let version_to_version = usecase
        .execute(
            CompareDocumentVersionsInput::versions(
                "workspace-1",
                "doc-1",
                "version-current",
                "version-target",
            ),
            &documents,
            &versions,
        )
        .expect("version to version");

    assert_eq!(current_to_version.diff(), version_to_version.diff());
    assert_eq!(
        current_to_version.title_delta(),
        &DocumentTitleDelta::Changed {
            before: "첫 줄".to_string(),
            after: "새 제목".to_string(),
        }
    );
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

#[test]
fn compare_document_versions_maps_sync_limit_to_stable_retryable_error() {
    let documents = FakeDocumentRepository::default();
    let mut versions = FakeVersionStore::default();
    versions.insert("workspace-1", "doc-1", "version-1", "left body\n");
    versions.insert("workspace-1", "doc-1", "version-2", "right body\n");
    let service =
        DocumentLineDiffService::with_policy(DiffPolicy::new(0, 4, 10, 10).expect("small policy"));

    let error = CompareDocumentVersionsUsecase::with_diff_service(service)
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
        .expect_err("oversized diff");

    assert_eq!(error, CompareDocumentVersionsError::TooLarge);
    assert_eq!(error.code(), "document.diff_too_large");
    assert!(error.retryable());
    assert_eq!(versions.history_list_count.get(), 0);
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
