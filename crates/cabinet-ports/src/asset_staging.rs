use cabinet_domain::asset_import_operation::AssetImportOperationId;
use cabinet_domain::workspace::WorkspaceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedAsset {
    operation_id: AssetImportOperationId,
    byte_size: u64,
}
impl StagedAsset {
    pub fn new(
        operation_id: AssetImportOperationId,
        actual: u64,
        expected: u64,
    ) -> Result<Self, AssetStagingError> {
        if actual == 0 || actual != expected {
            return Err(AssetStagingError::SizeMismatch);
        }
        Ok(Self {
            operation_id,
            byte_size: actual,
        })
    }
    pub fn operation_id(&self) -> &AssetImportOperationId {
        &self.operation_id
    }
    pub fn byte_size(&self) -> u64 {
        self.byte_size
    }
}

pub trait AssetStagingWriter {
    fn begin(
        &mut self,
        workspace: &WorkspaceId,
        operation: &AssetImportOperationId,
    ) -> Result<(), AssetStagingError>;
    fn append(
        &mut self,
        workspace: &WorkspaceId,
        operation: &AssetImportOperationId,
        offset: u64,
        bytes: &[u8],
    ) -> Result<(), AssetStagingError>;
    fn finalize(
        &mut self,
        workspace: &WorkspaceId,
        operation: &AssetImportOperationId,
        expected_size: u64,
    ) -> Result<StagedAsset, AssetStagingError>;
    fn cleanup(
        &mut self,
        workspace: &WorkspaceId,
        operation: &AssetImportOperationId,
    ) -> Result<(), AssetStagingError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetStagingError {
    InvalidInput,
    NotFound,
    OffsetConflict,
    SizeMismatch,
    StorageUnavailable,
}
impl AssetStagingError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "asset_staging.invalid_input",
            Self::NotFound => "asset_staging.not_found",
            Self::OffsetConflict => "asset_staging.offset_conflict",
            Self::SizeMismatch => "asset_staging.size_mismatch",
            Self::StorageUnavailable => "asset_staging.storage_unavailable",
        }
    }
}
