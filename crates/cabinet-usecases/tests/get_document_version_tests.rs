use std::cell::Cell;
use std::collections::HashMap;

use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::version::{
    DocumentSnapshotRef, VersionAuthor, VersionEntry, VersionId, VersionSummary,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::version_store::{
    HistoryPage, HistoryPageRequest, VersionRecord, VersionSnapshot, VersionStore,
    VersionStoreError,
};
use cabinet_usecases::document::{
    GetDocumentVersionError, GetDocumentVersionInput, GetDocumentVersionUsecase,
};

#[derive(Default)]
struct FakeVersionStore {
    records: HashMap<(String, String, String), VersionSnapshot>,
    snapshot_read_count: Cell<usize>,
    history_list_count: Cell<usize>,
    current_repository_read_count: Cell<usize>,
}

impl FakeVersionStore {
    fn insert(&mut self, workspace_id: &str, document_id: &str, version_id: &str, body: &str) {
        self.records.insert(
            (
                workspace_id.to_string(),
                document_id.to_string(),
                version_id.to_string(),
            ),
            VersionSnapshot::new(
                DocumentId::new(document_id).expect("document id"),
                DocumentSnapshotRef::new("snapshot-1").expect("snapshot ref"),
                DocumentBody::new(body, DocumentBodyPolicy::new(1024).expect("policy"))
                    .expect("body"),
            ),
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
            .records
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
fn get_document_version_reads_specific_snapshot_without_current_repository_or_history_list() {
    let mut store = FakeVersionStore::default();
    store.insert("workspace-1", "doc-1", "version-1", "Version body");
    let usecase = GetDocumentVersionUsecase::new();

    let output = usecase
        .execute(
            GetDocumentVersionInput::new("workspace-1", "doc-1", "version-1"),
            &store,
        )
        .expect("version snapshot");

    assert_eq!(output.snapshot().body().as_str(), "Version body");
    assert_eq!(store.snapshot_read_count.get(), 1);
    assert_eq!(store.history_list_count.get(), 0);
    assert_eq!(store.current_repository_read_count.get(), 0);
}

#[test]
fn get_document_version_reports_not_found_for_missing_snapshot() {
    let store = FakeVersionStore::default();
    let usecase = GetDocumentVersionUsecase::new();

    let error = usecase
        .execute(
            GetDocumentVersionInput::new("workspace-1", "doc-1", "version-404"),
            &store,
        )
        .expect_err("missing version must fail");

    assert_eq!(error, GetDocumentVersionError::NotFound);
    assert_eq!(store.snapshot_read_count.get(), 1);
    assert_eq!(store.history_list_count.get(), 0);
}

#[test]
fn get_document_version_rejects_invalid_input_before_store_read() {
    let store = FakeVersionStore::default();
    let usecase = GetDocumentVersionUsecase::new();

    let error = usecase
        .execute(
            GetDocumentVersionInput::new("workspace-1", " ", "version-1"),
            &store,
        )
        .expect_err("invalid input must fail");

    assert_eq!(error, GetDocumentVersionError::InvalidInput);
    assert_eq!(store.snapshot_read_count.get(), 0);
    assert_eq!(store.history_list_count.get(), 0);
}

#[allow(dead_code)]
fn version_record(document_id: &str, version_id: &str, body: &str) -> VersionRecord {
    VersionRecord::new(
        VersionEntry::new(
            VersionId::new(version_id).expect("version id"),
            DocumentId::new(document_id).expect("document id"),
            DocumentSnapshotRef::new("snapshot-1").expect("snapshot ref"),
            VersionAuthor::new("writer").expect("author"),
            VersionSummary::new("Saved").expect("summary"),
        )
        .expect("entry"),
        VersionSnapshot::new(
            DocumentId::new(document_id).expect("document id"),
            DocumentSnapshotRef::new("snapshot-1").expect("snapshot ref"),
            DocumentBody::new(body, DocumentBodyPolicy::new(1024).expect("policy")).expect("body"),
        ),
    )
    .expect("record")
}
