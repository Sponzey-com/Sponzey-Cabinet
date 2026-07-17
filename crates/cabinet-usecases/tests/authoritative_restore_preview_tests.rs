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
use cabinet_usecases::authoritative_restore_preview::{
    PreviewAuthoritativeDocumentRestoreError, PreviewAuthoritativeDocumentRestoreInput,
    PreviewAuthoritativeDocumentRestoreUsecase,
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
        panic!("preview must not write current pointer")
    }
}

#[derive(Default)]
struct FakeCommittedReader {
    records: HashMap<String, VersionRecord>,
    reads: Cell<usize>,
    error: Option<CommittedVersionRecordReadError>,
}

impl FakeCommittedReader {
    fn insert(&mut self, version_id: &str, body: &str, attachments: AttachmentSnapshotState) {
        self.records.insert(
            version_id.to_string(),
            version_record(version_id, body, attachments),
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
fn preview_derives_expected_current_from_pointer_and_returns_full_diff() {
    let pointer = pointer("version-current");
    let mut versions = FakeCommittedReader::default();
    versions.insert(
        "version-current",
        "# 현재 제목\n현재 본문\n",
        known(vec![reference('a', "현재 첨부")]),
    );
    versions.insert(
        "version-target",
        "# 과거 제목\n과거 본문\n",
        known(vec![reference('b', "과거 첨부")]),
    );

    let output = PreviewAuthoritativeDocumentRestoreUsecase::new()
        .execute(
            PreviewAuthoritativeDocumentRestoreInput::new("workspace-1", "doc-1", "version-target"),
            &pointer,
            &versions,
        )
        .unwrap();

    assert_eq!(
        output.expected_current_version_id().as_str(),
        "version-current"
    );
    assert_eq!(output.target_version_id().as_str(), "version-target");
    assert!(matches!(output.computation(), DiffComputation::Complete(_)));
    let AttachmentDiff::Known(attachments) = output.attachment_diff() else {
        panic!("known attachment diff")
    };
    assert_eq!(attachments.added()[0].label(), "과거 첨부");
    assert_eq!(attachments.removed()[0].label(), "현재 첨부");
    assert_eq!(pointer.reads.get(), 1);
    assert_eq!(versions.reads.get(), 2);
}

#[test]
fn preview_preserves_legacy_attachment_unknown_and_too_large_outcome() {
    let pointer = pointer("version-current");
    let mut versions = FakeCommittedReader::default();
    versions.insert(
        "version-current",
        "Current body\n",
        AttachmentSnapshotState::legacy_unknown(),
    );
    versions.insert("version-target", "Target body\n", known(Vec::new()));
    let usecase = PreviewAuthoritativeDocumentRestoreUsecase::with_policy(
        DiffPolicy::new(0, 4, 100, 100).unwrap(),
    );

    let output = usecase
        .execute(
            PreviewAuthoritativeDocumentRestoreInput::new("workspace-1", "doc-1", "version-target"),
            &pointer,
            &versions,
        )
        .unwrap();

    assert_eq!(
        output.computation(),
        &DiffComputation::TooLarge(DiffLimitReason::Bytes)
    );
    assert_eq!(output.attachment_diff(), &AttachmentDiff::LegacyUnknown);
}

#[test]
fn preview_maps_pointer_and_record_failures_without_side_effects() {
    let unavailable_pointer = FakePointer {
        current: None,
        reads: Cell::new(0),
        error: Some(CurrentDocumentVersionPointerError::StorageUnavailable),
    };
    let input =
        PreviewAuthoritativeDocumentRestoreInput::new("workspace-1", "doc-1", "version-target");
    assert_eq!(
        PreviewAuthoritativeDocumentRestoreUsecase::new()
            .execute(
                input.clone(),
                &unavailable_pointer,
                &FakeCommittedReader::default()
            )
            .unwrap_err(),
        PreviewAuthoritativeDocumentRestoreError::StorageUnavailable
    );

    let corrupt_pointer = FakePointer {
        current: None,
        reads: Cell::new(0),
        error: Some(CurrentDocumentVersionPointerError::CorruptedPointer),
    };
    assert_eq!(
        PreviewAuthoritativeDocumentRestoreUsecase::new()
            .execute(input, &corrupt_pointer, &FakeCommittedReader::default())
            .unwrap_err(),
        PreviewAuthoritativeDocumentRestoreError::CorruptedData
    );
}

fn pointer(version_id: &str) -> FakePointer {
    FakePointer {
        current: Some(VersionId::new(version_id).unwrap()),
        reads: Cell::new(0),
        error: None,
    }
}

fn version_record(
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
