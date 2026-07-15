use cabinet_domain::asset::AssetId;
use cabinet_domain::asset_import_operation::AssetImportOperationId;
use cabinet_domain::workspace::WorkspaceId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetObjectPublishOutcome {
    Created,
    AlreadyPresent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedAssetObject {
    asset_id: AssetId,
    byte_size: u64,
    outcome: AssetObjectPublishOutcome,
}
impl PublishedAssetObject {
    pub fn new(
        asset_id: AssetId,
        byte_size: u64,
        outcome: AssetObjectPublishOutcome,
    ) -> Result<Self, AssetObjectPublishError> {
        if byte_size == 0 {
            return Err(AssetObjectPublishError::SizeMismatch);
        }
        Ok(Self {
            asset_id,
            byte_size,
            outcome,
        })
    }
    pub fn asset_id(&self) -> &AssetId {
        &self.asset_id
    }
    pub fn byte_size(&self) -> u64 {
        self.byte_size
    }
    pub fn outcome(&self) -> AssetObjectPublishOutcome {
        self.outcome
    }
}

pub trait AssetObjectPublisher {
    fn publish(
        &mut self,
        workspace: &WorkspaceId,
        operation: &AssetImportOperationId,
        expected_size: u64,
    ) -> Result<PublishedAssetObject, AssetObjectPublishError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetObjectPublishError {
    InvalidConfig,
    StagingNotFound,
    SizeMismatch,
    InvalidHash,
    Conflict,
    StorageUnavailable,
}
impl AssetObjectPublishError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidConfig => "asset_publish.invalid_config",
            Self::StagingNotFound => "asset_publish.staging_not_found",
            Self::SizeMismatch => "asset_publish.size_mismatch",
            Self::InvalidHash => "asset_publish.invalid_hash",
            Self::Conflict => "asset_publish.conflict",
            Self::StorageUnavailable => "asset_publish.storage_unavailable",
        }
    }
}
