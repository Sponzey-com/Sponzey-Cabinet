use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::document_revision::{
    DocumentExpectedCurrentVersion, DocumentMutationKind, DocumentOperationId,
    DocumentOperationIdentity,
};
use cabinet_domain::version::{
    AttachmentSnapshotState, DocumentRevisionNumber, VersionAuthor, VersionEntry, VersionId,
    VersionSummary,
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
use cabinet_ports::version_store::{VersionRecord, VersionSnapshot};

use crate::document_revision_commit::{
    CommitDocumentRevisionError, CommitDocumentRevisionOutcomeKind, CommitDocumentRevisionUsecase,
};
use crate::document_revision_metadata::{
    GenerateDocumentRevisionMetadataError, GenerateDocumentRevisionMetadataInput,
    GenerateDocumentRevisionMetadataUsecase,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateDocumentRevisionInput {
    operation_id: String,
    workspace_id: String,
    document_id: String,
    body: String,
    author: String,
    summary: String,
}

impl CreateDocumentRevisionInput {
    pub fn new(
        operation_id: &str,
        workspace_id: &str,
        document_id: &str,
        body: &str,
        author: &str,
        summary: &str,
    ) -> Self {
        Self {
            operation_id: operation_id.to_string(),
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
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
pub struct CreateDocumentRevisionOutput {
    kind: CommitDocumentRevisionOutcomeKind,
    version_id: VersionId,
    revision_number: DocumentRevisionNumber,
}

impl CreateDocumentRevisionOutput {
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
pub enum CreateDocumentRevisionError {
    InvalidInput,
    FingerprintUnavailable,
    MetadataUnavailable,
    OperationConflict,
    CommitConflict,
    JournalUnavailable,
    CommitUnavailable,
    RecoveryRequired,
}

impl CreateDocumentRevisionError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "create_document_revision.invalid_input",
            Self::FingerprintUnavailable => "create_document_revision.fingerprint_unavailable",
            Self::MetadataUnavailable => "create_document_revision.metadata_unavailable",
            Self::OperationConflict => "create_document_revision.operation_conflict",
            Self::CommitConflict => "create_document_revision.commit_conflict",
            Self::JournalUnavailable => "create_document_revision.journal_unavailable",
            Self::CommitUnavailable => "create_document_revision.commit_unavailable",
            Self::RecoveryRequired => "create_document_revision.recovery_required",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreateDocumentRevisionUsecase {
    body_policy: DocumentBodyPolicy,
}

impl CreateDocumentRevisionUsecase {
    pub const fn new(body_policy: DocumentBodyPolicy) -> Self {
        Self { body_policy }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute<F, V, S, C, A, M, J>(
        &self,
        input: CreateDocumentRevisionInput,
        fingerprint_port: &F,
        version_generator: &V,
        snapshot_generator: &S,
        clock: &C,
        revision_allocator: &A,
        commit_port: &mut M,
        journal_port: &mut J,
    ) -> Result<CreateDocumentRevisionOutput, CreateDocumentRevisionError>
    where
        F: DocumentMutationFingerprintPort,
        V: DocumentVersionIdGenerator,
        S: DocumentSnapshotRefGenerator,
        C: DocumentRevisionClock,
        A: DocumentRevisionNumberAllocator,
        M: DocumentRevisionCommitPort,
        J: DocumentOperationJournalPort,
    {
        let parsed = ParsedCreateDocumentRevision::parse(input, self.body_policy)?;
        let attachment_state = AttachmentSnapshotState::known(Vec::new())
            .map_err(|_| CreateDocumentRevisionError::InvalidInput)?;
        let expected_current = DocumentExpectedCurrentVersion::MustNotExist;
        let fingerprint_input = DocumentMutationFingerprintInput::new(
            DocumentMutationKind::Create,
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
            .map_err(|_| CreateDocumentRevisionError::FingerprintUnavailable)?;
        let identity = DocumentOperationIdentity::new(
            parsed.operation_id,
            parsed.workspace_id.clone(),
            parsed.document_id.clone(),
            DocumentMutationKind::Create,
            expected_current.clone(),
        )
        .map_err(|_| CreateDocumentRevisionError::InvalidInput)?
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
        .map_err(|_| CreateDocumentRevisionError::InvalidInput)?;
        let snapshot = VersionSnapshot::with_attachment_state(
            parsed.document_id,
            metadata.snapshot_ref().clone(),
            parsed.body,
            attachment_state,
        );
        let record = VersionRecord::new(entry, snapshot)
            .map_err(|_| CreateDocumentRevisionError::InvalidInput)?;
        let request = DocumentRevisionCommitRequest::new(identity, record)
            .map_err(|_| CreateDocumentRevisionError::InvalidInput)?;
        let committed = CommitDocumentRevisionUsecase::new()
            .execute(request, commit_port, journal_port)
            .map_err(map_commit_error)?;

        Ok(CreateDocumentRevisionOutput {
            kind: committed.kind(),
            version_id: committed.result().version_id().clone(),
            revision_number: committed.result().revision_number(),
        })
    }
}

struct ParsedCreateDocumentRevision {
    operation_id: DocumentOperationId,
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    body: DocumentBody,
    author: VersionAuthor,
    summary: VersionSummary,
}

impl ParsedCreateDocumentRevision {
    fn parse(
        input: CreateDocumentRevisionInput,
        body_policy: DocumentBodyPolicy,
    ) -> Result<Self, CreateDocumentRevisionError> {
        Ok(Self {
            operation_id: DocumentOperationId::new(&input.operation_id)
                .map_err(|_| CreateDocumentRevisionError::InvalidInput)?,
            workspace_id: WorkspaceId::new(&input.workspace_id)
                .map_err(|_| CreateDocumentRevisionError::InvalidInput)?,
            document_id: DocumentId::new(&input.document_id)
                .map_err(|_| CreateDocumentRevisionError::InvalidInput)?,
            body: DocumentBody::new(&input.body, body_policy)
                .map_err(|_| CreateDocumentRevisionError::InvalidInput)?,
            author: VersionAuthor::new(&input.author)
                .map_err(|_| CreateDocumentRevisionError::InvalidInput)?,
            summary: VersionSummary::new(&input.summary)
                .map_err(|_| CreateDocumentRevisionError::InvalidInput)?,
        })
    }
}

fn replay_existing(
    identity: DocumentOperationIdentity,
    existing: DocumentOperationJournalRecord,
) -> Result<CreateDocumentRevisionOutput, CreateDocumentRevisionError> {
    if existing.identity() != &identity {
        return Err(CreateDocumentRevisionError::OperationConflict);
    }
    match existing.state() {
        DocumentOperationJournalState::Claimed => {
            Err(CreateDocumentRevisionError::RecoveryRequired)
        }
        DocumentOperationJournalState::Committed => {
            let result = existing
                .result()
                .ok_or(CreateDocumentRevisionError::JournalUnavailable)?;
            Ok(CreateDocumentRevisionOutput {
                kind: CommitDocumentRevisionOutcomeKind::Replayed,
                version_id: result.version_id().clone(),
                revision_number: result.revision_number(),
            })
        }
        DocumentOperationJournalState::Failed => match existing.failure() {
            Some(DocumentOperationTerminalFailure::Conflict) => {
                Err(CreateDocumentRevisionError::CommitConflict)
            }
            Some(DocumentOperationTerminalFailure::InvalidRequest) => {
                Err(CreateDocumentRevisionError::InvalidInput)
            }
            None => Err(CreateDocumentRevisionError::JournalUnavailable),
        },
    }
}

const fn map_journal_error(error: DocumentOperationJournalError) -> CreateDocumentRevisionError {
    match error {
        DocumentOperationJournalError::IdentityConflict => {
            CreateDocumentRevisionError::OperationConflict
        }
        DocumentOperationJournalError::NotClaimed
        | DocumentOperationJournalError::AlreadyCompleted
        | DocumentOperationJournalError::StorageUnavailable
        | DocumentOperationJournalError::CorruptedJournal => {
            CreateDocumentRevisionError::JournalUnavailable
        }
    }
}

const fn map_metadata_error(
    error: GenerateDocumentRevisionMetadataError,
) -> CreateDocumentRevisionError {
    match error {
        GenerateDocumentRevisionMetadataError::Conflict => {
            CreateDocumentRevisionError::CommitConflict
        }
        GenerateDocumentRevisionMetadataError::InvalidTimestamp
        | GenerateDocumentRevisionMetadataError::GenerationUnavailable
        | GenerateDocumentRevisionMetadataError::StorageUnavailable => {
            CreateDocumentRevisionError::MetadataUnavailable
        }
    }
}

const fn map_commit_error(error: CommitDocumentRevisionError) -> CreateDocumentRevisionError {
    match error {
        CommitDocumentRevisionError::InvalidRequest => CreateDocumentRevisionError::InvalidInput,
        CommitDocumentRevisionError::OperationConflict => {
            CreateDocumentRevisionError::OperationConflict
        }
        CommitDocumentRevisionError::JournalUnavailable => {
            CreateDocumentRevisionError::JournalUnavailable
        }
        CommitDocumentRevisionError::CommitConflict => CreateDocumentRevisionError::CommitConflict,
        CommitDocumentRevisionError::CommitUnavailable => {
            CreateDocumentRevisionError::CommitUnavailable
        }
        CommitDocumentRevisionError::RecoveryRequired => {
            CreateDocumentRevisionError::RecoveryRequired
        }
    }
}
