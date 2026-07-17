use std::cell::Cell;
use std::collections::HashMap;

use cabinet_domain::asset::{AssetId, AssetReference};
use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::document_diff_operation::{
    DocumentDiffOperation, DocumentDiffOperationId, DocumentDiffOperationState,
};
use cabinet_domain::document_diff_query::DocumentDiffQueryTarget;
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
use cabinet_usecases::document_diff::{DiffPolicy, DocumentLineDiffService};
use cabinet_usecases::document_diff_operation::{
    DocumentDiffOperationCreateOutcome, DocumentDiffOperationEntry,
    DocumentDiffOperationEntryError, DocumentDiffOperationPayload, DocumentDiffOperationRegistry,
    DocumentDiffOperationRegistryError, GetDocumentDiffOperationStatusInput,
    GetDocumentDiffOperationStatusUsecase, RunDocumentDiffOperationError,
    RunDocumentDiffOperationInput, RunDocumentDiffOperationUsecase,
};

struct FakeRegistry {
    entry: Option<DocumentDiffOperationEntry>,
    replace_expected: Vec<DocumentDiffOperationState>,
    conflict_on_replace: Option<usize>,
}

impl FakeRegistry {
    fn accepted(target: DocumentDiffQueryTarget) -> Self {
        let operation = DocumentDiffOperation::accepted(operation_id());
        Self {
            entry: Some(DocumentDiffOperationEntry::new(operation, target).unwrap()),
            replace_expected: Vec::new(),
            conflict_on_replace: None,
        }
    }
}

impl DocumentDiffOperationRegistry for FakeRegistry {
    fn create(
        &mut self,
        entry: DocumentDiffOperationEntry,
    ) -> Result<DocumentDiffOperationCreateOutcome, DocumentDiffOperationRegistryError> {
        self.entry = Some(entry);
        Ok(DocumentDiffOperationCreateOutcome::Created)
    }

    fn get(
        &self,
        _operation_id: &DocumentDiffOperationId,
    ) -> Result<Option<DocumentDiffOperationEntry>, DocumentDiffOperationRegistryError> {
        Ok(self.entry.clone())
    }

    fn replace(
        &mut self,
        entry: DocumentDiffOperationEntry,
        expected_state: DocumentDiffOperationState,
    ) -> Result<(), DocumentDiffOperationRegistryError> {
        self.replace_expected.push(expected_state);
        if self.conflict_on_replace == Some(self.replace_expected.len()) {
            if expected_state == DocumentDiffOperationState::Running {
                let cancelled = DocumentDiffOperation::restore(
                    operation_id(),
                    DocumentDiffOperationState::Cancelled,
                );
                self.entry = Some(
                    DocumentDiffOperationEntry::new(cancelled, entry.target().clone()).unwrap(),
                );
            }
            return Err(DocumentDiffOperationRegistryError::Conflict);
        }
        self.entry = Some(entry);
        Ok(())
    }
}

struct FakePointer {
    current: Option<VersionId>,
    reads: Cell<usize>,
}

impl CurrentDocumentVersionPointerPort for FakePointer {
    fn load_current_version(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
    ) -> Result<Option<VersionId>, CurrentDocumentVersionPointerError> {
        self.reads.set(self.reads.get() + 1);
        Ok(self.current.clone())
    }

    fn compare_and_set_current_version(
        &mut self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        _expected: Option<&VersionId>,
        _next: VersionId,
    ) -> Result<(), CurrentDocumentVersionPointerError> {
        panic!("diff operation must not write current pointer")
    }
}

#[derive(Default)]
struct FakeCommittedReader {
    records: HashMap<String, VersionRecord>,
    reads: Cell<usize>,
}

impl CommittedVersionRecordReader for FakeCommittedReader {
    fn get_committed_version_record(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        version_id: &VersionId,
    ) -> Result<Option<VersionRecord>, CommittedVersionRecordReadError> {
        self.reads.set(self.reads.get() + 1);
        Ok(self.records.get(version_id.as_str()).cloned())
    }
}

