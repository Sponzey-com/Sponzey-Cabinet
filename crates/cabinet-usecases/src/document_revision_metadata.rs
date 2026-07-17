use cabinet_domain::document::DocumentId;
use cabinet_domain::document_revision::DocumentExpectedCurrentVersion;
use cabinet_domain::version::{DocumentRevisionNumber, DocumentSnapshotRef, VersionId};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_revision_metadata::{
    DocumentRevisionClock, DocumentRevisionMetadataPortError, DocumentRevisionNumberAllocator,
    DocumentSnapshotRefGenerator, DocumentVersionIdGenerator,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerateDocumentRevisionMetadataInput {
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    expected_current: DocumentExpectedCurrentVersion,
}

impl GenerateDocumentRevisionMetadataInput {
    pub const fn new(
        workspace_id: WorkspaceId,
        document_id: DocumentId,
        expected_current: DocumentExpectedCurrentVersion,
    ) -> Self {
        Self {
            workspace_id,
            document_id,
            expected_current,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerateDocumentRevisionMetadataOutput {
    version_id: VersionId,
    snapshot_ref: DocumentSnapshotRef,
    created_at_epoch_ms: u64,
    revision_number: DocumentRevisionNumber,
}

impl GenerateDocumentRevisionMetadataOutput {
    pub const fn version_id(&self) -> &VersionId {
        &self.version_id
    }

    pub const fn snapshot_ref(&self) -> &DocumentSnapshotRef {
        &self.snapshot_ref
    }

    pub const fn created_at_epoch_ms(&self) -> u64 {
        self.created_at_epoch_ms
    }

    pub const fn revision_number(&self) -> DocumentRevisionNumber {
        self.revision_number
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerateDocumentRevisionMetadataError {
    InvalidTimestamp,
    GenerationUnavailable,
    Conflict,
    StorageUnavailable,
}

impl GenerateDocumentRevisionMetadataError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidTimestamp => "document_revision_metadata.invalid_timestamp",
            Self::GenerationUnavailable => "document_revision_metadata.generation_unavailable",
            Self::Conflict => "document_revision_metadata.conflict",
            Self::StorageUnavailable => "document_revision_metadata.storage_unavailable",
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct GenerateDocumentRevisionMetadataUsecase;

impl GenerateDocumentRevisionMetadataUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute<V, S, C, A>(
        &self,
        input: GenerateDocumentRevisionMetadataInput,
        version_generator: &V,
        snapshot_generator: &S,
        clock: &C,
        allocator: &A,
    ) -> Result<GenerateDocumentRevisionMetadataOutput, GenerateDocumentRevisionMetadataError>
    where
        V: DocumentVersionIdGenerator,
        S: DocumentSnapshotRefGenerator,
        C: DocumentRevisionClock,
        A: DocumentRevisionNumberAllocator,
    {
        let version_id = version_generator
            .generate_version_id()
            .map_err(map_port_error)?;
        let snapshot_ref = snapshot_generator
            .generate_snapshot_ref(&version_id)
            .map_err(map_port_error)?;
        let created_at_epoch_ms = clock.now_epoch_ms().map_err(map_port_error)?;
        if created_at_epoch_ms == 0 {
            return Err(GenerateDocumentRevisionMetadataError::InvalidTimestamp);
        }
        let revision_number = allocator
            .allocate_next_revision(
                &input.workspace_id,
                &input.document_id,
                &input.expected_current,
            )
            .map_err(map_port_error)?;

        Ok(GenerateDocumentRevisionMetadataOutput {
            version_id,
            snapshot_ref,
            created_at_epoch_ms,
            revision_number,
        })
    }
}

const fn map_port_error(
    error: DocumentRevisionMetadataPortError,
) -> GenerateDocumentRevisionMetadataError {
    match error {
        DocumentRevisionMetadataPortError::GenerationUnavailable => {
            GenerateDocumentRevisionMetadataError::GenerationUnavailable
        }
        DocumentRevisionMetadataPortError::Conflict => {
            GenerateDocumentRevisionMetadataError::Conflict
        }
        DocumentRevisionMetadataPortError::StorageUnavailable => {
            GenerateDocumentRevisionMetadataError::StorageUnavailable
        }
    }
}
