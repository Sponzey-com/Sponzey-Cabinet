use cabinet_domain::asset::{AssetId, AssetMetadata};
use cabinet_domain::workspace::WorkspaceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetObject {
    asset_id: AssetId,
    bytes: Vec<u8>,
}

impl AssetObject {
    pub fn new(asset_id: AssetId, bytes: Vec<u8>) -> Result<Self, AssetStoreError> {
        if bytes.is_empty() {
            return Err(AssetStoreError::InvalidAssetObject);
        }
        Ok(Self { asset_id, bytes })
    }

    pub fn asset_id(&self) -> &AssetId {
        &self.asset_id
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetRecord {
    metadata: AssetMetadata,
    object: AssetObject,
}

impl AssetRecord {
    pub fn new(metadata: AssetMetadata, object: AssetObject) -> Result<Self, AssetStoreError> {
        if metadata.id() != object.asset_id() || metadata.byte_size() != object.bytes().len() as u64
        {
            return Err(AssetStoreError::MismatchedAssetObject);
        }
        Ok(Self { metadata, object })
    }

    pub fn metadata(&self) -> &AssetMetadata {
        &self.metadata
    }

    pub fn object(&self) -> &AssetObject {
        &self.object
    }

    pub fn asset_id(&self) -> &AssetId {
        self.metadata.id()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetStorePutOutcome {
    Created,
    AlreadyPresent,
}

pub trait AssetStore {
    fn put_asset(
        &mut self,
        workspace_id: &WorkspaceId,
        record: AssetRecord,
    ) -> Result<AssetStorePutOutcome, AssetStoreError>;

    fn get_metadata(
        &self,
        workspace_id: &WorkspaceId,
        asset_id: &AssetId,
    ) -> Result<Option<AssetMetadata>, AssetStoreError>;

    fn get_object(
        &self,
        workspace_id: &WorkspaceId,
        asset_id: &AssetId,
    ) -> Result<Option<AssetObject>, AssetStoreError>;

    fn remove_asset(
        &mut self,
        workspace_id: &WorkspaceId,
        asset_id: &AssetId,
    ) -> Result<(), AssetStoreError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetStoreError {
    MismatchedAssetObject,
    InvalidAssetObject,
    StorageUnavailable,
    CorruptedMetadata,
    MissingObject,
    Conflict,
}

impl AssetStoreError {
    pub fn code(self) -> &'static str {
        match self {
            Self::MismatchedAssetObject => "asset_store.mismatched_asset_object",
            Self::InvalidAssetObject => "asset_store.invalid_asset_object",
            Self::StorageUnavailable => "asset_store.storage_unavailable",
            Self::CorruptedMetadata => "asset_store.corrupted_metadata",
            Self::MissingObject => "asset_store.missing_object",
            Self::Conflict => "asset_store.conflict",
        }
    }
}
