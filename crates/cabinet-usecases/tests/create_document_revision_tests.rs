use std::cell::RefCell;
use std::rc::Rc;

use cabinet_domain::document::{DocumentBodyPolicy, DocumentId};
use cabinet_domain::document_revision::{
    DocumentExpectedCurrentVersion, DocumentMutationFingerprint, DocumentMutationKind,
    DocumentOperationId, DocumentOperationIdentity,
};
use cabinet_domain::version::{DocumentRevisionNumber, DocumentSnapshotRef, VersionId};
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
use cabinet_usecases::create_document_revision::{
    CreateDocumentRevisionError, CreateDocumentRevisionInput, CreateDocumentRevisionUsecase,
};
use cabinet_usecases::document_revision_commit::CommitDocumentRevisionOutcomeKind;

struct Sources {
    calls: Rc<RefCell<Vec<&'static str>>>,
    fingerprint_error: Option<DocumentMutationFingerprintPortError>,
    metadata_error: Option<DocumentRevisionMetadataPortError>,
}

impl DocumentMutationFingerprintPort for Sources {
    fn fingerprint(
        &self,
        input: &DocumentMutationFingerprintInput,
    ) -> Result<DocumentMutationFingerprint, DocumentMutationFingerprintPortError> {
        self.calls.borrow_mut().push("fingerprint");
        if let Some(error) = self.fingerprint_error {
            return Err(error);
        }
        let digest = if input.body().as_str().contains("Changed") {
            "changed"
        } else {
            "first"
        };
        DocumentMutationFingerprint::new(&format!("sha256:{digest}"))
            .map_err(|_| DocumentMutationFingerprintPortError::GenerationUnavailable)
    }
}

impl DocumentVersionIdGenerator for Sources {
    fn generate_version_id(&self) -> Result<VersionId, DocumentRevisionMetadataPortError> {
        self.calls.borrow_mut().push("version");
        if let Some(error) = self.metadata_error {
            return Err(error);
        }
        Ok(VersionId::new("version-1").expect("version"))
    }
}

impl DocumentSnapshotRefGenerator for Sources {
    fn generate_snapshot_ref(
        &self,
        version_id: &VersionId,
    ) -> Result<DocumentSnapshotRef, DocumentRevisionMetadataPortError> {
        self.calls.borrow_mut().push("snapshot");
        DocumentSnapshotRef::new(&format!("snapshot:{}", version_id.as_str()))
            .map_err(|_| DocumentRevisionMetadataPortError::GenerationUnavailable)
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
            &DocumentExpectedCurrentVersion::MustNotExist
        );
        DocumentRevisionNumber::new(1)
            .map_err(|_| DocumentRevisionMetadataPortError::GenerationUnavailable)
    }
}

struct CommitFake {
    calls: Rc<RefCell<Vec<&'static str>>>,
    requests: Vec<DocumentRevisionCommitRequest>,
}

