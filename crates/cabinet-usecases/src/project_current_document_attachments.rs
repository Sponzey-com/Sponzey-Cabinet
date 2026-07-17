use cabinet_domain::document::DocumentId;
use cabinet_domain::version::AttachmentSnapshotState;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_attachment_projection::{
    CurrentDocumentAttachmentProjectionError, CurrentDocumentAttachmentProjectionOutcome,
    CurrentDocumentAttachmentProjectionRequest, CurrentDocumentAttachmentProjectionWriter,
};
use cabinet_ports::version_store::VersionRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectCurrentDocumentAttachmentsInput {
    workspace_id: String,
    document_id: String,
    record: VersionRecord,
}

impl ProjectCurrentDocumentAttachmentsInput {
    pub fn new(workspace_id: &str, document_id: &str, record: VersionRecord) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            record,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectCurrentDocumentAttachmentsOutcomeKind {
    Applied,
    AlreadyCurrent,
    LegacyPreserved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectCurrentDocumentAttachmentsOutput {
    kind: ProjectCurrentDocumentAttachmentsOutcomeKind,
}

impl ProjectCurrentDocumentAttachmentsOutput {
    pub const fn kind(self) -> ProjectCurrentDocumentAttachmentsOutcomeKind {
        self.kind
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectCurrentDocumentAttachmentsError {
    InvalidInput,
    CorruptedRecord,
    Conflict,
    StorageUnavailable,
    CorruptedProjection,
}

impl ProjectCurrentDocumentAttachmentsError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "project_current_document_attachments.invalid_input",
            Self::CorruptedRecord => "project_current_document_attachments.corrupted_record",
            Self::Conflict => "project_current_document_attachments.conflict",
            Self::StorageUnavailable => "project_current_document_attachments.storage_unavailable",
            Self::CorruptedProjection => {
                "project_current_document_attachments.corrupted_projection"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProjectCurrentDocumentAttachmentsUsecase;

impl ProjectCurrentDocumentAttachmentsUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute<W: CurrentDocumentAttachmentProjectionWriter>(
        &self,
        input: ProjectCurrentDocumentAttachmentsInput,
        writer: &mut W,
    ) -> Result<ProjectCurrentDocumentAttachmentsOutput, ProjectCurrentDocumentAttachmentsError>
    {
        let workspace_id = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| ProjectCurrentDocumentAttachmentsError::InvalidInput)?;
        let document_id = DocumentId::new(&input.document_id)
            .map_err(|_| ProjectCurrentDocumentAttachmentsError::InvalidInput)?;
        if input.record.document_id() != &document_id {
            return Err(ProjectCurrentDocumentAttachmentsError::CorruptedRecord);
        }
        let revision_number = input
            .record
            .entry()
            .revision_number()
            .ok_or(ProjectCurrentDocumentAttachmentsError::CorruptedRecord)?;
        let references = match input.record.snapshot().attachment_state() {
            AttachmentSnapshotState::LegacyUnknown => {
                return Ok(ProjectCurrentDocumentAttachmentsOutput {
                    kind: ProjectCurrentDocumentAttachmentsOutcomeKind::LegacyPreserved,
                });
            }
            AttachmentSnapshotState::Known(snapshot) => snapshot.references().to_vec(),
        };
        let request = CurrentDocumentAttachmentProjectionRequest::new(
            workspace_id,
            document_id,
            revision_number,
            references,
        )
        .map_err(|_| ProjectCurrentDocumentAttachmentsError::CorruptedRecord)?;
        let kind = match writer
            .replace_current_document_attachments(request)
            .map_err(map_writer_error)?
        {
            CurrentDocumentAttachmentProjectionOutcome::Applied => {
                ProjectCurrentDocumentAttachmentsOutcomeKind::Applied
            }
            CurrentDocumentAttachmentProjectionOutcome::AlreadyCurrent => {
                ProjectCurrentDocumentAttachmentsOutcomeKind::AlreadyCurrent
            }
        };
        Ok(ProjectCurrentDocumentAttachmentsOutput { kind })
    }
}

const fn map_writer_error(
    error: CurrentDocumentAttachmentProjectionError,
) -> ProjectCurrentDocumentAttachmentsError {
    match error {
        CurrentDocumentAttachmentProjectionError::InvalidRequest => {
            ProjectCurrentDocumentAttachmentsError::CorruptedRecord
        }
        CurrentDocumentAttachmentProjectionError::Conflict => {
            ProjectCurrentDocumentAttachmentsError::Conflict
        }
        CurrentDocumentAttachmentProjectionError::StorageUnavailable => {
            ProjectCurrentDocumentAttachmentsError::StorageUnavailable
        }
        CurrentDocumentAttachmentProjectionError::CorruptedProjection => {
            ProjectCurrentDocumentAttachmentsError::CorruptedProjection
        }
    }
}
