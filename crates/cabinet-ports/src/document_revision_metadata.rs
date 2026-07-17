use cabinet_domain::document::DocumentId;
use cabinet_domain::document_revision::DocumentExpectedCurrentVersion;
use cabinet_domain::version::{DocumentRevisionNumber, DocumentSnapshotRef, VersionId};
use cabinet_domain::workspace::WorkspaceId;

pub trait DocumentVersionIdGenerator {
    fn generate_version_id(&self) -> Result<VersionId, DocumentRevisionMetadataPortError>;
}

pub trait DocumentSnapshotRefGenerator {
    fn generate_snapshot_ref(
        &self,
        version_id: &VersionId,
    ) -> Result<DocumentSnapshotRef, DocumentRevisionMetadataPortError>;
}

pub trait DocumentRevisionClock {
    fn now_epoch_ms(&self) -> Result<u64, DocumentRevisionMetadataPortError>;
}

pub trait DocumentRevisionNumberAllocator {
    fn allocate_next_revision(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        expected_current: &DocumentExpectedCurrentVersion,
    ) -> Result<DocumentRevisionNumber, DocumentRevisionMetadataPortError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentRevisionMetadataPortError {
    GenerationUnavailable,
    Conflict,
    StorageUnavailable,
}

impl DocumentRevisionMetadataPortError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::GenerationUnavailable => "document_revision_metadata.generation_unavailable",
            Self::Conflict => "document_revision_metadata.conflict",
            Self::StorageUnavailable => "document_revision_metadata.storage_unavailable",
        }
    }
}