#[test]
fn entry_rejects_payload_and_operation_state_mismatch() {
    let target = versions_target();
    let running =
        DocumentDiffOperation::restore(operation_id(), DocumentDiffOperationState::Running);
    let completed =
        DocumentDiffOperation::restore(operation_id(), DocumentDiffOperationState::Completed);

    assert_eq!(
        DocumentDiffOperationEntry::with_payload(
            running,
            target.clone(),
            DocumentDiffOperationPayload::Failed {
                error_code: "document.diff.failed",
            },
        )
        .unwrap_err(),
        DocumentDiffOperationEntryError::StatePayloadMismatch
    );
    assert_eq!(
        DocumentDiffOperationEntry::with_payload(
            completed,
            target,
            DocumentDiffOperationPayload::Pending,
        )
        .unwrap_err(),
        DocumentDiffOperationEntryError::StatePayloadMismatch
    );
}

#[test]
fn worker_completes_with_authoritative_result_and_attachment_diff() {
    let mut registry = FakeRegistry::accepted(current_target());
    let pointer = pointer(Some("version-2"));
    let versions = version_reader(
        ("version-1", "Same\n", known(vec![reference('a', "A")])),
        (
            "version-2",
            "Same\n",
            known(vec![reference('a', "A"), reference('b', "B")]),
        ),
    );

    let output = worker(DiffPolicy::new(1, 1024, 100, 100).unwrap())
        .execute(
            RunDocumentDiffOperationInput::new("opaque-operation-1"),
            &mut registry,
            &pointer,
            &versions,
        )
        .unwrap();

    assert_eq!(output.state(), DocumentDiffOperationState::Completed);
    assert_eq!(output.failure_code(), None);
    assert_eq!(pointer.reads.get(), 1);
    assert_eq!(versions.reads.get(), 2);
    assert_eq!(
        registry.replace_expected,
        vec![
            DocumentDiffOperationState::Accepted,
            DocumentDiffOperationState::Running,
        ]
    );

    let status = GetDocumentDiffOperationStatusUsecase::new()
        .execute(
            GetDocumentDiffOperationStatusInput::new("opaque-operation-1"),
            &registry,
        )
        .unwrap();
    let result = status.result().expect("completed authoritative payload");
    assert_eq!(result.left_version_id().as_str(), "version-2");
    assert_eq!(result.right_version_id().as_str(), "version-1");
    let AttachmentDiff::Known(attachments) = result.attachment_diff() else {
        panic!("known attachment diff")
    };
    assert_eq!(attachments.removed()[0].label(), "B");
    assert_eq!(attachments.unchanged_count(), 1);
}

#[test]
fn version_pair_worker_does_not_read_current_pointer() {
    let mut registry = FakeRegistry::accepted(versions_target());
    let pointer = pointer(None);
    let versions = version_reader(
        ("version-1", "Left\n", known(Vec::new())),
        ("version-2", "Right\n", known(Vec::new())),
    );

    worker(DiffPolicy::new(1, 1024, 100, 100).unwrap())
        .execute(
            RunDocumentDiffOperationInput::new("opaque-operation-1"),
            &mut registry,
            &pointer,
            &versions,
        )
        .unwrap();

    assert_eq!(pointer.reads.get(), 0);
    assert_eq!(versions.reads.get(), 2);
}

#[test]
fn worker_maps_too_large_and_missing_committed_record_to_failed_payload() {
    let cases = [
        (
            version_reader(
                ("version-1", "left body\n", known(Vec::new())),
                ("version-2", "right body\n", known(Vec::new())),
            ),
            DiffPolicy::new(0, 4, 100, 100).unwrap(),
            "document.diff.background_limit_exceeded",
        ),
        (
            FakeCommittedReader::default(),
            DiffPolicy::new(0, 1024, 100, 100).unwrap(),
            "authoritative_document_diff.not_found",
        ),
    ];

    for (versions, policy, expected_code) in cases {
        let mut registry = FakeRegistry::accepted(versions_target());
        let output = worker(policy)
            .execute(
                RunDocumentDiffOperationInput::new("opaque-operation-1"),
                &mut registry,
                &pointer(None),
                &versions,
            )
            .unwrap();

        assert_eq!(output.state(), DocumentDiffOperationState::Failed);
        assert_eq!(output.failure_code(), Some(expected_code));
        assert!(matches!(
            registry.entry.as_ref().unwrap().payload(),
            DocumentDiffOperationPayload::Failed { error_code } if *error_code == expected_code
        ));
    }
}

