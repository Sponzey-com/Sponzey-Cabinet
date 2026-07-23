use cabinet_domain::asset::{AssetFileName, AssetId, AssetMediaType};
use cabinet_domain::workspace::WorkspaceId;

pub const MAX_ASSET_SEARCH_LIMIT: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetSearchQuery {
    text: String,
    limit: usize,
}

impl AssetSearchQuery {
    pub fn new(text: &str, limit: usize) -> Result<Self, AssetSearchError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(AssetSearchError::InvalidQuery);
        }
        if limit == 0 || limit > MAX_ASSET_SEARCH_LIMIT {
            return Err(AssetSearchError::InvalidLimit);
        }
        Ok(Self {
            text: text.to_string(),
            limit,
        })
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn limit(&self) -> usize {
        self.limit
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetSearchResult {
    asset_id: AssetId,
    file_name: AssetFileName,
    media_type: AssetMediaType,
    byte_size: u64,
    score: u32,
}

impl AssetSearchResult {
    pub fn new(
        asset_id: AssetId,
        file_name: AssetFileName,
        media_type: AssetMediaType,
        byte_size: u64,
        score: u32,
    ) -> Result<Self, AssetSearchError> {
        if byte_size == 0 {
            return Err(AssetSearchError::InvalidAssetMetadata);
        }
        Ok(Self {
            asset_id,
            file_name,
            media_type,
            byte_size,
            score,
        })
    }

    pub fn asset_id(&self) -> &AssetId {
        &self.asset_id
    }

    pub fn file_name(&self) -> &AssetFileName {
        &self.file_name
    }

    pub fn media_type(&self) -> &AssetMediaType {
        &self.media_type
    }

    pub fn byte_size(&self) -> u64 {
        self.byte_size
    }

    pub fn score(&self) -> u32 {
        self.score
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetSearchPage {
    results: Vec<AssetSearchResult>,
}

impl AssetSearchPage {
    pub fn new(results: Vec<AssetSearchResult>) -> Self {
        Self { results }
    }

    pub fn results(&self) -> &[AssetSearchResult] {
        &self.results
    }
}

pub trait AssetSearchIndex {
    fn search_assets(
        &self,
        workspace_id: &WorkspaceId,
        query: AssetSearchQuery,
    ) -> Result<AssetSearchPage, AssetSearchError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetSearchError {
    InvalidQuery,
    InvalidLimit,
    InvalidAssetMetadata,
    StorageUnavailable,
    CorruptedIndex,
}

impl AssetSearchError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidQuery => "asset_search_index.invalid_query",
            Self::InvalidLimit => "asset_search_index.invalid_limit",
            Self::InvalidAssetMetadata => "asset_search_index.invalid_asset_metadata",
            Self::StorageUnavailable => "asset_search_index.storage_unavailable",
            Self::CorruptedIndex => "asset_search_index.corrupted_index",
        }
    }
}
