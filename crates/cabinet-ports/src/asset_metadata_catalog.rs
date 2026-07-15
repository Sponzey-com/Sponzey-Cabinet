use cabinet_domain::asset::{AssetCatalogRecord, AssetId};
use cabinet_domain::workspace::WorkspaceId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetMetadataPutOutcome {
    Created,
    AlreadyPresent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetMetadataPage {
    records: Vec<AssetCatalogRecord>,
    next_cursor: Option<String>,
}
impl AssetMetadataPage {
    pub fn new(records: Vec<AssetCatalogRecord>, next_cursor: Option<String>) -> Self {
        Self {
            records,
            next_cursor,
        }
    }
    pub fn records(&self) -> &[AssetCatalogRecord] {
        &self.records
    }
    pub fn next_cursor(&self) -> Option<&str> {
        self.next_cursor.as_deref()
    }
}

pub trait AssetMetadataCatalog {
    fn put(
        &mut self,
        workspace: &WorkspaceId,
        record: AssetCatalogRecord,
    ) -> Result<AssetMetadataPutOutcome, AssetMetadataCatalogError>;
    fn get(
        &self,
        workspace: &WorkspaceId,
        asset_id: &AssetId,
    ) -> Result<Option<AssetCatalogRecord>, AssetMetadataCatalogError>;
    fn list(
        &self,
        workspace: &WorkspaceId,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<AssetMetadataPage, AssetMetadataCatalogError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetMetadataCatalogError {
    InvalidLimit,
    InvalidCursor,
    Conflict,
    StorageUnavailable,
    CorruptedRecord,
    UnsupportedSchema,
}
impl AssetMetadataCatalogError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimit => "asset_metadata.invalid_limit",
            Self::InvalidCursor => "asset_metadata.invalid_cursor",
            Self::Conflict => "asset_metadata.conflict",
            Self::StorageUnavailable => "asset_metadata.storage_unavailable",
            Self::CorruptedRecord => "asset_metadata.corrupted",
            Self::UnsupportedSchema => "asset_metadata.unsupported_schema",
        }
    }
}
