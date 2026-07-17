use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use cabinet_domain::asset::{AssetId, AssetReference};
use cabinet_domain::attachment_snapshot_mutation::AttachmentSnapshotDelta;
use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::document_revision::{
    DocumentExpectedCurrentVersion, DocumentMutationFingerprint, DocumentMutationKind,
    DocumentOperationId, DocumentOperationIdentity,
};
use cabinet_domain::version::{
    AttachmentSnapshotState, DocumentRevisionNumber, DocumentSnapshotRef, VersionAuthor,
    VersionEntry, VersionId, VersionSummary,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::committed_version_record_reader::{
    CommittedVersionRecordReadError, CommittedVersionRecordReader,
};
use cabinet_ports::current_document_version::{
    CurrentDocumentVersionPointerError, CurrentDocumentVersionPointerPort,
};
use cabinet_ports::document_mutation_fingerprint::{
    DocumentMutationFingerprintInput, DocumentMutationFingerprintPort,
    DocumentMutationFingerprintPortError,
};
use cabinet_ports::document_revision_commit::{
    DocumentOperationJournalClaim, DocumentOperationJournalError, DocumentOperationJournalPort,
    DocumentOperationJournalRecord, DocumentOperationTerminalFailure, DocumentRevisionCommitError,
    DocumentRevisionCommitPort, DocumentRevisionCommitRequest, DocumentRevisionCommitResult,
};
use cabinet_ports::document_revision_metadata::{
    DocumentRevisionClock, DocumentRevisionMetadataPortError, DocumentRevisionNumberAllocator,
    DocumentSnapshotRefGenerator, DocumentVersionIdGenerator,
};
use cabinet_ports::version_store::{VersionRecord, VersionSnapshot};
use cabinet_usecases::mutate_document_attachments::{
    MutateDocumentAttachmentsError, MutateDocumentAttachmentsInput,
    MutateDocumentAttachmentsOutcomeKind, MutateDocumentAttachmentsUsecase,
};

struct Sources {
    calls: Rc<RefCell<Vec<&'static str>>>,
}

impl DocumentMutationFingerprintPort for Sources {
    fn fingerprint(
        &self,
        _input: &DocumentMutationFingerprintInput,
    ) -> Result<DocumentMutationFingerprint, DocumentMutationFingerprintPortError> {
        self.calls.borrow_mut().push("fingerprint");
        Ok(DocumentMutationFingerprint::new("attachment-fingerprint").unwrap())
    }
}

impl DocumentVersionIdGenerator for Sources {
    fn generate_version_id(&self) -> Result<VersionId, DocumentRevisionMetadataPortError> {
        self.calls.borrow_mut().push("version");
        Ok(version_id("version-2"))
    }
}

impl DocumentSnapshotRefGenerator for Sources {
    fn generate_snapshot_ref(
        &self,
        _version_id: &VersionId,
    ) -> Result<DocumentSnapshotRef, DocumentRevisionMetadataPortError> {
        self.calls.borrow_mut().push("snapshot");
        Ok(DocumentSnapshotRef::new("snapshot-2").unwrap())
    }
}

impl DocumentRevisionClock for Sources {
    fn now_epoch_ms(&self) -> Result<u64, DocumentRevisionMetadataPortError> {
        self.calls.borrow_mut().push("clock");
        Ok(200)
    }
}

impl DocumentRevisionNumberAllocator for Sources {
    fn allocate_next_revision(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        _expected_current: &DocumentExpectedCurrentVersion,
    ) -> Result<DocumentRevisionNumber, DocumentRevisionMetadataPortError> {
        self.calls.borrow_mut().push("revision");
        Ok(DocumentRevisionNumber::new(2).unwrap())
    }
}

#[derive(Default)]
struct RecordReader {
    records: HashMap<String, VersionRecord>,
}

impl CommittedVersionRecordReader for RecordReader {
    fn get_committed_version_record(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        version_id: &VersionId,
    ) -> Result<Option<VersionRecord>, CommittedVersionRecordReadError> {
        Ok(self.records.get(version_id.as_str()).cloned())
    }
}

struct Pointer {
    current: Option<VersionId>,
    reads: Cell<usize>,
}

impl CurrentDocumentVersionPointerPort for Pointer {
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
        panic!("commit port owns pointer mutation")
    }
}

#[derive(Default)]
struct CommitFake {
    requests: Vec<DocumentRevisionCommitRequest>,
}

