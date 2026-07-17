use std::cell::RefCell;
use std::rc::Rc;

use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::document_revision::{
    DocumentExpectedCurrentVersion, DocumentMutationKind, DocumentOperationId,
    DocumentOperationIdentity,
};
use cabinet_domain::version::{
    DocumentRevisionNumber, DocumentSnapshotRef, VersionAuthor, VersionEntry, VersionId,
    VersionSummary,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_revision_commit::{
    DocumentOperationJournalClaim, DocumentOperationJournalError, DocumentOperationJournalPort,
    DocumentOperationJournalRecord, DocumentOperationTerminalFailure, DocumentRevisionCommitError,
    DocumentRevisionCommitPort, DocumentRevisionCommitRequest, DocumentRevisionCommitResult,
};
use cabinet_ports::version_store::{VersionRecord, VersionSnapshot};
use cabinet_usecases::document_revision_commit::{
    CommitDocumentRevisionError, CommitDocumentRevisionOutcomeKind, CommitDocumentRevisionUsecase,
};

struct FakeCommitPort {
    calls: Rc<RefCell<Vec<&'static str>>>,
    result: Result<DocumentRevisionCommitResult, DocumentRevisionCommitError>,
}

impl DocumentRevisionCommitPort for FakeCommitPort {
    fn commit_revision(
        &mut self,
        _request: DocumentRevisionCommitRequest,
    ) -> Result<DocumentRevisionCommitResult, DocumentRevisionCommitError> {
        self.calls.borrow_mut().push("commit");
        self.result.clone()
    }
}

struct FakeJournalPort {
    calls: Rc<RefCell<Vec<&'static str>>>,
    claim: Result<DocumentOperationJournalClaim, DocumentOperationJournalError>,
    complete_error: Option<DocumentOperationJournalError>,
    fail_error: Option<DocumentOperationJournalError>,
}

impl DocumentOperationJournalPort for FakeJournalPort {
    fn load_operation(
        &self,
        _operation_id: &DocumentOperationId,
    ) -> Result<Option<DocumentOperationJournalRecord>, DocumentOperationJournalError> {
        unreachable!("usecase does not need a separate load before atomic claim")
    }

    fn claim_operation(
        &mut self,
        _identity: DocumentOperationIdentity,
    ) -> Result<DocumentOperationJournalClaim, DocumentOperationJournalError> {
        self.calls.borrow_mut().push("claim");
        self.claim.clone()
    }

    fn complete_operation(
        &mut self,
        _operation_id: &DocumentOperationId,
        _result: DocumentRevisionCommitResult,
    ) -> Result<(), DocumentOperationJournalError> {
        self.calls.borrow_mut().push("complete");
        match self.complete_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    fn fail_operation(
        &mut self,
        _operation_id: &DocumentOperationId,
        _failure: DocumentOperationTerminalFailure,
    ) -> Result<(), DocumentOperationJournalError> {
        self.calls.borrow_mut().push("fail");
        match self.fail_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

#[test]
fn fresh_commit_claims_commits_and_completes_in_order() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut commit = successful_commit(Rc::clone(&calls));
    let mut journal = journal(
        Rc::clone(&calls),
        Ok(DocumentOperationJournalClaim::Claimed),
        None,
    );

    let output = CommitDocumentRevisionUsecase::new()
        .execute(request(), &mut commit, &mut journal)
        .expect("fresh commit");

    assert_eq!(output.kind(), CommitDocumentRevisionOutcomeKind::Fresh);
    assert_eq!(output.result().version_id().as_str(), "version-2");
    assert_eq!(*calls.borrow(), vec!["claim", "commit", "complete"]);
}

#[test]
fn committed_duplicate_replays_result_without_new_commit() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let result = commit_result();
    let existing = DocumentOperationJournalRecord::claimed(identity())
        .complete(result.clone())
        .expect("committed record");
    let mut commit = successful_commit(Rc::clone(&calls));
    let mut journal = journal(
        Rc::clone(&calls),
        Ok(DocumentOperationJournalClaim::Existing(existing)),
        None,
    );

    let output = CommitDocumentRevisionUsecase::new()
        .execute(request(), &mut commit, &mut journal)
        .expect("replayed result");

    assert_eq!(output.kind(), CommitDocumentRevisionOutcomeKind::Replayed);
    assert_eq!(output.result(), &result);
    assert_eq!(*calls.borrow(), vec!["claim"]);
}

#[test]
fn claimed_duplicate_requires_recovery_without_new_commit() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut commit = successful_commit(Rc::clone(&calls));
    let mut journal = journal(
        Rc::clone(&calls),
        Ok(DocumentOperationJournalClaim::Existing(
            DocumentOperationJournalRecord::claimed(identity()),
        )),
        None,
    );

    let error = CommitDocumentRevisionUsecase::new()
        .execute(request(), &mut commit, &mut journal)
        .expect_err("claimed duplicate requires recovery");

    assert_eq!(error, CommitDocumentRevisionError::RecoveryRequired);
    assert_eq!(error.code(), "document_revision.recovery_required");
    assert_eq!(*calls.borrow(), vec!["claim"]);
}

#[test]
fn claim_failure_stops_before_commit() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut commit = successful_commit(Rc::clone(&calls));
    let mut journal = journal(
        Rc::clone(&calls),
        Err(DocumentOperationJournalError::IdentityConflict),
        None,
    );

    let error = CommitDocumentRevisionUsecase::new()
        .execute(request(), &mut commit, &mut journal)
        .expect_err("claim conflict");

    assert_eq!(error, CommitDocumentRevisionError::OperationConflict);
    assert_eq!(*calls.borrow(), vec!["claim"]);
}

