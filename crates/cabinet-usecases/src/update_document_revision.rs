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
pub struct UpdateDocumentRevisionInput {
    operation_id: String,
    workspace_id: String,
    document_id: String,
    expected_current_version: String,
    body: String,
    author: String,
    summary: String,
}

impl UpdateDocumentRevisionInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        operation_id: &str,
        workspace_id: &str,
        document_id: &str,
        expected_current_version: &str,
        body: &str,
        author: &str,
        summary: &str,
    ) -> Self {
        Self {
            operation_id: operation_id.to_string(),
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            expected_current_version: expected_current_version.to_string(),
            body: body.to_string(),
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateDocumentRevisionOutput {
    kind: CommitDocumentRevisionOutcomeKind,
    version_id: VersionId,
    revision_number: DocumentRevisionNumber,
}

impl UpdateDocumentRevisionOutput {
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
pub enum UpdateDocumentRevisionError {
    InvalidInput,
    NotFound,
    StorageUnavailable,
    FingerprintUnavailable,
    MetadataUnavailable,
    OperationConflict,
    CommitConflict,
    JournalUnavailable,
    CommitUnavailable,
    RecoveryRequired,
}

impl UpdateDocumentRevisionError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "update_document_revision.invalid_input",
            Self::NotFound => "update_document_revision.not_found",
            Self::StorageUnavailable => "update_document_revision.storage_unavailable",
            Self::FingerprintUnavailable => "update_document_revision.fingerprint_unavailable",
            Self::MetadataUnavailable => "update_document_revision.metadata_unavailable",
            Self::OperationConflict => "update_document_revision.operation_conflict",
            Self::CommitConflict => "update_document_revision.commit_conflict",
            Self::JournalUnavailable => "update_document_revision.journal_unavailable",
            Self::CommitUnavailable => "update_document_revision.commit_unavailable",
            Self::RecoveryRequired => "update_document_revision.recovery_required",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateDocumentRevisionUsecase {
    body_policy: DocumentBodyPolicy,
}

impl UpdateDocumentRevisionUsecase {
    pub const fn new(body_policy: DocumentBodyPolicy) -> Self {
        Self { body_policy }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute<R, F, V, S, C, A, M, J>(
        &self,
        input: UpdateDocumentRevisionInput,
        version_store: &R,
        fingerprint_port: &F,
        version_generator: &V,
        snapshot_generator: &S,
        clock: &C,
        revision_allocator: &A,
        commit_port: &mut M,
        journal_port: &mut J,
    ) -> Result<UpdateDocumentRevisionOutput, UpdateDocumentRevisionError>
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
        let parsed = ParsedUpdateDocumentRevision::parse(input, self.body_policy)?;
        let expected_snapshot = version_store
            .get_version_snapshot(
                &parsed.workspace_id,
                &parsed.document_id,
                &parsed.expected_current_version,
            )
            .map_err(|_| UpdateDocumentRevisionError::StorageUnavailable)?
            .ok_or(UpdateDocumentRevisionError::NotFound)?;
        if expected_snapshot.document_id() != &parsed.document_id {
            return Err(UpdateDocumentRevisionError::StorageUnavailable);
        }
        let attachment_state = expected_snapshot.attachment_state().clone();
        let expected_current =
            DocumentExpectedCurrentVersion::MustMatch(parsed.expected_current_version.clone());
        let fingerprint_input = DocumentMutationFingerprintInput::new(
            DocumentMutationKind::Update,
            parsed.workspace_id.clone(),
            parsed.document_id.clone(),
            expected_current.clone(),
            parsed.body.clone(),
            parsed.author.clone(),
            parsed.summary.clone(),
            attachment_state.clone(),
        );
        let fingerprint = fingerprint_port
            .fingerprint(&fingerprint_input)
            .map_err(|_| UpdateDocumentRevisionError::FingerprintUnavailable)?;
        let identity = DocumentOperationIdentity::new(
            parsed.operation_id,
            parsed.workspace_id.clone(),
            parsed.document_id.clone(),
            DocumentMutationKind::Update,
            expected_current.clone(),
        )
        .map_err(|_| UpdateDocumentRevisionError::InvalidInput)?
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
        .map_err(|_| UpdateDocumentRevisionError::InvalidInput)?;
        let snapshot = VersionSnapshot::with_attachment_state(
            parsed.document_id,
            metadata.snapshot_ref().clone(),
            parsed.body,
            attachment_state,
        );
        let record = VersionRecord::new(entry, snapshot)
            .map_err(|_| UpdateDocumentRevisionError::InvalidInput)?;
        let request = DocumentRevisionCommitRequest::new(identity, record)
            .map_err(|_| UpdateDocumentRevisionError::InvalidInput)?;
        let committed = CommitDocumentRevisionUsecase::new()
            .execute(request, commit_port, journal_port)
            .map_err(map_commit_error)?;

        Ok(UpdateDocumentRevisionOutput {
            kind: committed.kind(),
            version_id: committed.result().version_id().clone(),
            revision_number: committed.result().revision_number(),
        })
    }
}

struct ParsedUpdateDocumentRevision {
    operation_id: DocumentOperationId,
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    expected_current_version: VersionId,
    body: DocumentBody,
    author: VersionAuthor,
    summary: VersionSummary,
}

impl ParsedUpdateDocumentRevision {
    fn parse(
        input: UpdateDocumentRevisionInput,
        body_policy: DocumentBodyPolicy,
    ) -> Result<Self, UpdateDocumentRevisionError> {
        Ok(Self {
            operation_id: DocumentOperationId::new(&input.operation_id)
                .map_err(|_| UpdateDocumentRevisionError::InvalidInput)?,
            workspace_id: WorkspaceId::new(&input.workspace_id)
                .map_err(|_| UpdateDocumentRevisionError::InvalidInput)?,
            document_id: DocumentId::new(&input.document_id)
                .map_err(|_| UpdateDocumentRevisionError::InvalidInput)?,
            expected_current_version: VersionId::new(&input.expected_current_version)
                .map_err(|_| UpdateDocumentRevisionError::InvalidInput)?,
            body: DocumentBody::new(&input.body, body_policy)
                .map_err(|_| UpdateDocumentRevisionError::InvalidInput)?,
            author: VersionAuthor::new(&input.author)
                .map_err(|_| UpdateDocumentRevisionError::InvalidInput)?,
            summary: VersionSummary::new(&input.summary)
                .map_err(|_| UpdateDocumentRevisionError::InvalidInput)?,
        })
    }
}

fn replay_existing(
    identity: DocumentOperationIdentity,
    existing: DocumentOperationJournalRecord,
) -> Result<UpdateDocumentRevisionOutput, UpdateDocumentRevisionError> {
    if existing.identity() != &identity {
        return Err(UpdateDocumentRevisionError::OperationConflict);
    }
    match existing.state() {
        DocumentOperationJournalState::Claimed => {
            Err(UpdateDocumentRevisionError::RecoveryRequired)
        }
        DocumentOperationJournalState::Committed => {
            let result = existing
                .result()
                .ok_or(UpdateDocumentRevisionError::JournalUnavailable)?;
            Ok(UpdateDocumentRevisionOutput {
                kind: CommitDocumentRevisionOutcomeKind::Replayed,
                version_id: result.version_id().clone(),
                revision_number: result.revision_number(),
            })
        }
        DocumentOperationJournalState::Failed => match existing.failure() {
            Some(DocumentOperationTerminalFailure::Conflict) => {
                Err(UpdateDocumentRevisionError::CommitConflict)
            }
            Some(DocumentOperationTerminalFailure::InvalidRequest) => {
                Err(UpdateDocumentRevisionError::InvalidInput)
            }
            None => Err(UpdateDocumentRevisionError::JournalUnavailable),
        },
    }
}

const fn map_journal_error(error: DocumentOperationJournalError) -> UpdateDocumentRevisionError {
    match error {
        DocumentOperationJournalError::IdentityConflict => {
            UpdateDocumentRevisionError::OperationConflict
        }
        DocumentOperationJournalError::NotClaimed
        | DocumentOperationJournalError::AlreadyCompleted
        | DocumentOperationJournalError::StorageUnavailable
        | DocumentOperationJournalError::CorruptedJournal => {
            UpdateDocumentRevisionError::JournalUnavailable
        }
    }
}

const fn map_metadata_error(
    error: GenerateDocumentRevisionMetadataError,
) -> UpdateDocumentRevisionError {
    match error {
        GenerateDocumentRevisionMetadataError::Conflict => {
            UpdateDocumentRevisionError::CommitConflict
        }
        GenerateDocumentRevisionMetadataError::InvalidTimestamp
        | GenerateDocumentRevisionMetadataError::GenerationUnavailable
        | GenerateDocumentRevisionMetadataError::StorageUnavailable => {
            UpdateDocumentRevisionError::MetadataUnavailable
        }
    }
}

const fn map_commit_error(error: CommitDocumentRevisionError) -> UpdateDocumentRevisionError {
    match error {
        CommitDocumentRevisionError::InvalidRequest => UpdateDocumentRevisionError::InvalidInput,
        CommitDocumentRevisionError::OperationConflict => {
            UpdateDocumentRevisionError::OperationConflict
        }
        CommitDocumentRevisionError::JournalUnavailable => {
            UpdateDocumentRevisionError::JournalUnavailable
        }
        CommitDocumentRevisionError::CommitConflict => UpdateDocumentRevisionError::CommitConflict,
        CommitDocumentRevisionError::CommitUnavailable => {
            UpdateDocumentRevisionError::CommitUnavailable
        }
        CommitDocumentRevisionError::RecoveryRequired => {
            UpdateDocumentRevisionError::RecoveryRequired
        }
    }
}
