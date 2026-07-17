use cabinet_ports::document_revision_commit::{
    DocumentOperationJournalClaim, DocumentOperationJournalError, DocumentOperationJournalPort,
    DocumentOperationTerminalFailure, DocumentRevisionCommitError, DocumentRevisionCommitPort,
    DocumentRevisionCommitRequest, DocumentRevisionCommitResult,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitDocumentRevisionOutcomeKind {
    Fresh,
    Replayed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitDocumentRevisionOutput {
    kind: CommitDocumentRevisionOutcomeKind,
    result: DocumentRevisionCommitResult,
}

impl CommitDocumentRevisionOutput {
    const fn new(
        kind: CommitDocumentRevisionOutcomeKind,
        result: DocumentRevisionCommitResult,
    ) -> Self {
        Self { kind, result }
    }

    pub const fn kind(&self) -> CommitDocumentRevisionOutcomeKind {
        self.kind
    }

    pub const fn result(&self) -> &DocumentRevisionCommitResult {
        &self.result
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitDocumentRevisionError {
    InvalidRequest,
    OperationConflict,
    JournalUnavailable,
    CommitConflict,
    CommitUnavailable,
    RecoveryRequired,
}

impl CommitDocumentRevisionError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidRequest => "document_revision.invalid_request",
            Self::OperationConflict => "document_revision.operation_conflict",
            Self::JournalUnavailable => "document_revision.journal_unavailable",
            Self::CommitConflict => "document_revision.commit_conflict",
            Self::CommitUnavailable => "document_revision.commit_unavailable",
            Self::RecoveryRequired => "document_revision.recovery_required",
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CommitDocumentRevisionUsecase;

impl CommitDocumentRevisionUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute<C, J>(
        &self,
        request: DocumentRevisionCommitRequest,
        commit_port: &mut C,
        journal_port: &mut J,
    ) -> Result<CommitDocumentRevisionOutput, CommitDocumentRevisionError>
    where
        C: DocumentRevisionCommitPort,
        J: DocumentOperationJournalPort,
    {
        let identity = request.identity().clone();
        let operation_id = identity.operation_id().clone();

        match journal_port
            .claim_operation(identity.clone())
            .map_err(map_claim_error)?
        {
            DocumentOperationJournalClaim::Existing(record) => {
                if record.identity() != &identity {
                    return Err(CommitDocumentRevisionError::OperationConflict);
                }

                if let Some(failure) = record.failure() {
                    return Err(map_terminal_failure(failure));
                }
                let result = record
                    .result()
                    .cloned()
                    .ok_or(CommitDocumentRevisionError::RecoveryRequired)?;
                Ok(CommitDocumentRevisionOutput::new(
                    CommitDocumentRevisionOutcomeKind::Replayed,
                    result,
                ))
            }
            DocumentOperationJournalClaim::Claimed => {
                let result = match commit_port.commit_revision(request) {
                    Ok(result) => result,
                    Err(DocumentRevisionCommitError::IdentityMismatch) => {
                        persist_terminal_failure(
                            journal_port,
                            &operation_id,
                            DocumentOperationTerminalFailure::InvalidRequest,
                        )?;
                        return Err(CommitDocumentRevisionError::InvalidRequest);
                    }
                    Err(DocumentRevisionCommitError::Conflict) => {
                        persist_terminal_failure(
                            journal_port,
                            &operation_id,
                            DocumentOperationTerminalFailure::Conflict,
                        )?;
                        return Err(CommitDocumentRevisionError::CommitConflict);
                    }
                    Err(error) => return Err(map_commit_error(error)),
                };

                journal_port
                    .complete_operation(&operation_id, result.clone())
                    .map_err(|_| CommitDocumentRevisionError::RecoveryRequired)?;

                Ok(CommitDocumentRevisionOutput::new(
                    CommitDocumentRevisionOutcomeKind::Fresh,
                    result,
                ))
            }
        }
    }
}

fn persist_terminal_failure<J: DocumentOperationJournalPort>(
    journal_port: &mut J,
    operation_id: &cabinet_domain::document_revision::DocumentOperationId,
    failure: DocumentOperationTerminalFailure,
) -> Result<(), CommitDocumentRevisionError> {
    journal_port
        .fail_operation(operation_id, failure)
        .map_err(|_| CommitDocumentRevisionError::JournalUnavailable)
}

const fn map_terminal_failure(
    failure: DocumentOperationTerminalFailure,
) -> CommitDocumentRevisionError {
    match failure {
        DocumentOperationTerminalFailure::Conflict => CommitDocumentRevisionError::CommitConflict,
        DocumentOperationTerminalFailure::InvalidRequest => {
            CommitDocumentRevisionError::InvalidRequest
        }
    }
}

fn map_claim_error(error: DocumentOperationJournalError) -> CommitDocumentRevisionError {
    match error {
        DocumentOperationJournalError::IdentityConflict => {
            CommitDocumentRevisionError::OperationConflict
        }
        DocumentOperationJournalError::StorageUnavailable
        | DocumentOperationJournalError::CorruptedJournal => {
            CommitDocumentRevisionError::JournalUnavailable
        }
        DocumentOperationJournalError::NotClaimed
        | DocumentOperationJournalError::AlreadyCompleted => {
            CommitDocumentRevisionError::JournalUnavailable
        }
    }
}

fn map_commit_error(error: DocumentRevisionCommitError) -> CommitDocumentRevisionError {
    match error {
        DocumentRevisionCommitError::IdentityMismatch => {
            CommitDocumentRevisionError::InvalidRequest
        }
        DocumentRevisionCommitError::Conflict => CommitDocumentRevisionError::CommitConflict,
        DocumentRevisionCommitError::StorageUnavailable => {
            CommitDocumentRevisionError::CommitUnavailable
        }
        DocumentRevisionCommitError::RecoveryRequired => {
            CommitDocumentRevisionError::RecoveryRequired
        }
    }
}
