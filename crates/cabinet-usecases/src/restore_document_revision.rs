use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::document_revision::{
    DocumentExpectedCurrentVersion, DocumentMutationKind, DocumentOperationId,
    DocumentOperationIdentity,
};
use cabinet_domain::version::{
    DocumentRevisionNumber, VersionAuthor, VersionEntry, VersionId, VersionSummary,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_mutation_fingerprint::{
    DocumentMutationFingerprintInput, DocumentMutationFingerprintPort,
};
use cabinet_ports::document_revision_commit::{
    DocumentOperationJournalError, DocumentOperationJournalPort, DocumentOperationJournalRecord,
    DocumentOperationJournalState, DocumentOperationTerminalFailure, DocumentRevisionCommitPort,
    DocumentRevisionCommitRequest,
};
use cabinet_ports::document_revision_metadata::{
    DocumentRevisionClock, DocumentRevisionNumberAllocator, DocumentSnapshotRefGenerator,
    DocumentVersionIdGenerator,
};
use cabinet_ports::version_store::{VersionRecord, VersionSnapshot, VersionStore};

use crate::document_revision_commit::{
    CommitDocumentRevisionError, CommitDocumentRevisionOutcomeKind, CommitDocumentRevisionUsecase,
};
use crate::document_revision_metadata::{
    GenerateDocumentRevisionMetadataError, GenerateDocumentRevisionMetadataInput,
    GenerateDocumentRevisionMetadataUsecase,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreDocumentRevisionInput {
    operation_id: String,
    workspace_id: String,
    document_id: String,
    target_version_id: String,
    expected_current_version_id: String,
    author: String,
    summary: String,
}

impl RestoreDocumentRevisionInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        operation_id: &str,
        workspace_id: &str,
        document_id: &str,
        target_version_id: &str,
        expected_current_version_id: &str,
        author: &str,
        summary: &str,
    ) -> Self {
        Self {
            operation_id: operation_id.to_string(),
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            target_version_id: target_version_id.to_string(),
            expected_current_version_id: expected_current_version_id.to_string(),
            author: author.to_string(),
            summary: summary.to_string(),
        }
    }

    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    pub fn document_id(&self) -> &str {
        &self.document_id
    }

    pub fn target_version_id(&self) -> &str {
        &self.target_version_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreDocumentRevisionOutput {
    kind: CommitDocumentRevisionOutcomeKind,
    version_id: VersionId,
    revision_number: DocumentRevisionNumber,
}

impl RestoreDocumentRevisionOutput {
    pub const fn kind(&self) -> CommitDocumentRevisionOutcomeKind {
        self.kind
    }

    pub const fn version_id(&self) -> &VersionId {
        &self.version_id
    }

    pub const fn revision_number(&self) -> DocumentRevisionNumber {
        self.revision_number
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreDocumentRevisionError {
    InvalidInput,
    NotFound,
    StorageUnavailable,
    FingerprintUnavailable,
    MetadataUnavailable,
    OperationConflict,
    CommitConflict,
    JournalUnavailable,
    CommitUnavailable,
    MissingDependency,
    RecoveryRequired,
}

impl RestoreDocumentRevisionError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "restore_document_revision.invalid_input",
            Self::NotFound => "restore_document_revision.not_found",
            Self::StorageUnavailable => "restore_document_revision.storage_unavailable",
            Self::FingerprintUnavailable => "restore_document_revision.fingerprint_unavailable",
            Self::MetadataUnavailable => "restore_document_revision.metadata_unavailable",
            Self::OperationConflict => "restore_document_revision.operation_conflict",
            Self::CommitConflict => "restore_document_revision.commit_conflict",
            Self::JournalUnavailable => "restore_document_revision.journal_unavailable",
            Self::CommitUnavailable => "restore_document_revision.commit_unavailable",
            Self::MissingDependency => "restore_document_revision.missing_dependency",
            Self::RecoveryRequired => "restore_document_revision.recovery_required",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestoreDocumentRevisionUsecase {
    body_policy: DocumentBodyPolicy,
}

impl RestoreDocumentRevisionUsecase {
    pub const fn new(body_policy: DocumentBodyPolicy) -> Self {
        Self { body_policy }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute<R, F, V, S, C, A, M, J>(
        &self,
        input: RestoreDocumentRevisionInput,
        version_store: &R,
        fingerprint_port: &F,
        version_generator: &V,
        snapshot_generator: &S,
        clock: &C,
        revision_allocator: &A,
        commit_port: &mut M,
        journal_port: &mut J,
    ) -> Result<RestoreDocumentRevisionOutput, RestoreDocumentRevisionError>
    where
        R: VersionStore,
        F: DocumentMutationFingerprintPort,
        V: DocumentVersionIdGenerator,
        S: DocumentSnapshotRefGenerator,
        C: DocumentRevisionClock,
        A: DocumentRevisionNumberAllocator,
        M: DocumentRevisionCommitPort,
        J: DocumentOperationJournalPort,
    {
        let parsed = ParsedRestoreDocumentRevision::parse(input)?;
        let target_snapshot = version_store
            .get_version_snapshot(
                &parsed.workspace_id,
                &parsed.document_id,
                &parsed.target_version_id,
            )
            .map_err(|_| RestoreDocumentRevisionError::StorageUnavailable)?
            .ok_or(RestoreDocumentRevisionError::NotFound)?;
        if target_snapshot.document_id() != &parsed.document_id {
            return Err(RestoreDocumentRevisionError::StorageUnavailable);
        }
        let body = DocumentBody::new(target_snapshot.body().as_str(), self.body_policy)
            .map_err(|_| RestoreDocumentRevisionError::InvalidInput)?;
        let attachment_state = target_snapshot.attachment_state().clone();
        let expected_current =
            DocumentExpectedCurrentVersion::MustMatch(parsed.expected_current_version.clone());
        let fingerprint = fingerprint_port
            .fingerprint(&DocumentMutationFingerprintInput::new(
                DocumentMutationKind::Restore,
                parsed.workspace_id.clone(),
                parsed.document_id.clone(),
                expected_current.clone(),
                body.clone(),
                parsed.author.clone(),
                parsed.summary.clone(),
                attachment_state.clone(),
            ))
            .map_err(|_| RestoreDocumentRevisionError::FingerprintUnavailable)?;
        let identity = DocumentOperationIdentity::new(
            parsed.operation_id,
            parsed.workspace_id.clone(),
            parsed.document_id.clone(),
            DocumentMutationKind::Restore,
            expected_current.clone(),
        )
        .map_err(|_| RestoreDocumentRevisionError::InvalidInput)?
        .with_request_fingerprint(fingerprint);

        if let Some(existing) = journal_port
            .load_operation(identity.operation_id())
            .map_err(map_journal_error)?
        {
            return replay_existing(identity, existing);
        }

        let metadata = GenerateDocumentRevisionMetadataUsecase::new()
            .execute(
                GenerateDocumentRevisionMetadataInput::new(
                    parsed.workspace_id,
                    parsed.document_id.clone(),
                    expected_current,
                ),
                version_generator,
                snapshot_generator,
                clock,
                revision_allocator,
            )
            .map_err(map_metadata_error)?;
        let entry = VersionEntry::new(
            metadata.version_id().clone(),
            parsed.document_id.clone(),
            metadata.snapshot_ref().clone(),
            parsed.author,
            parsed.summary,
        )
        .and_then(|entry| entry.with_created_at_epoch_ms(metadata.created_at_epoch_ms()))
        .and_then(|entry| entry.with_revision_number(metadata.revision_number()))
        .map_err(|_| RestoreDocumentRevisionError::InvalidInput)?;
        let snapshot = VersionSnapshot::with_attachment_state(
            parsed.document_id,
            metadata.snapshot_ref().clone(),
            body,
            attachment_state,
        );
        let record = VersionRecord::new(entry, snapshot)
            .map_err(|_| RestoreDocumentRevisionError::InvalidInput)?;
        let request = DocumentRevisionCommitRequest::new(identity, record)
            .map_err(|_| RestoreDocumentRevisionError::InvalidInput)?;
        let committed = CommitDocumentRevisionUsecase::new()
            .execute(request, commit_port, journal_port)
            .map_err(map_commit_error)?;

        Ok(RestoreDocumentRevisionOutput {
            kind: committed.kind(),
            version_id: committed.result().version_id().clone(),
            revision_number: committed.result().revision_number(),
        })
    }
}

struct ParsedRestoreDocumentRevision {
    operation_id: DocumentOperationId,
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    target_version_id: VersionId,
    expected_current_version: VersionId,
    author: VersionAuthor,
    summary: VersionSummary,
}

impl ParsedRestoreDocumentRevision {
    fn parse(input: RestoreDocumentRevisionInput) -> Result<Self, RestoreDocumentRevisionError> {
        Ok(Self {
            operation_id: DocumentOperationId::new(&input.operation_id)
                .map_err(|_| RestoreDocumentRevisionError::InvalidInput)?,
            workspace_id: WorkspaceId::new(&input.workspace_id)
                .map_err(|_| RestoreDocumentRevisionError::InvalidInput)?,
            document_id: DocumentId::new(&input.document_id)
                .map_err(|_| RestoreDocumentRevisionError::InvalidInput)?,
            target_version_id: VersionId::new(&input.target_version_id)
                .map_err(|_| RestoreDocumentRevisionError::InvalidInput)?,
            expected_current_version: VersionId::new(&input.expected_current_version_id)
                .map_err(|_| RestoreDocumentRevisionError::InvalidInput)?,
            author: VersionAuthor::new(&input.author)
                .map_err(|_| RestoreDocumentRevisionError::InvalidInput)?,
            summary: VersionSummary::new(&input.summary)
                .map_err(|_| RestoreDocumentRevisionError::InvalidInput)?,
        })
    }
}

fn replay_existing(
    identity: DocumentOperationIdentity,
    existing: DocumentOperationJournalRecord,
) -> Result<RestoreDocumentRevisionOutput, RestoreDocumentRevisionError> {
    if existing.identity() != &identity {
        return Err(RestoreDocumentRevisionError::OperationConflict);
    }
    match existing.state() {
        DocumentOperationJournalState::Claimed => {
            Err(RestoreDocumentRevisionError::RecoveryRequired)
        }
        DocumentOperationJournalState::Committed => {
            let result = existing
                .result()
                .ok_or(RestoreDocumentRevisionError::JournalUnavailable)?;
            Ok(RestoreDocumentRevisionOutput {
                kind: CommitDocumentRevisionOutcomeKind::Replayed,
                version_id: result.version_id().clone(),
                revision_number: result.revision_number(),
            })
        }
        DocumentOperationJournalState::Failed => match existing.failure() {
            Some(DocumentOperationTerminalFailure::Conflict) => {
                Err(RestoreDocumentRevisionError::CommitConflict)
            }
            Some(DocumentOperationTerminalFailure::InvalidRequest) => {
                Err(RestoreDocumentRevisionError::InvalidInput)
            }
            None => Err(RestoreDocumentRevisionError::JournalUnavailable),
        },
    }
}

const fn map_journal_error(error: DocumentOperationJournalError) -> RestoreDocumentRevisionError {
    match error {
        DocumentOperationJournalError::IdentityConflict => {
            RestoreDocumentRevisionError::OperationConflict
        }
        DocumentOperationJournalError::NotClaimed
        | DocumentOperationJournalError::AlreadyCompleted
        | DocumentOperationJournalError::StorageUnavailable
        | DocumentOperationJournalError::CorruptedJournal => {
            RestoreDocumentRevisionError::JournalUnavailable
        }
    }
}

const fn map_metadata_error(
    error: GenerateDocumentRevisionMetadataError,
) -> RestoreDocumentRevisionError {
    match error {
        GenerateDocumentRevisionMetadataError::Conflict => {
            RestoreDocumentRevisionError::CommitConflict
        }
        GenerateDocumentRevisionMetadataError::InvalidTimestamp
        | GenerateDocumentRevisionMetadataError::GenerationUnavailable
        | GenerateDocumentRevisionMetadataError::StorageUnavailable => {
            RestoreDocumentRevisionError::MetadataUnavailable
        }
    }
}

const fn map_commit_error(error: CommitDocumentRevisionError) -> RestoreDocumentRevisionError {
    match error {
        CommitDocumentRevisionError::InvalidRequest => RestoreDocumentRevisionError::InvalidInput,
        CommitDocumentRevisionError::OperationConflict => {
            RestoreDocumentRevisionError::OperationConflict
        }
        CommitDocumentRevisionError::JournalUnavailable => {
            RestoreDocumentRevisionError::JournalUnavailable
        }
        CommitDocumentRevisionError::CommitConflict => RestoreDocumentRevisionError::CommitConflict,
        CommitDocumentRevisionError::CommitUnavailable => {
            RestoreDocumentRevisionError::CommitUnavailable
        }
        CommitDocumentRevisionError::RecoveryRequired => {
            RestoreDocumentRevisionError::RecoveryRequired
        }
    }
}