#[test]
fn commit_failure_does_not_complete_journal() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut commit = FakeCommitPort {
        calls: Rc::clone(&calls),
        result: Err(DocumentRevisionCommitError::Conflict),
    };
    let mut journal = journal(
        Rc::clone(&calls),
        Ok(DocumentOperationJournalClaim::Claimed),
        None,
    );

    let error = CommitDocumentRevisionUsecase::new()
        .execute(request(), &mut commit, &mut journal)
        .expect_err("commit conflict");

    assert_eq!(error, CommitDocumentRevisionError::CommitConflict);
    assert_eq!(*calls.borrow(), vec!["claim", "commit", "fail"]);
}

#[test]
fn failed_duplicate_replays_conflict_without_new_commit() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let existing = DocumentOperationJournalRecord::claimed(identity())
        .fail(DocumentOperationTerminalFailure::Conflict)
        .expect("failed record");
    let mut commit = successful_commit(Rc::clone(&calls));
    let mut journal = journal(
        Rc::clone(&calls),
        Ok(DocumentOperationJournalClaim::Existing(existing)),
        None,
    );

    let error = CommitDocumentRevisionUsecase::new()
        .execute(request(), &mut commit, &mut journal)
        .expect_err("replayed conflict");

    assert_eq!(error, CommitDocumentRevisionError::CommitConflict);
    assert_eq!(*calls.borrow(), vec!["claim"]);
}

#[test]
fn failed_duplicate_replays_invalid_request_without_new_commit() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let existing = DocumentOperationJournalRecord::claimed(identity())
        .fail(DocumentOperationTerminalFailure::InvalidRequest)
        .expect("failed record");
    let mut commit = successful_commit(Rc::clone(&calls));
    let mut journal = journal(
        Rc::clone(&calls),
        Ok(DocumentOperationJournalClaim::Existing(existing)),
        None,
    );

    let error = CommitDocumentRevisionUsecase::new()
        .execute(request(), &mut commit, &mut journal)
        .expect_err("replayed invalid request");

    assert_eq!(error, CommitDocumentRevisionError::InvalidRequest);
    assert_eq!(*calls.borrow(), vec!["claim"]);
}

#[test]
fn terminal_failure_write_error_is_journal_unavailable() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut commit = FakeCommitPort {
        calls: Rc::clone(&calls),
        result: Err(DocumentRevisionCommitError::Conflict),
    };
    let mut journal = FakeJournalPort {
        calls: Rc::clone(&calls),
        claim: Ok(DocumentOperationJournalClaim::Claimed),
        complete_error: None,
        fail_error: Some(DocumentOperationJournalError::StorageUnavailable),
    };

    let error = CommitDocumentRevisionUsecase::new()
        .execute(request(), &mut commit, &mut journal)
        .expect_err("terminal failure persistence failed");

    assert_eq!(error, CommitDocumentRevisionError::JournalUnavailable);
    assert_eq!(*calls.borrow(), vec!["claim", "commit", "fail"]);
}

#[test]
fn journal_storage_failure_is_reported_before_commit() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut commit = successful_commit(Rc::clone(&calls));
    let mut journal = journal(
        Rc::clone(&calls),
        Err(DocumentOperationJournalError::StorageUnavailable),
        None,
    );

    let error = CommitDocumentRevisionUsecase::new()
        .execute(request(), &mut commit, &mut journal)
        .expect_err("journal unavailable");

    assert_eq!(error, CommitDocumentRevisionError::JournalUnavailable);
    assert_eq!(error.code(), "document_revision.journal_unavailable");
    assert_eq!(*calls.borrow(), vec!["claim"]);
}

