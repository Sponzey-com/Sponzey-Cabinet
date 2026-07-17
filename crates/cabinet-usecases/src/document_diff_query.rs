use crate::document_diff::{DiffComputation, DocumentLineDiffService};
use cabinet_domain::document_diff_query::{DocumentDiffQueryKind, DocumentDiffQueryTarget};
use cabinet_ports::document_repository::{DocumentRepository, DocumentRepositoryError};
use cabinet_ports::version_store::{VersionStore, VersionStoreError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecuteDocumentDiffQueryUsecase {
    diff_service: DocumentLineDiffService,
}

impl ExecuteDocumentDiffQueryUsecase {
    pub fn new() -> Self {
        Self::with_diff_service(DocumentLineDiffService::default())
    }

    pub const fn with_diff_service(diff_service: DocumentLineDiffService) -> Self {
        Self { diff_service }
    }

    pub fn execute(
        &self,
        target: &DocumentDiffQueryTarget,
        document_repository: &impl DocumentRepository,
        version_store: &impl VersionStore,
    ) -> Result<DiffComputation, ExecuteDocumentDiffQueryError> {
        let (left_body, right_body) = match target.kind() {
            DocumentDiffQueryKind::CurrentToVersion { version_id } => {
                let current = document_repository
                    .get_current_by_id(target.workspace_id(), target.document_id())
                    .map_err(map_document_repository_error)?
                    .ok_or(ExecuteDocumentDiffQueryError::NotFound)?;
                let version = version_store
                    .get_version_snapshot(target.workspace_id(), target.document_id(), version_id)
                    .map_err(map_version_store_error)?
                    .ok_or(ExecuteDocumentDiffQueryError::NotFound)?;
                (
                    current.body().as_str().to_string(),
                    version.body().as_str().to_string(),
                )
            }
            DocumentDiffQueryKind::Versions {
                left_version_id,
                right_version_id,
            } => {
                let left = version_store
                    .get_version_snapshot(
                        target.workspace_id(),
                        target.document_id(),
                        left_version_id,
                    )
                    .map_err(map_version_store_error)?
                    .ok_or(ExecuteDocumentDiffQueryError::NotFound)?;
                let right = version_store
                    .get_version_snapshot(
                        target.workspace_id(),
                        target.document_id(),
                        right_version_id,
                    )
                    .map_err(map_version_store_error)?
                    .ok_or(ExecuteDocumentDiffQueryError::NotFound)?;
                (
                    left.body().as_str().to_string(),
                    right.body().as_str().to_string(),
                )
            }
        };

        Ok(self.diff_service.compare(&left_body, &right_body))
    }
}

impl Default for ExecuteDocumentDiffQueryUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecuteDocumentDiffQueryError {
    NotFound,
    StorageUnavailable,
}

impl ExecuteDocumentDiffQueryError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::NotFound => "document.diff_target_not_found",
            Self::StorageUnavailable => "document.storage_unavailable",
        }
    }
}

fn map_document_repository_error(_error: DocumentRepositoryError) -> ExecuteDocumentDiffQueryError {
    ExecuteDocumentDiffQueryError::StorageUnavailable
}

fn map_version_store_error(_error: VersionStoreError) -> ExecuteDocumentDiffQueryError {
    ExecuteDocumentDiffQueryError::StorageUnavailable
}