impl DocumentRevisionCommitPort for CommitFake {
    fn commit_revision(
        &mut self,
        request: DocumentRevisionCommitRequest,
    ) -> Result<DocumentRevisionCommitResult, DocumentRevisionCommitError> {
        let result = DocumentRevisionCommitResult::new(
            request.record().version_id().clone(),
            request.record().entry().revision_number().unwrap(),
        );
        self.requests.push(request);
        Ok(result)
    }
}

#[derive(Default)]
struct JournalFake {
    record: Option<DocumentOperationJournalRecord>,
}

impl DocumentOperationJournalPort for JournalFake {
    fn load_operation(
        &self,
        _operation_id: &DocumentOperationId,
    ) -> Result<Option<DocumentOperationJournalRecord>, DocumentOperationJournalError> {
        Ok(self.record.clone())
    }

    fn claim_operation(
        &mut self,
        identity: DocumentOperationIdentity,
    ) -> Result<DocumentOperationJournalClaim, DocumentOperationJournalError> {
        self.record = Some(DocumentOperationJournalRecord::claimed(identity));
        Ok(DocumentOperationJournalClaim::Claimed)
    }

    fn complete_operation(
        &mut self,
        _operation_id: &DocumentOperationId,
        result: DocumentRevisionCommitResult,
    ) -> Result<(), DocumentOperationJournalError> {
        self.record = Some(self.record.take().unwrap().complete(result).unwrap());
        Ok(())
    }

    fn fail_operation(
        &mut self,
        _operation_id: &DocumentOperationId,
        failure: DocumentOperationTerminalFailure,
    ) -> Result<(), DocumentOperationJournalError> {
        self.record = Some(self.record.take().unwrap().fail(failure).unwrap());
        Ok(())
    }
}

#[test]
fn fresh_link_preserves_body_and_commits_known_attachment_revision() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let sources = Sources {
        calls: Rc::clone(&calls),
    };
    let reader = reader(known(Vec::new()));
    let pointer = pointer("version-1");
    let mut commit = CommitFake::default();
    let mut journal = JournalFake::default();

    let output = MutateDocumentAttachmentsUsecase::new()
        .execute(
            link_input("operation-1", asset_id('b'), "보고서"),
            &reader,
            &pointer,
            &sources,
            &sources,
            &sources,
            &sources,
            &sources,
            &mut commit,
            &mut journal,
        )
        .expect("link revision");

    assert_eq!(output.kind(), MutateDocumentAttachmentsOutcomeKind::Fresh);
    assert_eq!(output.delta(), AttachmentSnapshotDelta::Linked);
    assert_eq!(output.version_id().as_str(), "version-2");
    assert_eq!(commit.requests.len(), 1);
    let request = &commit.requests[0];
    assert_eq!(request.identity().kind(), DocumentMutationKind::LinkAsset);
    assert_eq!(
        request.record().snapshot().body().as_str(),
        "# 문서\n본문\n"
    );
    let references = request
        .record()
        .snapshot()
        .attachment_state()
        .references()
        .unwrap();
    assert_eq!(references.len(), 1);
    assert_eq!(references[0].label(), "보고서");
    assert_eq!(pointer.reads.get(), 0);
    assert_eq!(
        calls.borrow().as_slice(),
        ["fingerprint", "version", "snapshot", "clock", "revision"]
    );
}

#[test]
fn identical_link_is_no_change_only_when_expected_pointer_is_current() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let sources = Sources {
        calls: Rc::clone(&calls),
    };
    let reader = reader(known(vec![reference('a', "A")]));
    let current_pointer = pointer("version-1");
    let mut commit = CommitFake::default();
    let mut journal = JournalFake::default();

    let output = MutateDocumentAttachmentsUsecase::new()
        .execute(
            link_input("operation-noop", asset_id('a'), "A"),
            &reader,
            &current_pointer,
            &sources,
            &sources,
            &sources,
            &sources,
            &sources,
            &mut commit,
            &mut journal,
        )
        .expect("no change");
    assert_eq!(
        output.kind(),
        MutateDocumentAttachmentsOutcomeKind::NoChange
    );
    assert_eq!(output.version_id().as_str(), "version-1");
    assert!(commit.requests.is_empty());
    assert!(calls.borrow().is_empty());
    assert_eq!(current_pointer.reads.get(), 1);

    let stale_pointer = pointer("version-newer");
    let error = MutateDocumentAttachmentsUsecase::new()
        .execute(
            link_input("operation-stale", asset_id('a'), "A"),
            &reader,
            &stale_pointer,
            &sources,
            &sources,
            &sources,
            &sources,
            &sources,
            &mut commit,
            &mut journal,
        )
        .unwrap_err();
    assert_eq!(error, MutateDocumentAttachmentsError::CommitConflict);
}

