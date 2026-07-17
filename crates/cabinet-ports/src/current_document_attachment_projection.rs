use cabinet_domain::asset::AssetReference;
use cabinet_domain::document::DocumentId;
use cabinet_domain::version::DocumentRevisionNumber;
use cabinet_domain::workspace::WorkspaceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentDocumentAttachmentProjectionRequest {
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    revision_number: DocumentRevisionNumber,
    references: Vec<AssetReference>,
}

impl CurrentDocumentAttachmentProjectionRequest {
    pub fn new(
        workspace_id: WorkspaceId,
        document_id: DocumentId,
        revision_number: DocumentRevisionNumber,
        references: Vec<AssetReference>,
    ) -> Result<Self, CurrentDocumentAttachmentProjectionError> {
        if references
            .windows(2)
            .any(|pair| pair[0].asset_id().as_str() >= pair[1].asset_id().as_str())
        {
            return Err(CurrentDocumentAttachmentProjectionError::InvalidRequest);
        }
        Ok(Self {
            workspace_id,
            document_id,
            revision_number,
            references,
        })
    }

    pub const fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    pub const fn document_id(&self) -> &DocumentId {
        &self.document_id
    }

    pub const fn revision_number(&self) -> DocumentRevisionNumber {
        self.revision_number
    }

    pub fn references(&self) -> &[AssetReference] {
        &self.references
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentDocumentAttachmentProjectionOutcome {
    Applied,
    AlreadyCurrent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentDocumentAttachmentProjectionError {
    InvalidRequest,
    Conflict,
    StorageUnavailable,
    CorruptedProjection,
}

impl CurrentDocumentAttachmentProjectionError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidRequest => "current_document_attachments.invalid_request",
            Self::Conflict => "current_document_attachments.conflict",
            Self::StorageUnavailable => "current_document_attachments.storage_unavailable",
            Self::CorruptedProjection => "current_document_attachments.corrupted_projection",
        }
    }
}

pub trait CurrentDocumentAttachmentProjectionWriter {
    fn replace_current_document_attachments(
        &mut self,
        request: CurrentDocumentAttachmentProjectionRequest,
    ) -> Result<CurrentDocumentAttachmentProjectionOutcome, CurrentDocumentAttachmentProjectionError>;
}
