use cabinet_domain::document::DocumentId;
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::committed_version_record_reader::{
    CommittedVersionRecordReadError, CommittedVersionRecordReader,
};
use cabinet_ports::current_document_version::{
    CurrentDocumentVersionPointerError, CurrentDocumentVersionPointerPort,
};
use cabinet_ports::version_store::VersionRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GetAuthoritativeDocumentRevisionInput {
    Current {
        workspace_id: String,
        document_id: String,
    },
    Version {
        workspace_id: String,
        document_id: String,
        version_id: String,
    },
}

impl GetAuthoritativeDocumentRevisionInput {
    pub fn current(workspace_id: &str, document_id: &str) -> Self {
        Self::Current {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
        }
    }

    pub fn version(workspace_id: &str, document_id: &str, version_id: &str) -> Self {
        Self::Version {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            version_id: version_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetAuthoritativeDocumentRevisionOutput {
    record: VersionRecord,
}

impl GetAuthoritativeDocumentRevisionOutput {
    pub fn record(&self) -> &VersionRecord {
        &self.record
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GetAuthoritativeDocumentRevisionError {
    InvalidInput,
    NotFound,
    StorageUnavailable,
    CorruptedData,
}

impl GetAuthoritativeDocumentRevisionError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "authoritative_document_query.invalid_input",
            Self::NotFound => "authoritative_document_query.not_found",
            Self::StorageUnavailable => "authoritative_document_query.storage_unavailable",
            Self::CorruptedData => "authoritative_document_query.corrupted_data",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GetAuthoritativeDocumentRevisionUsecase;

impl GetAuthoritativeDocumentRevisionUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: GetAuthoritativeDocumentRevisionInput,
        pointer: &impl CurrentDocumentVersionPointerPort,
        versions: &impl CommittedVersionRecordReader,
    ) -> Result<GetAuthoritativeDocumentRevisionOutput, GetAuthoritativeDocumentRevisionError> {
        let (workspace_id, document_id, version_id) = match input {
            GetAuthoritativeDocumentRevisionInput::Current {
                workspace_id,
                document_id,
            } => {
                let workspace_id = parse_workspace(&workspace_id)?;
                let document_id = parse_document(&document_id)?;
                let version_id = pointer
                    .load_current_version(&workspace_id, &document_id)
                    .map_err(map_pointer_error)?
                    .ok_or(GetAuthoritativeDocumentRevisionError::NotFound)?;
                (workspace_id, document_id, version_id)
            }
            GetAuthoritativeDocumentRevisionInput::Version {
                workspace_id,
                document_id,
                version_id,
            } => (
                parse_workspace(&workspace_id)?,
                parse_document(&document_id)?,
                VersionId::new(&version_id)
                    .map_err(|_| GetAuthoritativeDocumentRevisionError::InvalidInput)?,
            ),
        };
        let record = versions
            .get_committed_version_record(&workspace_id, &document_id, &version_id)
            .map_err(map_version_error)?
            .ok_or(GetAuthoritativeDocumentRevisionError::NotFound)?;
        Ok(GetAuthoritativeDocumentRevisionOutput { record })
    }
}

fn parse_workspace(value: &str) -> Result<WorkspaceId, GetAuthoritativeDocumentRevisionError> {
    WorkspaceId::new(value).map_err(|_| GetAuthoritativeDocumentRevisionError::InvalidInput)
}

fn parse_document(value: &str) -> Result<DocumentId, GetAuthoritativeDocumentRevisionError> {
    DocumentId::new(value).map_err(|_| GetAuthoritativeDocumentRevisionError::InvalidInput)
}

const fn map_pointer_error(
    error: CurrentDocumentVersionPointerError,
) -> GetAuthoritativeDocumentRevisionError {
    match error {
        CurrentDocumentVersionPointerError::StorageUnavailable => {
            GetAuthoritativeDocumentRevisionError::StorageUnavailable
        }
        CurrentDocumentVersionPointerError::Conflict
        | CurrentDocumentVersionPointerError::CorruptedPointer => {
            GetAuthoritativeDocumentRevisionError::CorruptedData
        }
    }
}

const fn map_version_error(
    error: CommittedVersionRecordReadError,
) -> GetAuthoritativeDocumentRevisionError {
    match error {
        CommittedVersionRecordReadError::StorageUnavailable => {
            GetAuthoritativeDocumentRevisionError::StorageUnavailable
        }
        CommittedVersionRecordReadError::CorruptedRecord => {
            GetAuthoritativeDocumentRevisionError::CorruptedData
        }
    }
}