#[test]
fn legacy_unknown_is_never_treated_as_empty() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let sources = Sources { calls };
    let reader = reader(AttachmentSnapshotState::legacy_unknown());
    let mut commit = CommitFake::default();
    let mut journal = JournalFake::default();

    let error = MutateDocumentAttachmentsUsecase::new()
        .execute(
            link_input("operation-legacy", asset_id('a'), "A"),
            &reader,
            &pointer("version-1"),
            &sources,
            &sources,
            &sources,
            &sources,
            &sources,
            &mut commit,
            &mut journal,
        )
        .unwrap_err();

    assert_eq!(
        error,
        MutateDocumentAttachmentsError::LegacyBaselineRequired
    );
    assert!(commit.requests.is_empty());
}

#[test]
fn committed_operation_replays_without_new_metadata_or_commit() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let sources = Sources {
        calls: Rc::clone(&calls),
    };
    let reader = reader(known(Vec::new()));
    let identity = DocumentOperationIdentity::new(
        DocumentOperationId::new("operation-replay").unwrap(),
        WorkspaceId::new("workspace-1").unwrap(),
        DocumentId::new("doc-1").unwrap(),
        DocumentMutationKind::LinkAsset,
        DocumentExpectedCurrentVersion::MustMatch(version_id("version-1")),
    )
    .unwrap()
    .with_request_fingerprint(DocumentMutationFingerprint::new("attachment-fingerprint").unwrap());
    let result = DocumentRevisionCommitResult::new(
        version_id("version-2"),
        DocumentRevisionNumber::new(2).unwrap(),
    );
    let mut journal = JournalFake {
        record: Some(
            DocumentOperationJournalRecord::claimed(identity)
                .complete(result)
                .unwrap(),
        ),
    };
    let mut commit = CommitFake::default();

    let output = MutateDocumentAttachmentsUsecase::new()
        .execute(
            link_input("operation-replay", asset_id('a'), "A"),
            &reader,
            &pointer("version-2"),
            &sources,
            &sources,
            &sources,
            &sources,
            &sources,
            &mut commit,
            &mut journal,
        )
        .expect("replay");

    assert_eq!(
        output.kind(),
        MutateDocumentAttachmentsOutcomeKind::Replayed
    );
    assert_eq!(output.version_id().as_str(), "version-2");
    assert!(commit.requests.is_empty());
    assert_eq!(calls.borrow().as_slice(), ["fingerprint"]);
}

#[test]
fn unlink_preserves_other_references_and_uses_unlink_operation_kind() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let sources = Sources { calls };
    let reader = reader(known(vec![reference('a', "A"), reference('b', "B")]));
    let mut commit = CommitFake::default();
    let mut journal = JournalFake::default();

    let output = MutateDocumentAttachmentsUsecase::new()
        .execute(
            MutateDocumentAttachmentsInput::unlink(
                "operation-unlink",
                "workspace-1",
                "doc-1",
                "version-1",
                asset_id('a').as_str(),
                "local-user",
                "첨부 파일 해제",
            ),
            &reader,
            &pointer("version-1"),
            &sources,
            &sources,
            &sources,
            &sources,
            &sources,
            &mut commit,
            &mut journal,
        )
        .expect("unlink revision");

    assert_eq!(output.delta(), AttachmentSnapshotDelta::Unlinked);
    assert_eq!(commit.requests.len(), 1);
    assert_eq!(
        commit.requests[0].identity().kind(),
        DocumentMutationKind::UnlinkAsset
    );
    let references = commit.requests[0]
        .record()
        .snapshot()
        .attachment_state()
        .references()
        .unwrap();
    assert_eq!(references, &[reference('b', "B")]);
}

