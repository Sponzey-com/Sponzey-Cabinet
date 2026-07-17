use cabinet_domain::document::DocumentId;
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::committed_version_record_reader::{
    CommittedVersionRecordReadError, CommittedVersionRecordReader,
};
use cabinet_ports::current_document_version::{
    CurrentDocumentVersionPointerError, CurrentDocumentVersionPointerPort,
};

use crate::attachment_diff::{AttachmentDiff, compare_attachment_snapshots};
use crate::document_diff::{DiffComputation, DiffPolicy, DocumentLineDiffService};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompareAuthoritativeDocumentRevisionsInput {
    CurrentToVersion {
        workspace_id: String,
        document_id: String,
        version_id: String,
    },
    Versions {
        workspace_id: String,
        document_id: String,
        left_version_id: String,
        right_version_id: String,
    },
}

impl CompareAuthoritativeDocumentRevisionsInput {
    pub fn current_to_version(workspace_id: &str, document_id: &str, version_id: &str) -> Self {
        Self::CurrentToVersion {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            version_id: version_id.to_string(),
        }
    }

    pub fn versions(
        workspace_id: &str,
        document_id: &str,
        left_version_id: &str,
        right_version_id: &str,
    ) -> Self {
        Self::Versions {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            left_version_id: left_version_id.to_string(),
            right_version_id: right_version_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompareAuthoritativeDocumentRevisionsOutput {
    left_version_id: VersionId,
    right_version_id: VersionId,
    computation: DiffComputation,
    attachment_diff: AttachmentDiff,
}

impl CompareAuthoritativeDocumentRevisionsOutput {
    pub const fn new(
        left_version_id: VersionId,
        right_version_id: VersionId,
        computation: DiffComputation,
        attachment_diff: AttachmentDiff,
    ) -> Self {
        Self {
            left_version_id,
            right_version_id,
            computation,
            attachment_diff,
        }
    }

    pub fn left_version_id(&self) -> &VersionId {
        &self.left_version_id
    }

    pub fn right_version_id(&self) -> &VersionId {
        &self.right_version_id
    }

    pub fn computation(&self) -> &DiffComputation {
        &self.computation
    }

    pub fn attachment_diff(&self) -> &AttachmentDiff {
        &self.attachment_diff
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareAuthoritativeDocumentRevisionsError {
    InvalidInput,
    NotFound,
    StorageUnavailable,
    CorruptedData,
}

impl CompareAuthoritativeDocumentRevisionsError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "authoritative_document_diff.invalid_input",
            Self::NotFound => "authoritative_document_diff.not_found",
            Self::StorageUnavailable => "authoritative_document_diff.storage_unavailable",
            Self::CorruptedData => "authoritative_document_diff.corrupted_data",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompareAuthoritativeDocumentRevisionsUsecase {
    diff: DocumentLineDiffService,
}

impl CompareAuthoritativeDocumentRevisionsUsecase {
    pub fn new() -> Self {
        Self::with_policy(DiffPolicy::default())
    }

    pub const fn with_policy(policy: DiffPolicy) -> Self {
        Self {
            diff: DocumentLineDiffService::with_policy(policy),
        }
    }

    pub fn execute(
        &self,
        input: CompareAuthoritativeDocumentRevisionsInput,
        pointer: &impl CurrentDocumentVersionPointerPort,
        versions: &impl CommittedVersionRecordReader,
    ) -> Result<
        CompareAuthoritativeDocumentRevisionsOutput,
        CompareAuthoritativeDocumentRevisionsError,
    > {
        let (workspace_id, document_id, left_version_id, right_version_id) = match input {
            CompareAuthoritativeDocumentRevisionsInput::CurrentToVersion {
                workspace_id,
                document_id,
                version_id,
            } => {
                let workspace_id = parse_workspace(&workspace_id)?;
                let document_id = parse_document(&document_id)?;
                let current_version_id = pointer
                    .load_current_version(&workspace_id, &document_id)
                    .map_err(map_pointer_error)?
                    .ok_or(CompareAuthoritativeDocumentRevisionsError::NotFound)?;
                let target_version_id = parse_version(&version_id)?;
                (
                    workspace_id,
                    document_id,
                    current_version_id,
                    target_version_id,
                )
            }
            CompareAuthoritativeDocumentRevisionsInput::Versions {
                workspace_id,
                document_id,
                left_version_id,
                right_version_id,
            } => (
                parse_workspace(&workspace_id)?,
                parse_document(&document_id)?,
                parse_version(&left_version_id)?,
                parse_version(&right_version_id)?,
            ),
        };

        let left = read_record(versions, &workspace_id, &document_id, &left_version_id)?;
        let right = read_record(versions, &workspace_id, &document_id, &right_version_id)?;
        let computation = self.diff.compare(
            left.snapshot().body().as_str(),
            right.snapshot().body().as_str(),
        );
        let attachment_diff = compare_attachment_snapshots(
            left.snapshot().attachment_state(),
            right.snapshot().attachment_state(),
        );

        Ok(CompareAuthoritativeDocumentRevisionsOutput::new(
            left_version_id,
            right_version_id,
            computation,
            attachment_diff,
        ))
    }
}

impl Default for CompareAuthoritativeDocumentRevisionsUsecase {
    fn default() -> Self {
        Self::new()
    }
}

fn read_record(
    versions: &impl CommittedVersionRecordReader,
    workspace_id: &WorkspaceId,
    document_id: &DocumentId,
    version_id: &VersionId,
) -> Result<cabinet_ports::version_store::VersionRecord, CompareAuthoritativeDocumentRevisionsError>
{
    let record = versions
        .get_committed_version_record(workspace_id, document_id, version_id)
        .map_err(map_record_error)?
        .ok_or(CompareAuthoritativeDocumentRevisionsError::NotFound)?;
    if record.document_id() != document_id || record.version_id() != version_id {
        return Err(CompareAuthoritativeDocumentRevisionsError::CorruptedData);
    }
    Ok(record)
}

fn parse_workspace(value: &str) -> Result<WorkspaceId, CompareAuthoritativeDocumentRevisionsError> {
    WorkspaceId::new(value).map_err(|_| CompareAuthoritativeDocumentRevisionsError::InvalidInput)
}

fn parse_document(value: &str) -> Result<DocumentId, CompareAuthoritativeDocumentRevisionsError> {
    DocumentId::new(value).map_err(|_| CompareAuthoritativeDocumentRevisionsError::InvalidInput)
}

fn parse_version(value: &str) -> Result<VersionId, CompareAuthoritativeDocumentRevisionsError> {
    VersionId::new(value).map_err(|_| CompareAuthoritativeDocumentRevisionsError::InvalidInput)
}

const fn map_pointer_error(
    error: CurrentDocumentVersionPointerError,
) -> CompareAuthoritativeDocumentRevisionsError {
    match error {
        CurrentDocumentVersionPointerError::StorageUnavailable => {
            CompareAuthoritativeDocumentRevisionsError::StorageUnavailable
        }
        CurrentDocumentVersionPointerError::Conflict
        | CurrentDocumentVersionPointerError::CorruptedPointer => {
            CompareAuthoritativeDocumentRevisionsError::CorruptedData
        }
    }
}

const fn map_record_error(
    error: CommittedVersionRecordReadError,
) -> CompareAuthoritativeDocumentRevisionsError {
    match error {
        CommittedVersionRecordReadError::StorageUnavailable => {
            CompareAuthoritativeDocumentRevisionsError::StorageUnavailable
        }
        CommittedVersionRecordReadError::CorruptedRecord => {
            CompareAuthoritativeDocumentRevisionsError::CorruptedData
        }
    }
}
