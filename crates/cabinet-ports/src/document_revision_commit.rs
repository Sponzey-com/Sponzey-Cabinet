use cabinet_domain::document_revision::{DocumentOperationId, DocumentOperationIdentity};
use cabinet_domain::version::{DocumentRevisionNumber, VersionId};

use crate::version_store::VersionRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentRevisionCommitRequest {
    identity: DocumentOperationIdentity,
    record: VersionRecord,
}

impl DocumentRevisionCommitRequest {
    pub fn new(
        identity: DocumentOperationIdentity,
        record: VersionRecord,
    ) -> Result<Self, DocumentRevisionCommitError> {
        if identity.document_id() != record.document_id() {
            return Err(DocumentRevisionCommitError::IdentityMismatch);
        }
        Ok(Self { identity, record })
    }

    pub const fn identity(&self) -> &DocumentOperationIdentity {
        &self.identity
    }

    pub const fn record(&self) -> &VersionRecord {
        &self.record
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentRevisionCommitResult {
    version_id: VersionId,
    revision_number: DocumentRevisionNumber,
}

impl DocumentRevisionCommitResult {
    pub const fn new(version_id: VersionId, revision_number: DocumentRevisionNumber) -> Self {
        Self {
            version_id,
            revision_number,
        }
    }

    pub const fn version_id(&self) -> &VersionId {
        &self.version_id
    }

    pub const fn revision_number(&self) -> DocumentRevisionNumber {
        self.revision_number
    }
}

pub trait DocumentRevisionCommitPort {
    fn commit_revision(
        &mut self,
        request: DocumentRevisionCommitRequest,
    ) -> Result<DocumentRevisionCommitResult, DocumentRevisionCommitError>;
}

pub trait DocumentRevisionRecoveryPort {
    fn recover_revision(
        &mut self,
        identity: DocumentOperationIdentity,
    ) -> Result<DocumentRevisionCommitResult, DocumentRevisionCommitError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentRevisionCommitError {
    IdentityMismatch,
    Conflict,
    StorageUnavailable,
    RecoveryRequired,
}

impl DocumentRevisionCommitError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::IdentityMismatch => "document_revision_commit.identity_mismatch",
            Self::Conflict => "document_revision_commit.conflict",
            Self::StorageUnavailable => "document_revision_commit.storage_unavailable",
            Self::RecoveryRequired => "document_revision_commit.recovery_required",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentOperationJournalState {
    Claimed,
    Committed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentOperationTerminalFailure {
    Conflict,
    InvalidRequest,
}

impl DocumentOperationTerminalFailure {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Conflict => "document_operation.conflict",
            Self::InvalidRequest => "document_operation.invalid_request",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentOperationJournalRecord {
    identity: DocumentOperationIdentity,
    result: Option<DocumentRevisionCommitResult>,
    failure: Option<DocumentOperationTerminalFailure>,
}

impl DocumentOperationJournalRecord {
    pub const fn claimed(identity: DocumentOperationIdentity) -> Self {
        Self {
            identity,
            result: None,
            failure: None,
        }
    }

    pub fn complete(
        mut self,
        result: DocumentRevisionCommitResult,
    ) -> Result<Self, DocumentOperationJournalError> {
        if self.result.is_some() || self.failure.is_some() {
            return Err(DocumentOperationJournalError::AlreadyCompleted);
        }
        self.result = Some(result);
        Ok(self)
    }

    pub fn fail(
        mut self,
        failure: DocumentOperationTerminalFailure,
    ) -> Result<Self, DocumentOperationJournalError> {
        if self.result.is_some() || self.failure.is_some() {
            return Err(DocumentOperationJournalError::AlreadyCompleted);
        }
        self.failure = Some(failure);
        Ok(self)
    }

    pub const fn identity(&self) -> &DocumentOperationIdentity {
        &self.identity
    }

    pub const fn state(&self) -> DocumentOperationJournalState {
        if self.result.is_some() {
            DocumentOperationJournalState::Committed
        } else if self.failure.is_some() {
            DocumentOperationJournalState::Failed
        } else {
            DocumentOperationJournalState::Claimed
        }
    }

    pub const fn result(&self) -> Option<&DocumentRevisionCommitResult> {
        self.result.as_ref()
    }

    pub const fn failure(&self) -> Option<DocumentOperationTerminalFailure> {
        self.failure
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentOperationJournalClaim {
    Claimed,
    Existing(DocumentOperationJournalRecord),
}

pub trait DocumentOperationJournalPort {
    fn load_operation(
        &self,
        operation_id: &DocumentOperationId,
    ) -> Result<Option<DocumentOperationJournalRecord>, DocumentOperationJournalError>;

    fn claim_operation(
        &mut self,
        identity: DocumentOperationIdentity,
    ) -> Result<DocumentOperationJournalClaim, DocumentOperationJournalError>;

    fn complete_operation(
        &mut self,
        operation_id: &DocumentOperationId,
        result: DocumentRevisionCommitResult,
    ) -> Result<(), DocumentOperationJournalError>;

    fn fail_operation(
        &mut self,
        operation_id: &DocumentOperationId,
        failure: DocumentOperationTerminalFailure,
    ) -> Result<(), DocumentOperationJournalError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentOperationJournalError {
    IdentityConflict,
    NotClaimed,
    AlreadyCompleted,
    StorageUnavailable,
    CorruptedJournal,
}

impl DocumentOperationJournalError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::IdentityConflict => "document_operation_journal.identity_conflict",
            Self::NotClaimed => "document_operation_journal.not_claimed",
            Self::AlreadyCompleted => "document_operation_journal.already_completed",
            Self::StorageUnavailable => "document_operation_journal.storage_unavailable",
            Self::CorruptedJournal => "document_operation_journal.corrupted",
        }
    }
}
