use std::cell::Cell;
use std::collections::HashMap;

use cabinet_domain::asset::{AssetId, AssetReference};
use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::version::{
    AttachmentSnapshotState, DocumentSnapshotRef, VersionAuthor, VersionEntry, VersionId,
    VersionSummary,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::committed_version_record_reader::{
    CommittedVersionRecordReadError, CommittedVersionRecordReader,
};
use cabinet_ports::current_document_version::{
    CurrentDocumentVersionPointerError, CurrentDocumentVersionPointerPort,
};
use cabinet_ports::version_store::{VersionRecord, VersionSnapshot};
use cabinet_usecases::attachment_diff::AttachmentDiff;
use cabinet_usecases::authoritative_document_diff::{
    CompareAuthoritativeDocumentRevisionsError, CompareAuthoritativeDocumentRevisionsInput,
    CompareAuthoritativeDocumentRevisionsUsecase,
};
use cabinet_usecases::document_diff::{DiffComputation, DiffLimitReason, DiffPolicy};

struct FakePointer {
    current: Option<VersionId>,
    reads: Cell<usize>,
    error: Option<CurrentDocumentVersionPointerError>,
}

impl CurrentDocumentVersionPointerPort for FakePointer {
    fn load_current_version(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
    ) -> Result<Option<VersionId>, CurrentDocumentVersionPointerError> {
        self.reads.set(self.reads.get() + 1);
        self.error.map_or_else(|| Ok(self.current.clone()), Err)
    }

    fn compare_and_set_current_version(
        &mut self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        _expected: Option<&VersionId>,
        _next: VersionId,
    ) -> Result<(), CurrentDocumentVersionPointerError> {
        panic!("diff query must not write the pointer")
    }
}

#[derive(Default)]
struct FakeCommittedReader {
    records: HashMap<String, VersionRecord>,
    reads: Cell<usize>,
    error: Option<CommittedVersionRecordReadError>,
}

impl FakeCommittedReader {
    fn insert(&mut self, version_id: &str, body: &str) {
        self.records
            .insert(version_id.to_string(), version_record(version_id, body));
    }

    fn insert_with_attachments(
        &mut self,
        version_id: &str,
        body: &str,
        attachment_state: AttachmentSnapshotState,
    ) {
        self.records.insert(
            version_id.to_string(),
            version_record_with_attachments(version_id, body, attachment_state),
        );
    }
}

impl CommittedVersionRecordReader for FakeCommittedReader {
    fn get_committed_version_record(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        version_id: &VersionId,
    ) -> Result<Option<VersionRecord>, CommittedVersionRecordReadError> {
        self.reads.set(self.reads.get() + 1);
        self.error
            .map_or_else(|| Ok(self.records.get(version_id.as_str()).cloned()), Err)
    }
}

#[test]
fn current_to_version_reads_pointer_and_two_committed_records_only() {
    let pointer = pointer(Some("version-2"));
    let mut versions = FakeCommittedReader::default();
    versions.insert("version-1", "Old title\nold body\n");
    versions.insert("version-2", "Current title\ncurrent body\n");

    let output = CompareAuthoritativeDocumentRevisionsUsecase::new()
        .execute(
            CompareAuthoritativeDocumentRevisionsInput::current_to_version(
                "workspace-1",
                "doc-1",
                "version-1",
            ),
            &pointer,
            &versions,
        )
        .expect("authoritative diff");

    assert_eq!(output.left_version_id().as_str(), "version-2");
    assert_eq!(output.right_version_id().as_str(), "version-1");
    assert!(matches!(output.computation(), DiffComputation::Complete(_)));
    assert_eq!(pointer.reads.get(), 1);
    assert_eq!(versions.reads.get(), 2);
}

#[test]
fn version_pair_does_not_read_current_pointer() {
    let pointer = pointer(None);
    let mut versions = FakeCommittedReader::default();
    versions.insert("version-1", "Left\n");
    versions.insert("version-2", "Right\n");

    let output = CompareAuthoritativeDocumentRevisionsUsecase::new()
        .execute(
            CompareAuthoritativeDocumentRevisionsInput::versions(
                "workspace-1",
                "doc-1",
                "version-1",
                "version-2",
            ),
            &pointer,
            &versions,
        )
        .expect("version pair diff");

    assert_eq!(output.left_version_id().as_str(), "version-1");
    assert_eq!(output.right_version_id().as_str(), "version-2");
    assert_eq!(pointer.reads.get(), 0);
    assert_eq!(versions.reads.get(), 2);
}

#[test]
fn attachment_only_revision_is_part_of_authoritative_diff() {
    let pointer = pointer(Some("version-2"));
    let mut versions = FakeCommittedReader::default();
    versions.insert_with_attachments(
        "version-1",
        "Same title\nsame body\n",
        known(vec![reference('a', "A")]),
    );
    versions.insert_with_attachments(
        "version-2",
        "Same title\nsame body\n",
        known(vec![reference('a', "A"), reference('b', "B")]),
    );

    let output = CompareAuthoritativeDocumentRevisionsUsecase::new()
        .execute(
            CompareAuthoritativeDocumentRevisionsInput::current_to_version(
                "workspace-1",
                "doc-1",
                "version-1",
            ),
            &pointer,
            &versions,
        )
        .expect("attachment-only authoritative diff");

    let AttachmentDiff::Known(attachments) = output.attachment_diff() else {
        panic!("known attachment diff");
    };
    assert!(matches!(output.computation(), DiffComputation::Complete(_)));
    assert!(attachments.added().is_empty());
    assert_eq!(attachments.removed()[0].label(), "B");
    assert_eq!(attachments.unchanged_count(), 1);
}

#[test]
fn authoritative_diff_preserves_legacy_attachment_unknown() {
    let pointer = pointer(None);
    let mut versions = FakeCommittedReader::default();
    versions.insert_with_attachments(
        "version-1",
        "Same\n",
        AttachmentSnapshotState::legacy_unknown(),
    );
    versions.insert_with_attachments("version-2", "Same\n", known(Vec::new()));

    let output = CompareAuthoritativeDocumentRevisionsUsecase::new()
        .execute(
            CompareAuthoritativeDocumentRevisionsInput::versions(
                "workspace-1",
                "doc-1",
                "version-1",
                "version-2",
            ),
            &pointer,
            &versions,
        )
        .unwrap();

    assert_eq!(output.attachment_diff(), &AttachmentDiff::LegacyUnknown);
}

#[test]
fn injected_policy_preserves_too_large_as_a_non_error_outcome() {
    let pointer = pointer(None);
    let mut versions = FakeCommittedReader::default();
    versions.insert_with_attachments(
        "version-1",
        "Left body\n",
        known(vec![reference('a', "Before limit")]),
    );
    versions.insert_with_attachments(
        "version-2",
        "Right body\n",
        known(vec![
            reference('a', "Before limit"),
            reference('b', "After limit"),
        ]),
    );
    let policy = DiffPolicy::new(0, 4, 100, 100).unwrap();

    let output = CompareAuthoritativeDocumentRevisionsUsecase::with_policy(policy)
        .execute(
            CompareAuthoritativeDocumentRevisionsInput::versions(
                "workspace-1",
                "doc-1",
                "version-1",
                "version-2",
            ),
            &pointer,
            &versions,
        )
        .expect("bounded diff outcome");

    assert_eq!(
        output.computation(),
        &DiffComputation::TooLarge(DiffLimitReason::Bytes)
    );
    let AttachmentDiff::Known(attachments) = output.attachment_diff() else {
        panic!("known attachment diff must survive a body limit");
    };
    assert_eq!(attachments.added()[0].label(), "After limit");
    assert_eq!(attachments.unchanged_count(), 1);
}

#[test]
fn missing_and_corrupt_authoritative_data_are_typed_failures() {
    let pointer = pointer(Some("version-current"));
    let versions = FakeCommittedReader::default();
    let input = CompareAuthoritativeDocumentRevisionsInput::current_to_version(
        "workspace-1",
        "doc-1",
        "version-missing",
    );

    assert_eq!(
        CompareAuthoritativeDocumentRevisionsUsecase::new()
            .execute(input.clone(), &pointer, &versions)
            .unwrap_err(),
        CompareAuthoritativeDocumentRevisionsError::NotFound
    );

    let corrupt = FakeCommittedReader {
        error: Some(CommittedVersionRecordReadError::CorruptedRecord),
        ..Default::default()
    };
    assert_eq!(
        CompareAuthoritativeDocumentRevisionsUsecase::new()
            .execute(input, &pointer, &corrupt)
            .unwrap_err(),
        CompareAuthoritativeDocumentRevisionsError::CorruptedData
    );
}

fn pointer(version_id: Option<&str>) -> FakePointer {
    FakePointer {
        current: version_id.map(|value| VersionId::new(value).unwrap()),
        reads: Cell::new(0),
        error: None,
    }
}

fn version_record(version_id: &str, body: &str) -> VersionRecord {
    version_record_with_attachments(version_id, body, AttachmentSnapshotState::legacy_unknown())
}

fn version_record_with_attachments(
    version_id: &str,
    body: &str,
    attachment_state: AttachmentSnapshotState,
) -> VersionRecord {
    let document_id = DocumentId::new("doc-1").unwrap();
    let snapshot_ref = DocumentSnapshotRef::new(&format!("snapshot-{version_id}")).unwrap();
    let entry = VersionEntry::new(
        VersionId::new(version_id).unwrap(),
        document_id.clone(),
        snapshot_ref.clone(),
        VersionAuthor::new("local-user").unwrap(),
        VersionSummary::new("Test revision").unwrap(),
    )
    .unwrap();
    let snapshot = VersionSnapshot::with_attachment_state(
        document_id,
        snapshot_ref,
        DocumentBody::new(body, DocumentBodyPolicy::new(4096).unwrap()).unwrap(),
        attachment_state,
    );
    VersionRecord::new(entry, snapshot).unwrap()
}

fn known(references: Vec<AssetReference>) -> AttachmentSnapshotState {
    AttachmentSnapshotState::known(references).unwrap()
}

fn reference(seed: char, label: &str) -> AssetReference {
    AssetReference::new(
        AssetId::from_sha256_hex(&seed.to_string().repeat(64)).unwrap(),
        label,
    )
    .unwrap()
}
