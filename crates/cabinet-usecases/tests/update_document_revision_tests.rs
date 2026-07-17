use std::cell::RefCell;
use std::rc::Rc;

use cabinet_domain::asset::{AssetId, AssetReference};
use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::document_revision::{
    DocumentExpectedCurrentVersion, DocumentMutationFingerprint, DocumentMutationKind,
    DocumentOperationId, DocumentOperationIdentity,
};
use cabinet_domain::version::{
    AttachmentSnapshotState, DocumentRevisionNumber, DocumentSnapshotRef, VersionId,
};
use cabinet_domain::workspace::WorkspaceId;
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
use cabinet_ports::version_store::{
    HistoryPage, HistoryPageRequest, VersionRecord, VersionSnapshot, VersionStore,
    VersionStoreError,
};
use cabinet_usecases::document_revision_commit::CommitDocumentRevisionOutcomeKind;
use cabinet_usecases::update_document_revision::{
    UpdateDocumentRevisionError, UpdateDocumentRevisionInput, UpdateDocumentRevisionUsecase,
};

struct Sources {
    calls: Rc<RefCell<Vec<&'static str>>>,
}

impl DocumentMutationFingerprintPort for Sources {
    fn fingerprint(
        &self,
        input: &DocumentMutationFingerprintInput,
    ) -> Result<DocumentMutationFingerprint, DocumentMutationFingerprintPortError> {
        self.calls.borrow_mut().push("fingerprint");
        let digest = if input.body().as_str().contains("changed") {
            "sha256:changed"
        } else {
            "sha256:original"
        };
        DocumentMutationFingerprint::new(digest)
            .map_err(|_| DocumentMutationFingerprintPortError::GenerationUnavailable)
    }
}

impl DocumentVersionIdGenerator for Sources {
    fn generate_version_id(&self) -> Result<VersionId, DocumentRevisionMetadataPortError> {
        self.calls.borrow_mut().push("version");
        Ok(VersionId::new("version-2").expect("version"))
    }
}

impl DocumentSnapshotRefGenerator for Sources {
    fn generate_snapshot_ref(
        &self,
        _version_id: &VersionId,
    ) -> Result<DocumentSnapshotRef, DocumentRevisionMetadataPortError> {
        self.calls.borrow_mut().push("snapshot");
        Ok(DocumentSnapshotRef::new("snapshot-2").expect("snapshot"))
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
        expected_current: &DocumentExpectedCurrentVersion,
    ) -> Result<DocumentRevisionNumber, DocumentRevisionMetadataPortError> {
        self.calls.borrow_mut().push("revision");
        assert_eq!(
            expected_current,
            &DocumentExpectedCurrentVersion::MustMatch(version_id("version-1"))
        );
        Ok(DocumentRevisionNumber::new(2).expect("revision"))
    }
}

struct SnapshotStore {
    calls: Rc<RefCell<Vec<&'static str>>>,
    snapshot: Option<VersionSnapshot>,
    fail: bool,
}

impl VersionStore for SnapshotStore {
    fn append_version(
        &mut self,
        _workspace_id: &WorkspaceId,
        _record: VersionRecord,
    ) -> Result<(), VersionStoreError> {
        unreachable!("update usecase commits through commit port")
    }

    fn get_version_snapshot(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        _version_id: &VersionId,
    ) -> Result<Option<VersionSnapshot>, VersionStoreError> {
        self.calls.borrow_mut().push("store.read");
        if self.fail {
            return Err(VersionStoreError::StorageUnavailable);
        }
        Ok(self.snapshot.clone())
    }

    fn list_history(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        _request: HistoryPageRequest,
    ) -> Result<HistoryPage, VersionStoreError> {
        unreachable!("update usecase reads exact snapshot")
    }
}

struct CommitFake {
    calls: Rc<RefCell<Vec<&'static str>>>,
    requests: Vec<DocumentRevisionCommitRequest>,
    error: Option<DocumentRevisionCommitError>,
}

impl DocumentRevisionCommitPort for CommitFake {
    fn commit_revision(
        &mut self,
        request: DocumentRevisionCommitRequest,
    ) -> Result<DocumentRevisionCommitResult, DocumentRevisionCommitError> {
        self.calls.borrow_mut().push("commit");
        if let Some(error) = self.error {
            return Err(error);
        }
        let result = DocumentRevisionCommitResult::new(
            request.record().version_id().clone(),
            request
                .record()
                .entry()
                .revision_number()
                .expect("revision"),
        );
        self.requests.push(request);
        Ok(result)
    }
}

struct JournalFake {
    calls: Rc<RefCell<Vec<&'static str>>>,
    record: Option<DocumentOperationJournalRecord>,
}

impl DocumentOperationJournalPort for JournalFake {
    fn load_operation(
        &self,
        _operation_id: &DocumentOperationId,
    ) -> Result<Option<DocumentOperationJournalRecord>, DocumentOperationJournalError> {
        self.calls.borrow_mut().push("journal.load");
        Ok(self.record.clone())
    }