impl DocumentRevisionCommitPort for CommitFake {
    fn commit_revision(
        &mut self,
        request: DocumentRevisionCommitRequest,
    ) -> Result<DocumentRevisionCommitResult, DocumentRevisionCommitError> {
        self.calls.borrow_mut().push("commit");
        let result = DocumentRevisionCommitResult::new(
            request.record().version_id().clone(),
            request
                .record()
                .entry()
                .revision_number()
                .expect("assigned revision"),
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
        if let Some(record) = &self.record {
            return Ok(DocumentOperationJournalClaim::Existing(record.clone()));
        }
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
fn fresh_create_generates_internal_metadata_and_commits_known_empty_snapshot() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let sources = sources(Rc::clone(&calls));
    let mut commit = CommitFake {
        calls: Rc::clone(&calls),
        requests: Vec::new(),
    };
    let mut journal = JournalFake {
        calls: Rc::clone(&calls),
        record: None,
    };

    let output = usecase()
        .execute(
            input("# First\r\nbody"),
            &sources,
            &sources,
            &sources,
            &sources,
            &sources,
            &mut commit,
            &mut journal,
        )
        .expect("create revision");

    assert_eq!(output.kind(), CommitDocumentRevisionOutcomeKind::Fresh);
    assert_eq!(output.version_id().as_str(), "version-1");
    assert_eq!(output.revision_number().value(), 1);
    assert_eq!(commit.requests.len(), 1);
    let request = &commit.requests[0];
    assert_eq!(request.identity().kind(), DocumentMutationKind::Create);
    assert_eq!(
        request.identity().expected_current(),
        &DocumentExpectedCurrentVersion::MustNotExist
    );
    assert_eq!(
        request
            .identity()
            .request_fingerprint()
            .expect("fingerprint")
            .as_str(),
        "sha256:first"
    );
    assert_eq!(request.record().snapshot().body().as_str(), "# First\nbody");
    assert_eq!(
        request.record().snapshot().attachment_state().references(),
        Some(&[][..])
    );
    assert_eq!(request.record().entry().created_at_epoch_ms(), Some(200));
    assert_eq!(
        request.record().entry().revision_number(),
        Some(DocumentRevisionNumber::new(1).expect("revision"))
    );
    assert_eq!(
        *calls.borrow(),
        vec![
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
fn committed_retry_replays_before_metadata_and_changed_payload_conflicts() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let sources = sources(Rc::clone(&calls));
    let committed = DocumentOperationJournalRecord::claimed(identity_for_body("# First\nbody"))
        .complete(DocumentRevisionCommitResult::new(
            VersionId::new("version-existing").expect("version"),
            DocumentRevisionNumber::new(4).expect("revision"),
        ))
        .expect("committed");
    let mut commit = CommitFake {
        calls: Rc::clone(&calls),
        requests: Vec::new(),
    };
    let mut journal = JournalFake {
        calls: Rc::clone(&calls),
        record: Some(committed),
    };

    let output = usecase()
        .execute(
            input("# First\r\nbody"),
            &sources,
            &sources,
            &sources,
            &sources,
            &sources,
            &mut commit,
            &mut journal,
        )
        .expect("replay");
    assert_eq!(output.kind(), CommitDocumentRevisionOutcomeKind::Replayed);
    assert_eq!(output.version_id().as_str(), "version-existing");
    assert_eq!(*calls.borrow(), vec!["fingerprint", "journal.load"]);
    assert!(commit.requests.is_empty());

    calls.borrow_mut().clear();
    let error = usecase()
        .execute(
            input("# Changed\nbody"),
            &sources,
            &sources,
            &sources,
            &sources,
            &sources,
            &mut commit,
            &mut journal,
        )
        .expect_err("changed payload conflict");
    assert_eq!(error, CreateDocumentRevisionError::OperationConflict);
    assert_eq!(*calls.borrow(), vec!["fingerprint", "journal.load"]);
}

#[test]
fn claimed_retry_requires_recovery_before_metadata_generation() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let sources = sources(Rc::clone(&calls));
    let mut commit = CommitFake {
        calls: Rc::clone(&calls),
        requests: Vec::new(),
    };
    let mut journal = JournalFake {
        calls: Rc::clone(&calls),
        record: Some(DocumentOperationJournalRecord::claimed(identity_for_body(
            "# First\nbody",
        ))),
    };

    let error = usecase()
        .execute(
            input("# First\r\nbody"),
            &sources,
            &sources,
            &sources,
            &sources,
            &sources,
            &mut commit,
            &mut journal,
        )
        .expect_err("claimed retry needs recovery");

    assert_eq!(error, CreateDocumentRevisionError::RecoveryRequired);
    assert_eq!(*calls.borrow(), vec!["fingerprint", "journal.load"]);
    assert!(commit.requests.is_empty());
}

#[test]
fn invalid_body_and_fingerprint_failure_stop_before_journal_or_metadata() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut commit = CommitFake {
        calls: Rc::clone(&calls),
        requests: Vec::new(),
    };
    let mut journal = JournalFake {
        calls: Rc::clone(&calls),
        record: None,
    };
    let sources = sources(Rc::clone(&calls));
    let error =
        CreateDocumentRevisionUsecase::new(DocumentBodyPolicy::new(4).expect("body policy"))
            .execute(
                input("too long"),
                &sources,
                &sources,
                &sources,
                &sources,
                &sources,
                &mut commit,
                &mut journal,
            )
            .expect_err("invalid body");
    assert_eq!(error, CreateDocumentRevisionError::InvalidInput);
    assert!(calls.borrow().is_empty());

    let failing_sources = Sources {
        calls: Rc::clone(&calls),
        fingerprint_error: Some(DocumentMutationFingerprintPortError::GenerationUnavailable),
        metadata_error: None,
    };
    let error = usecase()
        .execute(
            input("valid"),
            &failing_sources,
            &failing_sources,
            &failing_sources,
            &failing_sources,
            &failing_sources,
            &mut commit,
            &mut journal,
        )
        .expect_err("fingerprint failure");
    assert_eq!(error, CreateDocumentRevisionError::FingerprintUnavailable);
    assert_eq!(*calls.borrow(), vec!["fingerprint"]);

    calls.borrow_mut().clear();
    let metadata_failure = Sources {
        calls: Rc::clone(&calls),
        fingerprint_error: None,
        metadata_error: Some(DocumentRevisionMetadataPortError::StorageUnavailable),
    };
    let error = usecase()
        .execute(
            input("valid"),
            &metadata_failure,
            &metadata_failure,
            &metadata_failure,
            &metadata_failure,
            &metadata_failure,
            &mut commit,
            &mut journal,
        )
        .expect_err("metadata failure");
    assert_eq!(error, CreateDocumentRevisionError::MetadataUnavailable);
    assert_eq!(
        *calls.borrow(),
        vec!["fingerprint", "journal.load", "version"]
    );
    assert_eq!(
        DocumentMutationFingerprintPortError::GenerationUnavailable.code(),
        "document_mutation_fingerprint.generation_unavailable"
    );
    assert_eq!(
        CreateDocumentRevisionError::RecoveryRequired.code(),
        "create_document_revision.recovery_required"
    );
}

fn usecase() -> CreateDocumentRevisionUsecase {
    CreateDocumentRevisionUsecase::new(DocumentBodyPolicy::new(1024).expect("body policy"))
}

fn input(body: &str) -> CreateDocumentRevisionInput {
    CreateDocumentRevisionInput::new(
        "operation-1",
        "workspace-1",
        "doc-1",
        body,
        "local-user",
        "Create document",
    )
}

fn sources(calls: Rc<RefCell<Vec<&'static str>>>) -> Sources {
    Sources {
        calls,
        fingerprint_error: None,
        metadata_error: None,
    }
}

fn identity_for_body(body: &str) -> DocumentOperationIdentity {
    DocumentOperationIdentity::new(
        DocumentOperationId::new("operation-1").expect("operation"),
        WorkspaceId::new("workspace-1").expect("workspace"),
        DocumentId::new("doc-1").expect("document"),
        DocumentMutationKind::Create,
        DocumentExpectedCurrentVersion::MustNotExist,
    )
    .expect("identity")
    .with_request_fingerprint(
        DocumentMutationFingerprint::new(if body.contains("Changed") {
            "sha256:changed"
        } else {
            "sha256:first"
        })
        .expect("fingerprint"),
    )
}
