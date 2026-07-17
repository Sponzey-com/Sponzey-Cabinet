use cabinet_domain::document::{DocumentMetadata, DocumentPath, DocumentTitle};
use cabinet_domain::version::{CurrentDocumentSnapshot, DocumentRevisionNumber, VersionId};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_revision_projection::{
    CurrentDocumentRevisionProjection, CurrentDocumentRevisionProjectionError,
    CurrentDocumentRevisionProjectionOutcome, CurrentDocumentRevisionProjectionWriter,
};
use cabinet_ports::document_repository::CurrentDocumentRecord;
use cabinet_ports::version_store::VersionRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectCurrentDocumentRevisionInput {
    workspace_id: String,
    path: String,
    record: VersionRecord,
}

impl ProjectCurrentDocumentRevisionInput {
    pub fn new(workspace_id: &str, path: &str, record: VersionRecord) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            path: path.to_string(),
            record,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectCurrentDocumentRevisionOutput {
    outcome: CurrentDocumentRevisionProjectionOutcome,
    version_id: VersionId,
    revision_number: DocumentRevisionNumber,
}

impl ProjectCurrentDocumentRevisionOutput {
    pub const fn outcome(&self) -> CurrentDocumentRevisionProjectionOutcome {
        self.outcome
    }

    pub fn version_id(&self) -> &VersionId {
        &self.version_id
    }

    pub const fn revision_number(&self) -> DocumentRevisionNumber {
        self.revision_number
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectCurrentDocumentRevisionError {
    InvalidInput,
    StaleRevision,
    RevisionConflict,
    StorageUnavailable,
    CorruptedProjection,
}

impl ProjectCurrentDocumentRevisionError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "current_document_projection.invalid_input",
            Self::StaleRevision => "current_document_projection.stale_revision",
            Self::RevisionConflict => "current_document_projection.revision_conflict",
            Self::StorageUnavailable => "current_document_projection.storage_unavailable",
            Self::CorruptedProjection => "current_document_projection.corrupted_projection",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProjectCurrentDocumentRevisionUsecase;

impl ProjectCurrentDocumentRevisionUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: ProjectCurrentDocumentRevisionInput,
        writer: &mut impl CurrentDocumentRevisionProjectionWriter,
    ) -> Result<ProjectCurrentDocumentRevisionOutput, ProjectCurrentDocumentRevisionError> {
        let workspace_id = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| ProjectCurrentDocumentRevisionError::InvalidInput)?;
        let path = DocumentPath::new(&input.path)
            .map_err(|_| ProjectCurrentDocumentRevisionError::InvalidInput)?;
        let revision_number = input
            .record
            .entry()
            .revision_number()
            .ok_or(ProjectCurrentDocumentRevisionError::InvalidInput)?;
        let version_id = input.record.version_id().clone();
        let document_id = input.record.document_id().clone();
        let body = input.record.snapshot().body().clone();
        let title = DocumentTitle::from_markdown_body(&body);
        let metadata = DocumentMetadata::new(document_id.clone(), title, path)
            .map_err(|_| ProjectCurrentDocumentRevisionError::InvalidInput)?;
        let current_snapshot = CurrentDocumentSnapshot::new(document_id, body);
        let current_record = CurrentDocumentRecord::new(metadata, current_snapshot)
            .map_err(|_| ProjectCurrentDocumentRevisionError::InvalidInput)?;
        let projection = CurrentDocumentRevisionProjection::new(
            current_record,
            version_id.clone(),
            revision_number,
        );
        let outcome = writer
            .write_current_projection(&workspace_id, projection)
            .map_err(map_projection_error)?;

        Ok(ProjectCurrentDocumentRevisionOutput {
            outcome,
            version_id,
            revision_number,
        })
    }
}

const fn map_projection_error(
    error: CurrentDocumentRevisionProjectionError,
) -> ProjectCurrentDocumentRevisionError {
    match error {
        CurrentDocumentRevisionProjectionError::StaleRevision => {
            ProjectCurrentDocumentRevisionError::StaleRevision
        }
        CurrentDocumentRevisionProjectionError::RevisionConflict => {
            ProjectCurrentDocumentRevisionError::RevisionConflict
        }
        CurrentDocumentRevisionProjectionError::StorageUnavailable => {
            ProjectCurrentDocumentRevisionError::StorageUnavailable
        }
        CurrentDocumentRevisionProjectionError::CorruptedProjection => {
            ProjectCurrentDocumentRevisionError::CorruptedProjection
        }
    }
}
