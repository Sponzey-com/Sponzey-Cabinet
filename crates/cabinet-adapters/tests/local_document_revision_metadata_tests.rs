use std::cell::Cell;

use cabinet_adapters::local_document_mutation_fingerprint::LocalDocumentMutationFingerprint;
use cabinet_adapters::local_document_revision_metadata::{
    LocalDocumentRevisionMetadataSource, LocalDocumentRevisionNumberAllocator,
};
use cabinet_domain::asset::{AssetId, AssetReference};
use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::document_revision::{DocumentExpectedCurrentVersion, DocumentMutationKind};
use cabinet_domain::version::{
    AttachmentSnapshotState, DocumentRevisionNumber, DocumentSnapshotRef, VersionAuthor,
    VersionEntry, VersionId, VersionSummary,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_version::{
    CurrentDocumentVersionPointerError, CurrentDocumentVersionPointerPort,
};
use cabinet_ports::document_mutation_fingerprint::{
    DocumentMutationFingerprintInput, DocumentMutationFingerprintPort,
};
use cabinet_ports::document_revision_metadata::{
    DocumentRevisionClock, DocumentRevisionMetadataPortError, DocumentRevisionNumberAllocator,
    DocumentSnapshotRefGenerator, DocumentVersionIdGenerator,
};
use cabinet_ports::version_store::{
    HistoryCursor, HistoryPage, HistoryPageRequest, VersionRecord, VersionSnapshot, VersionStore,
    VersionStoreError,
};

#[test]
fn local_fingerprint_is_deterministic_and_changes_with_canonical_payload() {
    let first = LocalDocumentMutationFingerprint::new()
        .fingerprint(&fingerprint_input("# Title\r\nbody", "author-a"))
        .expect("fingerprint");
    let restarted = LocalDocumentMutationFingerprint::new()
        .fingerprint(&fingerprint_input("# Title\nbody", "author-a"))
        .expect("same normalized fingerprint");
    let changed = LocalDocumentMutationFingerprint::new()
        .fingerprint(&fingerprint_input("# Title\nchanged", "author-a"))
        .expect("changed fingerprint");
    let changed_author = LocalDocumentMutationFingerprint::new()
        .fingerprint(&fingerprint_input("# Title\nbody", "author-b"))
        .expect("changed author fingerprint");
    let changed_expected = LocalDocumentMutationFingerprint::new()
        .fingerprint(&fingerprint_input_with(
            "# Title\nbody",
            "author-a",
            DocumentExpectedCurrentVersion::MustMatch(version_id(1)),
            Vec::new(),
        ))
        .expect("changed expected fingerprint");
    let changed_attachment = LocalDocumentMutationFingerprint::new()
        .fingerprint(&fingerprint_input_with(
            "# Title\nbody",
            "author-a",
            DocumentExpectedCurrentVersion::MustNotExist,
            vec![
                AssetReference::new(
                    AssetId::from_sha256_hex(&"a".repeat(64)).expect("asset"),
                    "reference label",
                )
                .expect("reference"),
            ],
        ))
        .expect("changed attachment fingerprint");

    assert_eq!(first, restarted);
    assert_ne!(first, changed);
    assert_ne!(first, changed_author);
    assert_ne!(first, changed_expected);
    assert_ne!(first, changed_attachment);
    assert!(first.as_str().starts_with("sha256:"));
    assert_eq!(first.as_str().len(), 71);
    assert!(!first.as_str().contains("Title"));
    assert!(
        first.as_str().as_bytes()[7..]
            .iter()
            .all(|value| value.is_ascii_hexdigit() && !value.is_ascii_uppercase())
    );
}

#[test]
fn local_metadata_source_generates_unique_ids_derived_snapshot_and_positive_clock() {
    let source = LocalDocumentRevisionMetadataSource::new();
    let first = source.generate_version_id().expect("first version");
    let second = source.generate_version_id().expect("second version");
    let snapshot = source.generate_snapshot_ref(&first).expect("snapshot ref");

    assert_ne!(first, second);
    assert!(first.as_str().starts_with("version:"));
    assert_eq!(snapshot.as_str(), format!("snapshot:{}", first.as_str()));
    assert!(source.now_epoch_ms().expect("clock") > 0);
}

#[test]
fn allocator_validates_pointer_and_finds_expected_revision_across_pages() {
    let entries = (1..=101)
        .map(|revision| version_entry(revision))
        .collect::<Vec<_>>();
    let versions = FakeVersionStore::new(entries);
    let pointer = FakePointer::new(Some(version_id(101)));
    let allocator = LocalDocumentRevisionNumberAllocator::new(&versions, &pointer);

    let revision = allocator
        .allocate_next_revision(
            &workspace_id(),
            &document_id(),
            &DocumentExpectedCurrentVersion::MustMatch(version_id(101)),
        )
        .expect("next revision");

    assert_eq!(revision.value(), 102);
    assert_eq!(pointer.load_count.get(), 1);
    assert_eq!(versions.list_count.get(), 2);
}

#[test]
fn allocator_handles_create_and_rejects_pointer_or_history_mismatch() {
    let empty_versions = FakeVersionStore::new(Vec::new());
    let empty_pointer = FakePointer::new(None);
    let allocator = LocalDocumentRevisionNumberAllocator::new(&empty_versions, &empty_pointer);
    assert_eq!(
        allocator
            .allocate_next_revision(
                &workspace_id(),
                &document_id(),
                &DocumentExpectedCurrentVersion::MustNotExist,
            )
            .expect("first revision")
            .value(),
        1
    );
    assert_eq!(empty_versions.list_count.get(), 0);

    let versions = FakeVersionStore::new(vec![version_entry(1)]);
    let mismatched_pointer = FakePointer::new(Some(version_id(2)));
    let allocator = LocalDocumentRevisionNumberAllocator::new(&versions, &mismatched_pointer);
    assert_eq!(
        allocator
            .allocate_next_revision(
                &workspace_id(),
                &document_id(),
                &DocumentExpectedCurrentVersion::MustMatch(version_id(1)),
            )
            .expect_err("pointer mismatch"),
        DocumentRevisionMetadataPortError::Conflict
    );
    assert_eq!(versions.list_count.get(), 0);

    let missing_versions = FakeVersionStore::new(vec![version_entry(1)]);
    let matching_pointer = FakePointer::new(Some(version_id(2)));
    let allocator = LocalDocumentRevisionNumberAllocator::new(&missing_versions, &matching_pointer);
    assert_eq!(
        allocator
            .allocate_next_revision(
                &workspace_id(),
                &document_id(),
                &DocumentExpectedCurrentVersion::MustMatch(version_id(2)),
            )
            .expect_err("missing expected history"),
        DocumentRevisionMetadataPortError::StorageUnavailable
    );

    let legacy_versions = FakeVersionStore::new(vec![legacy_version_entry("legacy-version")]);
    let legacy_pointer = FakePointer::new(Some(VersionId::new("legacy-version").expect("version")));
    let allocator = LocalDocumentRevisionNumberAllocator::new(&legacy_versions, &legacy_pointer);
    assert_eq!(
        allocator
            .allocate_next_revision(
                &workspace_id(),
                &document_id(),
                &DocumentExpectedCurrentVersion::MustMatch(
                    VersionId::new("legacy-version").expect("version")
                ),
            )
            .expect_err("legacy unassigned revision"),
        DocumentRevisionMetadataPortError::StorageUnavailable
    );

    let overflow_id = VersionId::new("overflow-version").expect("version");
    let overflow_entry = VersionEntry::new(
        overflow_id.clone(),
        document_id(),
        DocumentSnapshotRef::new("overflow-snapshot").expect("snapshot"),
        VersionAuthor::new("local-user").expect("author"),
        VersionSummary::new("Update").expect("summary"),
    )
    .expect("entry")
    .with_created_at_epoch_ms(u64::MAX)
    .expect("timestamp")
    .with_revision_number(DocumentRevisionNumber::new(u64::MAX).expect("revision"))
    .expect("assigned revision");
    let overflow_versions = FakeVersionStore::new(vec![overflow_entry]);
    let overflow_pointer = FakePointer::new(Some(overflow_id.clone()));
    let allocator =
        LocalDocumentRevisionNumberAllocator::new(&overflow_versions, &overflow_pointer);
    assert_eq!(
        allocator
            .allocate_next_revision(
                &workspace_id(),
                &document_id(),
                &DocumentExpectedCurrentVersion::MustMatch(overflow_id),
            )
            .expect_err("revision overflow"),
        DocumentRevisionMetadataPortError::StorageUnavailable
    );
}

fn fingerprint_input(body: &str, author: &str) -> DocumentMutationFingerprintInput {
    fingerprint_input_with(
        body,
        author,
        DocumentExpectedCurrentVersion::MustNotExist,
        Vec::new(),
    )
}

fn fingerprint_input_with(
    body: &str,
    author: &str,
    expected_current: DocumentExpectedCurrentVersion,
    references: Vec<AssetReference>,
) -> DocumentMutationFingerprintInput {
    DocumentMutationFingerprintInput::new(
        DocumentMutationKind::Create,
        workspace_id(),
        document_id(),
        expected_current,
        DocumentBody::new(body, DocumentBodyPolicy::new(1024).expect("body policy")).expect("body"),
        VersionAuthor::new(author).expect("author"),
        VersionSummary::new("Create").expect("summary"),
        AttachmentSnapshotState::known(references).expect("attachments"),
    )
}

fn version_entry(revision: u64) -> VersionEntry {
    VersionEntry::new(
        version_id(revision),
        document_id(),
        DocumentSnapshotRef::new(&format!("snapshot-{revision}")).expect("snapshot"),
        VersionAuthor::new("local-user").expect("author"),
        VersionSummary::new("Update").expect("summary"),
    )
    .expect("entry")
    .with_created_at_epoch_ms(revision)
    .expect("timestamp")
    .with_revision_number(DocumentRevisionNumber::new(revision).expect("revision"))
    .expect("assigned revision")
}

fn legacy_version_entry(version: &str) -> VersionEntry {
    VersionEntry::new(
        VersionId::new(version).expect("version"),
        document_id(),
        DocumentSnapshotRef::new("legacy-snapshot").expect("snapshot"),
        VersionAuthor::new("local-user").expect("author"),
        VersionSummary::new("Legacy").expect("summary"),
    )
    .expect("legacy entry")
}

fn version_id(revision: u64) -> VersionId {
    VersionId::new(&format!("version-{revision}")).expect("version")
}

fn workspace_id() -> WorkspaceId {
    WorkspaceId::new("workspace-1").expect("workspace")
}

fn document_id() -> DocumentId {
    DocumentId::new("doc-1").expect("document")
}

struct FakePointer {
    current: Option<VersionId>,
    load_count: Cell<usize>,
}

impl FakePointer {
    fn new(current: Option<VersionId>) -> Self {
        Self {
            current,
            load_count: Cell::new(0),
        }
    }
}

impl CurrentDocumentVersionPointerPort for FakePointer {
    fn load_current_version(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
    ) -> Result<Option<VersionId>, CurrentDocumentVersionPointerError> {
        self.load_count.set(self.load_count.get() + 1);
        Ok(self.current.clone())
    }

    fn compare_and_set_current_version(
        &mut self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        _expected: Option<&VersionId>,
        _next: VersionId,
    ) -> Result<(), CurrentDocumentVersionPointerError> {
        unreachable!("allocator is read only")
    }
}

struct FakeVersionStore {
    entries: Vec<VersionEntry>,
    list_count: Cell<usize>,
}

impl FakeVersionStore {
    fn new(entries: Vec<VersionEntry>) -> Self {
        Self {
            entries,
            list_count: Cell::new(0),
        }
    }
}

impl VersionStore for FakeVersionStore {
    fn append_version(
        &mut self,
        _workspace_id: &WorkspaceId,
        _record: VersionRecord,
    ) -> Result<(), VersionStoreError> {
        unreachable!("allocator is read only")
    }

    fn get_version_snapshot(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        _version_id: &VersionId,
    ) -> Result<Option<VersionSnapshot>, VersionStoreError> {
        unreachable!("allocator scans history")
    }

    fn list_history(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        request: HistoryPageRequest,
    ) -> Result<HistoryPage, VersionStoreError> {
        self.list_count.set(self.list_count.get() + 1);
        let start = request
            .cursor()
            .map(|cursor| cursor.as_str().parse::<usize>().expect("cursor"))
            .unwrap_or(0);
        let end = (start + request.limit()).min(self.entries.len());
        let next = (end < self.entries.len())
            .then(|| HistoryCursor::new(&end.to_string()).expect("next cursor"));
        Ok(HistoryPage::new(self.entries[start..end].to_vec(), next))
    }
}