    fn claim_operation(
        &mut self,
        identity: DocumentOperationIdentity,
    ) -> Result<DocumentOperationJournalClaim, DocumentOperationJournalError> {
        self.calls.borrow_mut().push("journal.claim");
        self.record = Some(DocumentOperationJournalRecord::claimed(identity));
        Ok(DocumentOperationJournalClaim::Claimed)
    }

    fn complete_operation(
        &mut self,
        _operation_id: &DocumentOperationId,
        result: DocumentRevisionCommitResult,
    ) -> Result<(), DocumentOperationJournalError> {
        self.calls.borrow_mut().push("journal.complete");
        self.record = Some(
            self.record
                .take()
                .expect("claimed")
                .complete(result)
                .expect("complete"),
        );
        Ok(())
    }

    fn fail_operation(
        &mut self,
        _operation_id: &DocumentOperationId,
        failure: DocumentOperationTerminalFailure,
    ) -> Result<(), DocumentOperationJournalError> {
        self.calls.borrow_mut().push("journal.fail");
        self.record = Some(
            self.record
                .take()
                .expect("claimed")
                .fail(failure)
                .expect("fail"),
        );
        Ok(())
    }
}

#[test]
fn fresh_update_preserves_known_attachments_and_commits_internal_metadata() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let sources = Sources {
        calls: Rc::clone(&calls),
    };
    let store = SnapshotStore {
        calls: Rc::clone(&calls),
        snapshot: Some(snapshot(
            AttachmentSnapshotState::known(vec![asset_reference()]).expect("known attachments"),
        )),
        fail: false,
    };
    let mut commit = new_commit(Rc::clone(&calls));
    let mut journal = new_journal(Rc::clone(&calls), None);

    let output = usecase()
        .execute(
            input("# Updated\r\nbody"),
            &store,
            &sources,
            &sources,
            &sources,
            &sources,
            &sources,
            &mut commit,
            &mut journal,
        )
        .expect("update");

    assert_eq!(output.kind(), CommitDocumentRevisionOutcomeKind::Fresh);
    assert_eq!(output.version_id().as_str(), "version-2");
    assert_eq!(output.revision_number().value(), 2);
    let request = &commit.requests[0];
    assert_eq!(request.identity().kind(), DocumentMutationKind::Update);
    assert_eq!(
        request.record().snapshot().body().as_str(),
        "# Updated\nbody"
    );
    assert_eq!(
        request
            .record()
            .snapshot()
            .attachment_state()
            .references()
            .expect("known"),
        &[asset_reference()]
    );
    assert_eq!(request.record().entry().created_at_epoch_ms(), Some(200));
    assert_eq!(
        *calls.borrow(),
        vec![
            "store.read",
            "fingerprint",
            "journal.load",
            "version",
            "snapshot",
            "clock",
            "revision",
            "journal.claim",
            "commit",
            "journal.complete"
        ]
    );
}

#[test]
fn legacy_attachment_state_is_preserved_and_missing_or_failed_read_stops_early() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let sources = Sources {
        calls: Rc::clone(&calls),
    };
    let legacy_store = SnapshotStore {
        calls: Rc::clone(&calls),
        snapshot: Some(snapshot(AttachmentSnapshotState::legacy_unknown())),
        fail: false,
    };
    let mut commit = new_commit(Rc::clone(&calls));
    let mut journal = new_journal(Rc::clone(&calls), None);
    usecase()
        .execute(
            input("updated"),
            &legacy_store,
            &sources,
            &sources,
            &sources,
            &sources,
            &sources,
            &mut commit,
            &mut journal,
        )
        .expect("legacy update");
    assert!(
        commit.requests[0]
            .record()
            .snapshot()
            .attachment_state()
            .is_legacy_unknown()
    );

    for (store, expected) in [
        (
            SnapshotStore {
                calls: Rc::clone(&calls),
                snapshot: None,
                fail: false,
            },
            UpdateDocumentRevisionError::NotFound,
        ),
        (
            SnapshotStore {
                calls: Rc::clone(&calls),
                snapshot: None,
                fail: true,
            },
            UpdateDocumentRevisionError::StorageUnavailable,
        ),
    ] {
        calls.borrow_mut().clear();
        let mut untouched_commit = new_commit(Rc::clone(&calls));
        let mut untouched_journal = new_journal(Rc::clone(&calls), None);
        assert_eq!(
            usecase()
                .execute(
                    input("updated"),
                    &store,
                    &sources,
                    &sources,
                    &sources,
                    &sources,
                    &sources,
                    &mut untouched_commit,
                    &mut untouched_journal,
                )
                .expect_err("read failure"),
            expected
        );
        assert_eq!(*calls.borrow(), vec!["store.read"]);
    }
}