#[test]
fn final_replace_conflict_does_not_overwrite_cancellation_or_store_result() {
    let mut registry = FakeRegistry::accepted(versions_target());
    registry.conflict_on_replace = Some(2);
    let versions = version_reader(
        ("version-1", "left\n", known(Vec::new())),
        ("version-2", "right\n", known(Vec::new())),
    );
    let error = worker(DiffPolicy::new(0, 1024, 100, 100).unwrap())
        .execute(
            RunDocumentDiffOperationInput::new("opaque-operation-1"),
            &mut registry,
            &pointer(None),
            &versions,
        )
        .unwrap_err();

    assert_eq!(error, RunDocumentDiffOperationError::Conflict);
    let stored = registry.entry.as_ref().unwrap();
    assert_eq!(
        stored.operation().state(),
        DocumentDiffOperationState::Cancelled
    );
    assert!(matches!(
        stored.payload(),
        DocumentDiffOperationPayload::Pending
    ));
}

#[test]
fn terminal_operation_is_not_recomputed() {
    let mut initial = FakeRegistry::accepted(versions_target());
    let pointer = pointer(None);
    let versions = version_reader(
        ("version-1", "left\n", known(Vec::new())),
        ("version-2", "right\n", known(Vec::new())),
    );
    let worker = worker(DiffPolicy::new(0, 1024, 100, 100).unwrap());
    worker
        .execute(
            RunDocumentDiffOperationInput::new("opaque-operation-1"),
            &mut initial,
            &pointer,
            &versions,
        )
        .unwrap();
    initial.replace_expected.clear();
    let reads_before = versions.reads.get();

    let output = worker
        .execute(
            RunDocumentDiffOperationInput::new("opaque-operation-1"),
            &mut initial,
            &pointer,
            &versions,
        )
        .unwrap();

    assert_eq!(output.state(), DocumentDiffOperationState::Completed);
    assert!(initial.replace_expected.is_empty());
    assert_eq!(pointer.reads.get(), 0);
    assert_eq!(versions.reads.get(), reads_before);
}

fn worker(policy: DiffPolicy) -> RunDocumentDiffOperationUsecase {
    RunDocumentDiffOperationUsecase::with_diff_service(DocumentLineDiffService::with_policy(policy))
}

fn current_target() -> DocumentDiffQueryTarget {
    DocumentDiffQueryTarget::current_to_version("workspace-1", "doc-1", "version-1").unwrap()
}

fn versions_target() -> DocumentDiffQueryTarget {
    DocumentDiffQueryTarget::versions("workspace-1", "doc-1", "version-1", "version-2").unwrap()
}

fn operation_id() -> DocumentDiffOperationId {
    DocumentDiffOperationId::new("opaque-operation-1").unwrap()
}

fn pointer(version_id: Option<&str>) -> FakePointer {
    FakePointer {
        current: version_id.map(|value| VersionId::new(value).unwrap()),
        reads: Cell::new(0),
    }
}

fn version_reader(
    left: (&str, &str, AttachmentSnapshotState),
    right: (&str, &str, AttachmentSnapshotState),
) -> FakeCommittedReader {
    let mut reader = FakeCommittedReader::default();
    for (version_id, body, attachments) in [left, right] {
        reader.records.insert(
            version_id.to_string(),
            version_record(version_id, body, attachments),
        );
    }
    reader
}

fn version_record(
    version_id: &str,
    body: &str,
    attachments: AttachmentSnapshotState,
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
        attachments,
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
