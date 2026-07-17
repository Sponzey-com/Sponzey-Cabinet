use cabinet_domain::version::{DocumentRevisionNumber, VersionId};
use cabinet_domain::workspace::WorkspaceId;

use crate::document_repository::CurrentDocumentRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentDocumentRevisionProjection {
    record: CurrentDocumentRecord,
    version_id: VersionId,
    revision_number: DocumentRevisionNumber,
}

impl CurrentDocumentRevisionProjection {
    pub fn new(
        record: CurrentDocumentRecord,
        version_id: VersionId,
        revision_number: DocumentRevisionNumber,
    ) -> Self {
        Self {
            record,
            version_id,
            revision_number,
        }
    }

    pub fn record(&self) -> &CurrentDocumentRecord {
        &self.record
    }

    pub fn version_id(&self) -> &VersionId {
        &self.version_id
    }

    pub const fn revision_number(&self) -> DocumentRevisionNumber {
        self.revision_number
    }

    pub fn into_record(self) -> CurrentDocumentRecord {
        self.record
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentDocumentRevisionProjectionOutcome {
    Applied,
    AlreadyCurrent,
}

pub trait CurrentDocumentRevisionProjectionWriter {
    fn write_current_projection(
        &mut self,
        workspace_id: &WorkspaceId,
        projection: CurrentDocumentRevisionProjection,
    ) -> Result<CurrentDocumentRevisionProjectionOutcome, CurrentDocumentRevisionProjectionError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentDocumentRevisionProjectionError {
    StaleRevision,
    RevisionConflict,
    StorageUnavailable,
    CorruptedProjection,
}