#[test]
fn committed_and_claimed_retry_short_circuit_before_metadata() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let sources = Sources {
        calls: Rc::clone(&calls),
    };
    let store = SnapshotStore {
        calls: Rc::clone(&calls),
        snapshot: Some(snapshot(
            AttachmentSnapshotState::known(Vec::new()).expect("known"),
        )),
        fail: false,
    };
    let identity = identity("sha256:original");
    let committed = DocumentOperationJournalRecord::claimed(identity.clone())
        .complete(DocumentRevisionCommitResult::new(
            version_id("version-existing"),
            DocumentRevisionNumber::new(5).expect("revision"),
        ))
        .expect("complete");
    let mut commit = new_commit(Rc::clone(&calls));
    let mut journal = new_journal(Rc::clone(&calls), Some(committed));

    let replayed = usecase()
        .execute(
            input("original"),
            &store,
            &sources,
            &sources,
            &sources,
            &sources,
            &sources,
            &mut commit,
            &mut journal,
        )
        .expect("replay");
    assert_eq!(replayed.kind(), CommitDocumentRevisionOutcomeKind::Replayed);
    assert_eq!(
        *calls.borrow(),
        vec!["store.read", "fingerprint", "journal.load"]
    );

    calls.borrow_mut().clear();
    let error = usecase()
        .execute(
            input("changed"),
            &store,
            &sources,
            &sources,
            &sources,
            &sources,
            &sources,
            &mut commit,
            &mut journal,
        )
        .expect_err("changed payload");
    assert_eq!(error, UpdateDocumentRevisionError::OperationConflict);

    calls.borrow_mut().clear();
    let mut claimed_journal = new_journal(
        Rc::clone(&calls),
        Some(DocumentOperationJournalRecord::claimed(identity)),
    );
    let error = usecase()
        .execute(
            input("original"),
            &store,
            &sources,
            &sources,
            &sources,
            &sources,
            &sources,
            &mut commit,
            &mut claimed_journal,
        )
        .expect_err("claimed");
    assert_eq!(error, UpdateDocumentRevisionError::RecoveryRequired);
}

#[test]
fn commit_conflict_is_terminal_and_typed() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let sources = Sources {
        calls: Rc::clone(&calls),
    };
    let store = SnapshotStore {
        calls: Rc::clone(&calls),
        snapshot: Some(snapshot(
            AttachmentSnapshotState::known(Vec::new()).expect("known"),
        )),
        fail: false,
    };
    let mut commit = new_commit(Rc::clone(&calls));
    commit.error = Some(DocumentRevisionCommitError::Conflict);
    let mut journal = new_journal(Rc::clone(&calls), None);

    let error = usecase()
        .execute(
            input("original"),
            &store,
            &sources,
            &sources,
            &sources,
            &sources,
            &sources,
            &mut commit,
            &mut journal,
        )
        .expect_err("stale conflict");
    assert_eq!(error, UpdateDocumentRevisionError::CommitConflict);
    assert_eq!(
        journal.record.expect("terminal").failure(),
        Some(DocumentOperationTerminalFailure::Conflict)
    );
}

fn usecase() -> UpdateDocumentRevisionUsecase {
    UpdateDocumentRevisionUsecase::new(DocumentBodyPolicy::new(1024).expect("policy"))
}

fn input(body: &str) -> UpdateDocumentRevisionInput {
    UpdateDocumentRevisionInput::new(
        "operation-1",
        "workspace-1",
        "doc-1",
        "version-1",
        body,
        "local-user",
        "Update document",
    )
}

fn snapshot(attachment_state: AttachmentSnapshotState) -> VersionSnapshot {
    VersionSnapshot::with_attachment_state(
        DocumentId::new("doc-1").expect("document"),
        DocumentSnapshotRef::new("snapshot-1").expect("snapshot"),
        DocumentBody::new("old body", DocumentBodyPolicy::new(1024).expect("policy"))
            .expect("body"),
        attachment_state,
    )
}

fn asset_reference() -> AssetReference {
    AssetReference::new(
        AssetId::from_sha256_hex(&"a".repeat(64)).expect("asset"),
        "Attachment",
    )
    .expect("reference")
}

fn identity(fingerprint: &str) -> DocumentOperationIdentity {
    DocumentOperationIdentity::new(
        DocumentOperationId::new("operation-1").expect("operation"),
        WorkspaceId::new("workspace-1").expect("workspace"),
        DocumentId::new("doc-1").expect("document"),
        DocumentMutationKind::Update,
        DocumentExpectedCurrentVersion::MustMatch(version_id("version-1")),
    )
    .expect("identity")
    .with_request_fingerprint(DocumentMutationFingerprint::new(fingerprint).expect("fingerprint"))
}

fn version_id(value: &str) -> VersionId {
    VersionId::new(value).expect("version")
}

fn new_commit(calls: Rc<RefCell<Vec<&'static str>>>) -> CommitFake {
    CommitFake {
        calls,
        requests: Vec::new(),
        error: None,
    }
}

fn new_journal(
    calls: Rc<RefCell<Vec<&'static str>>>,
    record: Option<DocumentOperationJournalRecord>,
) -> JournalFake {
    JournalFake { calls, record }
}