#[test]
fn commit_storage_failure_is_typed_and_does_not_complete_journal() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut commit = FakeCommitPort {
        calls: Rc::clone(&calls),
        result: Err(DocumentRevisionCommitError::StorageUnavailable),
    };
    let mut journal = journal(
        Rc::clone(&calls),
        Ok(DocumentOperationJournalClaim::Claimed),
        None,
    );

    let error = CommitDocumentRevisionUsecase::new()
        .execute(request(), &mut commit, &mut journal)
        .expect_err("commit unavailable");

    assert_eq!(error, CommitDocumentRevisionError::CommitUnavailable);
    assert_eq!(error.code(), "document_revision.commit_unavailable");
    assert_eq!(*calls.borrow(), vec!["claim", "commit"]);
}

#[test]
fn commit_recovery_required_is_preserved_without_journal_completion() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut commit = FakeCommitPort {
        calls: Rc::clone(&calls),
        result: Err(DocumentRevisionCommitError::RecoveryRequired),
    };
    let mut journal = journal(
        Rc::clone(&calls),
        Ok(DocumentOperationJournalClaim::Claimed),
        None,
    );

    let error = CommitDocumentRevisionUsecase::new()
        .execute(request(), &mut commit, &mut journal)
        .expect_err("commit requires recovery");

    assert_eq!(error, CommitDocumentRevisionError::RecoveryRequired);
    assert_eq!(*calls.borrow(), vec!["claim", "commit"]);
}

#[test]
fn journal_failure_after_commit_is_recovery_required() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut commit = successful_commit(Rc::clone(&calls));
    let mut journal = journal(
        Rc::clone(&calls),
        Ok(DocumentOperationJournalClaim::Claimed),
        Some(DocumentOperationJournalError::StorageUnavailable),
    );

    let error = CommitDocumentRevisionUsecase::new()
        .execute(request(), &mut commit, &mut journal)
        .expect_err("terminal journal write failed after commit");

    assert_eq!(error, CommitDocumentRevisionError::RecoveryRequired);
    assert_eq!(*calls.borrow(), vec!["claim", "commit", "complete"]);
}

fn successful_commit(calls: Rc<RefCell<Vec<&'static str>>>) -> FakeCommitPort {
    FakeCommitPort {
        calls,
        result: Ok(commit_result()),
    }
}

fn journal(
    calls: Rc<RefCell<Vec<&'static str>>>,
    claim: Result<DocumentOperationJournalClaim, DocumentOperationJournalError>,
    complete_error: Option<DocumentOperationJournalError>,
) -> FakeJournalPort {
    FakeJournalPort {
        calls,
        claim,
        complete_error,
        fail_error: None,
    }
}

fn request() -> DocumentRevisionCommitRequest {
    DocumentRevisionCommitRequest::new(identity(), version_record()).expect("request")
}

fn identity() -> DocumentOperationIdentity {
    DocumentOperationIdentity::new(
        DocumentOperationId::new("operation-1").expect("operation id"),
        WorkspaceId::new("workspace-1").expect("workspace id"),
        DocumentId::new("doc-1").expect("document id"),
        DocumentMutationKind::Update,
        DocumentExpectedCurrentVersion::MustMatch(
            VersionId::new("version-1").expect("current version"),
        ),
    )
    .expect("identity")
}

fn version_record() -> VersionRecord {
    VersionRecord::new(
        VersionEntry::new(
            VersionId::new("version-2").expect("version id"),
            DocumentId::new("doc-1").expect("document id"),
            DocumentSnapshotRef::new("snapshot-2").expect("snapshot ref"),
            VersionAuthor::new("writer").expect("author"),
            VersionSummary::new("Updated").expect("summary"),
        )
        .expect("entry"),
        VersionSnapshot::new(
            DocumentId::new("doc-1").expect("document id"),
            DocumentSnapshotRef::new("snapshot-2").expect("snapshot ref"),
            DocumentBody::new("Body", DocumentBodyPolicy::new(1024).expect("policy"))
                .expect("body"),
        ),
    )
    .expect("record")
}

fn commit_result() -> DocumentRevisionCommitResult {
    DocumentRevisionCommitResult::new(
        VersionId::new("version-2").expect("version"),
        DocumentRevisionNumber::new(2).expect("revision"),
    )
}
