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
    LineDiffKind, PreviewDocumentRestoreError, PreviewDocumentRestoreInput,
    PreviewDocumentRestoreUsecase,
};
use cabinet_usecases::document_diff::{DiffPolicy, DocumentLineDiffService, DocumentTitleDelta};

#[derive(Default)]
struct FakeDocumentRepository {
    current: HashMap<(String, String), CurrentDocumentRecord>,
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
}

impl DocumentRepository for FakeDocumentRepository {
    fn put_current(
        &mut self,
        _workspace_id: &WorkspaceId,
        _record: CurrentDocumentRecord,
    ) -> Result<(), DocumentRepositoryError> {
        self.put_count.set(self.put_count.get() + 1);
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
    append_count: Cell<usize>,
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
        self.append_count.set(self.append_count.get() + 1);
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

#[test]
fn preview_document_restore_returns_diff_without_writes() {
    let mut documents = FakeDocumentRepository::default();
    documents.insert(
        "workspace-1",
        current_record("doc-1", "current title\nline current\n"),
    );
    let mut versions = FakeVersionStore::default();
    versions.insert(
        "workspace-1",
        "doc-1",
        "version-1",
        "target title\nline restored\n",
    );
    let usecase = PreviewDocumentRestoreUsecase::new();

    let output = usecase
        .execute(
            PreviewDocumentRestoreInput::new("workspace-1", "doc-1", "version-1"),
            &documents,
            &versions,
        )
        .expect("preview");

    assert!(output.can_restore());
    assert_eq!(output.target_version_id().as_str(), "version-1");
    assert!(
        output
            .lines()
            .iter()
            .any(|line| line.kind() == LineDiffKind::Removed && line.text() == "line current")
    );
    assert!(
        output
            .lines()
            .iter()
            .any(|line| line.kind() == LineDiffKind::Added && line.text() == "line restored")
    );
    assert_eq!(documents.put_count.get(), 0);
    assert_eq!(versions.append_count.get(), 0);
    let expected = DocumentLineDiffService::default().compare(
        "current title\nline current\n",
        "target title\nline restored\n",
    );
    assert_eq!(output.diff(), expected.complete().expect("complete diff"));
    assert_eq!(
        output.title_delta(),
        &DocumentTitleDelta::Changed {
            before: "current title".to_string(),
            after: "target title".to_string(),
        }
    );
}

#[test]
fn preview_document_restore_reports_not_found_for_missing_target() {
    let mut documents = FakeDocumentRepository::default();
    documents.insert("workspace-1", current_record("doc-1", "line 1\n"));
    let versions = FakeVersionStore::default();
    let usecase = PreviewDocumentRestoreUsecase::new();

    let error = usecase
        .execute(
            PreviewDocumentRestoreInput::new("workspace-1", "doc-1", "version-404"),
            &documents,
            &versions,
        )
        .expect_err("missing target must fail");

    assert_eq!(error, PreviewDocumentRestoreError::NotFound);
}

#[test]
fn preview_document_restore_maps_sync_limit_to_the_same_stable_error() {
    let mut documents = FakeDocumentRepository::default();
    documents.insert("workspace-1", current_record("doc-1", "current body\n"));
    let mut versions = FakeVersionStore::default();
    versions.insert("workspace-1", "doc-1", "version-1", "target body\n");
    let service =
        DocumentLineDiffService::with_policy(DiffPolicy::new(0, 4, 10, 10).expect("small policy"));

    let error = PreviewDocumentRestoreUsecase::with_diff_service(service)
        .execute(
            PreviewDocumentRestoreInput::new("workspace-1", "doc-1", "version-1"),
            &documents,
            &versions,
        )
        .expect_err("oversized preview");

    assert_eq!(error, PreviewDocumentRestoreError::TooLarge);
    assert_eq!(error.code(), "document.diff_too_large");
    assert!(error.retryable());
    assert_eq!(documents.put_count.get(), 0);
    assert_eq!(versions.append_count.get(), 0);
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
