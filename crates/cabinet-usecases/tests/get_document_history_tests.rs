use std::cell::Cell;
use std::collections::HashMap;

use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::version::{
    DocumentSnapshotRef, VersionAuthor, VersionEntry, VersionId, VersionSummary,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::version_store::{
    HistoryCursor, HistoryPage, HistoryPageRequest, VersionRecord, VersionSnapshot, VersionStore,
    VersionStoreError,
};
use cabinet_usecases::document::{
    GetDocumentHistoryError, GetDocumentHistoryInput, GetDocumentHistoryUsecase,
};

#[derive(Default)]
struct FakeVersionStore {
    history: HashMap<(String, String), Vec<VersionEntry>>,
    list_history_count: Cell<usize>,
    snapshot_read_count: Cell<usize>,
    current_repository_read_count: Cell<usize>,
}

impl FakeVersionStore {
    fn insert_history(
        &mut self,
        workspace_id: &str,
        document_id: &str,
        entries: Vec<VersionEntry>,
    ) {
        self.history
            .insert((workspace_id.to_string(), document_id.to_string()), entries);
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
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        _version_id: &VersionId,
    ) -> Result<Option<VersionSnapshot>, VersionStoreError> {
        self.snapshot_read_count
            .set(self.snapshot_read_count.get() + 1);
        Ok(None)
    }

    fn list_history(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        request: HistoryPageRequest,
    ) -> Result<HistoryPage, VersionStoreError> {
        self.list_history_count
            .set(self.list_history_count.get() + 1);
        let entries = self
            .history
            .get(&(
                workspace_id.as_str().to_string(),
                document_id.as_str().to_string(),
            ))
            .cloned()
            .unwrap_or_default();
        let start = request
            .cursor()
            .map(|cursor| cursor.as_str().parse::<usize>())
            .transpose()
            .map_err(|_| VersionStoreError::CorruptedHistory)?
            .unwrap_or(0);
        let end = usize::min(start + request.limit(), entries.len());
        let next_cursor = if end < entries.len() {
            Some(HistoryCursor::new(&end.to_string()).expect("cursor"))
        } else {
            None
        };
        Ok(HistoryPage::new(entries[start..end].to_vec(), next_cursor))
    }
}

#[test]
fn get_document_history_uses_paginated_history_without_snapshot_or_current_reads() {
    let mut store = FakeVersionStore::default();
    store.insert_history(
        "workspace-1",
        "doc-1",
        vec![
            version_entry("doc-1", "version-1"),
            version_entry("doc-1", "version-2"),
            version_entry("doc-1", "version-3"),
        ],
    );
    let usecase = GetDocumentHistoryUsecase::new();

    let output = usecase
        .execute(
            GetDocumentHistoryInput::new("workspace-1", "doc-1", None, 2),
            &store,
        )
        .expect("history");

    assert_eq!(output.page().entries().len(), 2);
    assert_eq!(output.page().next_cursor().expect("next").as_str(), "2");
    assert_eq!(store.list_history_count.get(), 1);
    assert_eq!(store.snapshot_read_count.get(), 0);
    assert_eq!(store.current_repository_read_count.get(), 0);
}

#[test]
fn get_document_history_rejects_invalid_limit_before_store_call() {
    let store = FakeVersionStore::default();
    let usecase = GetDocumentHistoryUsecase::new();

    let error = usecase
        .execute(
            GetDocumentHistoryInput::new("workspace-1", "doc-1", None, 0),
            &store,
        )
        .expect_err("invalid limit must fail");

    assert_eq!(error, GetDocumentHistoryError::InvalidInput);
    assert_eq!(store.list_history_count.get(), 0);
}

#[test]
fn get_document_history_rejects_invalid_cursor_before_store_call() {
    let store = FakeVersionStore::default();
    let usecase = GetDocumentHistoryUsecase::new();

    let error = usecase
        .execute(
            GetDocumentHistoryInput::new("workspace-1", "doc-1", Some(" "), 2),
            &store,
        )
        .expect_err("invalid cursor must fail");

    assert_eq!(error, GetDocumentHistoryError::InvalidInput);
    assert_eq!(store.list_history_count.get(), 0);
}

fn version_entry(document_id: &str, version_id: &str) -> VersionEntry {
    VersionEntry::new(
        VersionId::new(version_id).expect("version id"),
        DocumentId::new(document_id).expect("document id"),
        DocumentSnapshotRef::new(&format!("snapshot-{version_id}")).expect("snapshot ref"),
        VersionAuthor::new("writer").expect("author"),
        VersionSummary::new("Saved").expect("summary"),
    )
    .expect("entry")
}

#[allow(dead_code)]
fn version_record(document_id: &str, version_id: &str, body: &str) -> VersionRecord {
    VersionRecord::new(
        version_entry(document_id, version_id),
        VersionSnapshot::new(
            DocumentId::new(document_id).expect("document id"),
            DocumentSnapshotRef::new(&format!("snapshot-{version_id}")).expect("snapshot ref"),
            DocumentBody::new(body, DocumentBodyPolicy::new(1024).expect("policy")).expect("body"),
        ),
    )
    .expect("record")
}
