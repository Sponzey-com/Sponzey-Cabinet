use cabinet_domain::document_revision::DocumentOperationId;
use cabinet_ports::document_revision_commit::{
    DocumentOperationJournalPort, DocumentOperationJournalState, DocumentOperationTerminalFailure,
    DocumentRevisionCommitError, DocumentRevisionCommitResult, DocumentRevisionRecoveryPort,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoverDocumentRevisionOutcomeKind {
    Recovered,
    AlreadyCommitted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoverDocumentRevisionOutput {
    kind: RecoverDocumentRevisionOutcomeKind,
    result: DocumentRevisionCommitResult,
}

impl RecoverDocumentRevisionOutput {
    const fn new(
        kind: RecoverDocumentRevisionOutcomeKind,
        result: DocumentRevisionCommitResult,
    ) -> Self {
        Self { kind, result }
    }

    pub const fn kind(&self) -> RecoverDocumentRevisionOutcomeKind {
        self.kind
    }

    pub const fn result(&self) -> &DocumentRevisionCommitResult {
        &self.result
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoverDocumentRevisionError {
    NotFound,
    TerminalConflict,
    TerminalInvalidRequest,
    JournalUnavailable,
    RecoveryRequired,
}

impl RecoverDocumentRevisionError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::NotFound => "document_revision_recovery.not_found",
            Self::TerminalConflict => "document_revision_recovery.terminal_conflict",
            Self::TerminalInvalidRequest => "document_revision_recovery.terminal_invalid_request",
            Self::JournalUnavailable => "document_revision_recovery.journal_unavailable",
            Self::RecoveryRequired => "document_revision_recovery.required",
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RecoverDocumentRevisionOperationUsecase;

impl RecoverDocumentRevisionOperationUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute<R, J>(
        &self,
        operation_id: DocumentOperationId,
        recovery: &mut R,
        journal: &mut J,
    ) -> Result<RecoverDocumentRevisionOutput, RecoverDocumentRevisionError>
    where
        R: DocumentRevisionRecoveryPort,
        J: DocumentOperationJournalPort,
    {
        let record = journal
            .load_operation(&operation_id)
            .map_err(|_| RecoverDocumentRevisionError::JournalUnavailable)?
            .ok_or(RecoverDocumentRevisionError::NotFound)?;

        match record.state() {
            DocumentOperationJournalState::Committed => {
                let result = record
                    .result()
                    .cloned()
                    .ok_or(RecoverDocumentRevisionError::JournalUnavailable)?;
                Ok(RecoverDocumentRevisionOutput::new(
                    RecoverDocumentRevisionOutcomeKind::AlreadyCommitted,
                    result,
                ))
            }
            DocumentOperationJournalState::Failed => Err(map_terminal_failure(
                record
                    .failure()
                    .ok_or(RecoverDocumentRevisionError::JournalUnavailable)?,
            )),
            DocumentOperationJournalState::Claimed => {
                let result = match recovery.recover_revision(record.identity().clone()) {
                    Ok(result) => result,
                    Err(DocumentRevisionCommitError::Conflict) => {
                        persist_failure(
                            journal,
                            &operation_id,
                            DocumentOperationTerminalFailure::Conflict,
                        )?;
                        return Err(RecoverDocumentRevisionError::TerminalConflict);
                    }
                    Err(DocumentRevisionCommitError::IdentityMismatch) => {
                        persist_failure(
                            journal,
                            &operation_id,
                            DocumentOperationTerminalFailure::InvalidRequest,
                        )?;
                        return Err(RecoverDocumentRevisionError::TerminalInvalidRequest);
                    }
                    Err(DocumentRevisionCommitError::StorageUnavailable)
                    | Err(DocumentRevisionCommitError::RecoveryRequired) => {
                        return Err(RecoverDocumentRevisionError::RecoveryRequired);
                    }
                };
                journal
                    .complete_operation(&operation_id, result.clone())
                    .map_err(|_| RecoverDocumentRevisionError::RecoveryRequired)?;
                Ok(RecoverDocumentRevisionOutput::new(
                    RecoverDocumentRevisionOutcomeKind::Recovered,
                    result,
                ))
            }
        }
    }
}

fn persist_failure<J: DocumentOperationJournalPort>(
    journal: &mut J,
    operation_id: &DocumentOperationId,
    failure: DocumentOperationTerminalFailure,
) -> Result<(), RecoverDocumentRevisionError> {
    journal
        .fail_operation(operation_id, failure)
        .map_err(|_| RecoverDocumentRevisionError::JournalUnavailable)
}

const fn map_terminal_failure(
    failure: DocumentOperationTerminalFailure,
) -> RecoverDocumentRevisionError {
    match failure {
        DocumentOperationTerminalFailure::Conflict => {
            RecoverDocumentRevisionError::TerminalConflict
        }
        DocumentOperationTerminalFailure::InvalidRequest => {
            RecoverDocumentRevisionError::TerminalInvalidRequest
        }
    }
}
