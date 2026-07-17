use cabinet_domain::document_revision::DocumentOperationId;
use cabinet_domain::version::{DocumentRevisionNumber, VersionId};
use cabinet_domain::workspace::WorkspaceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedVersion {
    version_id: VersionId,
    revision_number: DocumentRevisionNumber,
}

impl PublishedVersion {
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

pub trait VersionPublicationPort {
    fn publish_prepared(
        &mut self,
        workspace_id: &WorkspaceId,
        operation_id: &DocumentOperationId,
    ) -> Result<PublishedVersion, VersionPublicationError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionPublicationError {
    NotPrepared,
    Conflict,
    StorageUnavailable,
    CorruptedPublication,
}

impl VersionPublicationError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::NotPrepared => "version_publication.not_prepared",
            Self::Conflict => "version_publication.conflict",
            Self::StorageUnavailable => "version_publication.storage_unavailable",
            Self::CorruptedPublication => "version_publication.corrupted",
        }
    }
}
