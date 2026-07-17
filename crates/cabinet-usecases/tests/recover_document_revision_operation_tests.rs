use std::cell::RefCell;
use std::rc::Rc;

use cabinet_domain::document::DocumentId;
use cabinet_domain::document_revision::{
    DocumentExpectedCurrentVersion, DocumentMutationKind, DocumentOperationId,
    DocumentOperationIdentity,
};
use cabinet_domain::version::{DocumentRevisionNumber, VersionId};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_revision_commit::{
    DocumentOperationJournalClaim, DocumentOperationJournalError, DocumentOperationJournalPort,
    DocumentOperationJournalRecord, DocumentOperationTerminalFailure, DocumentRevisionCommitError,
    DocumentRevisionCommitResult, DocumentRevisionRecoveryPort,
};
use cabinet_usecases::document_revision_recovery::{
    RecoverDocumentRevisionError, RecoverDocumentRevisionOperationUsecase,
    RecoverDocumentRevisionOutcomeKind,
};

struct FakeRecoveryPort {
    calls: Rc<RefCell<Vec<&'static str>>>,
    result: Result<DocumentRevisionCommitResult, DocumentRevisionCommitError>,
}

impl DocumentRevisionRecoveryPort for FakeRecoveryPort {
    fn recover_revision(
        &mut self,
        _identity: DocumentOperationIdentity,
    ) -> Result<DocumentRevisionCommitResult, DocumentRevisionCommitError> {
        self.calls.borrow_mut().push("recover");
        self.result.clone()
    }
}

struct FakeJournal {
    calls: Rc<RefCell<Vec<&'static str>>>,
    record: Option<DocumentOperationJournalRecord>,
}

impl DocumentOperationJournalPort for FakeJournal {
    fn load_operation(
        &self,
        _operation_id: &DocumentOperationId,
    ) -> Result<Option<DocumentOperationJournalRecord>, DocumentOperationJournalError> {
        self.calls.borrow_mut().push("load");
        Ok(self.record.clone())
    }

    fn claim_operation(
        &mut self,
        _identity: DocumentOperationIdentity,
    ) -> Result<DocumentOperationJournalClaim, DocumentOperationJournalError> {
        unreachable!("recovery never claims a new operation")
    }

    fn complete_operation(
        &mut self,
        _operation_id: &DocumentOperationId,
        result: DocumentRevisionCommitResult,
    ) -> Result<(), DocumentOperationJournalError> {
        self.calls.borrow_mut().push("complete");
        self.record = Some(self.record.take().expect("claimed").complete(result)?);
        Ok(())
    }

    fn fail_operation(
        &mut self,
        _operation_id: &DocumentOperationId,
        failure: DocumentOperationTerminalFailure,
    ) -> Result<(), DocumentOperationJournalError> {
        self.calls.borrow_mut().push("fail");
        self.record = Some(self.record.take().expect("claimed").fail(failure)?);
        Ok(())
    }
}

#[test]
fn committed_and_failed_operations_return_terminal_outcomes_without_recovery() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let committed = DocumentOperationJournalRecord::claimed(identity())
        .complete(result())
        .expect("committed");
    let mut committed_journal = journal(Rc::clone(&calls), committed);
    let mut recovery = recovery(Rc::clone(&calls), Ok(result()));

    let output = RecoverDocumentRevisionOperationUsecase::new()
        .execute(operation(), &mut recovery, &mut committed_journal)
        .expect("already committed");
    assert_eq!(
        output.kind(),
        RecoverDocumentRevisionOutcomeKind::AlreadyCommitted
    );
    assert_eq!(*calls.borrow(), vec!["load"]);

    calls.borrow_mut().clear();
    let failed = DocumentOperationJournalRecord::claimed(identity())
        .fail(DocumentOperationTerminalFailure::Conflict)
        .expect("failed");
    let mut failed_journal = journal(Rc::clone(&calls), failed);
    let error = RecoverDocumentRevisionOperationUsecase::new()
        .execute(operation(), &mut recovery, &mut failed_journal)
        .expect_err("terminal failure");
    assert_eq!(error, RecoverDocumentRevisionError::TerminalConflict);
    assert_eq!(*calls.borrow(), vec!["load"]);
}

#[test]
fn claimed_success_recovers_and_completes_journal_in_order() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut journal = journal(
        Rc::clone(&calls),
        DocumentOperationJournalRecord::claimed(identity()),
    );
    let mut recovery = recovery(Rc::clone(&calls), Ok(result()));

    let output = RecoverDocumentRevisionOperationUsecase::new()
        .execute(operation(), &mut recovery, &mut journal)
        .expect("recovered");

    assert_eq!(output.kind(), RecoverDocumentRevisionOutcomeKind::Recovered);
    assert_eq!(*calls.borrow(), vec!["load", "recover", "complete"]);
}

#[test]
fn deterministic_recovery_conflict_becomes_terminal_failed() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut journal = journal(
        Rc::clone(&calls),
        DocumentOperationJournalRecord::claimed(identity()),
    );
    let mut recovery = recovery(
        Rc::clone(&calls),
        Err(DocumentRevisionCommitError::Conflict),
    );

    let error = RecoverDocumentRevisionOperationUsecase::new()
        .execute(operation(), &mut recovery, &mut journal)
        .expect_err("terminal conflict");

    assert_eq!(error, RecoverDocumentRevisionError::TerminalConflict);
    assert_eq!(*calls.borrow(), vec!["load", "recover", "fail"]);
    assert_eq!(
        journal.record.expect("record").failure(),
        Some(DocumentOperationTerminalFailure::Conflict)
    );
}

#[test]
fn unknown_partial_failure_keeps_claimed_for_later_recovery() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut journal = journal(
        Rc::clone(&calls),
        DocumentOperationJournalRecord::claimed(identity()),
    );
    let mut recovery = recovery(
        Rc::clone(&calls),
        Err(DocumentRevisionCommitError::RecoveryRequired),
    );

    let error = RecoverDocumentRevisionOperationUsecase::new()
        .execute(operation(), &mut recovery, &mut journal)
        .expect_err("still requires recovery");

    assert_eq!(error, RecoverDocumentRevisionError::RecoveryRequired);
    assert_eq!(*calls.borrow(), vec!["load", "recover"]);
    assert!(journal.record.expect("record").result().is_none());
}

fn journal(
    calls: Rc<RefCell<Vec<&'static str>>>,
    record: DocumentOperationJournalRecord,
) -> FakeJournal {
    FakeJournal {
        calls,
        record: Some(record),
    }
}

fn recovery(
    calls: Rc<RefCell<Vec<&'static str>>>,
    result: Result<DocumentRevisionCommitResult, DocumentRevisionCommitError>,
) -> FakeRecoveryPort {
    FakeRecoveryPort { calls, result }
}

fn identity() -> DocumentOperationIdentity {
    DocumentOperationIdentity::new(
        operation(),
        WorkspaceId::new("workspace-1").expect("workspace"),
        DocumentId::new("doc-1").expect("document"),
        DocumentMutationKind::Update,
        DocumentExpectedCurrentVersion::MustMatch(VersionId::new("version-1").expect("expected")),
    )
    .expect("identity")
}

fn operation() -> DocumentOperationId {
    DocumentOperationId::new("operation-1").expect("operation")
}

fn result() -> DocumentRevisionCommitResult {
    DocumentRevisionCommitResult::new(
        VersionId::new("version-2").expect("version"),
        DocumentRevisionNumber::new(2).expect("revision"),
    )
}
