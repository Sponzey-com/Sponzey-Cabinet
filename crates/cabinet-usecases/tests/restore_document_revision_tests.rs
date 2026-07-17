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
use cabinet_usecases::restore_document_revision::{
    RestoreDocumentRevisionError, RestoreDocumentRevisionInput, RestoreDocumentRevisionUsecase,
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
        assert_eq!(input.kind(), DocumentMutationKind::Restore);
        DocumentMutationFingerprint::new("sha256:restore")
            .map_err(|_| DocumentMutationFingerprintPortError::GenerationUnavailable)
    }
}

impl DocumentVersionIdGenerator for Sources {
    fn generate_version_id(&self) -> Result<VersionId, DocumentRevisionMetadataPortError> {
        self.calls.borrow_mut().push("version");
        Ok(version_id("version-restored"))
    }
}

impl DocumentSnapshotRefGenerator for Sources {
    fn generate_snapshot_ref(
        &self,
        _version_id: &VersionId,
    ) -> Result<DocumentSnapshotRef, DocumentRevisionMetadataPortError> {
        self.calls.borrow_mut().push("snapshot");
        Ok(DocumentSnapshotRef::new("snapshot-restored").expect("snapshot"))
    }
}

impl DocumentRevisionClock for Sources {
    fn now_epoch_ms(&self) -> Result<u64, DocumentRevisionMetadataPortError> {
        self.calls.borrow_mut().push("clock");
        Ok(300)
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
            &DocumentExpectedCurrentVersion::MustMatch(version_id("version-current"))
        );
        Ok(DocumentRevisionNumber::new(3).expect("revision"))
    }
}

struct SnapshotStore {
    calls: Rc<RefCell<Vec<&'static str>>>,
    snapshot: Option<VersionSnapshot>,
}

impl VersionStore for SnapshotStore {
    fn append_version(
        &mut self,
        _workspace_id: &WorkspaceId,
        _record: VersionRecord,
    ) -> Result<(), VersionStoreError> {
        unreachable!("restore commits through commit port")
    }

    fn get_version_snapshot(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        version_id: &VersionId,
    ) -> Result<Option<VersionSnapshot>, VersionStoreError> {
        self.calls.borrow_mut().push("store.read_target");
        assert_eq!(version_id.as_str(), "version-target");
        Ok(self.snapshot.clone())
    }

