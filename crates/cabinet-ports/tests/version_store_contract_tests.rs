use std::cell::Cell;
use std::collections::HashMap;

use cabinet_domain::asset::{AssetId, AssetReference};
use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::version::{
    AttachmentSnapshotState, DocumentRevisionNumber, DocumentSnapshotRef, VersionAuthor,
    VersionEntry, VersionId, VersionSummary,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::version_store::{
    HistoryCursor, HistoryPage, HistoryPageRequest, MAX_HISTORY_PAGE_LIMIT, VersionRecord,
    VersionSnapshot, VersionStore, VersionStoreError,
};

#[derive(Default)]
struct FakeVersionStore {
    by_version: HashMap<(String, String, String), VersionRecord>,
    history: HashMap<(String, String), Vec<VersionEntry>>,
    current_repository_reads: Cell<usize>,
    history_full_scan_count: Cell<usize>,
}

impl FakeVersionStore {
    fn current_repository_reads(&self) -> usize {
        self.current_repository_reads.get()
    }

    fn history_full_scan_count(&self) -> usize {
        self.history_full_scan_count.get()
    }
}

impl VersionStore for FakeVersionStore {
    fn append_version(
        &mut self,
        workspace_id: &WorkspaceId,
        record: VersionRecord,
    ) -> Result<(), VersionStoreError> {
        let key = (
            workspace_id.as_str().to_string(),
            record.document_id().as_str().to_string(),
            record.version_id().as_str().to_string(),
        );
        self.history
            .entry((
                workspace_id.as_str().to_string(),
                record.document_id().as_str().to_string(),
            ))
            .or_default()
            .push(record.entry().clone());
        self.by_version.insert(key, record);
        Ok(())
    }

    fn get_version_snapshot(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        version_id: &VersionId,
    ) -> Result<Option<VersionSnapshot>, VersionStoreError> {
        Ok(self
            .by_version
            .get(&(
                workspace_id.as_str().to_string(),
                document_id.as_str().to_string(),
                version_id.as_str().to_string(),
            ))
            .map(|record| record.snapshot().clone()))
    }

    fn list_history(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        request: HistoryPageRequest,
    ) -> Result<HistoryPage, VersionStoreError> {
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
fn body_only_snapshot_is_explicitly_legacy_unknown() {
    let snapshot = VersionSnapshot::new(
        DocumentId::new("doc-legacy").expect("document id"),
        DocumentSnapshotRef::new("snapshot-legacy").expect("snapshot ref"),
        document_body("Legacy body"),
    );

    assert!(snapshot.attachment_state().is_legacy_unknown());
    assert_eq!(snapshot.attachment_state().references(), None);
}

#[test]
fn explicit_known_attachment_snapshot_preserves_empty_and_canonical_references() {
    let empty = VersionSnapshot::with_attachment_state(
        DocumentId::new("doc-empty").expect("document id"),
        DocumentSnapshotRef::new("snapshot-empty").expect("snapshot ref"),
        document_body("No attachments"),
        AttachmentSnapshotState::known(Vec::new()).expect("known empty"),
    );
    let populated = VersionSnapshot::with_attachment_state(
        DocumentId::new("doc-known").expect("document id"),
        DocumentSnapshotRef::new("snapshot-known").expect("snapshot ref"),
        document_body("Known attachments"),
        AttachmentSnapshotState::known(vec![
            asset_reference('b', "Second"),
            asset_reference('a', "First"),
        ])
        .expect("known references"),
    );

    assert_eq!(empty.attachment_state().references(), Some(&[][..]));
    let references = populated
        .attachment_state()
        .references()
        .expect("known references");
    assert_eq!(references[0].label(), "First");
    assert_eq!(references[1].label(), "Second");
}

#[test]
fn version_record_keeps_explicit_attachment_state() {
    let attachment_state = AttachmentSnapshotState::known(vec![asset_reference('c', "Diagram")])
        .expect("known attachment");
    let snapshot = VersionSnapshot::with_attachment_state(
        DocumentId::new("doc-1").expect("document id"),
        DocumentSnapshotRef::new("snapshot-1").expect("snapshot ref"),
        document_body("Body"),
        attachment_state.clone(),
    );
    let record = VersionRecord::new(version_entry("doc-1", "version-1", "snapshot-1"), snapshot)
        .expect("matching record");

    assert_eq!(record.snapshot().attachment_state(), &attachment_state);
}

#[test]
fn version_record_rejects_mismatched_entry_and_snapshot_identity() {
    let entry = version_entry("doc-1", "version-1", "snapshot-1");
    let snapshot = VersionSnapshot::new(
        DocumentId::new("doc-2").expect("snapshot document id"),
        DocumentSnapshotRef::new("snapshot-1").expect("snapshot ref"),
        document_body("Body"),
    );

    let error = VersionRecord::new(entry, snapshot).expect_err("mismatch must fail");

    assert_eq!(error, VersionStoreError::MismatchedVersionSnapshot);
    assert_eq!(error.code(), "version_store.mismatched_version_snapshot");
}

#[test]
fn version_store_contract_gets_specific_snapshot_without_current_repository() {
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let record = version_record("doc-1", "version-1", "snapshot-1", "Version body");
    let document_id = record.document_id().clone();
    let version_id = record.version_id().clone();
    let mut store = FakeVersionStore::default();

    store
        .append_version(&workspace_id, record)
        .expect("append version");
    let snapshot = store
        .get_version_snapshot(&workspace_id, &document_id, &version_id)
        .expect("get snapshot")
        .expect("snapshot");

    assert_eq!(snapshot.body().as_str(), "Version body");
    assert_eq!(store.current_repository_reads(), 0);
}

#[test]
fn version_store_contract_paginates_history_without_full_history_scan() {
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let document_id = DocumentId::new("doc-1").expect("document id");
    let mut store = FakeVersionStore::default();

    for version_number in 1..=3 {
        let version_id = format!("version-{version_number}");
        let snapshot_ref = format!("snapshot-{version_number}");
        store
            .append_version(
                &workspace_id,
                version_record("doc-1", &version_id, &snapshot_ref, "Version body"),
            )
            .expect("append version");
    }

    let first_page = store
        .list_history(
            &workspace_id,
            &document_id,
            HistoryPageRequest::first(2).expect("request"),
        )
        .expect("first page");
    let second_page = store
        .list_history(
            &workspace_id,
            &document_id,
            HistoryPageRequest::after(first_page.next_cursor().cloned().expect("next cursor"), 2)
                .expect("request"),
        )
        .expect("second page");

    assert_eq!(first_page.entries().len(), 2);
    assert_eq!(first_page.next_cursor().expect("next").as_str(), "2");
    assert_eq!(second_page.entries().len(), 1);
    assert!(second_page.next_cursor().is_none());
    assert_eq!(store.history_full_scan_count(), 0);
}

#[test]
fn history_pagination_preserves_assigned_revision_numbers() {
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let document_id = DocumentId::new("doc-1").expect("document id");
    let entry = version_entry("doc-1", "version-1", "snapshot-1")
        .with_revision_number(DocumentRevisionNumber::new(1).expect("revision"))
        .expect("assign revision");
    let record = VersionRecord::new(
        entry,
        VersionSnapshot::new(
            document_id.clone(),
            DocumentSnapshotRef::new("snapshot-1").expect("snapshot ref"),
            document_body("Body"),
        ),
    )
    .expect("record");
    let mut store = FakeVersionStore::default();
    store
        .append_version(&workspace_id, record)
        .expect("append version");

    let page = store
        .list_history(
            &workspace_id,
            &document_id,
            HistoryPageRequest::first(1).expect("page request"),
        )
        .expect("history page");

    assert_eq!(
        page.entries()[0]
            .revision_number()
            .map(|number| number.value()),
        Some(1)
    );
}

#[test]
fn history_page_request_rejects_invalid_page_limit() {
    assert_eq!(
        HistoryPageRequest::first(0).expect_err("zero must fail"),
        VersionStoreError::InvalidHistoryPageLimit
    );
    assert_eq!(
        HistoryPageRequest::first(MAX_HISTORY_PAGE_LIMIT + 1).expect_err("above max must fail"),
        VersionStoreError::InvalidHistoryPageLimit
    );
}

fn version_record(
    document_id: &str,
    version_id: &str,
    snapshot_ref: &str,
    body: &str,
) -> VersionRecord {
    VersionRecord::new(
        version_entry(document_id, version_id, snapshot_ref),
        VersionSnapshot::new(
            DocumentId::new(document_id).expect("snapshot document id"),
            DocumentSnapshotRef::new(snapshot_ref).expect("snapshot ref"),
            document_body(body),
        ),
    )
    .expect("version record")
}

fn version_entry(document_id: &str, version_id: &str, snapshot_ref: &str) -> VersionEntry {
    VersionEntry::new(
        VersionId::new(version_id).expect("version id"),
        DocumentId::new(document_id).expect("document id"),
        DocumentSnapshotRef::new(snapshot_ref).expect("snapshot ref"),
        VersionAuthor::new("writer").expect("author"),
        VersionSummary::new("Saved document").expect("summary"),
    )
    .expect("version entry")
}

fn document_body(value: &str) -> DocumentBody {
    DocumentBody::new(value, DocumentBodyPolicy::new(1024).expect("policy")).expect("body")
}

fn asset_reference(hash_character: char, label: &str) -> AssetReference {
    let hash = hash_character.to_string().repeat(64);
    AssetReference::new(AssetId::from_sha256_hex(&hash).expect("asset id"), label)
        .expect("asset reference")
}
