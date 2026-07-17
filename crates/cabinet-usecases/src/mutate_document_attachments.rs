use cabinet_domain::asset::{AssetId, AssetReference};
use cabinet_domain::attachment_snapshot_mutation::{
    AttachmentSnapshotDelta, AttachmentSnapshotMutation, AttachmentSnapshotMutationError,
    transition_attachment_snapshot,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::document_revision::{
    DocumentExpectedCurrentVersion, DocumentMutationKind, DocumentOperationId,
    DocumentOperationIdentity,
};
use cabinet_domain::version::{
    DocumentRevisionNumber, VersionAuthor, VersionEntry, VersionId, VersionSummary,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::committed_version_record_reader::{
    CommittedVersionRecordReadError, CommittedVersionRecordReader,
};
use cabinet_ports::current_document_version::{
    CurrentDocumentVersionPointerError, CurrentDocumentVersionPointerPort,
};
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
pub struct MutateDocumentAttachmentsInput {
    operation_id: String,
    workspace_id: String,
    document_id: String,
    expected_current_version: String,
    mutation: AttachmentMutationInput,
    author: String,
    summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AttachmentMutationInput {
    Link { asset_id: String, label: String },
    Unlink { asset_id: String },
}

impl MutateDocumentAttachmentsInput {
    #[allow(clippy::too_many_arguments)]
    pub fn link(
        operation_id: &str,
        workspace_id: &str,
        document_id: &str,
        expected_current_version: &str,
        asset_id: &str,
        label: &str,
        author: &str,
        summary: &str,
    ) -> Self {
        Self {
            operation_id: operation_id.to_string(),
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            expected_current_version: expected_current_version.to_string(),
            mutation: AttachmentMutationInput::Link {
                asset_id: asset_id.to_string(),
                label: label.to_string(),
            },
            author: author.to_string(),
            summary: summary.to_string(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn unlink(
        operation_id: &str,
        workspace_id: &str,
        document_id: &str,
        expected_current_version: &str,
        asset_id: &str,
        author: &str,
        summary: &str,
    ) -> Self {
        Self {
            operation_id: operation_id.to_string(),
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            expected_current_version: expected_current_version.to_string(),
            mutation: AttachmentMutationInput::Unlink {
                asset_id: asset_id.to_string(),
            },
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutateDocumentAttachmentsOutcomeKind {
    Fresh,
    Replayed,
    NoChange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutateDocumentAttachmentsOutput {
    kind: MutateDocumentAttachmentsOutcomeKind,
    delta: AttachmentSnapshotDelta,
    version_id: VersionId,
    revision_number: DocumentRevisionNumber,
}

impl MutateDocumentAttachmentsOutput {
    pub const fn kind(&self) -> MutateDocumentAttachmentsOutcomeKind {
        self.kind
    }

    pub const fn delta(&self) -> AttachmentSnapshotDelta {
        self.delta
    }

    pub const fn version_id(&self) -> &VersionId {
        &self.version_id
    }

    pub const fn revision_number(&self) -> DocumentRevisionNumber {
        self.revision_number
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutateDocumentAttachmentsError {
    InvalidInput,
    NotFound,
    LegacyBaselineRequired,
    CorruptedData,
    StorageUnavailable,
    FingerprintUnavailable,
    MetadataUnavailable,
    OperationConflict,
    CommitConflict,
    JournalUnavailable,
    CommitUnavailable,
    RecoveryRequired,
}

impl MutateDocumentAttachmentsError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "mutate_document_attachments.invalid_input",
            Self::NotFound => "mutate_document_attachments.not_found",
            Self::LegacyBaselineRequired => "mutate_document_attachments.legacy_baseline_required",
            Self::CorruptedData => "mutate_document_attachments.corrupted_data",
            Self::StorageUnavailable => "mutate_document_attachments.storage_unavailable",
            Self::FingerprintUnavailable => "mutate_document_attachments.fingerprint_unavailable",
            Self::MetadataUnavailable => "mutate_document_attachments.metadata_unavailable",
            Self::OperationConflict => "mutate_document_attachments.operation_conflict",
            Self::CommitConflict => "mutate_document_attachments.commit_conflict",
            Self::JournalUnavailable => "mutate_document_attachments.journal_unavailable",
            Self::CommitUnavailable => "mutate_document_attachments.commit_unavailable",
            Self::RecoveryRequired => "mutate_document_attachments.recovery_required",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MutateDocumentAttachmentsUsecase;

impl MutateDocumentAttachmentsUsecase {
    pub const fn new() -> Self {
        Self
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute<R, P, F, V, S, C, A, M, J>(
        &self,
        input: MutateDocumentAttachmentsInput,
        records: &R,
        pointer: &P,
        fingerprint_port: &F,
        version_generator: &V,
        snapshot_generator: &S,
        clock: &C,
        revision_allocator: &A,
        commit_port: &mut M,
        journal_port: &mut J,
    ) -> Result<MutateDocumentAttachmentsOutput, MutateDocumentAttachmentsError>
    where
        R: CommittedVersionRecordReader,
        P: CurrentDocumentVersionPointerPort,
        F: DocumentMutationFingerprintPort,
        V: DocumentVersionIdGenerator,
        S: DocumentSnapshotRefGenerator,
        C: DocumentRevisionClock,
        A: DocumentRevisionNumberAllocator,
        M: DocumentRevisionCommitPort,
        J: DocumentOperationJournalPort,
    {
        let parsed = ParsedAttachmentMutation::parse(input)?;
        let expected_record = records
            .get_committed_version_record(
                &parsed.workspace_id,
                &parsed.document_id,
                &parsed.expected_current_version,
            )
            .map_err(map_record_error)?
            .ok_or(MutateDocumentAttachmentsError::NotFound)?;
        if expected_record.document_id() != &parsed.document_id
            || expected_record.version_id() != &parsed.expected_current_version
        {
            return Err(MutateDocumentAttachmentsError::CorruptedData);
        }
        let transitioned = transition_attachment_snapshot(
            expected_record.snapshot().attachment_state(),
            parsed.mutation,
            None,
        )
        .map_err(map_transition_error)?;
        let revision_number = expected_record
            .entry()
            .revision_number()
            .ok_or(MutateDocumentAttachmentsError::CorruptedData)?;

        if !transitioned.changed() {
            let current = pointer
                .load_current_version(&parsed.workspace_id, &parsed.document_id)
                .map_err(map_pointer_error)?;
            if current.as_ref() != Some(&parsed.expected_current_version) {
                return Err(MutateDocumentAttachmentsError::CommitConflict);
            }
            return Ok(MutateDocumentAttachmentsOutput {
                kind: MutateDocumentAttachmentsOutcomeKind::NoChange,
                delta: transitioned.delta(),
                version_id: parsed.expected_current_version,
                revision_number,
            });
        }

        let expected_current =
            DocumentExpectedCurrentVersion::MustMatch(parsed.expected_current_version.clone());
        let fingerprint = fingerprint_port
            .fingerprint(&DocumentMutationFingerprintInput::new(
                parsed.kind,
                parsed.workspace_id.clone(),
                parsed.document_id.clone(),
                expected_current.clone(),
                expected_record.snapshot().body().clone(),
                parsed.author.clone(),
                parsed.summary.clone(),
                transitioned.state().clone(),
            ))
            .map_err(|_| MutateDocumentAttachmentsError::FingerprintUnavailable)?;
        let identity = DocumentOperationIdentity::new(
            parsed.operation_id,
            parsed.workspace_id.clone(),
            parsed.document_id.clone(),
            parsed.kind,
            expected_current.clone(),
        )
        .map_err(|_| MutateDocumentAttachmentsError::InvalidInput)?
        .with_request_fingerprint(fingerprint);

        if let Some(existing) = journal_port
            .load_operation(identity.operation_id())
            .map_err(map_journal_error)?
        {
            return replay_existing(identity, existing, transitioned.delta());
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
        .map_err(|_| MutateDocumentAttachmentsError::InvalidInput)?;
        let snapshot = VersionSnapshot::with_attachment_state(
            parsed.document_id,
            metadata.snapshot_ref().clone(),
            expected_record.snapshot().body().clone(),
            transitioned.state().clone(),
        );
        let record = VersionRecord::new(entry, snapshot)
            .map_err(|_| MutateDocumentAttachmentsError::InvalidInput)?;
        let request = DocumentRevisionCommitRequest::new(identity, record)
            .map_err(|_| MutateDocumentAttachmentsError::InvalidInput)?;
        let committed = CommitDocumentRevisionUsecase::new()
            .execute(request, commit_port, journal_port)
            .map_err(map_commit_error)?;

        Ok(MutateDocumentAttachmentsOutput {
            kind: match committed.kind() {
                CommitDocumentRevisionOutcomeKind::Fresh => {
                    MutateDocumentAttachmentsOutcomeKind::Fresh
                }
                CommitDocumentRevisionOutcomeKind::Replayed => {
                    MutateDocumentAttachmentsOutcomeKind::Replayed
                }
            },
            delta: transitioned.delta(),
            version_id: committed.result().version_id().clone(),
            revision_number: committed.result().revision_number(),
        })
    }
}

struct ParsedAttachmentMutation {
    operation_id: DocumentOperationId,
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    expected_current_version: VersionId,
    kind: DocumentMutationKind,
    mutation: AttachmentSnapshotMutation,
    author: VersionAuthor,
    summary: VersionSummary,
}

impl ParsedAttachmentMutation {
    fn parse(
        input: MutateDocumentAttachmentsInput,
    ) -> Result<Self, MutateDocumentAttachmentsError> {
        let (kind, mutation) = match input.mutation {
            AttachmentMutationInput::Link { asset_id, label } => {
                let asset_id = AssetId::from_sha256_hex(&asset_id)
                    .map_err(|_| MutateDocumentAttachmentsError::InvalidInput)?;
                let reference = AssetReference::new(asset_id, &label)
                    .map_err(|_| MutateDocumentAttachmentsError::InvalidInput)?;
                (
                    DocumentMutationKind::LinkAsset,
                    AttachmentSnapshotMutation::Link(reference),
                )
            }
            AttachmentMutationInput::Unlink { asset_id } => (
                DocumentMutationKind::UnlinkAsset,
                AttachmentSnapshotMutation::Unlink(
                    AssetId::from_sha256_hex(&asset_id)
                        .map_err(|_| MutateDocumentAttachmentsError::InvalidInput)?,
                ),
            ),
        };
        Ok(Self {
            operation_id: DocumentOperationId::new(&input.operation_id)
                .map_err(|_| MutateDocumentAttachmentsError::InvalidInput)?,
            workspace_id: WorkspaceId::new(&input.workspace_id)
                .map_err(|_| MutateDocumentAttachmentsError::InvalidInput)?,
            document_id: DocumentId::new(&input.document_id)
                .map_err(|_| MutateDocumentAttachmentsError::InvalidInput)?,
            expected_current_version: VersionId::new(&input.expected_current_version)
                .map_err(|_| MutateDocumentAttachmentsError::InvalidInput)?,
            kind,
            mutation,
            author: VersionAuthor::new(&input.author)
                .map_err(|_| MutateDocumentAttachmentsError::InvalidInput)?,
            summary: VersionSummary::new(&input.summary)
                .map_err(|_| MutateDocumentAttachmentsError::InvalidInput)?,
        })
    }
}

fn replay_existing(
    identity: DocumentOperationIdentity,
    existing: DocumentOperationJournalRecord,
    delta: AttachmentSnapshotDelta,
) -> Result<MutateDocumentAttachmentsOutput, MutateDocumentAttachmentsError> {
    if existing.identity() != &identity {
        return Err(MutateDocumentAttachmentsError::OperationConflict);
    }
    match existing.state() {
        DocumentOperationJournalState::Claimed => {
            Err(MutateDocumentAttachmentsError::RecoveryRequired)
        }
        DocumentOperationJournalState::Committed => {
            let result = existing
                .result()
                .ok_or(MutateDocumentAttachmentsError::JournalUnavailable)?;
            Ok(MutateDocumentAttachmentsOutput {
                kind: MutateDocumentAttachmentsOutcomeKind::Replayed,
                delta,
                version_id: result.version_id().clone(),
                revision_number: result.revision_number(),
            })
        }
        DocumentOperationJournalState::Failed => match existing.failure() {
            Some(DocumentOperationTerminalFailure::Conflict) => {
                Err(MutateDocumentAttachmentsError::CommitConflict)
            }
            Some(DocumentOperationTerminalFailure::InvalidRequest) => {
                Err(MutateDocumentAttachmentsError::InvalidInput)
            }
            None => Err(MutateDocumentAttachmentsError::JournalUnavailable),
        },
    }
}

const fn map_transition_error(
    error: AttachmentSnapshotMutationError,
) -> MutateDocumentAttachmentsError {
    match error {
        AttachmentSnapshotMutationError::LegacyBaselineRequired => {
            MutateDocumentAttachmentsError::LegacyBaselineRequired
        }
        AttachmentSnapshotMutationError::InvalidBaseline => {
            MutateDocumentAttachmentsError::CorruptedData
        }
    }
}

const fn map_record_error(
    error: CommittedVersionRecordReadError,
) -> MutateDocumentAttachmentsError {
    match error {
        CommittedVersionRecordReadError::StorageUnavailable => {
            MutateDocumentAttachmentsError::StorageUnavailable
        }
        CommittedVersionRecordReadError::CorruptedRecord => {
            MutateDocumentAttachmentsError::CorruptedData
        }
    }
}

const fn map_pointer_error(
    error: CurrentDocumentVersionPointerError,
) -> MutateDocumentAttachmentsError {
    match error {
        CurrentDocumentVersionPointerError::StorageUnavailable => {
            MutateDocumentAttachmentsError::StorageUnavailable
        }
        CurrentDocumentVersionPointerError::Conflict => {
            MutateDocumentAttachmentsError::CommitConflict
        }
        CurrentDocumentVersionPointerError::CorruptedPointer => {
            MutateDocumentAttachmentsError::CorruptedData
        }
    }
}

const fn map_journal_error(error: DocumentOperationJournalError) -> MutateDocumentAttachmentsError {
    match error {
        DocumentOperationJournalError::IdentityConflict => {
            MutateDocumentAttachmentsError::OperationConflict
        }
        DocumentOperationJournalError::NotClaimed
        | DocumentOperationJournalError::AlreadyCompleted
        | DocumentOperationJournalError::StorageUnavailable
        | DocumentOperationJournalError::CorruptedJournal => {
            MutateDocumentAttachmentsError::JournalUnavailable
        }
    }
}

const fn map_metadata_error(
    error: GenerateDocumentRevisionMetadataError,
) -> MutateDocumentAttachmentsError {
    match error {
        GenerateDocumentRevisionMetadataError::Conflict => {
            MutateDocumentAttachmentsError::CommitConflict
        }
        GenerateDocumentRevisionMetadataError::InvalidTimestamp
        | GenerateDocumentRevisionMetadataError::GenerationUnavailable
        | GenerateDocumentRevisionMetadataError::StorageUnavailable => {
            MutateDocumentAttachmentsError::MetadataUnavailable
        }
    }
}

const fn map_commit_error(error: CommitDocumentRevisionError) -> MutateDocumentAttachmentsError {
    match error {
        CommitDocumentRevisionError::InvalidRequest => MutateDocumentAttachmentsError::InvalidInput,
        CommitDocumentRevisionError::OperationConflict => {
            MutateDocumentAttachmentsError::OperationConflict
        }
        CommitDocumentRevisionError::JournalUnavailable => {
            MutateDocumentAttachmentsError::JournalUnavailable
        }
        CommitDocumentRevisionError::CommitConflict => {
            MutateDocumentAttachmentsError::CommitConflict
        }
        CommitDocumentRevisionError::CommitUnavailable => {
            MutateDocumentAttachmentsError::CommitUnavailable
        }
        CommitDocumentRevisionError::RecoveryRequired => {
            MutateDocumentAttachmentsError::RecoveryRequired
        }
    }
}