    fn list_history(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        _request: HistoryPageRequest,
    ) -> Result<HistoryPage, VersionStoreError> {
        unreachable!("restore reads exact target")
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
fn restore_commits_target_body_and_known_attachments_with_internal_metadata() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let sources = Sources {
        calls: Rc::clone(&calls),
    };
    let store = SnapshotStore {
        calls: Rc::clone(&calls),
        snapshot: Some(target_snapshot(
            AttachmentSnapshotState::known(vec![attachment()]).unwrap(),
        )),
    };
    let mut commit = commit_fake(Rc::clone(&calls));
    let mut journal = JournalFake {
        calls: Rc::clone(&calls),
        record: None,
    };

    let output = usecase()
        .execute(
            input(),
            &store,
            &sources,
            &sources,
            &sources,
            &sources,
            &sources,
            &mut commit,
            &mut journal,
        )
        .expect("restore");

    assert_eq!(output.kind(), CommitDocumentRevisionOutcomeKind::Fresh);
    assert_eq!(output.version_id().as_str(), "version-restored");
    assert_eq!(output.revision_number().value(), 3);
    let request = &commit.requests[0];
    assert_eq!(request.identity().kind(), DocumentMutationKind::Restore);
    assert_eq!(
        request.record().snapshot().body().as_str(),
        "# 과거 제목\n과거 본문"
    );
    assert_eq!(
        request
            .record()
            .snapshot()
            .attachment_state()
            .references()
            .unwrap(),
        &[attachment()]
    );
    assert_eq!(
        *calls.borrow(),
        vec![
            "store.read_target",
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
fn restore_replays_same_operation_and_maps_stale_commit_conflict() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let sources = Sources {
        calls: Rc::clone(&calls),
    };
    let store = SnapshotStore {
        calls: Rc::clone(&calls),
        snapshot: Some(target_snapshot(AttachmentSnapshotState::legacy_unknown())),
    };
    let completed = DocumentOperationJournalRecord::claimed(identity())
        .complete(DocumentRevisionCommitResult::new(
            version_id("version-existing"),
            DocumentRevisionNumber::new(3).unwrap(),
        ))
        .unwrap();
    let mut commit = commit_fake(Rc::clone(&calls));
    let mut journal = JournalFake {
        calls: Rc::clone(&calls),
        record: Some(completed),
    };

    let replayed = usecase()
        .execute(
            input(),
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
    assert_eq!(replayed.version_id().as_str(), "version-existing");
    assert!(commit.requests.is_empty());

    let mut conflict_commit = commit_fake(Rc::clone(&calls));
    conflict_commit.error = Some(DocumentRevisionCommitError::Conflict);
    let mut fresh_journal = JournalFake {
        calls: Rc::clone(&calls),
        record: None,
    };
    let error = usecase()
        .execute(
            input(),
            &store,
            &sources,
            &sources,
            &sources,
            &sources,
            &sources,
            &mut conflict_commit,
            &mut fresh_journal,
        )
        .expect_err("stale conflict");
    assert_eq!(error, RestoreDocumentRevisionError::CommitConflict);
    assert_eq!(
        fresh_journal.record.unwrap().failure(),
        Some(DocumentOperationTerminalFailure::Conflict)
    );
}

#[test]
fn missing_target_stops_before_fingerprint_or_commit() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let sources = Sources {
        calls: Rc::clone(&calls),
    };
    let store = SnapshotStore {
        calls: Rc::clone(&calls),
        snapshot: None,
    };
    let mut commit = commit_fake(Rc::clone(&calls));
    let mut journal = JournalFake {
        calls: Rc::clone(&calls),
        record: None,
    };

    let error = usecase()
        .execute(
            input(),
            &store,
            &sources,
            &sources,
            &sources,
            &sources,
            &sources,
            &mut commit,
            &mut journal,
        )
        .expect_err("missing target");

    assert_eq!(error, RestoreDocumentRevisionError::NotFound);
    assert_eq!(*calls.borrow(), vec!["store.read_target"]);
    assert!(commit.requests.is_empty());
}

fn usecase() -> RestoreDocumentRevisionUsecase {
    RestoreDocumentRevisionUsecase::new(DocumentBodyPolicy::new(4096).unwrap())
}

fn input() -> RestoreDocumentRevisionInput {
    RestoreDocumentRevisionInput::new(
        "operation-restore-1",
        "workspace-1",
        "doc-1",
        "version-target",
        "version-current",
        "local-user",
        "Restore revision",
    )
}

fn target_snapshot(attachment_state: AttachmentSnapshotState) -> VersionSnapshot {
    VersionSnapshot::with_attachment_state(
        DocumentId::new("doc-1").unwrap(),
        DocumentSnapshotRef::new("snapshot-target").unwrap(),
        DocumentBody::new(
            "# 과거 제목\r\n과거 본문",
            DocumentBodyPolicy::new(4096).unwrap(),
        )
        .unwrap(),
        attachment_state,
    )
}

fn attachment() -> AssetReference {
    AssetReference::new(
        AssetId::from_sha256_hex(&"a".repeat(64)).unwrap(),
        "첨부 파일",
    )
    .unwrap()
}

fn identity() -> DocumentOperationIdentity {
    DocumentOperationIdentity::new(
        DocumentOperationId::new("operation-restore-1").unwrap(),
        WorkspaceId::new("workspace-1").unwrap(),
        DocumentId::new("doc-1").unwrap(),
        DocumentMutationKind::Restore,
        DocumentExpectedCurrentVersion::MustMatch(version_id("version-current")),
    )
    .unwrap()
    .with_request_fingerprint(DocumentMutationFingerprint::new("sha256:restore").unwrap())
}

fn version_id(value: &str) -> VersionId {
    VersionId::new(value).unwrap()
}

fn commit_fake(calls: Rc<RefCell<Vec<&'static str>>>) -> CommitFake {
    CommitFake {
        calls,
        requests: Vec::new(),
        error: None,
    }
}