#[test]
fn reused_operation_id_with_different_identity_is_rejected() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let sources = Sources { calls };
    let reader = reader(known(Vec::new()));
    let different_identity = DocumentOperationIdentity::new(
        DocumentOperationId::new("operation-conflict").unwrap(),
        WorkspaceId::new("workspace-1").unwrap(),
        DocumentId::new("doc-1").unwrap(),
        DocumentMutationKind::UnlinkAsset,
        DocumentExpectedCurrentVersion::MustMatch(version_id("version-1")),
    )
    .unwrap()
    .with_request_fingerprint(DocumentMutationFingerprint::new("different").unwrap());
    let mut journal = JournalFake {
        record: Some(DocumentOperationJournalRecord::claimed(different_identity)),
    };
    let mut commit = CommitFake::default();

    let error = MutateDocumentAttachmentsUsecase::new()
        .execute(
            link_input("operation-conflict", asset_id('a'), "A"),
            &reader,
            &pointer("version-1"),
            &sources,
            &sources,
            &sources,
            &sources,
            &sources,
            &mut commit,
            &mut journal,
        )
        .unwrap_err();

    assert_eq!(error, MutateDocumentAttachmentsError::OperationConflict);
    assert!(commit.requests.is_empty());
}

#[test]
fn matching_claimed_operation_requires_recovery_before_retry() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let sources = Sources { calls };
    let reader = reader(known(Vec::new()));
    let identity = DocumentOperationIdentity::new(
        DocumentOperationId::new("operation-claimed").unwrap(),
        WorkspaceId::new("workspace-1").unwrap(),
        DocumentId::new("doc-1").unwrap(),
        DocumentMutationKind::LinkAsset,
        DocumentExpectedCurrentVersion::MustMatch(version_id("version-1")),
    )
    .unwrap()
    .with_request_fingerprint(DocumentMutationFingerprint::new("attachment-fingerprint").unwrap());
    let mut journal = JournalFake {
        record: Some(DocumentOperationJournalRecord::claimed(identity)),
    };
    let mut commit = CommitFake::default();

    let error = MutateDocumentAttachmentsUsecase::new()
        .execute(
            link_input("operation-claimed", asset_id('a'), "A"),
            &reader,
            &pointer("version-1"),
            &sources,
            &sources,
            &sources,
            &sources,
            &sources,
            &mut commit,
            &mut journal,
        )
        .unwrap_err();

    assert_eq!(error, MutateDocumentAttachmentsError::RecoveryRequired);
    assert!(commit.requests.is_empty());
}

fn reader(state: AttachmentSnapshotState) -> RecordReader {
    let mut reader = RecordReader::default();
    reader
        .records
        .insert("version-1".into(), version_record(state));
    reader
}

fn pointer(version: &str) -> Pointer {
    Pointer {
        current: Some(version_id(version)),
        reads: Cell::new(0),
    }
}

fn link_input(
    operation_id: &str,
    asset_id: AssetId,
    label: &str,
) -> MutateDocumentAttachmentsInput {
    MutateDocumentAttachmentsInput::link(
        operation_id,
        "workspace-1",
        "doc-1",
        "version-1",
        asset_id.as_str(),
        label,
        "local-user",
        "첨부 파일 연결",
    )
}

fn version_record(state: AttachmentSnapshotState) -> VersionRecord {
    let document_id = DocumentId::new("doc-1").unwrap();
    let snapshot_ref = DocumentSnapshotRef::new("snapshot-1").unwrap();
    let entry = VersionEntry::new(
        version_id("version-1"),
        document_id.clone(),
        snapshot_ref.clone(),
        VersionAuthor::new("local-user").unwrap(),
        VersionSummary::new("Create").unwrap(),
    )
    .unwrap()
    .with_created_at_epoch_ms(100)
    .unwrap()
    .with_revision_number(DocumentRevisionNumber::new(1).unwrap())
    .unwrap();
    let snapshot = VersionSnapshot::with_attachment_state(
        document_id,
        snapshot_ref,
        DocumentBody::new("# 문서\n본문\n", DocumentBodyPolicy::new(4096).unwrap()).unwrap(),
        state,
    );
    VersionRecord::new(entry, snapshot).unwrap()
}

fn known(references: Vec<AssetReference>) -> AttachmentSnapshotState {
    AttachmentSnapshotState::known(references).unwrap()
}

fn reference(character: char, label: &str) -> AssetReference {
    AssetReference::new(asset_id(character), label).unwrap()
}

fn asset_id(character: char) -> AssetId {
    AssetId::from_sha256_hex(&std::iter::repeat_n(character, 64).collect::<String>()).unwrap()
}

fn version_id(value: &str) -> VersionId {
    VersionId::new(value).unwrap()
}
