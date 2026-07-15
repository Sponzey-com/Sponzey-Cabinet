use cabinet_domain::asset_import_operation::{
    AssetImportOperation, AssetImportOperationId, AssetImportState,
};
use cabinet_domain::workspace::WorkspaceId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetImportOperationCreateOutcome {
    Created,
    AlreadyExists,
}

pub trait AssetImportOperationRepository {
    fn create(
        &mut self,
        operation: AssetImportOperation,
    ) -> Result<AssetImportOperationCreateOutcome, AssetImportOperationRepositoryError>;

    fn get(
        &self,
        operation_id: &AssetImportOperationId,
    ) -> Result<Option<AssetImportOperation>, AssetImportOperationRepositoryError>;

    fn replace(
        &mut self,
        operation: AssetImportOperation,
        expected_state: AssetImportState,
    ) -> Result<(), AssetImportOperationRepositoryError>;

    fn list_active(
        &self,
        workspace_id: &WorkspaceId,
        limit: usize,
    ) -> Result<Vec<AssetImportOperation>, AssetImportOperationRepositoryError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetImportOperationRepositoryError {
    InvalidLimit,
    NotFound,
    Conflict,
    StorageUnavailable,
    CorruptedRecord,
    UnsupportedSchema,
}

impl AssetImportOperationRepositoryError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimit => "asset_import_repository.invalid_limit",
            Self::NotFound => "asset_import_repository.not_found",
            Self::Conflict => "asset_import_repository.conflict",
            Self::StorageUnavailable => "asset_import_repository.storage_unavailable",
            Self::CorruptedRecord => "asset_import_repository.corrupted",
            Self::UnsupportedSchema => "asset_import_repository.unsupported_schema",
        }
    }
}
