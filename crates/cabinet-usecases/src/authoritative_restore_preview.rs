use cabinet_domain::version::VersionId;
use cabinet_ports::committed_version_record_reader::CommittedVersionRecordReader;
use cabinet_ports::current_document_version::CurrentDocumentVersionPointerPort;

use crate::attachment_diff::AttachmentDiff;
use crate::authoritative_document_diff::{
    CompareAuthoritativeDocumentRevisionsError, CompareAuthoritativeDocumentRevisionsInput,
    CompareAuthoritativeDocumentRevisionsUsecase,
};
use crate::document_diff::{DiffComputation, DiffPolicy};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewAuthoritativeDocumentRestoreInput {
    workspace_id: String,
    document_id: String,
    target_version_id: String,
}

impl PreviewAuthoritativeDocumentRestoreInput {
    pub fn new(workspace_id: &str, document_id: &str, target_version_id: &str) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            target_version_id: target_version_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewAuthoritativeDocumentRestoreOutput {
    expected_current_version_id: VersionId,
    target_version_id: VersionId,
    computation: DiffComputation,
    attachment_diff: AttachmentDiff,
}

impl PreviewAuthoritativeDocumentRestoreOutput {
    pub fn expected_current_version_id(&self) -> &VersionId {
        &self.expected_current_version_id
    }

    pub fn target_version_id(&self) -> &VersionId {
        &self.target_version_id
    }

    pub fn computation(&self) -> &DiffComputation {
        &self.computation
    }

    pub fn attachment_diff(&self) -> &AttachmentDiff {
        &self.attachment_diff
    }

    pub fn can_restore(&self) -> bool {
        matches!(self.computation, DiffComputation::Complete(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewAuthoritativeDocumentRestoreError {
    InvalidInput,
    NotFound,
    StorageUnavailable,
    CorruptedData,
}

impl PreviewAuthoritativeDocumentRestoreError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "document.restore_preview.invalid_input",
            Self::NotFound => "document.restore_preview.not_found",
            Self::StorageUnavailable => "document.restore_preview.storage_unavailable",
            Self::CorruptedData => "document.restore_preview.corrupted_data",
        }
    }

    pub const fn retryable(self) -> bool {
        matches!(self, Self::StorageUnavailable)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewAuthoritativeDocumentRestoreUsecase {
    compare: CompareAuthoritativeDocumentRevisionsUsecase,
}

impl PreviewAuthoritativeDocumentRestoreUsecase {
    pub fn new() -> Self {
        Self::with_policy(DiffPolicy::default())
    }

    pub const fn with_policy(policy: DiffPolicy) -> Self {
        Self {
            compare: CompareAuthoritativeDocumentRevisionsUsecase::with_policy(policy),
        }
    }

    pub fn execute(
        &self,
        input: PreviewAuthoritativeDocumentRestoreInput,
        pointer: &impl CurrentDocumentVersionPointerPort,
        versions: &impl CommittedVersionRecordReader,
    ) -> Result<PreviewAuthoritativeDocumentRestoreOutput, PreviewAuthoritativeDocumentRestoreError>
    {
        let output = self
            .compare
            .execute(
                CompareAuthoritativeDocumentRevisionsInput::current_to_version(
                    &input.workspace_id,
                    &input.document_id,
                    &input.target_version_id,
                ),
                pointer,
                versions,
            )
            .map_err(map_compare_error)?;
        Ok(PreviewAuthoritativeDocumentRestoreOutput {
            expected_current_version_id: output.left_version_id().clone(),
            target_version_id: output.right_version_id().clone(),
            computation: output.computation().clone(),
            attachment_diff: output.attachment_diff().clone(),
        })
    }
}

impl Default for PreviewAuthoritativeDocumentRestoreUsecase {
    fn default() -> Self {
        Self::new()
    }
}

const fn map_compare_error(
    error: CompareAuthoritativeDocumentRevisionsError,
) -> PreviewAuthoritativeDocumentRestoreError {
    match error {
        CompareAuthoritativeDocumentRevisionsError::InvalidInput => {
            PreviewAuthoritativeDocumentRestoreError::InvalidInput
        }
        CompareAuthoritativeDocumentRevisionsError::NotFound => {
            PreviewAuthoritativeDocumentRestoreError::NotFound
        }
        CompareAuthoritativeDocumentRevisionsError::StorageUnavailable => {
            PreviewAuthoritativeDocumentRestoreError::StorageUnavailable
        }
        CompareAuthoritativeDocumentRevisionsError::CorruptedData => {
            PreviewAuthoritativeDocumentRestoreError::CorruptedData
        }
    }
}
